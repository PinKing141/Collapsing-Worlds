use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

/// The basic name of an entity.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Name(pub String);

/// Represents the mundane life of an entity.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct CivilianIdentity {
    pub job_title: String,
    pub salary: u32,
    pub suspicion_meter: u8,
}

/// Represents the alter-ego.
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct SuperIdentity {
    pub hero_name: String,
    pub is_masked: bool,
    pub reputation: i32,
}
