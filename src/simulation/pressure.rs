use bevy_ecs::prelude::*;

use crate::rules::use_power::PressureModifiers;

#[derive(Resource, Debug, Clone, Copy)]
pub struct PressureState {
    pub temporal: f32,
    pub identity: f32,
    pub institutional: f32,
    pub moral: f32,
    pub resource: f32,
    pub psychological: f32,
}

impl Default for PressureState {
    fn default() -> Self {
        Self {
            temporal: 0.0,
            identity: 0.0,
            institutional: 0.0,
            moral: 0.0,
            resource: 0.0,
            psychological: 0.0,
        }
    }
}

impl PressureState {
    pub fn to_modifiers(&self) -> PressureModifiers {
        let cost_pressure = ((self.resource + self.temporal) / 200.0).clamp(0.0, 1.0);
        let risk_pressure = ((self.identity + self.institutional + self.moral + self.psychological)
            / 400.0)
            .clamp(0.0, 1.0);

        PressureModifiers {
            cost_scale: 1.0 + cost_pressure as f64 * 0.15,
            risk_scale: 1.0 + risk_pressure as f64 * 0.2,
        }
    }
}
