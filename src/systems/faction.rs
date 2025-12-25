use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;

use crate::data::factions::{
    load_faction_data, DataError, DetectionProfile, FactionData, FactionInstanceDefinition,
    FactionScope, FactionTypeDefinition, ResponseProfile,
};
use crate::rules::signature::SignatureType;
use crate::simulation::city::{CityState, LocationId, LocationState, LocationTag};
use crate::simulation::evidence::WorldEvidence;

const DEFAULT_FACTION_TYPES_PATH: &str = "./assets/data/faction_types.json";
const DEFAULT_FACTION_INSTANCES_PATH: &str = "./assets/data/factions_city.json";

#[derive(Resource, Debug, Default)]
pub struct FactionEventLog(pub Vec<FactionEvent>);

#[derive(Debug, Clone)]
pub struct FactionEvent {
    pub faction_id: String,
    pub faction_type_id: String,
    pub location_id: LocationId,
    pub level: String,
    pub actions: Vec<crate::data::factions::ResponseAction>,
}

#[derive(Resource, Debug)]
pub struct FactionDirector {
    types: HashMap<String, FactionTypeDefinition>,
    instances: Vec<FactionInstanceDefinition>,
    last_levels: HashMap<(String, u32), String>,
}

impl Default for FactionDirector {
    fn default() -> Self {
        Self {
            types: HashMap::new(),
            instances: Vec::new(),
            last_levels: HashMap::new(),
        }
    }
}

impl FactionDirector {
    pub fn load_default() -> Result<Self, DataError> {
        Self::load_from_paths(DEFAULT_FACTION_TYPES_PATH, DEFAULT_FACTION_INSTANCES_PATH)
    }

    pub fn load_from_paths(
        types_path: impl AsRef<std::path::Path>,
        instances_path: impl AsRef<std::path::Path>,
    ) -> Result<Self, DataError> {
        let data = load_faction_data(types_path, instances_path)?;
        Ok(Self::from_data(data))
    }

    pub fn from_data(data: FactionData) -> Self {
        let types = data
            .types
            .faction_types
            .into_iter()
            .map(|def| (def.id.clone(), def))
            .collect::<HashMap<_, _>>();

        Self {
            types,
            instances: data.instances.factions,
            last_levels: HashMap::new(),
        }
    }
}

pub fn faction_director_system(
    mut director: ResMut<FactionDirector>,
    city: Res<CityState>,
    evidence: Res<WorldEvidence>,
    mut log: ResMut<FactionEventLog>,
) {
    run_faction_director(&mut director, &city, &evidence, &mut log);
}

pub fn run_faction_director(
    director: &mut FactionDirector,
    city: &CityState,
    evidence: &WorldEvidence,
    log: &mut FactionEventLog,
) {
    log.0.clear();

    if director.instances.is_empty() {
        director.last_levels.clear();
        return;
    }

    let signatures_by_location = collect_signatures_by_location(evidence);

    let instances = director.instances.clone();
    for instance in instances.iter() {
        let Some(type_def) = director.types.get(&instance.type_id).cloned() else {
            continue;
        };

        for location in city.locations.values() {
            if !scope_matches(&instance.scope, location) {
                clear_last_level(director, &instance.id, location.id.0);
                continue;
            }

            let detection = detection_profile_owned(instance, &type_def);
            if location.heat < detection.heat_min.unwrap_or(0) as i32 {
                clear_last_level(director, &instance.id, location.id.0);
                continue;
            }

            if !detection.signature_types.is_empty()
                && !location_has_signatures(location, &signatures_by_location, &detection)
            {
                clear_last_level(director, &instance.id, location.id.0);
                continue;
            }

            let response = response_profile_owned(instance, &type_def);
            let Some(threshold) = select_threshold(&response, location.heat) else {
                clear_last_level(director, &instance.id, location.id.0);
                continue;
            };

            let key = (instance.id.clone(), location.id.0);
            if director
                .last_levels
                .get(&key)
                .map(|level| level == &threshold.level)
                .unwrap_or(false)
            {
                continue;
            }

            director
                .last_levels
                .insert(key, threshold.level.clone());
            log.0.push(FactionEvent {
                faction_id: instance.id.clone(),
                faction_type_id: instance.type_id.clone(),
                location_id: location.id,
                level: threshold.level.clone(),
                actions: threshold.actions.clone(),
            });
        }
    }
}

fn collect_signatures_by_location(
    evidence: &WorldEvidence,
) -> HashMap<LocationId, HashSet<SignatureType>> {
    let mut out: HashMap<LocationId, HashSet<SignatureType>> = HashMap::new();
    for event in evidence.signatures.iter() {
        out.entry(event.location_id)
            .or_default()
            .insert(event.signature.signature.signature_type);
    }
    out
}

fn detection_profile_owned(
    instance: &FactionInstanceDefinition,
    type_def: &FactionTypeDefinition,
) -> DetectionProfile {
    instance
        .detection_override
        .clone()
        .unwrap_or_else(|| type_def.detection.clone())
}

fn response_profile_owned(
    instance: &FactionInstanceDefinition,
    type_def: &FactionTypeDefinition,
) -> ResponseProfile {
    instance
        .response_override
        .clone()
        .unwrap_or_else(|| type_def.response.clone())
}

fn select_threshold(response: &ResponseProfile, heat: i32) -> Option<&crate::data::factions::ResponseThreshold> {
    response
        .thresholds
        .iter()
        .filter(|threshold| heat >= threshold.heat as i32)
        .max_by_key(|threshold| threshold.heat)
}

fn scope_matches(scope: &FactionScope, location: &LocationState) -> bool {
    if scope.location_ids.is_empty() && scope.location_tags.is_empty() {
        return true;
    }

    let id_match = scope.location_ids.iter().any(|id| *id == location.id.0);
    let tag_match = scope.location_tags.iter().any(|tag| {
        let expected = map_location_tag(*tag);
        location.tags.contains(&expected)
    });

    id_match || tag_match
}

fn map_location_tag(tag: crate::data::factions::LocationTag) -> LocationTag {
    match tag {
        crate::data::factions::LocationTag::Public => LocationTag::Public,
        crate::data::factions::LocationTag::Residential => LocationTag::Residential,
        crate::data::factions::LocationTag::Industrial => LocationTag::Industrial,
        crate::data::factions::LocationTag::HighSecurity => LocationTag::HighSecurity,
    }
}

fn location_has_signatures(
    location: &LocationState,
    signatures_by_location: &HashMap<LocationId, HashSet<SignatureType>>,
    detection: &DetectionProfile,
) -> bool {
    let Some(found) = signatures_by_location.get(&location.id) else {
        return false;
    };
    detection.signature_types.iter().any(|sig| {
        if !found.contains(sig) {
            return false;
        }
        matches_detection(sig, location)
    })
}

fn matches_detection(signature: &SignatureType, location: &LocationState) -> bool {
    match signature {
        SignatureType::VisualAnomaly => {
            location.surveillance_level > 0 || location.tags.contains(&LocationTag::Public)
        }
        _ => true,
    }
}

fn clear_last_level(director: &mut FactionDirector, faction_id: &str, location_id: u32) {
    let key = (faction_id.to_string(), location_id);
    director.last_levels.remove(&key);
}
