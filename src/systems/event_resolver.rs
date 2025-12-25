use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::rules::signature::SignatureType;
use crate::simulation::case::{CaseEventLog, CaseRegistry};
use crate::simulation::city::{CityState, LocationId};
use crate::simulation::evidence::WorldEvidence;
use crate::systems::faction::{FactionEvent, FactionEventLog};

#[derive(Resource, Debug, Default)]
pub struct ResolvedFactionEventLog(pub Vec<FactionEvent>);

pub fn event_resolver_system(
    mut events: ResMut<FactionEventLog>,
    mut resolved: ResMut<ResolvedFactionEventLog>,
    mut city: ResMut<CityState>,
    evidence: Res<WorldEvidence>,
    mut cases: ResMut<CaseRegistry>,
    mut case_log: ResMut<CaseEventLog>,
) {
    resolve_faction_events(
        &mut events,
        &mut resolved,
        &mut city,
        &evidence,
        &mut cases,
        &mut case_log,
    );
}

pub fn resolve_faction_events(
    events: &mut FactionEventLog,
    resolved: &mut ResolvedFactionEventLog,
    city: &mut CityState,
    evidence: &WorldEvidence,
    cases: &mut CaseRegistry,
    case_log: &mut CaseEventLog,
) {
    resolved.0.clear();

    for event in events.0.drain(..) {
        let Some(location) = city.locations.get_mut(&event.location_id) else {
            continue;
        };

        for action in &event.actions {
            match action.kind.as_str() {
                "SPAWN_PATROL" => {
                    location.police_units = location.police_units.saturating_add(1);
                    location.police_presence = (location.police_presence + 5).clamp(0, 100);
                    location.surveillance_level = (location.surveillance_level + 2).clamp(0, 100);
                }
                "SPAWN_TACTICAL" => {
                    location.police_units = location.police_units.saturating_add(2);
                    location.police_presence = (location.police_presence + 10).clamp(0, 100);
                    location.surveillance_level = (location.surveillance_level + 5).clamp(0, 100);
                }
                "START_INVESTIGATION" => {
                    location.investigators = location.investigators.saturating_add(1);
                    if !cases.has_active_case(&event.faction_id, event.location_id) {
                        let pattern = derive_signature_pattern(evidence, event.location_id);
                        let case_id = cases.create_case(
                            event.faction_id.clone(),
                            event.location_id,
                            pattern,
                            true,
                        );
                        case_log.0.push(format!(
                            "Case {} opened by {} at location {}",
                            case_id, event.faction_id, event.location_id.0
                        ));
                    }
                }
                "ESCALATE_SECURITY" => {
                    location.lockdown_level = (location.lockdown_level + 10).clamp(0, 100);
                    location.surveillance_level = (location.surveillance_level + 10).clamp(0, 100);
                    location.police_presence = (location.police_presence + 10).clamp(0, 100);
                    location.police_units = location.police_units.saturating_add(1);
                }
                "PROXY_CRIME" => {
                    location.gang_units = location.gang_units.saturating_add(1);
                    location.crime_pressure = (location.crime_pressure + 5).clamp(0, 100);
                }
                _ => {}
            }
        }

        let entry = location
            .faction_influence
            .entry(event.faction_id.clone())
            .or_insert(0);
        *entry = entry.saturating_add(5);

        resolved.0.push(event);
    }
}

fn derive_signature_pattern(
    evidence: &WorldEvidence,
    location_id: LocationId,
) -> Vec<SignatureType> {
    let mut counts: HashMap<SignatureType, u32> = HashMap::new();
    for event in evidence.signatures.iter() {
        if event.location_id == location_id {
            *counts
                .entry(event.signature.signature.signature_type)
                .or_insert(0) += 1;
        }
    }

    let mut entries: Vec<(SignatureType, u32)> = counts.into_iter().collect();
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    entries.into_iter().take(3).map(|(sig, _)| sig).collect()
}
