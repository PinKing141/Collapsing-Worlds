use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::components::identity::CivilianIdentity;
use crate::components::persona::{Alignment, PersonaStack, PersonaType, RiskModifiers, SuspicionDelta};
use crate::components::world::{EntityId, Position};
use crate::core::world::{ActionIntent, ActionQueue};
use crate::simulation::case::CaseRegistry;
use crate::simulation::city::CityState;
use crate::simulation::identity_evidence::{IdentityEvidenceStore, PersonaHint};

/// System: adjusts persona suspicion based on intents, environment, and active cases.
pub fn suspicion_system(
    intents: Res<ActionQueue>,
    city: Res<CityState>,
    cases: Res<CaseRegistry>,
    identity: Res<IdentityEvidenceStore>,
    mut personas: Query<(
        &EntityId,
        &Position,
        &mut PersonaStack,
        Option<&Alignment>,
        Option<&mut CivilianIdentity>,
    )>,
) {
    let deltas = collect_intent_deltas(&intents.0);

    for (id, pos, mut stack, alignment, mut civilian) in personas.iter_mut() {
        let delta = deltas.get(&id.0).cloned().unwrap_or_default();
        let alignment = alignment.copied().unwrap_or(Alignment::Hero);
        let location_id = city.location_for_position(pos);
        let location = city.locations.get(&location_id);

        apply_suspicion_to_stack(&mut stack, alignment, location, &cases, &identity, delta);

        if let Some(civilian) = civilian.as_mut() {
            if let Some(active) = stack.active_persona() {
                if active.persona_type == PersonaType::Civilian {
                    civilian.suspicion_meter = active.suspicion.civilian_suspicion;
                }
            }
        }
    }
}

pub fn apply_suspicion_for_intents(
    stack: &mut PersonaStack,
    alignment: Alignment,
    position: &Position,
    city: &CityState,
    cases: &CaseRegistry,
    identity: &IdentityEvidenceStore,
    intents: &[ActionIntent],
    entity_id: u32,
) {
    let deltas = collect_intent_deltas(intents);
    let delta = deltas.get(&entity_id).cloned().unwrap_or_default();
    let location_id = city.location_for_position(position);
    let location = city.locations.get(&location_id);
    apply_suspicion_to_stack(stack, alignment, location, cases, identity, delta);
}

fn apply_suspicion_to_stack(
    stack: &mut PersonaStack,
    alignment: Alignment,
    location: Option<&crate::simulation::city::LocationState>,
    cases: &CaseRegistry,
    identity: &IdentityEvidenceStore,
    mut delta: SuspicionDelta,
) {
    delta.public_suspicion -= 1;
    delta.civilian_suspicion -= 1;
    delta.wanted_level -= 1;
    delta.exposure_risk -= 1;

    let active_persona = match stack.active_persona_mut() {
        Some(persona) => persona,
        None => return,
    };

    let alignment_mod = alignment.suspicion_multiplier();
    let combined_mod = merge_modifiers(&active_persona.risk_modifiers, &alignment_mod);

    if let Some(location) = location {
        if active_persona.persona_type == PersonaType::Masked {
            if location.tags.contains(&crate::simulation::city::LocationTag::Public) {
                delta.public_suspicion += 1;
                delta.exposure_risk += 1;
            }
            if location.surveillance_level > 30 {
                delta.exposure_risk += 1;
            }
            let pressure = case_pressure(cases, location.id, PersonaType::Masked);
            delta.public_suspicion += pressure.public_suspicion;
            delta.wanted_level += pressure.wanted_level;
            delta.exposure_risk += pressure.exposure_risk;
        } else if location.tags.contains(&crate::simulation::city::LocationTag::Residential) {
            delta.civilian_suspicion -= 1;
        }

        let pressure = case_pressure(cases, location.id, active_persona.persona_type);
        delta.civilian_suspicion += pressure.civilian_suspicion;
        delta.exposure_risk += pressure.exposure_risk;

        let evidence_delta = evidence_pressure(identity, location.id, active_persona.persona_type);
        delta.public_suspicion += evidence_delta.public_suspicion;
        delta.civilian_suspicion += evidence_delta.civilian_suspicion;
        delta.exposure_risk += evidence_delta.exposure_risk;
    }

    let scaled = scale_delta(delta, &combined_mod);
    active_persona.suspicion.apply_delta(&scaled);
}

