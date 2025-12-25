use bevy_ecs::prelude::*;

use crate::components::identity::SuperIdentity;
use crate::components::persona::{PersonaStack, PersonaType};
use crate::components::world::{EntityId, Position};
use crate::core::world::{ActionIntent, ActionQueue};
use crate::simulation::city::CityState;
use crate::simulation::evidence::WorldEvidence;
use crate::simulation::time::GameTime;
use crate::rules::signature::SignatureType;

#[derive(Resource, Debug, Default)]
pub struct PersonaEventLog(pub Vec<String>);

#[derive(Debug, Clone)]
pub enum PersonaSwitchError {
    UnknownPersona,
    AlreadyActive,
    SwitchBlockedByWitnesses,
    SwitchBlockedByLocation,
    SwitchOnCooldown,
}

#[derive(Debug, Clone)]
pub struct PersonaSwitchResult {
    pub new_persona_id: String,
    pub new_persona_type: PersonaType,
    pub suspicion_applied: bool,
}

pub fn persona_switch_system(
    intents: Res<ActionQueue>,
    time: Res<GameTime>,
    city: Res<CityState>,
    evidence: Res<WorldEvidence>,
    mut log: ResMut<PersonaEventLog>,
    mut personas: Query<(
        &EntityId,
        &Position,
        &mut PersonaStack,
        Option<&mut SuperIdentity>,
    )>,
) {
    log.0.clear();

    for intent in intents.0.iter() {
        let ActionIntent::SwitchPersona { entity_id, persona_id } = intent else {
            continue;
        };

        if let Some((_, pos, mut stack, maybe_super)) = personas
            .iter_mut()
            .find(|(id, _, _, _)| id.0 == *entity_id)
        {
            if stack.active_persona_id == *persona_id {
                log.0.push(format!(
                    "Entity {} already active in persona {}.",
                    entity_id, persona_id
                ));
                continue;
            }

            let location_id = city.location_for_position(pos);
            let location = match city.locations.get(&location_id) {
                Some(location) => location,
                None => continue,
            };

            let witnesses = if location.tags.contains(&crate::simulation::city::LocationTag::Public)
                && location.heat > 25
            {
                1
            } else {
                0
            };
            let has_visual_anomaly = evidence.signatures.iter().any(|event| {
                event.location_id == location_id
                    && event.signature.signature.signature_type == SignatureType::VisualAnomaly
            });

            match attempt_switch(
                &mut stack,
                persona_id,
                time.tick,
                location,
                witnesses,
                has_visual_anomaly,
            ) {
                Ok(result) => {
                    if let Some(mut super_id) = maybe_super {
                        super_id.is_masked = result.new_persona_type == PersonaType::Masked;
                    }
                    log.0.push(format!(
                        "Entity {} switched to persona {}.",
                        entity_id, result.new_persona_id
                    ));
                }
                Err(err) => log.0.push(format!(
                    "Entity {} persona switch failed: {:?}",
                    entity_id, err
                )),
            }
        }
    }
}

pub fn attempt_switch(
    stack: &mut PersonaStack,
    persona_id: &str,
    current_tick: u64,
    location: &crate::simulation::city::LocationState,
    witnesses: u32,
    has_visual_anomaly: bool,
) -> Result<PersonaSwitchResult, PersonaSwitchError> {
    let (target_id, target_type) = {
        let Some(target) = stack.personas.iter().find(|p| p.persona_id == persona_id) else {
            return Err(PersonaSwitchError::UnknownPersona);
        };
        (target.persona_id.clone(), target.persona_type)
    };
    if stack.active_persona_id == persona_id {
        return Err(PersonaSwitchError::AlreadyActive);
    }
    if current_tick < stack.next_switch_tick {
        return Err(PersonaSwitchError::SwitchOnCooldown);
    }
    if !stack.can_switch_to(persona_id, &location.tags) {
        return Err(PersonaSwitchError::SwitchBlockedByLocation);
    }
    if target_type == PersonaType::Civilian
        && location.tags.contains(&crate::simulation::city::LocationTag::Public)
        && witnesses > 0
    {
        return Err(PersonaSwitchError::SwitchBlockedByWitnesses);
    }

    stack.active_persona_id = persona_id.to_string();
    stack.next_switch_tick = current_tick + 1;

    let mut suspicion_applied = false;
    if target_type == PersonaType::Civilian && has_visual_anomaly {
        if let Some(active) = stack.active_persona_mut() {
            active.suspicion.civilian_suspicion =
                (active.suspicion.civilian_suspicion + 2).clamp(0, 100);
            suspicion_applied = true;
        }
    }

    Ok(PersonaSwitchResult {
        new_persona_id: target_id,
        new_persona_type: target_type,
        suspicion_applied,
    })
}
