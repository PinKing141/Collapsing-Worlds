use bevy_ecs::prelude::*;

use crate::rules::use_power::PressureModifiers;
use crate::simulation::case::{CaseRegistry, CaseStatus};
use crate::simulation::pressure::PressureState;
use crate::simulation::storylet_state::StoryletState;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransformationEvent {
    pub state: TransformationState,
    pub trigger: TransformationTrigger,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndgameState {
    pub phase: Option<TransformationState>,
}

impl Default for EndgameState {
    fn default() -> Self {
        Self { phase: None }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EndgameModifiers {
    pub cost_scale: f64,
    pub risk_scale: f64,
}

impl Default for EndgameModifiers {
    fn default() -> Self {
        Self {
            cost_scale: 1.0,
            risk_scale: 1.0,
        }
    }
}

impl EndgameState {
    pub fn label(&self) -> &'static str {
        endgame_phase_label(self.phase)
    }

    pub fn modifiers(&self) -> EndgameModifiers {
        match self.phase {
            None => EndgameModifiers::default(),
            Some(TransformationState::Exposed) => EndgameModifiers {
                cost_scale: 1.05,
                risk_scale: 1.15,
            },
            Some(TransformationState::Registration) => EndgameModifiers {
                cost_scale: 1.1,
                risk_scale: 1.2,
            },
            Some(TransformationState::CosmicJudgement) => EndgameModifiers {
                cost_scale: 1.15,
                risk_scale: 1.35,
            },
            Some(TransformationState::Ascension) => EndgameModifiers {
                cost_scale: 0.95,
                risk_scale: 1.25,
            },
            Some(TransformationState::Exile) => EndgameModifiers {
                cost_scale: 1.2,
                risk_scale: 1.4,
            },
        }
    }

    pub fn apply_modifiers(&self, base: PressureModifiers) -> PressureModifiers {
        let modifiers = self.modifiers();
        PressureModifiers {
            cost_scale: base.cost_scale * modifiers.cost_scale,
            risk_scale: base.risk_scale * modifiers.risk_scale,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EndgameTransition {
    pub event: TransformationEvent,
    pub narrative: &'static str,
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

pub fn apply_transformation_event(
    endgame_state: &mut EndgameState,
    storylet_state: &mut StoryletState,
    event: TransformationEvent,
) -> Option<EndgameTransition> {
    if endgame_state.phase == Some(event.state) {
        return None;
    }

    endgame_state.phase = Some(event.state);
    storylet_state
        .flags
        .insert(transformation_flag(event.state).to_string(), true);

    let narrative = match event.state {
        TransformationState::Exposed => handle_exposed(storylet_state),
        TransformationState::Registration => handle_registration(storylet_state),
        TransformationState::CosmicJudgement => handle_cosmic(storylet_state),
        _ => transformation_text(event.state),
    };

    Some(EndgameTransition { event, narrative })
}

pub fn endgame_phase_label(state: Option<TransformationState>) -> &'static str {
    match state {
        None => "Dormant",
        Some(TransformationState::Exposed) => "Exposed",
        Some(TransformationState::Registration) => "Registration",
        Some(TransformationState::CosmicJudgement) => "Cosmic Judgement",
        Some(TransformationState::Ascension) => "Ascension",
        Some(TransformationState::Exile) => "Exile",
    }
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

fn handle_exposed(storylet_state: &mut StoryletState) -> &'static str {
    storylet_state
        .flags
        .insert("endgame_exposed".to_string(), true);
    transformation_text(TransformationState::Exposed)
}

fn handle_registration(storylet_state: &mut StoryletState) -> &'static str {
    storylet_state
        .flags
        .insert("endgame_registration".to_string(), true);
    transformation_text(TransformationState::Registration)
}

fn handle_cosmic(storylet_state: &mut StoryletState) -> &'static str {
    storylet_state
        .flags
        .insert("endgame_cosmic".to_string(), true);
    transformation_text(TransformationState::CosmicJudgement)
}
