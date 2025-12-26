use std::collections::HashSet;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;

use superhero_universe::components::persona::{
    hero_persona_stack, Alignment, PersonaStack, PersonaType,
};
use superhero_universe::components::world::Position;
use superhero_universe::content::{ExpressionId, PowerId, PowerRepository, SqlitePowerRepository};
use superhero_universe::core::world::ActionIntent;
use superhero_universe::data::civilian_events::{load_civilian_event_catalog, CivilianStorylet};
use superhero_universe::data::global_events::{load_global_event_catalog, GlobalEventDefinition};
use superhero_universe::data::endgame_events::{load_endgame_event_catalog, EndgameEvent};
use superhero_universe::data::nemesis::load_nemesis_action_catalog;
use superhero_universe::data::storylets::{load_storylet_catalog, Storylet};
use superhero_universe::rules::{
    can_use, use_power, ActorState, CostType, PressureModifiers, TargetContext, UseContext,
    WorldState,
};
use superhero_universe::simulation::agents::{
    tick_agents, AgentEvent, AgentEventLog, AgentRegistry,
};
use superhero_universe::simulation::case::{CaseEventLog, CaseRegistry};
use superhero_universe::simulation::city::{CityEventLog, CityState, LocationTag};
use superhero_universe::simulation::civilian::{
    apply_civilian_effects, tick_civilian_life, CivilianState,
};
use superhero_universe::simulation::combat::{
    CombatConsequences, CombatEnd, CombatIntent, CombatPressureDelta, CombatScale, CombatState,
};
use superhero_universe::simulation::endgame::{
    apply_transformation_event, evaluate_transformation, EndgameState, TransformationState,
};
use superhero_universe::simulation::evidence::WorldEvidence;
use superhero_universe::simulation::growth::{
    record_expression_use, select_evolution_candidate, GrowthState,
};
use superhero_universe::simulation::identity_evidence::{
    combat_consequence_modifiers, IdentityEvidenceModifiers, IdentityEvidenceStore, PersonaHint,
};
use superhero_universe::simulation::origin::{
    current_origin_stage, load_origin_catalog, load_origin_path_catalog, register_origin_event,
    select_origin_paths, start_origin_path, tick_origin_path, OriginPathCatalog,
    OriginPathDefinition, OriginQuestState, OriginStageReward,
};
use superhero_universe::simulation::pressure::PressureState;
use superhero_universe::simulation::region::{GlobalEventLog, RegionEventLog, RegionState};
use superhero_universe::simulation::storylet_state::StoryletState;
use superhero_universe::simulation::storylets::{
    is_punctuation_storylet, storylet_has_gate_requirements, StoryletLibrary,
};
use superhero_universe::simulation::time::GameTime;
use superhero_universe::systems::case::update_cases;
use superhero_universe::systems::civilian::apply_civilian_pressure;
use superhero_universe::systems::combat_loop::{
    combat_post_consequences, combat_tick, force_escalate, force_escape, resolve_combat,
    start_combat,
};
use superhero_universe::systems::event_resolver::{
    resolve_faction_events, ResolvedFactionEventLog,
};
use superhero_universe::systems::faction::{
    run_faction_director, FactionDirector, FactionEventLog,
};
use superhero_universe::systems::heat::{
    apply_combat_consequence_heat, apply_signatures, decay_heat, WorldEventLog,
};
use superhero_universe::systems::persona::{attempt_switch, PersonaSwitchError};
use superhero_universe::systems::pressure::update_pressure;
use superhero_universe::systems::region::{
    run_global_faction_director, run_region_update, GlobalFactionDirector, GlobalFactionEventLog,
};
use superhero_universe::systems::suspicion::apply_suspicion_for_intents;
use superhero_universe::systems::units::update_units;
use superhero_universe::ui::authoring::render_authoring_dashboard;
use superhero_universe::world::{WorldDb, WorldDbState, WorldRepository};

const DEFAULT_PUNCTUATION_TURNS: i32 = 2;
const DEFAULT_PUNCTUATION_COOLDOWN_TURNS: i32 = 3;

