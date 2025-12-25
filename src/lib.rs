// Re-export core modules for use by the binary or other consumers
pub mod components;
pub mod content;
pub mod core;
pub mod data;
pub mod rules;
pub mod simulation;
pub mod systems;
pub mod ui;
pub mod world;

// Expose the main Game wrapper and types needed for interaction
pub use crate::core::serialization::SaveState;
pub use crate::core::world::{ActionIntent, EntitySummary, Game, Snapshot};
