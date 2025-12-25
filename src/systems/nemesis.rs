use bevy_ecs::prelude::*;

use crate::data::nemesis::{
    load_nemesis_action_catalog, NemesisActionDefinition, NemesisPressureDelta,
    NemesisSignatureTrace,
};
use crate::rules::signature::{SignatureInstance, SignatureSpec, SignatureType};
use crate::simulation::case::{CaseRegistry, CaseStatus, CaseTargetType};
use crate::simulation::city::{CityState, LocationId};
use crate::simulation::evidence::WorldEvidence;
use crate::simulation::nemesis::{NemesisCandidate, NemesisMemory, NemesisState};
use crate::simulation::pressure::PressureState;
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
        Self { actions: Vec::new() }
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
    mut cases: ResMut<CaseRegistry>,
    mut evidence: ResMut<WorldEvidence>,
    mut pressure: ResMut<PressureState>,
    time: Res<GameTime>,
    mut log: ResMut<NemesisEventLog>,
) {
    run_nemesis_system(
        &director,
        &mut state,
        &city,
        &mut cases,
        &mut evidence,
        &mut pressure,
        &time,
        &mut log,
    );
}

pub fn run_nemesis_system(
    director: &NemesisDirector,
    state: &mut NemesisState,
    city: &CityState,
    cases: &mut CaseRegistry,
    evidence: &mut WorldEvidence,
    pressure: &mut PressureState,
    time: &GameTime,
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
        match state
            .candidates
            .iter_mut()
            .find(|candidate| {
                candidate.faction_id == snapshot.faction_id
                    && candidate.location_id == snapshot.location_id
            }) {
            Some(candidate) => {
                candidate.heat = heat;
                candidate.case_progress = snapshot.progress;
                update_memory(&mut candidate.memory, snapshot);
            }
            None => {
                let mut memory = NemesisMemory::default();
                update_memory(&mut memory, snapshot);
                state.candidates.push(NemesisCandidate {
                    faction_id: snapshot.faction_id.clone(),
                    location_id: snapshot.location_id,
                    heat,
                    case_progress: snapshot.progress,
                    memory,
                    adaptation_level: 0,
                    is_nemesis: false,
                    last_action_tick: 0,
                });
            }
        }
    }

    let mut actions_to_apply: Vec<NemesisActionTrigger> = Vec::new();

    for candidate in state.candidates.iter_mut() {
        let Some(snapshot) = case_snapshots.iter().find(|case| {
            case.faction_id == candidate.faction_id && case.location_id == candidate.location_id
        }) else {
            continue;
        };

        let mut highest_level = candidate.adaptation_level;
        for threshold in state.thresholds.iter() {
            if candidate.heat >= threshold.min_heat
                && candidate.case_progress >= threshold.min_case_progress
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

        let cooldown = state
            .thresholds
            .iter()
            .filter(|threshold| threshold.level <= candidate.adaptation_level)
            .max_by_key(|threshold| threshold.level)
            .map(|threshold| threshold.cooldown)
            .unwrap_or(3);

        if !candidate.is_nemesis {
            continue;
        }

        if time.tick.saturating_sub(candidate.last_action_tick) < cooldown {
            continue;
        }

        let Some(action) = select_action(director, candidate.heat, snapshot.progress) else {
            continue;
        };

        candidate.last_action_tick = time.tick;
        actions_to_apply.push(NemesisActionTrigger {
            faction_id: candidate.faction_id.clone(),
            location_id: candidate.location_id,
            action,
        });
    }

    for trigger in actions_to_apply {
        apply_action(
            &trigger,
            cases,
            evidence,
            pressure,
            log,
        );
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

fn update_memory(memory: &mut NemesisMemory, snapshot: &CaseSnapshot) {
    for signature in &snapshot.signatures {
        memory.record_signature(*signature);
    }
    memory.record_form(case_form(snapshot.target_type));
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
) -> Option<NemesisActionDefinition> {
    let mut candidates: Vec<&NemesisActionDefinition> = director
        .actions
        .iter()
        .filter(|action| action.min_heat as i32 <= heat && action.min_case_progress <= case_progress)
        .collect();
    candidates.sort_by_key(|action| (action.min_heat, action.min_case_progress));
    candidates.last().map(|action| (*action).clone())
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
            let next = (case.progress as i32 + trigger.action.case_progress_delta)
                .clamp(0, 100) as u32;
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