fn main() {
    println!("Initializing Superhero Universe (Rules Debug)...");
    let (content_db_path, world_db_path) = parse_paths(env::args().collect());
    if !content_db_path.exists() {
        eprintln!(
            "DB not found at {}. Use --db <path> to point at a valid SQLite file.",
            content_db_path.display()
        );
        std::process::exit(1);
    }

    let repo = match SqlitePowerRepository::open(&content_db_path) {
        Ok(repo) => repo,
        Err(err) => {
            eprintln!("Failed to open DB: {}", err);
            std::process::exit(1);
        }
    };

    let mut world_repo: Box<dyn WorldRepository> = match WorldDb::open(&world_db_path) {
        Ok(db) => Box::new(db),
        Err(err) => {
            eprintln!("Failed to open world DB: {}", err);
            std::process::exit(1);
        }
    };
    let world_state = match world_repo.load_or_init() {
        Ok(state) => state,
        Err(err) => {
            eprintln!("Failed to load world state: {}", err);
            WorldDbState::default()
        }
    };

    print_stats(&repo);

    let mut actor = ActorState {
        stamina: 10,
        focus: 10,
        ..Default::default()
    };
    let mut evidence = WorldEvidence::default();
    let mut identity_evidence = IdentityEvidenceStore::default();
    let mut world = WorldState {
        turn: world_state.world_turn,
        pressure: PressureModifiers::default(),
    };
    let mut game_time = world_state.game_time;
    let mut city = world_state.city;
    let mut city_events = CityEventLog::default();
    let mut event_log = WorldEventLog::default();
    let mut region = RegionState::default();
    let mut region_events = RegionEventLog::default();
    let mut persona_stack = hero_persona_stack();
    let alignment = Alignment::Hero;
    let mut storylet_state = world_state.storylet_state;
    let mut growth = world_state.growth;
    let storylets = load_storylet_library();
    let global_events = match load_global_event_catalog("./assets/data/global_events.json") {
        Ok(catalog) => catalog.events,
        Err(err) => {
            eprintln!("Failed to load global events: {}", err);
            Vec::new()
        }
    };
    let origin_paths = match load_origin_path_catalog("./assets/data/origin_paths.json") {
        Ok(catalog) => catalog,
        Err(err) => {
            eprintln!("Failed to load origin paths: {}", err);
            OriginPathCatalog::default()
        }
    };
    let mut origin_quest = OriginQuestState::default();
    let mut player_pos = Position { x: 0, y: 0 };
    let mut pressure = PressureState::default();
    let mut faction_director = match FactionDirector::load_default() {
        Ok(director) => director,
        Err(err) => {
            eprintln!("Failed to load faction data: {}", err);
            FactionDirector::default()
        }
    };
    let mut faction_events = FactionEventLog::default();
    let mut resolved_faction_events = ResolvedFactionEventLog::default();
    let mut global_faction_director = GlobalFactionDirector::load_default();
    let mut global_faction_events = GlobalFactionEventLog::default();
    let mut global_event_log = GlobalEventLog::default();
    let mut cases = world_state.cases;
    let mut case_log = CaseEventLog::default();
    let mut agents = match AgentRegistry::load_default() {
        Ok(registry) => registry,
        Err(err) => {
            eprintln!("Failed to load agent data: {}", err);
            AgentRegistry::default()
        }
    };
    let mut agent_events = AgentEventLog::default();
    let mut combat = world_state.combat;
    let mut endgame_state = EndgameState::default();
    let mut target = TargetContext {
        distance_m: Some(10),
        has_line_of_sight: true,
        has_contact: false,
        in_public: true,
        witnesses: 0,
    };
    let mut civilian_state = CivilianState::default();
    update_pressure(&mut pressure, &city, &evidence, &cases, &game_time);
    apply_civilian_pressure(&civilian_state, &mut pressure);
    apply_pressure_modifiers(&mut world, &pressure, &endgame_state);
    run_region_update(
        &mut region,
        &city,
        &pressure,
        &mut city_events,
        &mut region_events,
    );
    run_global_faction_director(
        &mut global_faction_director,
        &region,
        &mut global_faction_events,
    );

    let civilian_events = load_civilian_event_library();
    let endgame_events = load_endgame_event_library();

    println!("Commands: stats | power <id> | use <expression_id> | ctx | loc | persona | personas | growth [expr|unlock|mastery] | switch <persona_id> | storylets [all] | punctuation <on|off|turns> | author | civilian [events|resolve <event_id> <choice_id>] | origin [paths|choose|status|event|tick] | set <field> <value> | cd | scene | events | cases | combat <start|use|intent|tick|log|resolve|force_escape|force_escalate> | tick [n] | quit");
    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }
        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("").to_lowercase();

        match cmd.as_str() {
            "quit" | "exit" => break,
            "help" => {
                println!("Commands: stats | power <id> | use <expression_id> | ctx | loc | persona | personas | growth [expr|unlock|mastery] | switch <persona_id> | storylets [all] | punctuation <on|off|turns> | author | civilian [events|resolve <event_id> <choice_id>] | origin [paths|choose|status|event|tick] | set <field> <value> | cd | scene | events | cases | combat <start|use|intent|tick|log|resolve|force_escape|force_escalate> | tick [n] | quit");
            }
            "stats" => {
                print_stats(&repo);
            }
            "power" | "list" => {
                if let Some(id_raw) = parts.next() {
                    match id_raw.parse::<i64>() {
                        Ok(power_id) => {
                            print_power(&repo, power_id);
                        }
                        Err(_) => println!("Invalid power_id: {}", id_raw),
                    }
                } else {
                    println!("Usage: power <power_id>");
                }
            }
            "ctx" => {
                print_context(&target, &world, &city, &pressure, &endgame_state);
            }
            "loc" => {
                print_location(&city);
            }
            "persona" => {
                print_persona_state(&persona_stack, alignment, &city, &cases);
            }
            "personas" => {
                print_persona_stack(&persona_stack, world.turn);
            }
            "growth" => {
                let sub = parts.next();
                match sub {
                    None => print_growth_state(&growth),
                    Some("expr") => {
                        if let Some(expr_id) = parts.next() {
                            print_growth_expr(&growth, expr_id);
                        } else {
                            println!("Usage: growth expr <expression_id>");
                        }
                    }
                    Some("unlock") => {
                        if let Some(expr_id) = parts.next() {
                            let expr_id = ExpressionId(expr_id.to_string());
                            growth.unlocked_expressions.insert(expr_id.clone());
                            println!("Unlocked expression {}", expr_id.0);
                        } else {
                            println!("Usage: growth unlock <expression_id>");
                        }
                    }
                    Some("mastery") => {
                        if let (Some(expr_id), Some(uses_raw)) = (parts.next(), parts.next()) {
                            if let Ok(uses) = uses_raw.parse::<u32>() {
                                seed_mastery(&mut growth, expr_id, uses);
                            } else {
                                println!("Usage: growth mastery <expression_id> <uses>");
                            }
                        } else {
                            println!("Usage: growth mastery <expression_id> <uses>");
                        }
                    }
                    Some(_) => {
                        println!("Usage: growth [expr|unlock|mastery]");
                    }
                }
            }
            "set" => {
                let field = parts.next().unwrap_or("");
                let value = parts.next().unwrap_or("");
                if update_context(&mut target, field, value).is_err() {
                    println!("Usage: set dist <m> | set los <on|off> | set contact <on|off> | set public <on|off> | set witnesses <n>");
                }
            }
            "switch" => {
                if let Some(persona_id) = parts.next() {
                    let intents = vec![ActionIntent::SwitchPersona {
                        entity_id: 1,
                        persona_id: persona_id.to_string(),
                    }];
                    handle_switch(
                        persona_id,
                        &mut persona_stack,
                        world.turn,
                        &city,
                        &evidence,
                        &target,
                    );
                    apply_suspicion_for_intents(
                        &mut persona_stack,
                        alignment,
                        &player_pos,
                        &city,
                        &cases,
                        &identity_evidence,
                        &intents,
                        1,
                    );
                    print_persona_state(&persona_stack, alignment, &city, &cases);
                } else {
                    println!("Usage: switch <persona_id>");
                }
            }
            "use" => {
                if let Some(expr_raw) = parts.next() {
                    let expr_id = ExpressionId(expr_raw.to_string());
                    match repo.expression(&expr_id) {
                        Ok(expr) => {
                            let mut ctx = UseContext {
                                actor: &mut actor,
                                world: &world,
                                mastery: growth.mastery.get(&expr.id).map(|entry| entry.stage),
                                unlocked: Some(&growth.unlocked_expressions),
                            };
                            match can_use(&ctx, &expr, &target) {
                                Ok(_) => match use_power(&mut ctx, &expr, &target) {
                                    Ok(result) => {
                                        print_use_result(&result);
                                        let location_id = city.active_location;
                                        apply_action_signatures(
                                            &result.emitted_signatures,
                                            location_id,
                                            world.turn,
                                            target.witnesses,
                                            target.in_public,
                                            PersonaHint::Unknown,
                                            None,
                                            &mut city,
                                            &mut city_events,
                                            &mut evidence,
                                            &mut identity_evidence,
                                            &mut faction_director,
                                            &mut faction_events,
                                            &mut resolved_faction_events,
                                            &mut cases,
                                            &mut case_log,
                                            &mut persona_stack,
                                            alignment,
                                            &player_pos,
                                            &mut event_log,
                                        );
                                        update_pressure(
                                            &mut pressure,
                                            &city,
                                            &evidence,
                                            &cases,
                                            &game_time,
                                        );
                                        apply_civilian_pressure(&civilian_state, &mut pressure);
                                        apply_pressure_modifiers(
                                            &mut world,
                                            &pressure,
                                            &endgame_state,
                                        );
                                        run_region_update(
                                            &mut region,
                                            &city,
                                            &pressure,
                                            &mut city_events,
                                            &mut region_events,
                                        );
                                        run_global_faction_director(
                                            &mut global_faction_director,
                                            &region,
                                            &mut global_faction_events,
                                        );
                                        handle_endgame_transition(
                                            &cases,
                                            &pressure,
                                            &resolved_faction_events,
                                            &mut world,
                                            &mut storylet_state,
                                            &mut endgame_state,
                                        );
                                        persist_world_state(
                                            &mut *world_repo,
                                            &world,
                                            &game_time,
                                            &city,
                                            &cases,
                                            &combat,
                                            &growth,
                                            &storylet_state,
                                        );
                                        print_event_log(&mut event_log);
                                        println!(
                                            "Actor: stamina={}, focus={}, cooldowns={}",
                                            actor.stamina,
                                            actor.focus,
                                            actor.cooldowns.len()
                                        );
                                        apply_growth_on_use(
                                            &mut growth,
                                            &expr,
                                            &repo,
                                            &mut storylet_state,
                                        );
                                    }
                                    Err(err) => println!("use_power failed: {:?}", err),
                                },
                                Err(err) => print_use_error(&err, &expr, &target, &actor),
                            }
                        }
                        Err(err) => println!("Expression not found: {}", err),
                    }
                } else {
                    println!("Usage: use <expression_id>");
                }
            }
            "cd" => {
                print_cooldowns(&actor);
            }
            "scene" => {
                print_scene(&evidence);
            }
            "storylets" => {
                let all = matches!(parts.next(), Some("all"));
                if all {
                    list_storylets_all(&storylets, alignment);
                } else {
                    list_storylets_available(
                        &storylets,
                        alignment,
                        &persona_stack,
                        &storylet_state,
                        &endgame_state,
                        &city,
                        &evidence,
                        &cases,
                        &pressure,
                        &game_time,
                    );
                }
            }
            "punctuation" => {
                let action = parts.next();
                match action {
                    None => {
                        if storylet_state.punctuation.only {
                            println!(
                                "Punctuation layer: ON ({} turns remaining)",
                                storylet_state.punctuation.remaining_turns
                            );
                        } else {
                            println!("Punctuation layer: OFF");
                        }
                    }
                    Some("off") => {
                        storylet_state.punctuation.clear();
                        println!("Punctuation layer disabled.");
                    }
                    Some("on") => {
                        let turns = parts
                            .next()
                            .and_then(|raw| raw.parse::<i32>().ok())
                            .unwrap_or(DEFAULT_PUNCTUATION_TURNS);
                        storylet_state.punctuation.activate(turns);
                        println!(
                            "Punctuation layer enabled for {} turns.",
                            storylet_state.punctuation.remaining_turns
                        );
                    }
                    Some(raw) => {
                        if let Ok(turns) = raw.parse::<i32>() {
                            storylet_state.punctuation.activate(turns);
                            println!(
                                "Punctuation layer enabled for {} turns.",
                                storylet_state.punctuation.remaining_turns
                            );
                        } else {
                            println!("Usage: punctuation <on|off|turns>");
                        }
                    }
                }
            }
            "author" => {
                let origin_catalog = match load_origin_catalog("./assets/data/origins.json") {
                    Ok(catalog) => catalog,
                    Err(err) => {
                        println!("Failed to load origins: {}", err);
                        continue;
                    }
                };
                let nemesis_catalog =
                    match load_nemesis_action_catalog("./assets/data/nemesis_actions.json") {
                        Ok(catalog) => catalog,
                        Err(err) => {
                            println!("Failed to load nemesis actions: {}", err);
                            continue;
                        }
                    };
                let panel = render_authoring_dashboard(
                    &storylets,
                    &civilian_events,
                    &endgame_events,
                    &endgame_state,
                    &origin_catalog,
                    &origin_paths,
                    &nemesis_catalog,
                );
                println!("{}", panel);
            }
            "civilian" => {
                let sub = parts.next();
                match sub {
                    None => {
                        print_civilian_status(&civilian_state, &game_time);
                        print_pending_civilian_events(
                            &civilian_state,
                            &civilian_events,
                            &game_time,
                        );
                    }
                    Some("events") => {
                        print_pending_civilian_events(
                            &civilian_state,
                            &civilian_events,
                            &game_time,
                        );
                    }
                    Some("resolve") => {
                        let Some(event_id) = parts.next() else {
                            println!("Usage: civilian resolve <event_id> <choice_id>");
                            continue;
                        };
                        let Some(choice_id) = parts.next() else {
                            println!("Usage: civilian resolve <event_id> <choice_id>");
                            continue;
                        };
                        resolve_civilian_event(
                            &mut civilian_state,
                            &civilian_events,
                            event_id,
                            choice_id,
                        );
                        apply_civilian_pressure(&civilian_state, &mut pressure);
                    }
                    Some(_) => {
                        println!("Usage: civilian [events|resolve <event_id> <choice_id>]");
                    }
                }
            }
            "events" => {
                print_faction_events(&mut resolved_faction_events);
            }
            "cases" => {
                print_cases(&cases);
                print_case_log(&mut case_log);
            }
            "origin" => {
                let sub = parts.next().unwrap_or("").to_lowercase();
                match sub.as_str() {
                    "paths" => {
                        let seed = parts
                            .next()
                            .and_then(|raw| raw.parse::<u64>().ok())
                            .unwrap_or(world.turn);
                        let count = parts
                            .next()
                            .and_then(|raw| raw.parse::<usize>().ok())
                            .unwrap_or(3);
                        let options = select_origin_paths(&origin_paths, None, seed, count);
                        print_origin_paths(&options, seed);
                    }
                    "choose" => {
                        let Some(path_id) = parts.next() else {
                            println!("Usage: origin choose <path_id>");
                            continue;
                        };
                        match start_origin_path(&mut origin_quest, &origin_paths, path_id) {
                            Ok(path) => {
                                println!("Origin path selected: {} - {}", path.label, path.summary);
                                print_origin_path_status(&origin_quest, &origin_paths);
                            }
                            Err(err) => println!("Failed to start origin path: {}", err),
                        }
                    }
                    "status" => {
                        print_origin_path_status(&origin_quest, &origin_paths);
                    }
                    "event" => {
                        let Some(event_tag) = parts.next() else {
                            println!("Usage: origin event <tag>");
                            continue;
                        };
                        let rewards =
                            register_origin_event(&mut origin_quest, &origin_paths, event_tag);
                        apply_origin_rewards(&rewards, &mut pressure);
                        apply_pressure_modifiers(&mut world, &pressure, &endgame_state);
                        if !rewards.is_empty() {
                            print_origin_path_status(&origin_quest, &origin_paths);
                        }
                    }
                    "tick" => {
                        let count = parts
                            .next()
                            .and_then(|raw| raw.parse::<u32>().ok())
                            .unwrap_or(1);
                        for _ in 0..count {
                            let rewards = tick_origin_path(&mut origin_quest, &origin_paths);
                            apply_origin_rewards(&rewards, &mut pressure);
                            apply_pressure_modifiers(&mut world, &pressure, &endgame_state);
                            if !rewards.is_empty() {
                                print_origin_path_status(&origin_quest, &origin_paths);
                            }
                        }
                    }
                    "" => {
                        println!("Usage: origin [paths|choose|status|event|tick]");
                    }
                    _ => {
                        println!("Usage: origin [paths|choose|status|event|tick]");
                    }
                }
            }
            "combat" => {
                let sub = parts.next().unwrap_or("").to_lowercase();
                match sub.as_str() {
                    "start" => {
                        let label = parts.next().unwrap_or("incident");
                        let scale = parts
                            .next()
                            .and_then(parse_combat_scale)
                            .unwrap_or(CombatScale::Street);
                        let opponent_count = parts
                            .next()
                            .and_then(|v| v.parse::<u32>().ok())
                            .unwrap_or(2);
                        let player_name = persona_stack
                            .active_persona()
                            .map(|p| p.label.clone())
                            .unwrap_or_else(|| "Player".to_string());
                        start_combat(
                            &mut combat,
                            city.active_location,
                            label,
                            scale,
                            &player_name,
                            opponent_count,
                            world.turn,
                        );
                        print_combat_status(&combat);
                    }
                    "use" => {
                        if !combat.active {
                            println!("No active combat. Use `combat start <label>` first.");
                        } else if let Some(expr_raw) = parts.next() {
                            combat.pending_player_expression =
                                Some(ExpressionId(expr_raw.to_string()));
                            println!("Queued expression {} for combat.", expr_raw);
                        } else {
                            println!("Usage: combat use <expression_id>");
                        }
                    }
                    "intent" => {
                        if !combat.active {
                            println!("No active combat. Use `combat start <label>` first.");
                        } else if let Some(raw) = parts.next() {
                            match parse_combat_intent(raw) {
                                Some(intent) => {
                                    if let Some(player) = combat.player_mut() {
                                        player.intent = intent;
                                        combat
                                            .log
                                            .push(format!("Player intent set to {:?}.", intent));
                                    }
                                    println!("Player intent -> {:?}.", intent);
                                }
                                None => {
                                    println!("Usage: combat intent <attack|escape|hold|capture>");
                                }
                            }
                        } else {
                            println!("Usage: combat intent <attack|escape|hold|capture>");
                        }
                    }
                    "tick" => {
                        if !combat.active {
                            println!("No active combat. Use `combat start <label>` first.");
                        } else {
                            let count = parts
                                .next()
                                .and_then(|v| v.parse::<u32>().ok())
                                .unwrap_or(1);
                            for _ in 0..count {
                                let expr_def = match combat.pending_player_expression.as_ref() {
                                    Some(expr_id) => match repo.expression(expr_id) {
                                        Ok(expr) => Some(expr),
                                        Err(err) => {
                                            println!("Expression not found: {}", err);
                                            None
                                        }
                                    },
                                    None => None,
                                };
                                let mastery_stage = combat
                                    .pending_player_expression
                                    .as_ref()
                                    .and_then(|expr_id| growth.mastery.get(expr_id))
                                    .map(|entry| entry.stage);

                                let tick_result = combat_tick(
                                    &mut combat,
                                    &mut actor,
                                    &world,
                                    &target,
                                    expr_def.as_ref(),
                                    mastery_stage,
                                    Some(&growth.unlocked_expressions),
                                );

                                if tick_result.used_success {
                                    if let Some(expr) = expr_def.as_ref() {
                                        apply_growth_on_use(
                                            &mut growth,
                                            expr,
                                            &repo,
                                            &mut storylet_state,
                                        );
                                    }
                                }

                                if !tick_result.emitted_signatures.is_empty() {
                                    let witnesses = target.witnesses.saturating_add(2);
                                    apply_action_signatures(
                                        &tick_result.emitted_signatures,
                                        combat.location_id,
                                        world.turn,
                                        witnesses,
                                        target.in_public,
                                        PersonaHint::Unknown,
                                        None,
                                        &mut city,
                                        &mut city_events,
                                        &mut evidence,
                                        &mut identity_evidence,
                                        &mut faction_director,
                                        &mut faction_events,
                                        &mut resolved_faction_events,
                                        &mut cases,
                                        &mut case_log,
                                        &mut persona_stack,
                                        alignment,
                                        &player_pos,
                                        &mut event_log,
                                    );
                                }

                                world.turn += 1;
                                tick_cooldowns(&mut actor);
                                update_units(&mut city);
                                evidence.tick_decay();
                                decay_heat(&mut city, &cases, &mut city_events);
                                game_time.advance();
                                tick_civilian_life(&mut civilian_state, &game_time);
                                storylet_state.tick();
                                update_pressure(
                                    &mut pressure,
                                    &city,
                                    &evidence,
                                    &cases,
                                    &game_time,
                                );
                                apply_civilian_pressure(&civilian_state, &mut pressure);
                                apply_pressure_modifiers(&mut world, &pressure, &endgame_state);
                                run_region_update(
                                    &mut region,
                                    &city,
                                    &pressure,
                                    &mut city_events,
                                    &mut region_events,
                                );
                                run_global_faction_director(
                                    &mut global_faction_director,
                                    &region,
                                    &mut global_faction_events,
                                );
                                let end_reason = tick_result.ended;
                                if let Some(end_reason) = end_reason {
                                    let consequences =
                                        tick_result.post_combat_consequences.unwrap_or_else(|| {
                                            combat_post_consequences(
                                                &mut combat,
                                                end_reason,
                                                &target,
                                            )
                                        });
                                    handle_combat_end_consequences(
                                        end_reason,
                                        consequences,
                                        combat.location_id,
                                        &mut world,
                                        &target,
                                        &mut pressure,
                                        &endgame_state,
                                        &mut city,
                                        &mut city_events,
                                        &mut evidence,
                                        &mut identity_evidence,
                                        &mut faction_director,
                                        &mut faction_events,
                                        &mut resolved_faction_events,
                                        &mut cases,
                                        &mut case_log,
                                        &mut persona_stack,
                                        alignment,
                                        &player_pos,
                                        &mut event_log,
                                    );
                                }
                                handle_endgame_transition(
                                    &cases,
                                    &pressure,
                                    &resolved_faction_events,
                                    &mut world,
                                    &mut storylet_state,
                                    &mut endgame_state,
                                );
                                if let Some(end_reason) = end_reason {
                                    println!("Combat ended: {}", format_combat_end(end_reason));
                                    break;
                                }
                            }
                            print_combat_status(&combat);
                            print_event_log(&mut event_log);
                        }
                    }
                    "log" => {
                        print_combat_log(&combat);
                    }
                    "resolve" => {
                        if let Some(end_reason) = resolve_combat(&mut combat) {
                            let consequences =
                                combat_post_consequences(&mut combat, end_reason, &target);
                            handle_combat_end_consequences(
                                end_reason,
                                consequences,
                                combat.location_id,
                                &mut world,
                                &target,
                                &mut pressure,
                                &endgame_state,
                                &mut city,
                                &mut city_events,
                                &mut evidence,
                                &mut identity_evidence,
                                &mut faction_director,
                                &mut faction_events,
                                &mut resolved_faction_events,
                                &mut cases,
                                &mut case_log,
                                &mut persona_stack,
                                alignment,
                                &player_pos,
                                &mut event_log,
                            );
                            println!("Combat ended: {}", format_combat_end(end_reason));
                        } else {
                            println!("No active combat.");
                        }
                    }
                    "force_escape" => {
                        if let Some(end_reason) = force_escape(&mut combat) {
                            let consequences =
                                combat_post_consequences(&mut combat, end_reason, &target);
                            handle_combat_end_consequences(
                                end_reason,
                                consequences,
                                combat.location_id,
                                &mut world,
                                &target,
                                &mut pressure,
                                &endgame_state,
                                &mut city,
                                &mut city_events,
                                &mut evidence,
                                &mut identity_evidence,
                                &mut faction_director,
                                &mut faction_events,
                                &mut resolved_faction_events,
                                &mut cases,
                                &mut case_log,
                                &mut persona_stack,
                                alignment,
                                &player_pos,
                                &mut event_log,
                            );
                            println!("Combat ended: {}", format_combat_end(end_reason));
                        } else {
                            println!("No active combat.");
                        }
                    }
                    "force_escalate" => {
                        if force_escalate(&mut combat) {
                            println!("Combat escalated to {:?}.", combat.scale);
                        } else {
                            println!("No escalation available.");
                        }
                    }
                    _ => {
                        println!("Usage: combat start <label> [scale] [opponents] | combat use <expression_id> | combat intent <attack|escape|hold|capture> | combat tick [n] | combat log | combat resolve | combat force_escape | combat force_escalate");
                    }
                }
            }
            "tick" => {
                let count = parts
                    .next()
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(1);
                tick_world(
                    &mut world,
                    &mut actor,
                    &mut evidence,
                    &mut identity_evidence,
                    &mut city,
                    &mut city_events,
                    &mut faction_director,
                    &mut faction_events,
                    &mut resolved_faction_events,
                    &mut cases,
                    &mut case_log,
                    &mut agents,
                    &mut agent_events,
                    &mut persona_stack,
                    &storylets,
                    alignment,
                    &mut player_pos,
                    &mut game_time,
                    &mut civilian_state,
                    &mut storylet_state,
                    &mut endgame_state,
                    &mut pressure,
                    &mut region,
                    &mut region_events,
                    &mut global_faction_director,
                    &mut global_faction_events,
                    &global_events,
                    &mut global_event_log,
                    &mut origin_quest,
                    &origin_paths,
                    count,
                );
                handle_endgame_transition(
                    &cases,
                    &pressure,
                    &resolved_faction_events,
                    &mut world,
                    &mut storylet_state,
                    &mut endgame_state,
                );
                persist_world_state(
                    &mut *world_repo,
                    &world,
                    &game_time,
                    &city,
                    &cases,
                    &combat,
                    &growth,
                    &storylet_state,
                );
                print_tick_summary(
                    &world,
                    &persona_stack,
                    &city,
                    &cases,
                    &pressure,
                    &endgame_state,
                );
                print_case_log(&mut case_log);
            }
            _ => {
                println!("Unknown command. Type 'help'.");
            }
        }
    }

    persist_world_state(
        &mut *world_repo,
        &world,
        &game_time,
        &city,
        &cases,
        &combat,
        &growth,
        &storylet_state,
    );
}

