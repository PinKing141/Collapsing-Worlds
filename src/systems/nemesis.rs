use bevy_ecs::prelude::*;

use crate::data::nemesis::{
    load_nemesis_action_catalog, NemesisActionDefinition, NemesisPressureDelta,
    NemesisSignatureTrace,
};
use crate::rules::signature::{SignatureInstance, SignatureSpec, SignatureType};
use crate::simulation::case::{CaseRegistry, CaseStatus, CaseTargetType};
use crate::simulation::city::{CityState, LocationId};
use crate::simulation::evidence::WorldEvidence;
use crate::simulation::identity_evidence::IdentityEvidenceStore;
use crate::simulation::nemesis::{
    NemesisCandidate, NemesisMemory, NemesisPersonaArc, NemesisState,
};
use crate::simulation::pressure::PressureState;
use crate::simulation::region::{GlobalEscalation, RegionState};
use crate::simulation::storylet_state::StoryletState;
use crate::simulation::time::GameTime;

const DEFAULT_NEMESIS_ACTIONS_PATH: &str = "./assets/data/nemesis_actions.json";

#[derive(Resource, Debug, Default)]
pub struct NemesisEventLog(pub Vec<String>);

#[derive(Resource, Debug, Clone)]
pub struct NemesisDirector {
    pub actions: Vec<NemesisActionDefinition>,
}

impl Default for NemesisDirector {
    fn default() -> Self {
        Self {
            actions: Vec::new(),
        }
    }
}

impl NemesisDirector {
    pub fn load_default() -> Result<Self, crate::data::nemesis::DataError> {
        let catalog = load_nemesis_action_catalog(DEFAULT_NEMESIS_ACTIONS_PATH)?;
        Ok(Self {
            actions: catalog.actions,
        })
    }
}

pub fn nemesis_system(
    director: Res<NemesisDirector>,
    mut state: ResMut<NemesisState>,
    city: Res<CityState>,
    region: Res<RegionState>,
    mut cases: ResMut<CaseRegistry>,
    mut evidence: ResMut<WorldEvidence>,
    identity_evidence: Res<IdentityEvidenceStore>,
    mut pressure: ResMut<PressureState>,
    time: Res<GameTime>,
    mut storylets: ResMut<StoryletState>,
    mut log: ResMut<NemesisEventLog>,
) {
    run_nemesis_system(
        &director,
        &mut state,
        &city,
        &region,
        &mut cases,
        &mut evidence,
        &identity_evidence,
        &mut pressure,
        &time,
        &mut storylets,
        &mut log,
    );
}

