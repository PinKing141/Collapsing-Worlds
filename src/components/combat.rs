use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

/// Simple health pool for combat resolution.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Health {
    pub fn new(max: i32) -> Self {
        Self { current: max, max }
    }
}