fn apply_growth_on_use(
    growth: &mut GrowthState,
    expr: &superhero_universe::rules::ExpressionDef,
    repo: &dyn PowerRepository,
    storylet_state: &mut StoryletState,
) {
    let stage_change = record_expression_use(growth, expr);
    if let Some(stage) = stage_change {
        println!("Mastery advanced: {} -> {:?}", expr.id.0, stage);
        if let Some(flag) = mastery_stage_flag(stage) {
            storylet_state.flags.insert(flag.to_string(), true);
            storylet_state
                .flags
                .insert(format!("{}_{}", flag, expr.id.0), true);
        }
        if let Ok(candidates) = repo.expressions_for_power(expr.power_id) {
            if let Some(unlocked) =
                select_evolution_candidate(expr, &candidates, &growth.unlocked_expressions)
            {
                growth.unlocked_expressions.insert(unlocked.clone());
                println!("Evolution unlocked expression {}", unlocked.0);
            }
        }
    }
}

fn mastery_stage_flag(stage: superhero_universe::rules::MasteryStage) -> Option<&'static str> {
    match stage {
        superhero_universe::rules::MasteryStage::Precise => Some("mastery_precise"),
        superhero_universe::rules::MasteryStage::Silent => Some("mastery_silent"),
        superhero_universe::rules::MasteryStage::Iconic => Some("mastery_iconic"),
        _ => None,
    }
}