pub fn run_nemesis_system(
    director: &NemesisDirector,
    state: &mut NemesisState,
    city: &CityState,
    region: &RegionState,
    cases: &mut CaseRegistry,
    evidence: &mut WorldEvidence,
    identity_evidence: &IdentityEvidenceStore,
    pressure: &mut PressureState,
    time: &GameTime,
    storylets: &mut StoryletState,
    log: &mut NemesisEventLog,
) {
    log.0.clear();

    let case_snapshots: Vec<CaseSnapshot> = cases
        .cases
        .iter()
        .filter(|case| case.status == CaseStatus::Active)
        .map(|case| CaseSnapshot {
            faction_id: case.faction_id.clone(),
            location_id: case.location_id,
            progress: case.progress,
            target_type: case.target_type,
            signatures: case.signature_pattern.clone(),
        })
        .collect();

    state.candidates.retain(|candidate| {
        case_snapshots.iter().any(|case| {
            case.faction_id == candidate.faction_id && case.location_id == candidate.location_id
        })
    });

    for snapshot in &case_snapshots {
        let heat = city
            .locations
            .get(&snapshot.location_id)
            .map(|loc| loc.heat)
            .unwrap_or(0);
        match state.candidates.iter_mut().find(|candidate| {
            candidate.faction_id == snapshot.faction_id
                && candidate.location_id == snapshot.location_id
        }) {
            Some(candidate) => {
                candidate.heat = heat;
                candidate.case_progress = snapshot.progress;
                update_memory(&mut candidate.memory, snapshot, identity_evidence);
            }
            None => {
                let mut memory = NemesisMemory::default();
                update_memory(&mut memory, snapshot, identity_evidence);
                state.candidates.push(NemesisCandidate {
                    faction_id: snapshot.faction_id.clone(),
                    location_id: snapshot.location_id,
                    heat,
                    case_progress: snapshot.progress,
                    memory,
                    adaptation_level: 0,
                    persona_arc: NemesisPersonaArc::SecretHunt,
                    is_nemesis: false,
                    last_action_tick: 0,
                    last_storylet_tick: 0,
                });
            }
        }
    }

    let mut actions_to_apply: Vec<NemesisActionTrigger> = Vec::new();

    let escalation_mods = nemesis_escalation_modifiers(region.global_pressure.escalation);

    for candidate in state.candidates.iter_mut() {
        let Some(snapshot) = case_snapshots.iter().find(|case| {
            case.faction_id == candidate.faction_id && case.location_id == candidate.location_id
        }) else {
            continue;
        };

        let mut highest_level = candidate.adaptation_level;
        for threshold in state.thresholds.iter() {
            let min_heat = (threshold.min_heat as i32 + escalation_mods.heat_delta).max(0) as i32;
            let min_progress =
                (threshold.min_case_progress as i32 + escalation_mods.case_delta).max(0) as u32;
            if candidate.heat >= min_heat
                && candidate.case_progress >= min_progress
                && threshold.level > highest_level
            {
                highest_level = threshold.level;
            }
        }

        if highest_level > candidate.adaptation_level {
            candidate.adaptation_level = highest_level;
            if candidate.adaptation_level > 0 {
                candidate.is_nemesis = true;
                log.0.push(format!(
                    "Nemesis escalates: {} at location {} (level {}).",
                    candidate.faction_id, candidate.location_id.0, candidate.adaptation_level
                ));
            }
        }

        let next_arc = determine_persona_arc(candidate);
        if next_arc != candidate.persona_arc {
            candidate.persona_arc = next_arc;
            log.0.push(format!(
                "Nemesis persona arc shifts: {} at location {} now {:?}.",
                candidate.faction_id, candidate.location_id.0, candidate.persona_arc
            ));
        }

        let base_cooldown = state
            .thresholds
            .iter()
            .filter(|threshold| threshold.level <= candidate.adaptation_level)
            .max_by_key(|threshold| threshold.level)
            .map(|threshold| threshold.cooldown)
            .unwrap_or(3);
        let cooldown = apply_cooldown_modifier(base_cooldown, escalation_mods.cooldown_delta);

        if !candidate.is_nemesis {
            continue;
        }

        if time.tick.saturating_sub(candidate.last_action_tick) < cooldown {
            continue;
        }

        let focus = build_counter_focus(candidate);
        let Some(action) =
            select_action(director, candidate.heat, snapshot.progress, focus.as_ref())
        else {
            continue;
        };

        candidate.last_action_tick = time.tick;
        actions_to_apply.push(NemesisActionTrigger {
            faction_id: candidate.faction_id.clone(),
            location_id: candidate.location_id,
            action,
        });

        if should_trigger_confrontation(candidate, snapshot, time.tick) {
            candidate.last_storylet_tick = time.tick;
            emit_confrontation_storylet(candidate, storylets, log);
        }
    }

    for trigger in actions_to_apply {
        apply_action(&trigger, cases, evidence, pressure, log);
    }
}

#[derive(Debug, Clone)]
struct CaseSnapshot {
    faction_id: String,
    location_id: LocationId,
    progress: u32,
    target_type: CaseTargetType,
    signatures: Vec<SignatureType>,
}

#[derive(Debug, Clone)]
struct NemesisActionTrigger {
    faction_id: String,
    location_id: LocationId,
    action: NemesisActionDefinition,
}

