use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

/// Associates an entity with a named faction.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Faction {
    pub name: String,
}