fn print_growth_expr(growth: &GrowthState, expr_id: &str) {
    let expr_id = ExpressionId(expr_id.to_string());
    let unlocked = growth.unlocked_expressions.contains(&expr_id);
    match growth.mastery.get(&expr_id) {
        Some(entry) => {
            println!(
                "Expression {} -> unlocked={} stage={:?} uses={}",
                expr_id.0, unlocked, entry.stage, entry.uses
            );
        }
        None => {
            println!(
                "Expression {} -> unlocked={} stage=RAW uses=0",
                expr_id.0, unlocked
            );
        }
    }
}

fn seed_mastery(growth: &mut GrowthState, expr_id: &str, uses: u32) {
    let expr_id = ExpressionId(expr_id.to_string());
    let stage = superhero_universe::rules::stage_from_uses(uses);
    growth.unlocked_expressions.insert(expr_id.clone());
    growth.mastery.insert(
        expr_id.clone(),
        superhero_universe::simulation::growth::ExpressionMastery { stage, uses },
    );
    println!(
        "Mastery seeded: {} -> {:?} (uses={})",
        expr_id.0, stage, uses
    );
}

fn parse_paths(args: Vec<String>) -> (PathBuf, PathBuf) {
    let mut iter = args.iter();
    let mut content_path = PathBuf::from("./assets/db/content_v1.db");
    let mut world_path = PathBuf::from("./assets/db/world.db");
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--db" => {
                if let Some(value) = iter.next() {
                    content_path = PathBuf::from(value);
                }
            }
            "--world" => {
                if let Some(value) = iter.next() {
                    world_path = PathBuf::from(value);
                }
            }
            _ => {}
        }
    }
    (content_path, world_path)
}

fn print_stats(repo: &dyn PowerRepository) {
    match repo.stats() {
        Ok(stats) => println!(
            "Stats: powers={}, expressions={}, acquisition_profiles={}",
            stats.power_count, stats.expression_count, stats.acquisition_count
        ),
        Err(err) => println!("Stats unavailable: {}", err),
    }
}

fn print_power(repo: &dyn PowerRepository, power_id: i64) {
    let info = match repo.power_info(PowerId(power_id)) {
        Ok(info) => info,
        Err(err) => {
            println!("Failed to query power: {}", err);
            return;
        }
    };

    let Some(info) = info else {
        println!("No power found for id {}", power_id);
        return;
    };

    println!("Power [{}]: {}", power_id, info.name);
    if let Some(overview) = info.overview {
        if !overview.trim().is_empty() {
            println!("Overview: {}", overview.trim());
        }
    }
    if let Some(description) = info.description {
        if !description.trim().is_empty() {
            println!("Description: {}", description.trim());
        }
    }
    if let Some(short) = info.text_short {
        if !short.trim().is_empty() {
            println!("Short: {}", short.trim());
        }
    }
    if let Some(mech) = info.text_mechanical {
        if !mech.trim().is_empty() {
            println!("Mechanical: {}", mech.trim());
        }
    }

    match repo.expressions_for_power(PowerId(power_id)) {
        Ok(expressions) => {
            println!("Expressions:");
            for expr in expressions {
                println!(
                    "  [{}] {} ({:?}/{:?}/{:?})",
                    expr.id.0, expr.text.ui_name, expr.form, expr.delivery, expr.scale
                );
            }
        }
        Err(err) => println!("Failed to load expressions: {}", err),
    }
}

fn print_use_result(result: &superhero_universe::rules::UseResult) {
    println!("Mastery: {:?}", result.mastery_stage);
    let mut stamina = 0;
    let mut focus = 0;
    let mut resource = 0;
    let mut cooldown = None;
    for cost in &result.applied_costs {
        match cost.cost_type {
            CostType::Stamina => stamina += cost.value.unwrap_or(0),
            CostType::Focus => focus += cost.value.unwrap_or(0),
            CostType::Resource => resource += cost.value.unwrap_or(0),
            CostType::Cooldown => cooldown = cost.value.or(cooldown),
            CostType::Risk => {}
        }
    }
    println!(
        "Costs: stamina={}, focus={}, resource={}, cooldown={:?}",
        stamina, focus, resource, cooldown
    );

    if result.emitted_signatures.is_empty() {
        println!("Signatures: none");
    } else {
        println!("Signatures:");
        for sig in &result.emitted_signatures {
            println!(
                "  {:?} strength={} persistence={}",
                sig.signature.signature_type, sig.signature.strength, sig.remaining_turns
            );
        }
    }
}

fn print_event_log(log: &mut WorldEventLog) {
    if log.0.is_empty() {
        return;
    }
    println!("World events:");
    for entry in log.0.drain(..) {
        println!("  {}", entry);
    }
}

fn print_context(
    target: &TargetContext,
    world: &WorldState,
    city: &CityState,
    pressure: &PressureState,
    endgame_state: &EndgameState,
) {
    let active = city
        .locations
        .get(&city.active_location)
        .map(|loc| (loc.heat, loc.response))
        .unwrap_or((0, superhero_universe::simulation::city::HeatResponse::None));
    println!(
        "Context: turn={}, dist_m={:?}, los={}, contact={}, public={}, witnesses={}, location={}, heat={}, response={:?}",
        world.turn,
        target.distance_m,
        target.has_line_of_sight,
        target.has_contact,
        target.in_public,
        target.witnesses,
        city.active_location.0,
        active.0,
        active.1
    );
    println!(
        "Pressure: temporal={:.1} identity={:.1} institutional={:.1} moral={:.1} resource={:.1} psychological={:.1}",
        pressure.temporal,
        pressure.identity,
        pressure.institutional,
        pressure.moral,
        pressure.resource,
        pressure.psychological
    );
    let modifiers = endgame_state.modifiers();
    println!(
        "Endgame: {} | modifiers cost={:.2} risk={:.2}",
        endgame_state.label(),
        modifiers.cost_scale,
        modifiers.risk_scale
    );
}

fn apply_pressure_modifiers(
    world: &mut WorldState,
    pressure: &PressureState,
    endgame_state: &EndgameState,
) {
    let base = pressure.to_modifiers();
    world.pressure = endgame_state.apply_modifiers(base);
}

fn print_combat_status(state: &CombatState) {
    if !state.active {
        println!("Combat: inactive");
        return;
    }
    let player_stress = state.player().map(|p| p.stress).unwrap_or(0);
    let opponents = state.active_opponent_count();
    println!(
        "Combat: {:?} | tick={} | opponents={} | player_stress={}",
        state.scale, state.tick, opponents, player_stress
    );
}

fn print_combat_log(state: &CombatState) {
    if state.log.is_empty() {
        println!("Combat log: empty");
        return;
    }
    println!("Combat log:");
    for entry in state.log.iter() {
        println!("  {}", entry);
    }
}

fn format_combat_end(end: CombatEnd) -> &'static str {
    match end {
        CombatEnd::PlayerEscaped => "player escaped",
        CombatEnd::PlayerDefeated => "player defeated",
        CombatEnd::OpponentsDefeated => "opponents defeated",
        CombatEnd::Resolved => "resolved",
    }
}

fn parse_combat_scale(raw: &str) -> Option<CombatScale> {
    match raw.to_lowercase().as_str() {
        "street" => Some(CombatScale::Street),
        "district" => Some(CombatScale::District),
        "city" => Some(CombatScale::City),
        "national" => Some(CombatScale::National),
        "cosmic" => Some(CombatScale::Cosmic),
        _ => None,
    }
}

fn parse_combat_intent(raw: &str) -> Option<CombatIntent> {
    match raw.to_lowercase().as_str() {
        "attack" => Some(CombatIntent::Attack),
        "escape" => Some(CombatIntent::Escape),
        "hold" => Some(CombatIntent::Hold),
        "capture" => Some(CombatIntent::Capture),
        _ => None,
    }
}

fn print_persona_state(
    stack: &PersonaStack,
    alignment: Alignment,
    city: &CityState,
    cases: &CaseRegistry,
) {
    println!(
        "Alignment: {:?} | Active persona: {}",
        alignment, stack.active_persona_id
    );
    if let Some(active) = stack.active_persona() {
        println!(
            "Active type: {:?} | Label: {}",
            active.persona_type, active.label
        );
    }
    for persona in &stack.personas {
        println!(
            "Persona {} ({:?}) -> public={} civilian={} wanted={} exposure={}",
            persona.persona_id,
            persona.persona_type,
            persona.suspicion.public_suspicion,
            persona.suspicion.civilian_suspicion,
            persona.suspicion.wanted_level,
            persona.suspicion.exposure_risk
        );
    }
    let active_location = city.active_location;
    let max_progress = cases
        .cases
        .iter()
        .filter(|case| case.location_id == active_location)
        .map(|case| case.progress)
        .max()
        .unwrap_or(0);
    if let Some(location) = city.locations.get(&active_location) {
        println!(
            "Pressure: heat={} cases_max_progress={}",
            location.heat, max_progress
        );
    }
}

fn print_growth_state(growth: &GrowthState) {
    println!(
        "Growth: mastery={} unlocked={} pressure_resistance={}",
        growth.mastery.len(),
        growth.unlocked_expressions.len(),
        growth.pressure_resistance
    );
    println!(
        "Reputation: trust={} fear={} infamy={} symbolism={}",
        growth.reputation.trust,
        growth.reputation.fear,
        growth.reputation.infamy,
        growth.reputation.symbolism
    );
}

fn print_persona_stack(stack: &PersonaStack, turn: u64) {
    println!("Persona stack:");
    for persona in &stack.personas {
        let cooldown = if turn < stack.next_switch_tick {
            stack.next_switch_tick - turn
        } else {
            0
        };
        println!(
            "  {} | {:?} | label={} | switch_cd={} turns",
            persona.persona_id, persona.persona_type, persona.label, cooldown
        );
    }
}