#[derive(Debug, Clone)]
struct NemesisCounterFocus {
    signature: Option<SignatureType>,
    form: Option<String>,
}

fn update_memory(
    memory: &mut NemesisMemory,
    snapshot: &CaseSnapshot,
    identity_evidence: &IdentityEvidenceStore,
) {
    for signature in &snapshot.signatures {
        memory.record_signature(*signature);
    }
    memory.record_signature_pattern(snapshot.signatures.clone());
    memory.record_form(case_form(snapshot.target_type));
    record_identity_traits(memory, identity_evidence, snapshot.location_id);
}

fn case_form(target_type: CaseTargetType) -> String {
    match target_type {
        CaseTargetType::UnknownMasked => "UNKNOWN_MASKED".to_string(),
        CaseTargetType::KnownMasked => "KNOWN_MASKED".to_string(),
        CaseTargetType::CivilianLink => "CIVILIAN_LINK".to_string(),
    }
}

fn select_action(
    director: &NemesisDirector,
    heat: i32,
    case_progress: u32,
    focus: Option<&NemesisCounterFocus>,
) -> Option<NemesisActionDefinition> {
    let mut candidates: Vec<&NemesisActionDefinition> = director
        .actions
        .iter()
        .filter(|action| {
            action.min_heat as i32 <= heat && action.min_case_progress <= case_progress
        })
        .collect();
    if let Some(focus) = focus {
        let focused: Vec<&NemesisActionDefinition> = candidates
            .iter()
            .copied()
            .filter(|action| action_matches_focus(action, focus))
            .collect();
        if !focused.is_empty() {
            candidates = focused;
        }
    }
    candidates.sort_by_key(|action| (action.min_heat, action.min_case_progress));
    candidates.last().map(|action| (*action).clone())
}

fn action_matches_focus(action: &NemesisActionDefinition, focus: &NemesisCounterFocus) -> bool {
    if let Some(signature) = focus.signature {
        if action.signature_focus.iter().any(|sig| *sig == signature) {
            return true;
        }
    }
    if let Some(form) = &focus.form {
        if action
            .form_focus
            .iter()
            .any(|focus_form| focus_form == form)
        {
            return true;
        }
    }
    false
}

fn build_counter_focus(candidate: &NemesisCandidate) -> Option<NemesisCounterFocus> {
    if candidate.adaptation_level == 0 {
        return None;
    }
    let signature = candidate.memory.most_common_signature(2);
    let form = match candidate.persona_arc {
        NemesisPersonaArc::SecretHunt => candidate.memory.most_common_form(2),
        NemesisPersonaArc::PublicThreat => None,
    };
    if signature.is_none() && form.is_none() {
        None
    } else {
        Some(NemesisCounterFocus { signature, form })
    }
}

fn determine_persona_arc(candidate: &NemesisCandidate) -> NemesisPersonaArc {
    let identity_pressure = candidate.memory.identity_traits.len() >= 2;
    if candidate.heat >= 60 || candidate.case_progress >= 70 {
        NemesisPersonaArc::PublicThreat
    } else if identity_pressure {
        NemesisPersonaArc::SecretHunt
    } else {
        NemesisPersonaArc::SecretHunt
    }
}

fn record_identity_traits(
    memory: &mut NemesisMemory,
    identity_evidence: &IdentityEvidenceStore,
    location_id: LocationId,
) {
    for item in identity_evidence.items.iter() {
        if item.location_id != location_id {
            continue;
        }
        for trait_name in &item.suspect_features {
            memory.record_identity_trait(trait_name.clone());
        }
    }
}

fn should_trigger_confrontation(
    candidate: &NemesisCandidate,
    snapshot: &CaseSnapshot,
    tick: u64,
) -> bool {
    if !candidate.is_nemesis {
        return false;
    }
    let heat_trigger = candidate.heat >= 70;
    let progress_trigger = snapshot.progress >= 80;
    let cooldown_ready = tick.saturating_sub(candidate.last_storylet_tick) >= 5;
    cooldown_ready && (heat_trigger || progress_trigger)
}

