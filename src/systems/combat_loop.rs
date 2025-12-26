use crate::rules::expression::{ExpressionDef, ExpressionForm};
use crate::rules::mastery::MasteryStage;
use crate::rules::power::ExpressionId;
use crate::rules::signature::{SignatureInstance, SignatureSpec, SignatureType};
use crate::rules::use_power::{use_power, ActorState, TargetContext, UseContext, UseError, WorldState};
use crate::simulation::combat::{
    CombatConsequence, CombatConsequences, CombatEnd, CombatIntent, CombatPressureDelta,
    CombatScale, CombatSide, CombatState, Combatant,
};
use crate::simulation::city::LocationId;

#[derive(Debug)]
pub struct CombatTickResult {
    pub emitted_signatures: Vec<SignatureInstance>,
    pub ended: Option<CombatEnd>,
    pub escalated: bool,
    pub post_combat_consequences: Option<CombatConsequences>,
    pub used_expression_id: Option<ExpressionId>,
    pub used_success: bool,
}

impl Default for CombatTickResult {
    fn default() -> Self {
        Self {
            emitted_signatures: Vec::new(),
            ended: None,
            escalated: false,
            post_combat_consequences: None,
            used_expression_id: None,
            used_success: false,
        }
    }
}

pub fn start_combat(
    state: &mut CombatState,
    location_id: LocationId,
    source: &str,
    scale: CombatScale,
    player_name: &str,
    opponent_count: u32,
    seed: u64,
) {
    state.active = true;
    state.source = source.to_string();
    state.location_id = location_id;
    state.scale = scale;
    state.tick = 0;
    state.log.clear();
    state.combatants.clear();
    state.pending_player_expression = None;
    state.escape_progress = 0;
    state.rng_state = seed_combat_rng(seed, location_id, scale, source, opponent_count);

    state.combatants.push(Combatant {
        id: 1,
        name: player_name.to_string(),
        side: CombatSide::Player,
        stress: 0,
        intent: CombatIntent::Attack,
        is_player: true,
    });

    for i in 0..opponent_count {
        state.combatants.push(Combatant {
            id: 100 + i,
            name: format!("Opponent {}", i + 1),
            side: CombatSide::Opponent,
            stress: 0,
            intent: CombatIntent::Attack,
            is_player: false,
        });
    }

    state
        .log
        .push(format!("Combat started: {} ({:?})", source, scale));
}

pub fn combat_tick(
    state: &mut CombatState,
    actor: &mut ActorState,
    world: &WorldState,
    target: &TargetContext,
    player_expr: Option<&ExpressionDef>,
    mastery_stage: Option<MasteryStage>,
    unlocked: Option<&std::collections::HashSet<ExpressionId>>,
) -> CombatTickResult {
    let mut result = CombatTickResult::default();
    if !state.active {
        return result;
    }

    state.tick += 1;
    state
        .log
        .push(format!("-- Combat tick {} --", state.tick));

    let player_intent = state
        .player()
        .map(|p| p.intent)
        .unwrap_or(CombatIntent::Hold);

    if player_intent == CombatIntent::Escape {
        state.escape_progress = state.escape_progress.saturating_add(1);
        state.log.push("Player attempts to escape.".to_string());
    } else {
        state.escape_progress = 0;
    }

    let allow_power = player_intent != CombatIntent::Escape;
    if allow_power {
        if let Some(expr) = player_expr {
            let mut ctx = UseContext {
                actor,
                world,
                mastery: mastery_stage,
                unlocked,
            };
            match use_power(&mut ctx, expr, target) {
                Ok(use_result) => {
                    result.emitted_signatures.extend(use_result.emitted_signatures);
                    result.used_expression_id = Some(expr.id.clone());
                    result.used_success = true;
                    let stress = stress_from_form(expr.form);
                    if let Some(target_idx) = state
                        .combatants
                        .iter()
                        .position(|c| c.side == CombatSide::Opponent && c.stress < 100)
                    {
                        let target_name = state.combatants[target_idx].name.clone();
                        let next = state.combatants[target_idx].stress + stress;
                        state.combatants[target_idx].stress = next;
                        state.log.push(format!(
                            "{} hits {} (stress +{}).",
                            "Player", target_name, stress
                        ));
                    }
                }
                Err(err) => log_use_failure(state, err),
            }
        } else {
            state
                .log
                .push("Player hesitates (no expression queued).".to_string());
        }
    } else {
        state
            .log
            .push("Player focuses on escape (no power use).".to_string());
    }

    let mut npc_attackers = 0;
    let mut npc_signatures = Vec::new();
    for opponent in state
        .combatants
        .iter_mut()
        .filter(|c| c.side == CombatSide::Opponent && c.stress < 100)
    {
        if opponent.stress >= 70 {
            opponent.intent = CombatIntent::Escape;
        }
        match opponent.intent {
            CombatIntent::Attack => {
                npc_attackers += 1;
                state
                    .log
                    .push(format!("{} presses the attack.", opponent.name));
                npc_signatures.push(default_npc_signature(state.scale));
            }
            CombatIntent::Escape => {
                opponent.stress = 100;
                state.log.push(format!("{} flees.", opponent.name));
            }
            _ => {
                state.log.push(format!("{} holds position.", opponent.name));
            }
        }
    }

    let npc_stress = npc_attackers * npc_stress_from_scale(state.scale);
    if npc_stress > 0 {
        if let Some(player) = state.player_mut() {
            player.stress += npc_stress;
            state.log.push(format!(
                "Player takes pressure (stress +{}).",
                npc_stress
            ));
        }
    }

    result.emitted_signatures.extend(npc_signatures);

    let ended = evaluate_combat_end(state);
    if let Some(end_reason) = ended {
        finish_combat(state, end_reason);
        result.ended = Some(end_reason);
        result.post_combat_consequences =
            Some(combat_post_consequences(state, end_reason, target));
        return finalize_signatures(state.scale, result);
    }

    let intensity: i64 = result
        .emitted_signatures
        .iter()
        .map(|sig| sig.signature.strength)
        .sum();
    if intensity >= escalation_threshold(state.scale) {
        if let Some(next) = next_scale(state.scale) {
            state.scale = next;
            state
                .log
                .push(format!("Combat escalates to {:?}.", next));
            result.escalated = true;
        }
    }

    finalize_signatures(state.scale, result)
}