fn handle_switch(
    persona_id: &str,
    stack: &mut PersonaStack,
    turn: u64,
    city: &CityState,
    evidence: &WorldEvidence,
    target: &TargetContext,
) {
    let location_id = city.active_location;
    let Some(location) = city.locations.get(&location_id) else {
        println!("Unknown location.");
        return;
    };
    let witnesses = if target.in_public {
        target.witnesses
    } else {
        0
    };
    let has_visual = evidence.signatures.iter().any(|event| {
        event.location_id == location_id
            && event.signature.signature.signature_type
                == superhero_universe::rules::SignatureType::VisualAnomaly
    });

    match attempt_switch(stack, persona_id, turn, location, witnesses, has_visual) {
        Ok(result) => {
            println!(
                "Switch succeeded: {} -> {:?}",
                result.new_persona_id, result.new_persona_type
            );
            if result.suspicion_applied {
                println!("Suspicion increased due to risky switch.");
            }
        }
        Err(err) => {
            print_switch_error(err);
        }
    }
}

fn print_switch_error(err: PersonaSwitchError) {
    let reason = match err {
        PersonaSwitchError::UnknownPersona => "Unknown persona",
        PersonaSwitchError::AlreadyActive => "Already active",
        PersonaSwitchError::SwitchBlockedByWitnesses => "Blocked by witnesses",
        PersonaSwitchError::SwitchBlockedByLocation => "Blocked by location",
        PersonaSwitchError::SwitchOnCooldown => "Switch on cooldown",
    };
    println!("Switch failed: {}", reason);
}

fn print_location(city: &CityState) {
    let Some(location) = city.locations.get(&city.active_location) else {
        println!("Location not found.");
        return;
    };
    println!(
        "Location {} | heat={} crime_pressure={} response={:?}",
        location.id.0, location.heat, location.crime_pressure, location.response
    );
    println!(
        "Security: police_presence={} surveillance_level={} lockdown_level={}",
        location.police_presence, location.surveillance_level, location.lockdown_level
    );
    println!(
        "Units: police_units={} investigators={} gang_units={}",
        location.police_units, location.investigators, location.gang_units
    );
    if !location.faction_influence.is_empty() {
        println!("Influence:");
        for (faction, influence) in &location.faction_influence {
            println!("  {} -> {}", faction, influence);
        }
    }
}

fn print_faction_events(events: &mut ResolvedFactionEventLog) {
    if events.0.is_empty() {
        println!("Faction events: none");
        return;
    }
    println!("Faction events:");
    for event in events.0.drain(..) {
        println!(
            "  {} -> {} at loc {} (level {})",
            event.faction_id, event.faction_type_id, event.location_id.0, event.level
        );
        for action in event.actions {
            println!("    action {}", action.kind);
        }
    }
}

fn print_cases(cases: &CaseRegistry) {
    if cases.cases.is_empty() {
        println!("Cases: none");
        return;
    }
    println!("Cases:");
    for case in &cases.cases {
        println!(
            "  Case {} | faction={} loc={} progress={} status={:?} target={:?}",
            case.case_id,
            case.faction_id,
            case.location_id.0,
            case.progress,
            case.status,
            case.target_type
        );
        if !case.signature_pattern.is_empty() {
            println!("    signatures: {:?}", case.signature_pattern);
        }
    }
}

fn print_case_log(log: &mut CaseEventLog) {
    if log.0.is_empty() {
        return;
    }
    println!("Case events:");
    for entry in log.0.drain(..) {
        println!("  {}", entry);
    }
}

fn update_context(target: &mut TargetContext, field: &str, value: &str) -> Result<(), ()> {
    match field {
        "dist" | "distance" => {
            let dist = value.parse::<i64>().map_err(|_| ())?;
            target.distance_m = Some(dist);
            Ok(())
        }
        "los" => match value {
            "on" | "true" => {
                target.has_line_of_sight = true;
                Ok(())
            }
            "off" | "false" => {
                target.has_line_of_sight = false;
                Ok(())
            }
            _ => Err(()),
        },
        "contact" => match value {
            "on" | "true" => {
                target.has_contact = true;
                Ok(())
            }
            "off" | "false" => {
                target.has_contact = false;
                Ok(())
            }
            _ => Err(()),
        },
        "public" => match value {
            "on" | "true" => {
                target.in_public = true;
                Ok(())
            }
            "off" | "false" => {
                target.in_public = false;
                Ok(())
            }
            _ => Err(()),
        },
        "witnesses" => {
            let count = value.parse::<u32>().map_err(|_| ())?;
            target.witnesses = count;
            Ok(())
        }
        _ => Err(()),
    }
}

fn print_cooldowns(actor: &ActorState) {
    if actor.cooldowns.is_empty() {
        println!("Cooldowns: none");
        return;
    }
    println!("Cooldowns:");
    for (expr_id, turns) in actor.cooldowns.iter() {
        println!("  {} -> {}", expr_id.0, turns);
    }
}

fn print_scene(scene: &WorldEvidence) {
    if scene.signatures.is_empty() {
        println!("Scene evidence: none");
        return;
    }
    println!("Scene evidence:");
    for item in &scene.signatures {
        println!(
            "  loc={} {:?} strength={} remaining={}",
            item.location_id.0,
            item.signature.signature.signature_type,
            item.signature.signature.strength,
            item.signature.remaining_turns
        );
    }
}

fn print_civilian_status(state: &CivilianState, time: &GameTime) {
    let pressure = state.pressure_targets();
    println!("Civilian status @ {}:", time.to_string());
    println!(
        "  Job: {:?} ({:?} L{} | satisfaction={} stability={})",
        state.job_status,
        state.job.role,
        state.job.level,
        state.job.satisfaction,
        state.job.stability
    );
    println!(
        "  Finances: cash={} debt={} rent={} rent_due_in={} wage={}",
        state.finances.cash,
        state.finances.debt,
        state.finances.rent,
        state.finances.rent_due_in,
        state.finances.wage
    );
    println!(
        "  Economy: liquid={} savings={} investments={} assets={} liabilities={}",
        state.economy.liquid,
        state.economy.savings,
        state.economy.investments,
        state.economy.assets,
        state.economy.liabilities
    );
    println!(
        "  Net worth: {} ({}) | income={} expenses={} | gadget fund={}",
        state.economy.net_worth(),
        state.economy.wealth_tier(),
        state.economy.monthly_income,
        state.economy.monthly_expenses,
        state.economy.gadget_fund
    );
    println!(
        "  Social: support={} strain={} obligation={}",
        state.social.support, state.social.strain, state.social.obligation
    );
    println!(
        "  Reputation: career={} community={} media={}",
        state.reputation.career, state.reputation.community, state.reputation.media
    );
    println!(
        "  Rewards: income_boost={} safehouse={} access={}",
        state.rewards.income_boost, state.rewards.safehouse, state.rewards.access
    );
    if state.contacts.is_empty() {
        println!("  Contacts: none");
    } else {
        println!("  Contacts:");
        for contact in &state.contacts {
            println!(
                "    {} -> {:?} (bond={} influence={})",
                contact.name, contact.level, contact.bond, contact.influence
            );
        }
    }
    println!(
        "  Civilian pressure targets: temporal={:.1} resource={:.1} moral={:.1} identity={:.1}",
        pressure.temporal, pressure.resource, pressure.moral, pressure.identity
    );
}

fn print_pending_civilian_events(
    state: &CivilianState,
    library: &[CivilianStorylet],
    time: &GameTime,
) {
    if state.pending_events.is_empty() {
        println!("Civilian events: none");
        return;
    }
    println!("Civilian events:");
    for event in &state.pending_events {
        let age = time.tick.saturating_sub(event.created_tick);
        match find_civilian_event(library, &event.storylet_id) {
            Some(def) => {
                println!("  {} | {} | queued {} ticks ago", def.id, def.title, age);
                for choice in &def.choices {
                    println!("    [{}] {}", choice.id, choice.text);
                }
            }
            None => {
                println!(
                    "  {} | (missing definition) | queued {} ticks ago",
                    event.storylet_id, age
                );
            }
        }
    }
}

fn resolve_civilian_event(
    state: &mut CivilianState,
    library: &[CivilianStorylet],
    event_id: &str,
    choice_id: &str,
) {
    let Some(event_def) = find_civilian_event(library, event_id) else {
        println!("Unknown civilian event: {}", event_id);
        return;
    };
    let Some(choice) = event_def
        .choices
        .iter()
        .find(|choice| choice.id == choice_id)
    else {
        println!("Unknown choice {} for event {}", choice_id, event_id);
        return;
    };
    let mut applied = apply_civilian_effects(state, &event_def.effects);
    applied.extend(apply_civilian_effects(state, &choice.effects));
    if let Some(index) = state
        .pending_events
        .iter()
        .position(|event| event.storylet_id == event_id)
    {
        state.pending_events.remove(index);
    }
    println!("Resolved {} -> {}", event_def.title, choice.text);
    if applied.is_empty() {
        println!("No civilian effects applied.");
    } else {
        println!("Applied effects:");
        for entry in applied {
            println!("  {}", entry);
        }
    }
}

fn find_civilian_event<'a>(
    library: &'a [CivilianStorylet],
    event_id: &str,
) -> Option<&'a CivilianStorylet> {
    library.iter().find(|event| event.id == event_id)
}

fn record_identity_evidence(
    identity: &mut IdentityEvidenceStore,
    city: &CityState,
    location_id: superhero_universe::simulation::city::LocationId,
    turn: u64,
    signatures: &[superhero_universe::rules::SignatureInstance],
    witnesses: u32,
    persona_hint: PersonaHint,
    modifiers: Option<IdentityEvidenceModifiers>,
) {
    let (surveillance, in_public) = city
        .locations
        .get(&location_id)
        .map(|loc| {
            (
                loc.surveillance_level,
                loc.tags
                    .contains(&superhero_universe::simulation::city::LocationTag::Public),
            )
        })
        .unwrap_or((0, true));
    let modifiers = modifiers.unwrap_or_default();
    let witness_count = if in_public {
        witnesses.max(1).saturating_add(modifiers.witness_bonus)
    } else {
        0
    };
    let visual_quality =
        (surveillance as i32 + (witness_count as i32 * 10) + modifiers.visual_bonus).clamp(0, 100)
            as u8;
    for sig in signatures {
        identity.record(
            location_id,
            turn,
            vec![sig.signature.signature_type],
            witness_count,
            visual_quality,
            persona_hint,
            Vec::new(),
        );
    }
}

