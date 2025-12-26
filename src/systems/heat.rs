use bevy_ecs::prelude::*;

use crate::components::world::{Player, Position};
use crate::rules::signature::{SignatureInstance, SignatureType};
use crate::simulation::case::CaseRegistry;
<<<<<<< Updated upstream
<<<<<<< Updated upstream
use crate::simulation::city::{CityEvent, CityEventKind, CityEventLog, CityState, HeatResponse, LocationId, LocationTag};
use crate::simulation::combat::CombatConsequence;
=======
use crate::simulation::city::{CityEvent, CityEventKind, CityEventLog, CityId, CityState, HeatResponse, LocationId, LocationTag};
>>>>>>> Stashed changes
=======
use crate::simulation::city::{CityEvent, CityEventKind, CityEventLog, CityId, CityState, HeatResponse, LocationId, LocationTag};
>>>>>>> Stashed changes
use crate::simulation::evidence::WorldEvidence;
use crate::simulation::identity_evidence::{IdentityEvidenceStore, PersonaHint};
use crate::simulation::time::GameTime;

/// Resource capturing recent world responses to heat thresholds.
#[derive(Resource, Default, Debug)]
pub struct WorldEventLog(pub Vec<String>);

/// System: updates the active location based on the player position.
pub fn update_active_location_system(
    mut city: ResMut<CityState>,
    query: Query<&Position, With<Player>>,
) {
    if let Ok(pos) = query.get_single() {
        city.active_location = city.location_for_position(pos);
    }
}

/// System: applies signature heat and decays evidence persistence.
pub fn signature_heat_system(
    mut city: ResMut<CityState>,
    mut evidence: ResMut<WorldEvidence>,
    mut identity: ResMut<IdentityEvidenceStore>,
    time: Res<GameTime>,
    mut log: ResMut<WorldEventLog>,
    mut city_events: ResMut<CityEventLog>,
) {
    log.0.clear();

    for event in evidence.signatures.iter_mut() {
        let (in_public, surveillance_level, witness_count) = city
            .locations
            .get(&event.location_id)
            .map(|location| {
                let in_public = location.tags.contains(&LocationTag::Public);
                let witness_count = if in_public {
                    2 + (location.surveillance_level / 20).max(0) as u32
                } else {
                    0
                };
                (in_public, location.surveillance_level, witness_count)
            })
            .unwrap_or((true, 0, 0));
        apply_signatures(
            &mut city,
            event.location_id,
            std::slice::from_ref(&event.signature),
            witness_count,
            in_public,
            &mut log,
            &mut city_events,
        );

        if event.is_new {
            event.is_new = false;
            let visual_quality = (surveillance_level as i32 + (witness_count as i32 * 10))
                .clamp(0, 100) as u8;
            identity.record(
                event.location_id,
                time.tick,
                vec![event.signature.signature.signature_type],
                witness_count,
                visual_quality,
                PersonaHint::Unknown,
                Vec::new(),
            );
        }
    }

    evidence.tick_decay();
}

/// System: passive heat decay across all locations.
pub fn heat_decay_system(
    mut city: ResMut<CityState>,
    cases: Res<CaseRegistry>,
    mut city_events: ResMut<CityEventLog>,
) {
    decay_heat(&mut city, &cases, &mut city_events);
}

pub fn apply_signatures(
    city: &mut CityState,
    location_id: LocationId,
    signatures: &[SignatureInstance],
    witnesses: u32,
    in_public: bool,
    log: &mut WorldEventLog,
    city_events: &mut CityEventLog,
) {
    let mut total_delta = 0;
    for sig in signatures {
        let mut delta = heat_from_signature(sig);
        if in_public {
            delta += 1;
        }
        if witnesses > 0 {
            delta += witnesses.min(5) as i32;
        }
        total_delta += delta;
    }

    let city_id = city.city_id;
    if let Some(location) = city.locations.get_mut(&location_id) {
        location.heat = (location.heat + total_delta).clamp(0, 100);
        update_response(location, log, city_id, city_events);
    }
}

pub fn decay_heat(city: &mut CityState, cases: &CaseRegistry, city_events: &mut CityEventLog) {
    let mut log = WorldEventLog::default();
    let city_id = city.city_id;
    for location in city.locations.values_mut() {
        let mut decay: i32 = 1;
        if location.police_presence >= 30 {
            decay += 1;
        }
        if location.lockdown_level >= 30 {
            decay += 1;
        }
        if location.crime_pressure >= 12 {
            decay = decay.saturating_sub(1);
        }
        if cases.any_heat_lock(location.id) {
            decay = decay.saturating_sub(1);
        }
        location.heat = (location.heat - decay).max(0);
        update_response(location, &mut log, city_id, city_events);
<<<<<<< Updated upstream
    }
}

pub fn apply_combat_consequence_heat(
    city: &mut CityState,
    location_id: LocationId,
    consequence: CombatConsequence,
    log: &mut WorldEventLog,
    city_events: &mut CityEventLog,
) {
    let public_delta = (consequence.publicness as i32 / 18).max(0);
    let collateral_delta = (consequence.collateral as i32 / 14).max(0);
    let notoriety_delta = (consequence.notoriety as i32 / 24).max(0);
    let total_delta = public_delta + collateral_delta + notoriety_delta;

    if total_delta == 0 {
        return;
    }

    if let Some(location) = city.locations.get_mut(&location_id) {
        location.heat = (location.heat + total_delta).clamp(0, 100);
        update_response(location, log, city, city_events);
=======
>>>>>>> Stashed changes
    }
}

fn update_response(
    location: &mut crate::simulation::city::LocationState,
    log: &mut WorldEventLog,
    city_id: CityId,
    city_events: &mut CityEventLog,
) {
    let next = response_for_heat(location.heat);
    if next != location.response {
        location.response = next;
        log.0.push(format!(
            "Location {} response -> {:?}",
            location.id.0, location.response
        ));
        city_events.0.push(CityEvent {
            city_id,
            location_id: location.id,
            kind: CityEventKind::HeatResponseChanged {
                response: location.response,
            },
        });
    }
}

fn heat_from_signature(signature: &SignatureInstance) -> i32 {
    let base = ((signature.signature.strength / 10).max(1)) as i32;
    match signature.signature.signature_type {
        SignatureType::VisualAnomaly => base + 1,
        SignatureType::EmSpike => base + 2,
        SignatureType::ThermalBloom => base + 1,
        SignatureType::AcousticShock => base + 1,
        SignatureType::ChemicalResidue => base + 1,
        SignatureType::PsychicEcho => base + 1,
        SignatureType::DimensionalResidue => base + 2,
        SignatureType::GraviticDisturbance => base + 2,
        SignatureType::ArcaneResonance => base + 2,
        SignatureType::CausalImprint => base + 2,
        SignatureType::KineticStress => base + 1,
        SignatureType::RadiationTrace => base + 2,
        SignatureType::BioMarker => base,
    }
}

fn response_for_heat(heat: i32) -> HeatResponse {
    if heat >= 70 {
        HeatResponse::FactionAttention
    } else if heat >= 50 {
        HeatResponse::Investigation
    } else if heat >= 30 {
        HeatResponse::PolicePatrol
    } else {
        HeatResponse::None
    }
}
