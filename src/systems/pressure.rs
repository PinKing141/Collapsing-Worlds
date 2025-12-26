use bevy_ecs::prelude::*;

use crate::simulation::case::{CaseRegistry, CaseStatus};
use crate::simulation::city::CityState;
use crate::simulation::evidence::WorldEvidence;
use crate::simulation::pressure::PressureState;
use crate::simulation::time::GameTime;

pub fn pressure_system(
    mut pressure: ResMut<PressureState>,
    city: Res<CityState>,
    evidence: Res<WorldEvidence>,
    cases: Res<CaseRegistry>,
    time: Res<GameTime>,
) {
    update_pressure(&mut pressure, &city, &evidence, &cases, &time);
}

pub fn update_pressure(
    pressure: &mut PressureState,
    city: &CityState,
    evidence: &WorldEvidence,
    cases: &CaseRegistry,
    time: &GameTime,
) {
    let location_id = city.active_location;
    let heat = city
        .locations
        .get(&location_id)
        .map(|loc| loc.heat)
        .unwrap_or(0) as f32;

    let evidence_strength: f32 = evidence
        .signatures
        .iter()
        .filter(|event| event.location_id == location_id)
        .map(|event| event.signature.signature.strength as f32)
        .sum();
    let evidence_pressure = (evidence_strength / 5.0).clamp(0.0, 100.0);

    let mut active_case_count: f32 = 0.0;
    let mut max_case_progress: f32 = 0.0;
    for case in cases.cases.iter() {
        if case.location_id != location_id || case.status != CaseStatus::Active {
            continue;
        }
        active_case_count += 1.0;
        max_case_progress = max_case_progress.max(case.progress as f32);
    }

    let time_pressure = (time.hour as f32 / 23.0) * 100.0;
    let case_pressure = (max_case_progress + active_case_count * 8.0).clamp(0.0, 100.0);

    let temporal_target = (time_pressure + heat * 0.1).clamp(0.0, 100.0);
    let identity_target = (evidence_pressure * 0.7 + case_pressure * 0.6).clamp(0.0, 100.0);
    let institutional_target = (heat * 0.6 + case_pressure * 0.4).clamp(0.0, 100.0);
    let moral_target = (case_pressure * 0.5 + heat * 0.2 + evidence_pressure * 0.2)
        .clamp(0.0, 100.0);
    let resource_target = (heat * 0.4 + case_pressure * 0.4 + time_pressure * 0.2)
        .clamp(0.0, 100.0);
    let psychological_target = (heat * 0.5 + evidence_pressure * 0.4 + case_pressure * 0.2)
        .clamp(0.0, 100.0);

    let step = 2.0 + active_case_count * 0.25;
    pressure.temporal = approach(pressure.temporal, temporal_target, step);
    pressure.identity = approach(pressure.identity, identity_target, step);
    pressure.institutional = approach(pressure.institutional, institutional_target, step);
    pressure.moral = approach(pressure.moral, moral_target, step);
    pressure.resource = approach(pressure.resource, resource_target, step);
    pressure.psychological = approach(pressure.psychological, psychological_target, step);
}

fn approach(current: f32, target: f32, step: f32) -> f32 {
    if (current - target).abs() <= step {
        target
    } else if current < target {
        (current + step).min(100.0)
    } else {
        (current - step).max(0.0)
    }
}