pub fn force_escape(state: &mut CombatState) -> Option<CombatEnd> {
    if !state.active {
        return None;
    }
    finish_combat(state, CombatEnd::PlayerEscaped);
    Some(CombatEnd::PlayerEscaped)
}

pub fn force_escalate(state: &mut CombatState) -> bool {
    if !state.active {
        return false;
    }
    if let Some(next) = next_scale(state.scale) {
        state.scale = next;
        state
            .log
            .push(format!("Combat escalates to {:?} (forced).", next));
        true
    } else {
        false
    }
}

pub fn resolve_combat(state: &mut CombatState) -> Option<CombatEnd> {
    if !state.active {
        return None;
    }
    finish_combat(state, CombatEnd::Resolved);
    Some(CombatEnd::Resolved)
}

pub fn combat_end_consequences(
    end: CombatEnd,
    scale: CombatScale,
    combat_consequence: CombatConsequence,
) -> CombatConsequences {
    let (strength, persistence) = match scale {
        CombatScale::Street => (18, 4),
        CombatScale::District => (24, 5),
        CombatScale::City => (30, 6),
        CombatScale::National => (38, 7),
        CombatScale::Cosmic => (46, 8),
    };

    let signatures = match end {
        CombatEnd::PlayerEscaped => vec![
            SignatureSpec {
                signature_type: SignatureType::AcousticShock,
                strength,
                persistence_turns: persistence,
            },
            SignatureSpec {
                signature_type: SignatureType::VisualAnomaly,
                strength: strength.saturating_sub(6),
                persistence_turns: persistence,
            },
        ],
        CombatEnd::PlayerDefeated => vec![
            SignatureSpec {
                signature_type: SignatureType::BioMarker,
                strength,
                persistence_turns: persistence + 1,
            },
            SignatureSpec {
                signature_type: SignatureType::KineticStress,
                strength: strength.saturating_sub(4),
                persistence_turns: persistence,
            },
        ],
        CombatEnd::OpponentsDefeated => vec![
            SignatureSpec {
                signature_type: SignatureType::KineticStress,
                strength,
                persistence_turns: persistence,
            },
            SignatureSpec {
                signature_type: SignatureType::VisualAnomaly,
                strength: strength.saturating_sub(8),
                persistence_turns: persistence,
            },
        ],
        CombatEnd::Resolved => vec![
            SignatureSpec {
                signature_type: SignatureType::CausalImprint,
                strength,
                persistence_turns: persistence + 1,
            },
            SignatureSpec {
                signature_type: SignatureType::PsychicEcho,
                strength: strength.saturating_sub(10),
                persistence_turns: persistence,
            },
        ],
    };

    let scale_factor = match scale {
        CombatScale::Street => 1.0,
        CombatScale::District => 1.2,
        CombatScale::City => 1.4,
        CombatScale::National => 1.7,
        CombatScale::Cosmic => 2.0,
    };

    let base_delta = match end {
        CombatEnd::PlayerEscaped => CombatPressureDelta {
            temporal: 1.5,
            identity: 1.2,
            institutional: 0.6,
            moral: 0.5,
            resource: 0.3,
            psychological: 1.0,
        },
        CombatEnd::PlayerDefeated => CombatPressureDelta {
            temporal: 0.8,
            identity: 2.8,
            institutional: 1.6,
            moral: 2.2,
            resource: 0.9,
            psychological: 2.6,
        },
        CombatEnd::OpponentsDefeated => CombatPressureDelta {
            temporal: 0.7,
            identity: 1.8,
            institutional: 1.3,
            moral: 1.9,
            resource: 0.6,
            psychological: 0.8,
        },
        CombatEnd::Resolved => CombatPressureDelta {
            temporal: 0.6,
            identity: 1.0,
            institutional: 1.4,
            moral: 0.9,
            resource: 1.1,
            psychological: 0.7,
        },
    };

    let pressure_delta = CombatPressureDelta {
        temporal: base_delta.temporal * scale_factor,
        identity: base_delta.identity * scale_factor * identity_factor(combat_consequence),
        institutional: base_delta.institutional * scale_factor * institutional_factor(combat_consequence),
        moral: base_delta.moral * scale_factor * collateral_factor(combat_consequence),
        resource: base_delta.resource * scale_factor * resource_factor(combat_consequence),
        psychological: base_delta.psychological * scale_factor * notoriety_factor(combat_consequence),
    };

    CombatConsequences {
        signatures: signatures
            .into_iter()
            .map(|spec| spec.to_instance())
            .collect(),
        pressure_delta,
        combat_consequence,
    }
}