fn collect_intent_deltas(intents: &[ActionIntent]) -> HashMap<u32, SuspicionDelta> {
    let mut deltas: HashMap<u32, SuspicionDelta> = HashMap::new();
    for intent in intents.iter() {
        match intent {
            ActionIntent::Interact { entity_id } => {
                deltas.entry(*entity_id).or_default().civilian_suspicion += 2;
            }
            ActionIntent::Attack { attacker_id, .. } => {
                let entry = deltas.entry(*attacker_id).or_default();
                entry.public_suspicion += 4;
                entry.wanted_level += 5;
                entry.exposure_risk += 2;
            }
            ActionIntent::Rest { entity_id } => {
                let entry = deltas.entry(*entity_id).or_default();
                entry.civilian_suspicion -= 1;
                entry.exposure_risk -= 2;
            }
            ActionIntent::Move { .. }
            | ActionIntent::Wait
            | ActionIntent::SwitchPersona { .. } => {}
        }
    }
    deltas
}

fn merge_modifiers(base: &RiskModifiers, alignment: &RiskModifiers) -> RiskModifiers {
    RiskModifiers {
        public_suspicion: base.public_suspicion * alignment.public_suspicion,
        civilian_suspicion: base.civilian_suspicion * alignment.civilian_suspicion,
        wanted_level: base.wanted_level * alignment.wanted_level,
        exposure_risk: base.exposure_risk * alignment.exposure_risk,
    }
}

fn scale_delta(delta: SuspicionDelta, mods: &RiskModifiers) -> SuspicionDelta {
    SuspicionDelta {
        public_suspicion: (delta.public_suspicion as f32 * mods.public_suspicion).round() as i32,
        civilian_suspicion: (delta.civilian_suspicion as f32 * mods.civilian_suspicion).round()
            as i32,
        wanted_level: (delta.wanted_level as f32 * mods.wanted_level).round() as i32,
        exposure_risk: (delta.exposure_risk as f32 * mods.exposure_risk).round() as i32,
    }
}

fn case_pressure(
    cases: &CaseRegistry,
    location_id: crate::simulation::city::LocationId,
    persona_type: PersonaType,
) -> SuspicionDelta {
    let mut delta = SuspicionDelta::default();

    for case in cases.cases.iter() {
        if case.location_id != location_id || case.status != crate::simulation::case::CaseStatus::Active {
            continue;
        }

        let pressure = if case.progress >= 85 {
            3
        } else if case.progress >= 60 {
            2
        } else if case.progress >= 30 {
            1
        } else {
            0
        };

        if pressure == 0 {
            continue;
        }

        match case.target_type {
            crate::simulation::case::CaseTargetType::UnknownMasked
            | crate::simulation::case::CaseTargetType::KnownMasked => {
                if persona_type == PersonaType::Masked {
                    delta.public_suspicion += pressure;
                    delta.wanted_level += pressure;
                }
            }
            crate::simulation::case::CaseTargetType::CivilianLink => {
                if persona_type == PersonaType::Civilian {
                    delta.civilian_suspicion += pressure;
                    delta.exposure_risk += pressure;
                }
            }
        }
    }

    delta
}

fn evidence_pressure(
    identity: &IdentityEvidenceStore,
    location_id: crate::simulation::city::LocationId,
    persona_type: PersonaType,
) -> SuspicionDelta {
    let mut delta = SuspicionDelta::default();

    for item in identity.items.iter() {
        if item.location_id != location_id {
            continue;
        }
        let strength = (item.visual_quality as i32 / 25).max(0);
        if strength == 0 {
            continue;
        }
        match persona_type {
            PersonaType::Masked => {
                delta.public_suspicion += strength;
                delta.exposure_risk += (strength / 2).max(1);
            }
            PersonaType::Civilian => {
                if item.persona_hint == PersonaHint::Civilian {
                    delta.civilian_suspicion += strength;
                    delta.exposure_risk += (strength / 2).max(1);
                }
            }
        }
    }

    delta
}
