use bevy_ecs::prelude::*;

use crate::simulation::civilian::{tick_civilian_life, CivilianState};
use crate::simulation::pressure::PressureState;
use crate::simulation::time::GameTime;

pub fn civilian_system(
    mut civilian: ResMut<CivilianState>,
    time: Res<GameTime>,
    mut pressure: ResMut<PressureState>,
) {
    tick_civilian_life(&mut civilian, &time);
    apply_civilian_pressure(&civilian, &mut pressure);
}

pub fn apply_civilian_pressure(civilian: &CivilianState, pressure: &mut PressureState) {
    let targets = civilian.pressure_targets();
    pressure.temporal = (pressure.temporal + targets.temporal * 0.35).clamp(0.0, 100.0);
    pressure.resource = (pressure.resource + targets.resource * 0.4).clamp(0.0, 100.0);
    pressure.moral = (pressure.moral + targets.moral * 0.3).clamp(0.0, 100.0);
}
