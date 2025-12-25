use bevy_ecs::prelude::*;

use crate::components::combat::Health;
use crate::components::world::EntityId;
use crate::core::world::{ActionIntent, ActionQueue};

/// Resource capturing the most recent combat entries.
#[derive(Resource, Default, Debug)]
pub struct CombatLog(pub Vec<String>);

const BASE_DAMAGE: i32 = 10;

/// System: resolves attack intents and records them.
pub fn combat_system(
    intents: Res<ActionQueue>,
    mut log: ResMut<CombatLog>,
    mut healths: Query<(&EntityId, &mut Health)>,
) {
    log.0.clear();
    for intent in intents.0.iter() {
        if let ActionIntent::Attack {
            attacker_id,
            target_id,
        } = intent
        {
            let mut applied = false;

            if let Some(tid) = target_id {
                for (entity_id, mut health) in healths.iter_mut() {
                    if entity_id.0 == *tid {
                        apply_damage(entity_id.0, &mut health, BASE_DAMAGE, &mut log.0);
                        applied = true;
                        break;
                    }
                }
            } else {
                for (entity_id, mut health) in healths.iter_mut() {
                    if entity_id.0 != *attacker_id {
                        apply_damage(entity_id.0, &mut health, BASE_DAMAGE, &mut log.0);
                        applied = true;
                        break;
                    }
                }
            }

            if !applied {
                log.0
                    .push(format!("Entity {} swings but finds no target.", attacker_id));
            }
        }
    }
}

fn apply_damage(target_uid: u32, health: &mut Health, amount: i32, log: &mut Vec<String>) {
    health.current = (health.current - amount).max(0);
    if health.current == 0 {
        log.push(format!("Entity {} is defeated.", target_uid));
    } else {
        log.push(format!(
            "Entity {} takes {} damage ({} / {}).",
            target_uid,
            amount,
            health.current,
            health.max
        ));
    }
}