fn apply_agent_events(
    agent_events: &AgentEventLog,
    turn: u64,
    city: &mut CityState,
    city_events: &mut CityEventLog,
    evidence: &mut WorldEvidence,
    identity_evidence: &mut IdentityEvidenceStore,
    event_log: &mut WorldEventLog,
) {
    for event in &agent_events.0 {
        if let AgentEvent::Incident {
            location_id,
            signatures,
            ..
        } = event
        {
            apply_agent_incident(
                signatures,
                *location_id,
                turn,
                city,
                city_events,
                evidence,
                identity_evidence,
                event_log,
            );
        }
    }
}

fn apply_agent_incident(
    signatures: &[superhero_universe::rules::SignatureInstance],
    location_id: superhero_universe::simulation::city::LocationId,
    turn: u64,
    city: &mut CityState,
    city_events: &mut CityEventLog,
    evidence: &mut WorldEvidence,
    identity_evidence: &mut IdentityEvidenceStore,
    event_log: &mut WorldEventLog,
) {
    if signatures.is_empty() {
        return;
    }

    let (in_public, witnesses) = city
        .locations
        .get(&location_id)
        .map(|location| {
            let in_public = location.tags.contains(&LocationTag::Public);
            let witnesses = if in_public {
                2 + (location.surveillance_level / 20).max(0) as u32
            } else {
                0
            };
            (in_public, witnesses)
        })
        .unwrap_or((true, 0));

    evidence.emit(location_id, signatures);
    apply_signatures(
        city,
        location_id,
        signatures,
        witnesses,
        in_public,
        event_log,
        city_events,
    );
    record_identity_evidence(
        identity_evidence,
        city,
        location_id,
        turn,
        signatures,
        witnesses,
        PersonaHint::Unknown,
        None,
    );
}

fn apply_action_signatures(
    signatures: &[superhero_universe::rules::SignatureInstance],
    location_id: superhero_universe::simulation::city::LocationId,
    turn: u64,
    witnesses: u32,
    in_public: bool,
    persona_hint: PersonaHint,
    identity_modifiers: Option<IdentityEvidenceModifiers>,
    city: &mut CityState,
    city_events: &mut CityEventLog,
    evidence: &mut WorldEvidence,
    identity_evidence: &mut IdentityEvidenceStore,
    faction_director: &mut FactionDirector,
    faction_events: &mut FactionEventLog,
    resolved_faction_events: &mut ResolvedFactionEventLog,
    cases: &mut CaseRegistry,
    case_log: &mut CaseEventLog,
    persona_stack: &mut PersonaStack,
    alignment: Alignment,
    player_pos: &Position,
    event_log: &mut WorldEventLog,
) {
    evidence.emit(location_id, signatures);
    apply_signatures(
        city,
        location_id,
        signatures,
        witnesses,
        in_public,
        event_log,
        city_events,
    );
    record_identity_evidence(
        identity_evidence,
        city,
        location_id,
        turn,
        signatures,
        witnesses,
        persona_hint,
        identity_modifiers,
    );
    run_faction_director(faction_director, city, evidence, faction_events);
    resolve_faction_events(
        faction_events,
        resolved_faction_events,
        city,
        evidence,
        cases,
        case_log,
    );
    update_cases(cases, city, evidence, identity_evidence, case_log);
    apply_suspicion_for_intents(
        persona_stack,
        alignment,
        player_pos,
        city,
        cases,
        identity_evidence,
        &[],
        1,
    );
}

fn apply_combat_pressure_delta(pressure: &mut PressureState, delta: CombatPressureDelta) {
    pressure.temporal = (pressure.temporal + delta.temporal).clamp(0.0, 100.0);
    pressure.identity = (pressure.identity + delta.identity).clamp(0.0, 100.0);
    pressure.institutional = (pressure.institutional + delta.institutional).clamp(0.0, 100.0);
    pressure.moral = (pressure.moral + delta.moral).clamp(0.0, 100.0);
    pressure.resource = (pressure.resource + delta.resource).clamp(0.0, 100.0);
    pressure.psychological = (pressure.psychological + delta.psychological).clamp(0.0, 100.0);
}

fn combat_case_progress_summary(
    cases: &CaseRegistry,
    location_id: superhero_universe::simulation::city::LocationId,
) -> Vec<String> {
    cases
        .cases
        .iter()
        .filter(|case| case.location_id == location_id)
        .map(|case| format!("case#{}:{}", case.case_id, case.progress))
        .collect()
}

fn handle_combat_end_consequences(
    end: CombatEnd,
    consequences: CombatConsequences,
    location_id: superhero_universe::simulation::city::LocationId,
    world: &mut WorldState,
    target: &TargetContext,
    pressure: &mut PressureState,
    endgame_state: &EndgameState,
    city: &mut CityState,
    city_events: &mut CityEventLog,
    evidence: &mut WorldEvidence,
    identity_evidence: &mut IdentityEvidenceStore,
    faction_director: &mut FactionDirector,
    faction_events: &mut FactionEventLog,
    resolved_faction_events: &mut ResolvedFactionEventLog,
    cases: &mut CaseRegistry,
    case_log: &mut CaseEventLog,
    persona_stack: &mut PersonaStack,
    alignment: Alignment,
    player_pos: &Position,
    event_log: &mut WorldEventLog,
) {
    if !consequences.signatures.is_empty() {
        let witnesses = target.witnesses.saturating_add(4);
        let identity_modifiers = combat_consequence_modifiers(consequences.combat_consequence);
        apply_action_signatures(
            &consequences.signatures,
            location_id,
            world.turn,
            witnesses,
            target.in_public,
            PersonaHint::Unknown,
            Some(identity_modifiers),
            city,
            city_events,
            evidence,
            identity_evidence,
            faction_director,
            faction_events,
            resolved_faction_events,
            cases,
            case_log,
            persona_stack,
            alignment,
            player_pos,
            event_log,
        );
    }

    apply_combat_consequence_heat(
        city,
        location_id,
        consequences.combat_consequence,
        event_log,
        city_events,
    );

    apply_combat_pressure_delta(pressure, consequences.pressure_delta);
    apply_pressure_modifiers(world, pressure, endgame_state);

    let case_summary = combat_case_progress_summary(cases, location_id);
    println!(
        "Combat fallout: end={:?} signatures={} consequence(p={} c={} n={}) pressure(t={:.1} id={:.1} inst={:.1} moral={:.1} res={:.1} psy={:.1}) evidence={} identity_evidence={} cases=[{}]",
        end,
        consequences.signatures.len(),
        consequences.combat_consequence.publicness,
        consequences.combat_consequence.collateral,
        consequences.combat_consequence.notoriety,
        pressure.temporal,
        pressure.identity,
        pressure.institutional,
        pressure.moral,
        pressure.resource,
        pressure.psychological,
        evidence.signatures.len(),
        identity_evidence.items.len(),
        case_summary.join(", ")
    );
}

fn tick_world(
    world: &mut WorldState,
    actor: &mut ActorState,
    scene: &mut WorldEvidence,
    identity_evidence: &mut IdentityEvidenceStore,
    city: &mut CityState,
    city_events: &mut CityEventLog,
    faction_director: &mut FactionDirector,
    faction_events: &mut FactionEventLog,
    resolved_faction_events: &mut ResolvedFactionEventLog,
    cases: &mut CaseRegistry,
    case_log: &mut CaseEventLog,
    agents: &mut AgentRegistry,
    agent_events: &mut AgentEventLog,
    persona_stack: &mut PersonaStack,
    storylets: &StoryletLibrary,
    alignment: Alignment,
    position: &mut Position,
    game_time: &mut GameTime,
    civilian_state: &mut CivilianState,
    storylet_state: &mut StoryletState,
    endgame_state: &mut EndgameState,
    pressure: &mut PressureState,
    region: &mut RegionState,
    region_events: &mut RegionEventLog,
    global_faction_director: &mut GlobalFactionDirector,
    global_faction_events: &mut GlobalFactionEventLog,
    global_event_catalog: &[GlobalEventDefinition],
    global_event_log: &mut GlobalEventLog,
    origin_quest: &mut OriginQuestState,
    origin_paths: &OriginPathCatalog,
    turns: u32,
) {
    let mut agent_event_log = WorldEventLog::default();
    for _ in 0..turns {
        world.turn += 1;
        tick_cooldowns(actor);
        update_units(city);
        scene.tick_decay();
        decay_heat(city, cases, city_events);
        game_time.advance();
        tick_agents(agents, city, game_time, agent_events);
        agent_event_log.0.clear();
        apply_agent_events(
            agent_events,
            world.turn,
            city,
            city_events,
            scene,
            identity_evidence,
            &mut agent_event_log,
        );
        tick_civilian_life(civilian_state, game_time);
        storylet_state.tick();
        let rewards = tick_origin_path(origin_quest, origin_paths);
        apply_origin_rewards(rewards.as_slice(), pressure);
        run_faction_director(faction_director, city, scene, faction_events);
        resolve_faction_events(
            faction_events,
            resolved_faction_events,
            city,
            scene,
            cases,
            case_log,
        );
        update_cases(cases, city, scene, identity_evidence, case_log);
        apply_suspicion_for_intents(
            persona_stack,
            alignment,
            position,
            city,
            cases,
            identity_evidence,
            &[],
            1,
        );
        update_pressure(pressure, city, scene, cases, game_time);
        apply_civilian_pressure(civilian_state, pressure);
        apply_pressure_modifiers(world, pressure, endgame_state);
        let ctx = build_storylet_context(
            alignment,
            persona_stack,
            storylet_state,
            endgame_state,
            city,
            scene,
            cases,
            pressure,
            game_time,
        );
        if let Some(storylet) = select_storylet_for_turn(storylets, alignment, storylet_state, &ctx)
        {
            println!(
                "Storylet triggered: {} | {}",
                storylet.id, storylet.text_stub
            );
        }
        run_region_update(region, city, pressure, city_events, region_events);
        if !global_event_catalog.is_empty() {
            region.evaluate_global_events(global_event_catalog, global_event_log);
            apply_global_event_effects(global_event_log, storylet_state, pressure);
            region.update_global_pressure(pressure);
        }
        run_global_faction_director(global_faction_director, region, global_faction_events);
    }
}

fn apply_global_event_effects(
    global_events: &GlobalEventLog,
    storylet_state: &mut StoryletState,
    pressure: &mut PressureState,
) {
    for event in &global_events.0 {
        if !event.storylet_flag.is_empty() {
            storylet_state
                .flags
                .insert(event.storylet_flag.clone(), true);
        }
        if event.pressure_boost != 0.0 {
            pressure.temporal = (pressure.temporal + event.pressure_boost).clamp(0.0, 100.0);
            pressure.identity = (pressure.identity + event.pressure_boost).clamp(0.0, 100.0);
            pressure.institutional =
                (pressure.institutional + event.pressure_boost).clamp(0.0, 100.0);
            pressure.moral = (pressure.moral + event.pressure_boost).clamp(0.0, 100.0);
            pressure.resource = (pressure.resource + event.pressure_boost).clamp(0.0, 100.0);
            pressure.psychological =
                (pressure.psychological + event.pressure_boost).clamp(0.0, 100.0);
        }
    }
}