fn finalize_signatures(scale: CombatScale, mut result: CombatTickResult) -> CombatTickResult {
    result.emitted_signatures = amplify_signatures(&result.emitted_signatures, scale);
    result
}

fn finish_combat(state: &mut CombatState, reason: CombatEnd) {
    state.active = false;
    state.pending_player_expression = None;
    state.escape_progress = 0;
    state.log.push(format!("Combat ends: {:?}.", reason));
}

fn evaluate_combat_end(state: &mut CombatState) -> Option<CombatEnd> {
    let player_down = state
        .player()
        .map(|p| p.stress >= 100)
        .unwrap_or(false);
    if player_down {
        return Some(CombatEnd::PlayerDefeated);
    }

    let opponents_left = state.active_opponent_count();
    if opponents_left == 0 {
        return Some(CombatEnd::OpponentsDefeated);
    }

    let player_escaping = state
        .player()
        .map(|p| p.intent == CombatIntent::Escape)
        .unwrap_or(false);
    if player_escaping && state.escape_progress >= 2 {
        return Some(CombatEnd::PlayerEscaped);
    }

    None
}

fn stress_from_form(form: ExpressionForm) -> i32 {
    match form {
        ExpressionForm::Beam | ExpressionForm::Projectile => 24,
        ExpressionForm::Zone | ExpressionForm::Aura => 20,
        ExpressionForm::Touch => 16,
        ExpressionForm::Summon => 28,
        ExpressionForm::Movement => 10,
        ExpressionForm::Sense => 6,
        ExpressionForm::Construct => 18,
        ExpressionForm::Passive => 0,
    }
}

fn npc_stress_from_scale(scale: CombatScale) -> i32 {
    match scale {
        CombatScale::Street => 6,
        CombatScale::District => 8,
        CombatScale::City => 10,
        CombatScale::National => 12,
        CombatScale::Cosmic => 16,
    }
}

fn default_npc_signature(scale: CombatScale) -> SignatureInstance {
    let base = match scale {
        CombatScale::Street => 12,
        CombatScale::District => 16,
        CombatScale::City => 20,
        CombatScale::National => 26,
        CombatScale::Cosmic => 32,
    };
    SignatureSpec {
        signature_type: SignatureType::KineticStress,
        strength: base,
        persistence_turns: 3,
    }
    .to_instance()
}

fn escalation_threshold(scale: CombatScale) -> i64 {
    match scale {
        CombatScale::Street => 70,
        CombatScale::District => 95,
        CombatScale::City => 120,
        CombatScale::National => 150,
        CombatScale::Cosmic => 999,
    }
}

fn next_scale(scale: CombatScale) -> Option<CombatScale> {
    match scale {
        CombatScale::Street => Some(CombatScale::District),
        CombatScale::District => Some(CombatScale::City),
        CombatScale::City => Some(CombatScale::National),
        CombatScale::National => Some(CombatScale::Cosmic),
        CombatScale::Cosmic => None,
    }
}