fn emit_confrontation_storylet(
    candidate: &NemesisCandidate,
    storylets: &mut StoryletState,
    log: &mut NemesisEventLog,
) {
    let flag = format!("nemesis_confrontation_{}", candidate.faction_id);
    if storylets.flags.get(&flag).copied().unwrap_or(false) {
        return;
    }
    storylets.flags.insert(flag.clone(), true);
    storylets
        .flags
        .insert("nemesis_confrontation".to_string(), true);
    storylets.punctuation.activate(2);
    log.0.push(format!(
        "Nemesis confrontation storylet triggered for {} at location {}.",
        candidate.faction_id, candidate.location_id.0
    ));
}

#[derive(Debug, Clone, Copy)]
struct NemesisEscalationModifiers {
    heat_delta: i32,
    case_delta: i32,
    cooldown_delta: i64,
}

fn nemesis_escalation_modifiers(escalation: GlobalEscalation) -> NemesisEscalationModifiers {
    match escalation {
        GlobalEscalation::Stable => NemesisEscalationModifiers {
            heat_delta: 0,
            case_delta: 0,
            cooldown_delta: 0,
        },
        GlobalEscalation::Tense => NemesisEscalationModifiers {
            heat_delta: -4,
            case_delta: -3,
            cooldown_delta: -1,
        },
        GlobalEscalation::Crisis => NemesisEscalationModifiers {
            heat_delta: -8,
            case_delta: -6,
            cooldown_delta: -1,
        },
        GlobalEscalation::Cosmic => NemesisEscalationModifiers {
            heat_delta: -12,
            case_delta: -10,
            cooldown_delta: -2,
        },
    }
}

fn apply_cooldown_modifier(base: u64, delta: i64) -> u64 {
    if delta >= 0 {
        base.saturating_add(delta as u64)
    } else {
        base.saturating_sub(delta.unsigned_abs())
    }
}

fn apply_action(
    trigger: &NemesisActionTrigger,
    cases: &mut CaseRegistry,
    evidence: &mut WorldEvidence,
    pressure: &mut PressureState,
    log: &mut NemesisEventLog,
) {
    let signatures = build_signature_instances(&trigger.action.signature_traces);
    if !signatures.is_empty() {
        evidence.emit(trigger.location_id, &signatures);
    }

    if trigger.action.case_progress_delta != 0 {
        if let Some(case) = cases.find_case_mut(&trigger.faction_id, trigger.location_id) {
            let next =
                (case.progress as i32 + trigger.action.case_progress_delta).clamp(0, 100) as u32;
            case.progress = next;
            case.pressure_actions.push(trigger.action.id.clone());
        }
    }

    apply_pressure_delta(pressure, &trigger.action.pressure_delta);

    log.0.push(format!(
        "Nemesis action {} at location {}.",
        trigger.action.id, trigger.location_id.0
    ));
}

fn build_signature_instances(traces: &[NemesisSignatureTrace]) -> Vec<SignatureInstance> {
    traces
        .iter()
        .map(|trace| {
            SignatureSpec {
                signature_type: trace.signature_type,
                strength: trace.strength,
                persistence_turns: trace.persistence_turns,
            }
            .to_instance()
        })
        .collect()
}

fn apply_pressure_delta(pressure: &mut PressureState, delta: &NemesisPressureDelta) {
    pressure.temporal = (pressure.temporal + delta.temporal).clamp(0.0, 100.0);
    pressure.identity = (pressure.identity + delta.identity).clamp(0.0, 100.0);
    pressure.institutional = (pressure.institutional + delta.institutional).clamp(0.0, 100.0);
    pressure.moral = (pressure.moral + delta.moral).clamp(0.0, 100.0);
    pressure.resource = (pressure.resource + delta.resource).clamp(0.0, 100.0);
    pressure.psychological = (pressure.psychological + delta.psychological).clamp(0.0, 100.0);
}