fn handle_transformation(
fn handle_endgame_transition(
    cases: &CaseRegistry,
    pressure: &PressureState,
    faction_events: &ResolvedFactionEventLog,
    world: &mut WorldState,
    storylet_state: &mut StoryletState,
    endgame_state: &mut EndgameState,
) {
    if let Some(event) = evaluate_transformation(cases, pressure, faction_events) {
        if let Some(update) =
            apply_transformation_event(endgame_state, storylet_state, event)
        {
            apply_pressure_modifiers(world, pressure, endgame_state);
            println!(
                "Endgame triggered ({:?}): {}",
                update.event.trigger, update.narrative
            );
        }
    }
}

fn tick_cooldowns(actor: &mut ActorState) {
    let mut to_clear = Vec::new();
    for (expr_id, remaining) in actor.cooldowns.iter_mut() {
        if *remaining > 0 {
            *remaining -= 1;
        }
        if *remaining <= 0 {
            to_clear.push(expr_id.clone());
        }
    }
    for expr_id in to_clear {
        actor.cooldowns.remove(&expr_id);
    }
}

fn print_use_error(
    err: &superhero_universe::rules::UseError,
    expr: &superhero_universe::rules::ExpressionDef,
    target: &TargetContext,
    actor: &ActorState,
) {
    println!("can_use failed: {:?}", err);
    match err {
        superhero_universe::rules::UseError::Locked => {
            println!("Expression is locked. Use `growth unlock <expression_id>`.");
        }
        superhero_universe::rules::UseError::ConstraintFailed(reason) => {
            println!(
                "Constraint: {} | requires_los={} requires_contact={} range_m={:?}",
                reason,
                expr.constraints.requires_los,
                expr.constraints.requires_contact,
                expr.constraints.range_m
            );
            println!(
                "Context: dist_m={:?} los={} contact={}",
                target.distance_m, target.has_line_of_sight, target.has_contact
            );
        }
        superhero_universe::rules::UseError::OnCooldown => {
            if let Some(turns) = actor.cooldowns.get(&expr.id) {
                println!("Cooldown remaining: {}", turns);
            }
        }
        superhero_universe::rules::UseError::NotEnoughStamina => {
            let required = sum_costs(&expr.costs, CostType::Stamina);
            println!("Stamina: have={}, need={}", actor.stamina, required);
        }
        superhero_universe::rules::UseError::NotEnoughFocus => {
            let required = sum_costs(&expr.costs, CostType::Focus);
            println!("Focus: have={}, need={}", actor.focus, required);
        }
        superhero_universe::rules::UseError::MissingResource => {
            let required = sum_costs(&expr.costs, CostType::Resource);
            let available = actor.resources.get("resource").copied().unwrap_or(0);
            println!("Resource: have={}, need={}", available, required);
        }
    }
}

fn sum_costs(costs: &[superhero_universe::rules::CostSpec], cost_type: CostType) -> i64 {
    costs
        .iter()
        .filter(|c| c.cost_type == cost_type)
        .filter_map(|c| c.value)
        .sum()
}

fn persist_world_state(
    world_db: &mut dyn WorldRepository,
    world: &WorldState,
    game_time: &GameTime,
    city: &CityState,
    cases: &CaseRegistry,
    combat: &CombatState,
    growth: &GrowthState,
    storylet_state: &StoryletState,
) {
    let state = WorldDbState {
        world_turn: world.turn,
        game_time: game_time.clone(),
        city: city.clone(),
        cases: cases.clone(),
        combat: combat.clone(),
        growth: growth.clone(),
        storylet_state: storylet_state.clone(),
    };
    if let Err(err) = world_db.save_state(&state) {
        eprintln!("Failed to persist world state: {}", err);
    }
}

fn load_storylet_library() -> StoryletLibrary {
    StoryletLibrary {
        hero: load_storylet_file("./assets/data/storylets_hero.json"),
        vigilante: load_storylet_file("./assets/data/storylets_vigilante.json"),
        villain: load_storylet_file("./assets/data/storylets_villain.json"),
    }
}

fn load_storylet_file(path: &str) -> Vec<Storylet> {
    match load_storylet_catalog(path) {
        Ok(catalog) => catalog.storylets,
        Err(err) => {
            eprintln!("Failed to load storylets from {}: {}", path, err);
            Vec::new()
        }
    }
}

fn load_civilian_event_library() -> Vec<CivilianStorylet> {
    match load_civilian_event_catalog("./assets/data/civilian_events.json") {
        Ok(catalog) => catalog.events,
        Err(err) => {
            eprintln!("Failed to load civilian events: {}", err);
            Vec::new()
        }
    }
}

fn load_endgame_event_library() -> Vec<EndgameEvent> {
    match load_endgame_event_catalog("./assets/data/endgame_events.json") {
        Ok(catalog) => catalog.events,
        Err(err) => {
            eprintln!("Failed to load endgame events: {}", err);
            Vec::new()
        }
    }
}

struct StoryletContext {
    alignment: Alignment,
    active_persona: Option<PersonaType>,
    flags: HashSet<String>,
    endgame_state: Option<TransformationState>,
    public_suspicion: i32,
    civilian_suspicion: i32,
    wanted_level: i32,
    exposure_risk: i32,
    heat: i32,
    surveillance_level: i32,
    crime_pressure: i32,
    case_progress: i32,
    has_visible_signatures: bool,
    is_day: bool,
    stress: i32,
    reputation: i32,
    pressure_identity: i32,
    pressure_moral: i32,
    pressure_institutional: i32,
    pressure_resource: i32,
    pressure_temporal: i32,
    pressure_psychological: i32,
    flags: HashSet<String>,
}

struct StoryletEligibility {
    eligible: bool,
    matched: Vec<String>,
}

fn list_storylets_all(library: &StoryletLibrary, alignment: Alignment) {
    let storylets = library.for_alignment(alignment);
    if storylets.is_empty() {
        println!("Storylets: none");
        return;
    }
    println!("Storylets ({:?}):", alignment);
    for storylet in storylets {
        let prereqs = if storylet.preconditions.is_empty() {
            "always".to_string()
        } else {
            storylet.preconditions.join(" & ")
        };
        println!("  {} | {:?} | {}", storylet.id, storylet.category, prereqs);
    }
}

fn list_storylets_available(
    library: &StoryletLibrary,
    alignment: Alignment,
    persona_stack: &PersonaStack,
    storylet_state: &StoryletState,
    endgame_state: &EndgameState,
    city: &CityState,
    evidence: &WorldEvidence,
    cases: &CaseRegistry,
    pressure: &PressureState,
    game_time: &GameTime,
) {
    let ctx = build_storylet_context(
        alignment,
        persona_stack,
        storylet_state,
        endgame_state,
        city,
        evidence,
        cases,
        pressure,
        game_time,
    );
    let mut count = 0;
    if storylet_state.punctuation.only {
        println!(
            "Punctuation layer active ({} turns remaining).",
            storylet_state.punctuation.remaining_turns
        );
    }
    if storylet_state.punctuation_cooldown > 0 {
        println!(
            "Punctuation cooldown active ({} turns remaining).",
            storylet_state.punctuation_cooldown
        );
    }
    println!("Storylets available:");
    for storylet in library.for_alignment(alignment) {
        if !storylet_passes_state_gates(storylet, storylet_state) {
            continue;
        }
        let eval = evaluate_storylet(storylet, &ctx);
        if !eval.eligible {
            continue;
        }
        let reason = if eval.matched.is_empty() {
            "always".to_string()
        } else {
            eval.matched.join(" & ")
        };
        println!("  {} | {:?} | {}", storylet.id, storylet.category, reason);
        count += 1;
    }
    if count == 0 {
        println!("  (none)");
    }
}

fn storylet_passes_state_gates(storylet: &Storylet, storylet_state: &StoryletState) -> bool {
    if storylet_state.fired.contains(&storylet.id) {
        return false;
    }
    if storylet_state
        .cooldowns
        .get(&storylet.id)
        .copied()
        .unwrap_or(0)
        > 0
    {
        return false;
    }
    if !storylet_has_gate_requirements(storylet) {
        return false;
    }
    if storylet_state.punctuation.only && !is_punctuation_storylet(storylet) {
        return false;
    }
    if storylet_state.punctuation_cooldown > 0 && is_punctuation_storylet(storylet) {
        return false;
    }
    true
}

fn build_storylet_context(
    alignment: Alignment,
    persona_stack: &PersonaStack,
    storylet_state: &StoryletState,
    endgame_state: &EndgameState,
    city: &CityState,
    evidence: &WorldEvidence,
    cases: &CaseRegistry,
    pressure: &PressureState,
    game_time: &GameTime,
) -> StoryletContext {
    let active_persona = persona_stack.active_persona();
    let (public_suspicion, civilian_suspicion, wanted_level, exposure_risk) = active_persona
        .map(|persona| {
            (
                persona.suspicion.public_suspicion as i32,
                persona.suspicion.civilian_suspicion as i32,
                persona.suspicion.wanted_level as i32,
                persona.suspicion.exposure_risk as i32,
            )
        })
        .unwrap_or((0, 0, 0, 0));

    let location = city.locations.get(&city.active_location);
    let heat = location.map(|loc| loc.heat).unwrap_or(0);
    let surveillance_level = location.map(|loc| loc.surveillance_level).unwrap_or(0);
    let crime_pressure = location.map(|loc| loc.crime_pressure).unwrap_or(0);

    let case_progress = cases
        .cases
        .iter()
        .filter(|case| case.location_id == city.active_location)
        .map(|case| case.progress as i32)
        .max()
        .unwrap_or(0);

    let has_visible_signatures = evidence.signatures.iter().any(|event| {
        event.location_id == city.active_location && event.signature.remaining_turns > 0
    });
    let flags = storylet_state
        .flags
        .iter()
        .filter_map(|(flag, enabled)| enabled.then(|| flag.clone()))
        .collect();

    let flags = storylet_state
        .flags
        .iter()
        .filter_map(|(flag, enabled)| if *enabled { Some(flag.clone()) } else { None })
        .collect();

    StoryletContext {
        alignment,
        active_persona: active_persona.map(|persona| persona.persona_type),
        flags,
        endgame_state: endgame_state.phase,
        public_suspicion,
        civilian_suspicion,
        wanted_level,
        exposure_risk,
        heat,
        surveillance_level,
        crime_pressure,
        case_progress,
        has_visible_signatures,
        is_day: game_time.is_day,
        stress: 0,
        reputation: 0,
        pressure_identity: pressure.identity.round() as i32,
        pressure_moral: pressure.moral.round() as i32,
        pressure_institutional: pressure.institutional.round() as i32,
        pressure_resource: pressure.resource.round() as i32,
        pressure_temporal: pressure.temporal.round() as i32,
        pressure_psychological: pressure.psychological.round() as i32,
        flags,
    }
}

fn select_storylet_for_turn<'a>(
    library: &'a StoryletLibrary,
    alignment: Alignment,
    storylet_state: &mut StoryletState,
    ctx: &StoryletContext,
) -> Option<&'a Storylet> {
    for storylet in library.for_alignment(alignment) {
        if !storylet_passes_state_gates(storylet, storylet_state) {
            continue;
        }
        if !evaluate_storylet(storylet, ctx).eligible {
            continue;
        }
        storylet_state.fired.insert(storylet.id.clone());
        if is_punctuation_storylet(storylet) {
            storylet_state.punctuation_cooldown = DEFAULT_PUNCTUATION_COOLDOWN_TURNS;
        }
        return Some(storylet);
    }
    None
}

