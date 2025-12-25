use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

/// Represents a physical location in the city grid.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// Stable identifier for addressing entities externally.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityId(pub u32);

/// Marker component for the human player to distinguish them from NPCs.
#[derive(Component, Debug)]
pub struct Player;
