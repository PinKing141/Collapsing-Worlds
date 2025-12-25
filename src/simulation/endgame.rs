use crate::simulation::case::{CaseRegistry, CaseStatus};
use crate::simulation::pressure::PressureState;
use crate::systems::event_resolver::ResolvedFactionEventLog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformationState {
    Exposed,
    Registration,
    CosmicJudgement,
    Ascension,
    Exile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformationTrigger {
    CaseCollapse,
    PressureSpike,
    FactionAttention,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformationEvent {
    pub state: TransformationState,
    pub trigger: TransformationTrigger,
}

pub fn evaluate_transformation(
    cases: &CaseRegistry,
    pressure: &PressureState,
    faction_events: &ResolvedFactionEventLog,
) -> Option<TransformationEvent> {
    if let Some(state) = evaluate_case_pressure(cases) {
        return Some(TransformationEvent {
            state,
            trigger: TransformationTrigger::CaseCollapse,
        });
    }

    if let Some(state) = evaluate_pressure_state(pressure) {
        return Some(TransformationEvent {
            state,
            trigger: TransformationTrigger::PressureSpike,
        });
    }

    if let Some(state) = evaluate_faction_attention(faction_events) {
        return Some(TransformationEvent {
            state,
            trigger: TransformationTrigger::FactionAttention,
        });
    }

    None
}

fn evaluate_case_pressure(cases: &CaseRegistry) -> Option<TransformationState> {
    let active_cases: Vec<_> = cases
        .cases
        .iter()
        .filter(|case| case.status == CaseStatus::Active)
        .collect();
    if active_cases.is_empty() {
        return None;
    }

    let max_progress = active_cases
        .iter()
        .map(|case| case.progress)
        .max()
        .unwrap_or(0);
    if max_progress >= 90 {
        return Some(TransformationState::Exposed);
    }
    if max_progress >= 70 {
        return Some(TransformationState::Registration);
    }
    None
}

fn evaluate_pressure_state(pressure: &PressureState) -> Option<TransformationState> {
    let mut spikes = 0;
    if pressure.identity >= 85.0 {
        spikes += 1;
    }
    if pressure.institutional >= 85.0 {
        spikes += 1;
    }
    if pressure.psychological >= 85.0 {
        spikes += 1;
    }
    if pressure.moral >= 80.0 {
        spikes += 1;
    }

    if spikes >= 3 {
        return Some(TransformationState::CosmicJudgement);
    }
    if pressure.temporal >= 80.0 && pressure.resource >= 80.0 {
        return Some(TransformationState::Ascension);
    }
    None
}

fn evaluate_faction_attention(
    faction_events: &ResolvedFactionEventLog,
) -> Option<TransformationState> {
    let mut escalations = 0;
    for event in &faction_events.0 {
        if event.level.eq_ignore_ascii_case("critical")
            || event.level.eq_ignore_ascii_case("max")
        {
            escalations += 1;
        }
    }

    if escalations >= 2 {
        return Some(TransformationState::Exile);
    }

    None
}