fn evaluate_storylet(storylet: &Storylet, ctx: &StoryletContext) -> StoryletEligibility {
    let mut matched = Vec::new();
    for condition in &storylet.preconditions {
        if eval_condition(condition, ctx) {
            matched.push(condition.clone());
        } else {
            return StoryletEligibility {
                eligible: false,
                matched: Vec::new(),
            };
        }
    }
    StoryletEligibility {
        eligible: true,
        matched,
    }
}

fn eval_condition(condition: &str, ctx: &StoryletContext) -> bool {
    let cond = condition.trim();
    if cond.is_empty() {
        return true;
    }
    if cond == "time.is_day" {
        return ctx.is_day;
    }
    if cond == "time.is_night" {
        return !ctx.is_day;
    }
    if cond == "signatures.visible" {
        return ctx.has_visible_signatures;
    }
    if let Some(flag) = cond.strip_prefix("flag.") {
        return ctx.flags.contains(flag);
    }

    let parts: Vec<&str> = cond.split_whitespace().collect();
    if parts.len() != 3 {
        return false;
    }

    let left = parts[0];
    let op = parts[1];
    let right = parts[2];

    match left {
        "alignment" => {
            let Some(expected) = parse_alignment(right) else {
                return false;
            };
            match op {
                "==" => ctx.alignment == expected,
                "!=" => ctx.alignment != expected,
                _ => false,
            }
        }
        "persona" => {
            let Some(expected) = parse_persona_type(right) else {
                return false;
            };
            match op {
                "==" => ctx.active_persona == Some(expected),
                "!=" => ctx.active_persona != Some(expected),
                _ => false,
            }
        }
        "endgame.state" => {
            let Some(expected) = parse_endgame_state(right) else {
                return false;
            };
            match op {
                "==" => ctx.endgame_state == Some(expected),
                "!=" => ctx.endgame_state != Some(expected),
                _ => false,
            }
        }
        _ => {
            let Some(left_value) = numeric_metric(left, ctx) else {
                return false;
            };
            let Ok(right_value) = right.parse::<i32>() else {
                return false;
            };
            compare_numeric(left_value, right_value, op)
        }
    }
}

fn parse_alignment(value: &str) -> Option<Alignment> {
    match value.to_ascii_uppercase().as_str() {
        "HERO" => Some(Alignment::Hero),
        "VIGILANTE" => Some(Alignment::Vigilante),
        "VILLAIN" => Some(Alignment::Villain),
        _ => None,
    }
}

fn parse_persona_type(value: &str) -> Option<PersonaType> {
    match value.to_ascii_uppercase().as_str() {
        "CIVILIAN" => Some(PersonaType::Civilian),
        "MASKED" => Some(PersonaType::Masked),
        _ => None,
    }
}

fn parse_endgame_state(value: &str) -> Option<TransformationState> {
    match value.to_ascii_uppercase().as_str() {
        "EXPOSED" => Some(TransformationState::Exposed),
        "REGISTRATION" => Some(TransformationState::Registration),
        "COSMIC_JUDGEMENT" | "COSMIC_JUDGMENT" => Some(TransformationState::CosmicJudgement),
        "ASCENSION" => Some(TransformationState::Ascension),
        "EXILE" => Some(TransformationState::Exile),
        _ => None,
    }
}

fn numeric_metric(key: &str, ctx: &StoryletContext) -> Option<i32> {
    match key {
        "public.suspicion" => Some(ctx.public_suspicion),
        "civilian.suspicion" => Some(ctx.civilian_suspicion),
        "wanted.level" => Some(ctx.wanted_level),
        "exposure.risk" => Some(ctx.exposure_risk),
        "heat" => Some(ctx.heat),
        "surveillance.level" => Some(ctx.surveillance_level),
        "gang.pressure" | "crime.pressure" => Some(ctx.crime_pressure),
        "case.progress" => Some(ctx.case_progress),
        "stress" => Some(ctx.stress),
        "reputation" => Some(ctx.reputation),
        "pressure.identity" => Some(ctx.pressure_identity),
        "pressure.moral" => Some(ctx.pressure_moral),
        "pressure.institutional" => Some(ctx.pressure_institutional),
        "pressure.resource" => Some(ctx.pressure_resource),
        "pressure.temporal" => Some(ctx.pressure_temporal),
        "pressure.psychological" => Some(ctx.pressure_psychological),
        _ => None,
    }
}

fn compare_numeric(left: i32, right: i32, op: &str) -> bool {
    match op {
        ">=" => left >= right,
        "<=" => left <= right,
        ">" => left > right,
        "<" => left < right,
        "==" => left == right,
        "!=" => left != right,
        _ => false,
    }
}

fn print_tick_summary(
    world: &WorldState,
    stack: &PersonaStack,
    city: &CityState,
    cases: &CaseRegistry,
    pressure: &PressureState,
    endgame_state: &EndgameState,
) {
    let location = city.locations.get(&city.active_location);
    let (heat, response, crime) = location
        .map(|loc| (loc.heat, loc.response, loc.crime_pressure))
        .unwrap_or((
            0,
            superhero_universe::simulation::city::HeatResponse::None,
            0,
        ));
    println!(
        "Turn {} | heat={} response={:?} crime_pressure={}",
        world.turn, heat, response, crime
    );
    if let Some(active) = stack.active_persona() {
        println!(
            "Active persona {} ({:?}) -> public={} civilian={} wanted={} exposure={}",
            active.persona_id,
            active.persona_type,
            active.suspicion.public_suspicion,
            active.suspicion.civilian_suspicion,
            active.suspicion.wanted_level,
            active.suspicion.exposure_risk
        );
    }
    let max_progress = cases
        .cases
        .iter()
        .filter(|case| case.location_id == city.active_location)
        .map(|case| case.progress)
        .max()
        .unwrap_or(0);
    if max_progress > 0 {
        println!("Case progress (max): {}", max_progress);
    }
    println!(
        "Pressure: temporal={:.1} identity={:.1} institutional={:.1} moral={:.1} resource={:.1} psychological={:.1}",
        pressure.temporal,
        pressure.identity,
        pressure.institutional,
        pressure.moral,
        pressure.resource,
        pressure.psychological
    );
    let modifiers = endgame_state.modifiers();
    println!(
        "Endgame: {} | modifiers cost={:.2} risk={:.2}",
        endgame_state.label(),
        modifiers.cost_scale,
        modifiers.risk_scale
    );
}

fn print_origin_paths(paths: &[OriginPathDefinition], seed: u64) {
    if paths.is_empty() {
        println!("No origin paths available for seed {}.", seed);
        return;
    }
    println!("Origin paths (seed={}):", seed);
    for path in paths {
        println!("  {} - {}: {}", path.id, path.label, path.summary);
        if !path.stages.is_empty() {
            let stage_ids: Vec<&str> = path.stages.iter().map(|stage| stage.id.as_str()).collect();
            println!("    stages: {}", stage_ids.join(" -> "));
        }
    }
}

fn print_origin_path_status(state: &OriginQuestState, catalog: &OriginPathCatalog) {
    let Some(path_id) = state.path_id.as_deref() else {
        println!("Origin path: none selected.");
        return;
    };
    let path = catalog.paths.iter().find(|path| path.id == path_id);
    let Some(path) = path else {
        println!("Origin path: {} (missing definition)", path_id);
        return;
    };
    println!("Origin path: {} - {}", path.label, path.summary);
    if state.completed {
        println!(
            "  status: complete ({} stages)",
            state.completed_stages.len()
        );
        return;
    }
    if let Some(stage) = current_origin_stage(state, catalog) {
        let needed = stage.requirement.progress_needed.max(1);
        println!(
            "  stage: {} ({}) progress {}/{}",
            stage.id, stage.label, state.stage_progress, needed
        );
    } else {
        println!("  stage: none");
    }
}

fn apply_origin_rewards(rewards: &[OriginStageReward], pressure: &mut PressureState) {
    if rewards.is_empty() {
        return;
    }
    for reward in rewards {
        if reward.reputation_delta != 0 {
            println!("Origin reward: reputation {}", reward.reputation_delta);
        }
        if reward.pressure_delta.temporal != 0.0
            || reward.pressure_delta.identity != 0.0
            || reward.pressure_delta.institutional != 0.0
            || reward.pressure_delta.moral != 0.0
            || reward.pressure_delta.resource != 0.0
            || reward.pressure_delta.psychological != 0.0
        {
            pressure.temporal =
                (pressure.temporal + reward.pressure_delta.temporal).clamp(0.0, 100.0);
            pressure.identity =
                (pressure.identity + reward.pressure_delta.identity).clamp(0.0, 100.0);
            pressure.institutional =
                (pressure.institutional + reward.pressure_delta.institutional).clamp(0.0, 100.0);
            pressure.moral = (pressure.moral + reward.pressure_delta.moral).clamp(0.0, 100.0);
            pressure.resource =
                (pressure.resource + reward.pressure_delta.resource).clamp(0.0, 100.0);
            pressure.psychological =
                (pressure.psychological + reward.pressure_delta.psychological).clamp(0.0, 100.0);
            println!("Origin reward: pressure updated.");
        }
        if !reward.mutation_tags.is_empty() {
            println!(
                "Origin reward: mutations {}",
                reward.mutation_tags.join(", ")
            );
        }
        if let Some(notes) = reward.notes.as_deref() {
            println!("Origin reward: {}", notes);
        }
    }
}
