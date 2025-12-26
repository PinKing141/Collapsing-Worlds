use bevy_ecs::prelude::*;

use crate::simulation::civilian::{tick_civilian_economy, CivilianState};
use crate::simulation::time::GameTime;

/// Runs the daily economy tick (income + upkeep) for civilian finances.
pub fn economy_system(mut civilian: ResMut<CivilianState>, time: Res<GameTime>) {
    tick_civilian_economy(&mut civilian, &time);
}
