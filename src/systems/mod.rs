pub mod combat;
pub mod combat_loop;
pub mod case;
pub mod civilian;
pub mod economy;
pub mod event_resolver;
pub mod faction;
pub mod heat;
pub mod nemesis;
pub mod persona;
pub mod pressure;
pub mod suspicion;
pub mod units;

use bevy_ecs::prelude::*;
use crate::components::world::Position;
use crate::core::world::{ActionIntent, ActionQueue};
use crate::components::world::EntityId;

/// System: Processes movement intents.
pub fn movement_system(
    // We now read the ActionQueue resource, not Vec directly
    intents: Res<ActionQueue>,
    mut query: Query<(&EntityId, &mut Position)>,
) {
    for intent in intents.0.iter() {
        if let ActionIntent::Move { entity_id, dx, dy } = intent {
            for (id, mut pos) in query.iter_mut() {
                if id.0 == *entity_id {
                    pos.x += dx;
                    pos.y += dy;
                }
            }
        }
    }
}