fn amplify_signatures(signatures: &[SignatureInstance], scale: CombatScale) -> Vec<SignatureInstance> {
    let bonus = match scale {
        CombatScale::Street => (6, 1),
        CombatScale::District => (10, 1),
        CombatScale::City => (14, 2),
        CombatScale::National => (18, 3),
        CombatScale::Cosmic => (24, 4),
    };

    signatures
        .iter()
        .map(|sig| {
            let mut boosted = sig.clone();
            boosted.signature.strength += bonus.0;
            boosted.signature.persistence_turns += bonus.1;
            boosted.remaining_turns += bonus.1;
            boosted
        })
        .collect()
}

fn log_use_failure(state: &mut CombatState, err: UseError) {
    state.log.push(format!("Power use failed: {:?}", err));
}

pub fn combat_post_consequences(
    state: &mut CombatState,
    end: CombatEnd,
    target: &TargetContext,
) -> CombatConsequences {
    let combat_consequence = combat_consequence_metadata(state, end, target);
    combat_end_consequences(end, state.scale, combat_consequence)
}

fn combat_consequence_metadata(
    state: &mut CombatState,
    end: CombatEnd,
    target: &TargetContext,
) -> CombatConsequence {
    let public_base = if target.in_public { 55 } else { 20 };
    let witness_boost = (target.witnesses.min(10) as i32) * 4;
    let scale_public = match state.scale {
        CombatScale::Street => 0,
        CombatScale::District => 4,
        CombatScale::City => 10,
        CombatScale::National => 18,
        CombatScale::Cosmic => 28,
    };
    let publicness = clamp_u8(
        public_base + witness_boost + scale_public + roll_range(&mut state.rng_state, -6, 6),
    );

    let collateral_base = match state.scale {
        CombatScale::Street => 12,
        CombatScale::District => 20,
        CombatScale::City => 32,
        CombatScale::National => 46,
        CombatScale::Cosmic => 62,
    };
    let collateral_shift = match end {
        CombatEnd::PlayerEscaped => -4,
        CombatEnd::PlayerDefeated => 12,
        CombatEnd::OpponentsDefeated => 8,
        CombatEnd::Resolved => -6,
    };
    let collateral = clamp_u8(
        collateral_base + collateral_shift + roll_range(&mut state.rng_state, -8, 8),
    );

    let notoriety_base = match state.scale {
        CombatScale::Street => 18,
        CombatScale::District => 28,
        CombatScale::City => 42,
        CombatScale::National => 58,
        CombatScale::Cosmic => 72,
    };
    let notoriety_shift = match end {
        CombatEnd::PlayerEscaped => 6,
        CombatEnd::PlayerDefeated => 16,
        CombatEnd::OpponentsDefeated => 10,
        CombatEnd::Resolved => -4,
    };
    let notoriety = clamp_u8(
        notoriety_base
            + notoriety_shift
            + (publicness as i32 / 3)
            + roll_range(&mut state.rng_state, -6, 6),
    );

    CombatConsequence {
        publicness,
        collateral,
        notoriety,
    }
}

fn seed_combat_rng(
    seed: u64,
    location_id: LocationId,
    scale: CombatScale,
    source: &str,
    opponent_count: u32,
) -> u64 {
    seed ^ (location_id.0 as u64).wrapping_mul(31)
        ^ scale_seed(scale)
        ^ hash_seed(source)
        ^ (opponent_count as u64).wrapping_shl(5)
}

fn scale_seed(scale: CombatScale) -> u64 {
    match scale {
        CombatScale::Street => 0x11,
        CombatScale::District => 0x22,
        CombatScale::City => 0x33,
        CombatScale::National => 0x44,
        CombatScale::Cosmic => 0x55,
    }
}

fn next_u64(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1);
    *state
}

fn roll_range(state: &mut u64, min: i32, max: i32) -> i32 {
    if min >= max {
        return min;
    }
    let span = (max - min + 1) as u64;
    let roll = (next_u64(state) % span) as i32;
    min + roll
}

fn clamp_u8(value: i32) -> u8 {
    value.clamp(0, 100) as u8
}

fn hash_seed(value: &str) -> u64 {
    let mut hash = 1469598103934665603u64;
    for byte in value.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn identity_factor(consequence: CombatConsequence) -> f32 {
    1.0 + (consequence.publicness as f32 / 160.0)
}

fn institutional_factor(consequence: CombatConsequence) -> f32 {
    1.0 + ((consequence.publicness as f32 + consequence.collateral as f32) / 260.0)
}

fn collateral_factor(consequence: CombatConsequence) -> f32 {
    1.0 + (consequence.collateral as f32 / 150.0)
}

fn resource_factor(consequence: CombatConsequence) -> f32 {
    1.0 + (consequence.collateral as f32 / 240.0)
}

fn notoriety_factor(consequence: CombatConsequence) -> f32 {
    1.0 + (consequence.notoriety as f32 / 180.0)
}
