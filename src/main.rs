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
use superhero_universe::data::storylets::{load_storylet_catalog, Storylet};
use superhero_universe::rules::{
    can_use, use_power, ActorState, CostType, PressureModifiers, TargetContext, UseContext,
    WorldState,
};
use superhero_universe::simulation::case::{CaseEventLog, CaseRegistry};
use superhero_universe::simulation::city::{CityEventLog, CityState};
use superhero_universe::simulation::civilian::{
    apply_civilian_effects, tick_civilian_life, CivilianState,
};
use superhero_universe::simulation::combat::{
    CombatEnd, CombatIntent, CombatOutcome, CombatScale, CombatState,
};
use superhero_universe::simulation::endgame::{evaluate_transformation, TransformationState};
use superhero_universe::simulation::evidence::WorldEvidence;
use superhero_universe::simulation::growth::{
    record_expression_use, select_evolution_candidate, GrowthState,
};
use superhero_universe::simulation::identity_evidence::{IdentityEvidenceStore, PersonaHint};
use superhero_universe::simulation::pressure::PressureState;
use superhero_universe::simulation::region::{RegionEventLog, RegionState};
use superhero_universe::simulation::storylet_state::StoryletState;
use superhero_universe::simulation::storylets::StoryletLibrary;
use superhero_universe::simulation::time::GameTime;
use superhero_universe::systems::case::update_cases;
use superhero_universe::systems::civilian::apply_civilian_pressure;
use superhero_universe::systems::combat_loop::{
    combat_tick, force_escalate, force_escape, resolve_combat, start_combat,
};
use superhero_universe::systems::event_resolver::{
    resolve_faction_events, ResolvedFactionEventLog,
};
use superhero_universe::systems::faction::{
    run_faction_director, FactionDirector, FactionEventLog,
};
use superhero_universe::systems::heat::{apply_signatures, decay_heat, WorldEventLog};
use superhero_universe::systems::persona::{attempt_switch, PersonaSwitchError};
use superhero_universe::systems::pressure::update_pressure;
use superhero_universe::systems::region::{
    run_global_faction_director, run_region_update, GlobalFactionDirector, GlobalFactionEventLog,
};
use superhero_universe::systems::suspicion::apply_suspicion_for_intents;
use superhero_universe::systems::units::update_units;
use superhero_universe::world::{WorldDb, WorldDbState, WorldRepository};

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
    let mut cases = world_state.cases;
    let mut case_log = CaseEventLog::default();
    let mut combat = world_state.combat;
    let mut transformation_state: Option<TransformationState> = None;
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
    world.pressure = pressure.to_modifiers();
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

    println!("Commands: stats | power <id> | use <expression_id> | ctx | loc | persona | personas | growth [expr|unlock|mastery] | switch <persona_id> | storylets [all] | civilian [events|resolve <event_id> <choice_id>] | set <field> <value> | cd | scene | events | cases | combat <start|use|intent|tick|log|resolve|force_escape|force_escalate> | tick [n] | quit");
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
                println!("Commands: stats | power <id> | use <expression_id> | ctx | loc | persona | personas | growth [expr|unlock|mastery] | switch <persona_id> | storylets [all] | civilian [events|resolve <event_id> <choice_id>] | set <field> <value> | cd | scene | events | cases | combat <start|use|intent|tick|log|resolve|force_escape|force_escalate> | tick [n] | quit");
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
                print_context(&target, &world, &city, &pressure);
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
                                        world.pressure = pressure.to_modifiers();
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
                                        handle_transformation(
                                            &cases,
                                            &pressure,
                                            &resolved_faction_events,
                                            &mut storylet_state,
                                            &mut transformation_state,
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
                        &city,
                        &evidence,
                        &cases,
                        &game_time,
                    );
                }
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
                                world.pressure = pressure.to_modifiers();
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
                                handle_transformation(
                                    &cases,
                                    &pressure,
                                    &resolved_faction_events,
                                    &mut storylet_state,
                                    &mut transformation_state,
                                );

                                if let Some(end_reason) = tick_result.ended {
                                    if let Some(outcome) = tick_result.outcome.as_ref() {
                                        apply_combat_outcome(
                                            outcome,
                                            combat.location_id,
                                            world.turn,
                                            &target,
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
                                            &mut pressure,
                                            &mut world,
                                        );
                                        combat.last_outcome = None;
                                    }
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
                            if let Some(outcome) = combat.last_outcome.take() {
                                apply_combat_outcome(
                                    &outcome,
                                    combat.location_id,
                                    world.turn,
                                    &target,
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
                                    &mut pressure,
                                    &mut world,
                                );
                            }
                            println!("Combat ended: {}", format_combat_end(end_reason));
                        } else {
                            println!("No active combat.");
                        }
                    }
                    "force_escape" => {
                        if let Some(end_reason) = force_escape(&mut combat) {
                            if let Some(outcome) = combat.last_outcome.take() {
                                apply_combat_outcome(
                                    &outcome,
                                    combat.location_id,
                                    world.turn,
                                    &target,
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
                                    &mut pressure,
                                    &mut world,
                                );
                            }
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
                    &mut persona_stack,
                    alignment,
                    &mut player_pos,
                    &mut game_time,
                    &mut civilian_state,
                    &mut storylet_state,
                    &mut pressure,
                    &mut region,
                    &mut region_events,
                    &mut global_faction_director,
                    &mut global_faction_events,
                    count,
                );
                handle_transformation(
                    &cases,
                    &pressure,
                    &resolved_faction_events,
                    &mut storylet_state,
                    &mut transformation_state,
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
                print_tick_summary(&world, &persona_stack, &city, &cases, &pressure);
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
    println!("  Job: {:?}", state.job_status);
    println!(
        "  Finances: cash={} debt={} rent={} rent_due_in={} wage={}",
        state.finances.cash,
        state.finances.debt,
        state.finances.rent,
        state.finances.rent_due_in,
        state.finances.wage
    );
    println!(
        "  Social: support={} strain={} obligation={}",
        state.social.support, state.social.strain, state.social.obligation
    );
    println!(
        "  Civilian pressure targets: temporal={:.1} resource={:.1} moral={:.1}",
        pressure.temporal, pressure.resource, pressure.moral
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
    let witness_count = if in_public { witnesses.max(1) } else { 0 };
    let visual_quality = (surveillance as i32 + (witness_count as i32 * 10)).clamp(0, 100) as u8;
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

fn apply_action_signatures(
    signatures: &[superhero_universe::rules::SignatureInstance],
    location_id: superhero_universe::simulation::city::LocationId,
    turn: u64,
    witnesses: u32,
    in_public: bool,
    persona_hint: PersonaHint,
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

fn apply_combat_outcome(
    outcome: &CombatOutcome,
    location_id: superhero_universe::simulation::city::LocationId,
    turn: u64,
    target: &TargetContext,
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
    pressure: &mut PressureState,
    world: &mut WorldState,
) {
    let before_pressure = *pressure;
    outcome.pressure_delta.apply(pressure);
    world.pressure = pressure.to_modifiers();

    let persona_hint = match persona_stack
        .active_persona()
        .map(|persona| persona.persona_type)
    {
        Some(PersonaType::Civilian) => PersonaHint::Civilian,
        Some(PersonaType::Masked) => PersonaHint::Masked,
        None => PersonaHint::Unknown,
    };

    let witnesses = target.witnesses.saturating_add(outcome.witness_bonus);
    apply_action_signatures(
        &outcome.signatures,
        location_id,
        turn,
        witnesses,
        target.in_public,
        persona_hint,
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

    let max_case_progress = cases
        .cases
        .iter()
        .filter(|case| case.location_id == location_id)
        .map(|case| case.progress)
        .max()
        .unwrap_or(0);
    println!(
        "Combat outcome applied: {:?} signatures={} witnesses={} pressure_delta=({:.1},{:.1},{:.1},{:.1},{:.1},{:.1})",
        outcome.end,
        outcome.signatures.len(),
        witnesses,
        outcome.pressure_delta.temporal,
        outcome.pressure_delta.identity,
        outcome.pressure_delta.institutional,
        outcome.pressure_delta.moral,
        outcome.pressure_delta.resource,
        outcome.pressure_delta.psychological
    );
    println!(
        "Pressure: temporal={:.1}->{:.1} identity={:.1}->{:.1} institutional={:.1}->{:.1} moral={:.1}->{:.1} resource={:.1}->{:.1} psychological={:.1}->{:.1}",
        before_pressure.temporal,
        pressure.temporal,
        before_pressure.identity,
        pressure.identity,
        before_pressure.institutional,
        pressure.institutional,
        before_pressure.moral,
        pressure.moral,
        before_pressure.resource,
        pressure.resource,
        before_pressure.psychological,
        pressure.psychological
    );
    println!(
        "Evidence totals: signatures={} identity_items={} max_case_progress_at_location={}",
        evidence.signatures.len(),
        identity_evidence.items.len(),
        max_case_progress
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
    persona_stack: &mut PersonaStack,
    alignment: Alignment,
    position: &mut Position,
    game_time: &mut GameTime,
    civilian_state: &mut CivilianState,
    storylet_state: &mut StoryletState,
    pressure: &mut PressureState,
    region: &mut RegionState,
    region_events: &mut RegionEventLog,
    global_faction_director: &mut GlobalFactionDirector,
    global_faction_events: &mut GlobalFactionEventLog,
    turns: u32,
) {
    for _ in 0..turns {
        world.turn += 1;
        tick_cooldowns(actor);
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
        update_units(city);
        scene.tick_decay();
        decay_heat(city, cases, city_events);
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
        game_time.advance();
        tick_civilian_life(civilian_state, game_time);
        storylet_state.tick();
        update_pressure(pressure, city, scene, cases, game_time);
        apply_civilian_pressure(civilian_state, pressure);
        world.pressure = pressure.to_modifiers();
        run_region_update(region, city, pressure, city_events, region_events);
        run_global_faction_director(global_faction_director, region, global_faction_events);
    }
}

fn handle_transformation(
    cases: &CaseRegistry,
    pressure: &PressureState,
    faction_events: &ResolvedFactionEventLog,
    storylet_state: &mut StoryletState,
    last_state: &mut Option<TransformationState>,
) {
    if let Some(event) = evaluate_transformation(cases, pressure, faction_events) {
        let flag = transformation_flag(event.state);
        if storylet_state.flags.get(flag).copied().unwrap_or(false) {
            return;
        }
        storylet_state.flags.insert(flag.to_string(), true);
        *last_state = Some(event.state);
        println!(
            "Transformation triggered ({:?}): {}",
            event.trigger,
            transformation_text(event.state)
        );
    }
}

fn transformation_flag(state: TransformationState) -> &'static str {
    match state {
        TransformationState::Exposed => "transformation_exposed",
        TransformationState::Registration => "transformation_registration",
        TransformationState::CosmicJudgement => "transformation_cosmic_judgement",
        TransformationState::Ascension => "transformation_ascension",
        TransformationState::Exile => "transformation_exile",
    }
}

fn transformation_text(state: TransformationState) -> &'static str {
    match state {
        TransformationState::Exposed => {
            "Your cover breaks. The city has a name and a face for you."
        }
        TransformationState::Registration => {
            "The registries open. Compliance or resistance becomes the story."
        }
        TransformationState::CosmicJudgement => {
            "The signal rises beyond the city. Something vast takes notice."
        }
        TransformationState::Ascension => {
            "Your power crests. The world bends to the new gravity you carry."
        }
        TransformationState::Exile => "Faction attention becomes a net. Retreat to survive.",
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

struct StoryletContext {
    alignment: Alignment,
    active_persona: Option<PersonaType>,
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
    city: &CityState,
    evidence: &WorldEvidence,
    cases: &CaseRegistry,
    game_time: &GameTime,
) {
    let ctx = build_storylet_context(alignment, persona_stack, city, evidence, cases, game_time);
    let mut count = 0;
    println!("Storylets available:");
    for storylet in library.for_alignment(alignment) {
        if storylet_state.fired.contains(&storylet.id) {
            continue;
        }
        if storylet_state
            .cooldowns
            .get(&storylet.id)
            .copied()
            .unwrap_or(0)
            > 0
        {
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

fn build_storylet_context(
    alignment: Alignment,
    persona_stack: &PersonaStack,
    city: &CityState,
    evidence: &WorldEvidence,
    cases: &CaseRegistry,
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

    StoryletContext {
        alignment,
        active_persona: active_persona.map(|persona| persona.persona_type),
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
    }
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
}
