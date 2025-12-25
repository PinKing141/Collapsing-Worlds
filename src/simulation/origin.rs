use std::fs;
use std::path::Path;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::identity::SuperIdentity;
use crate::components::world::Position;
use crate::rules::signature::{SignatureInstance, SignatureSpec, SignatureType};
use crate::simulation::cast::CharacterPower;
use crate::simulation::city::CityState;
use crate::simulation::evidence::WorldEvidence;
use crate::simulation::pressure::PressureState;

const DEFAULT_ORIGINS_PATH: &str = "./assets/data/origins.json";
const DEFAULT_CONTENT_DB_PATH: &str = "./assets/db/content_v1.db";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginCatalog {
    pub origins: Vec<OriginDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginDefinition {
    pub id: String,
    pub label: String,
    pub class_code: String,
    pub subtype_code: String,
    pub summary: String,
    #[serde(default)]
    pub acquisition_hooks: Vec<OriginHook>,
    pub effects: OriginEffects,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginHook {
    pub event_kind: String,
    #[serde(default)]
    pub delivery_channel: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginEffects {
    #[serde(default)]
    pub initial_power_count: usize,
    #[serde(default)]
    pub initial_mutation_count: usize,
    #[serde(default)]
    pub mutation_tags: Vec<String>,
    #[serde(default)]
    pub reputation_delta: i32,
    #[serde(default)]
    pub pressure_delta: PressureDelta,
    #[serde(default)]
    pub signatures: Vec<OriginSignature>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PressureDelta {
    #[serde(default)]
    pub temporal: f32,
    #[serde(default)]
    pub identity: f32,
    #[serde(default)]
    pub institutional: f32,
    #[serde(default)]
    pub moral: f32,
    #[serde(default)]
    pub resource: f32,
    #[serde(default)]
    pub psychological: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginSignature {
    pub signature_type: SignatureType,
    pub strength: i64,
    pub persistence_turns: i64,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct OriginState {
    pub origin_id: Option<String>,
    pub origin_label: Option<String>,
    pub origin_class: Option<String>,
    pub origin_subtype: Option<String>,
    pub summary: Option<String>,
    pub powers: Vec<CharacterPower>,
    pub mutations: Vec<String>,
}

#[derive(Debug)]
pub enum OriginError {
    Io(std::io::Error),
    Parse(serde_json::Error),
    Db(rusqlite::Error),
}

impl std::fmt::Display for OriginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OriginError::Io(err) => write!(f, "I/O error: {}", err),
            OriginError::Parse(err) => write!(f, "Parse error: {}", err),
            OriginError::Db(err) => write!(f, "Database error: {}", err),
        }
    }
}

impl std::error::Error for OriginError {}

#[derive(Debug, Clone)]
struct AcquisitionProfile {
    acq_id: String,
    power_id: i64,
    rarity_weight: i64,
}

impl OriginState {
    pub fn from_definition(origin: &OriginDefinition) -> Self {
        Self {
            origin_id: Some(origin.id.clone()),
            origin_label: Some(origin.label.clone()),
            origin_class: Some(origin.class_code.clone()),
            origin_subtype: Some(origin.subtype_code.clone()),
            summary: Some(origin.summary.clone()),
            powers: Vec::new(),
            mutations: Vec::new(),
        }
    }

    pub fn award_initial_powers(
        &mut self,
        origin: &OriginDefinition,
        db_path: &Path,
        seed: u64,
    ) -> Result<(), OriginError> {
        let mut profiles = load_acquisition_profiles(db_path, origin)?;
        if profiles.is_empty() {
            return Ok(());
        }

        let mut rng = seed ^ hash_seed(&origin.id);
        let mut remaining = origin.effects.initial_power_count;
        while remaining > 0 && !profiles.is_empty() {
            let idx = weighted_choice_index(&profiles, &mut rng);
            let profile = profiles.swap_remove(idx);
            self.powers.push(CharacterPower {
                power_id: profile.power_id,
                expression_id: None,
                acq_id: Some(profile.acq_id),
                mastery: 0,
            });
            remaining -= 1;
        }

        let mutation_limit = origin.effects.initial_mutation_count;
        if mutation_limit > 0 {
            self.mutations = origin
                .effects
                .mutation_tags
                .iter()
                .cloned()
                .take(mutation_limit)
                .collect();
        }

        if self.mutations.is_empty() && self.powers.is_empty() {
            self.mutations = origin.effects.mutation_tags.clone();
        }

        Ok(())
    }

    pub fn apply_effects(&self, world: &mut World, player: Entity, origin: &OriginDefinition) {
        if origin.effects.reputation_delta != 0 {
            if let Some(mut sup) = world.get_mut::<SuperIdentity>(player) {
                sup.reputation += origin.effects.reputation_delta;
            }
        }

        if let Some(mut pressure) = world.get_resource_mut::<PressureState>() {
            pressure.temporal = (pressure.temporal + origin.effects.pressure_delta.temporal)
                .clamp(0.0, 100.0);
            pressure.identity = (pressure.identity + origin.effects.pressure_delta.identity)
                .clamp(0.0, 100.0);
            pressure.institutional = (pressure.institutional
                + origin.effects.pressure_delta.institutional)
                .clamp(0.0, 100.0);
            pressure.moral = (pressure.moral + origin.effects.pressure_delta.moral)
                .clamp(0.0, 100.0);
            pressure.resource = (pressure.resource + origin.effects.pressure_delta.resource)
                .clamp(0.0, 100.0);
            pressure.psychological = (pressure.psychological
                + origin.effects.pressure_delta.psychological)
                .clamp(0.0, 100.0);
        }

        let signatures = origin_signatures(origin);
        if !signatures.is_empty() {
            let location_id = world
                .get_resource::<CityState>()
                .and_then(|city| {
                    world
                        .get::<Position>(player)
                        .map(|pos| city.location_for_position(pos))
                })
                .unwrap_or_else(|| world.resource::<CityState>().active_location);
            let mut evidence = world.resource_mut::<WorldEvidence>();
            evidence.emit(location_id, &signatures);
        }
    }
}

pub fn assign_origin_for_player(world: &mut World, player: Entity, seed: u64) {
    let catalog = match load_origin_catalog(DEFAULT_ORIGINS_PATH) {
        Ok(catalog) => catalog,
        Err(err) => {
            eprintln!("Failed to load origins: {}", err);
            return;
        }
    };

    let Some(origin) = select_origin(&catalog, seed) else {
        eprintln!("No origins available to select.");
        return;
    };

    let mut state = OriginState::from_definition(&origin);
    if let Err(err) = state.award_initial_powers(&origin, Path::new(DEFAULT_CONTENT_DB_PATH), seed) {
        eprintln!("Failed to award origin powers: {}", err);
    }
    state.apply_effects(world, player, &origin);
    world.insert_resource(state);
}

pub fn load_origin_catalog(path: &str) -> Result<OriginCatalog, OriginError> {
    let data = fs::read_to_string(path).map_err(OriginError::Io)?;
    serde_json::from_str(&data).map_err(OriginError::Parse)
}

fn select_origin(catalog: &OriginCatalog, seed: u64) -> Option<OriginDefinition> {
    if catalog.origins.is_empty() {
        return None;
    }
    let idx = (seed as usize) % catalog.origins.len();
    catalog.origins.get(idx).cloned()
}

fn origin_signatures(origin: &OriginDefinition) -> Vec<SignatureInstance> {
    origin
        .effects
        .signatures
        .iter()
        .map(|sig| {
            SignatureSpec {
                signature_type: sig.signature_type,
                strength: sig.strength,
                persistence_turns: sig.persistence_turns,
            }
            .to_instance()
        })
        .collect()
}

fn load_acquisition_profiles(
    db_path: &Path,
    origin: &OriginDefinition,
) -> Result<Vec<AcquisitionProfile>, OriginError> {
    let conn = rusqlite::Connection::open(db_path).map_err(OriginError::Db)?;
    let mut stmt = conn
        .prepare(
            "SELECT acq_id, power_id, rarity_weight\
             FROM power_acquisition_profile\
             WHERE is_enabled = 1\
               AND origin_class = ?1\
               AND (origin_subtype = ?2 OR origin_subtype IS NULL OR origin_subtype = '')",
        )
        .map_err(OriginError::Db)?;

    let rows = stmt
        .query_map((&origin.class_code, &origin.subtype_code), |row| {
            Ok(AcquisitionProfile {
                acq_id: row.get(0)?,
                power_id: row.get(1)?,
                rarity_weight: row.get(2)?,
            })
        })
        .map_err(OriginError::Db)?;

    let mut profiles = Vec::new();
    for row in rows {
        profiles.push(row.map_err(OriginError::Db)?);
    }

    Ok(profiles)
}

fn weighted_choice_index(profiles: &[AcquisitionProfile], rng: &mut u64) -> usize {
    let total_weight: i64 = profiles
        .iter()
        .map(|p| p.rarity_weight.max(1))
        .sum();
    if total_weight <= 0 {
        return ((*rng as usize) % profiles.len()).min(profiles.len() - 1);
    }
    let roll = (next_u64(rng) % (total_weight as u64)).max(0) as i64;
    let mut acc = 0;
    for (idx, profile) in profiles.iter().enumerate() {
        acc += profile.rarity_weight.max(1);
        if roll < acc {
            return idx;
        }
    }
    profiles.len().saturating_sub(1)
}

fn next_u64(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1);
    *state
}

fn hash_seed(value: &str) -> u64 {
    let mut hash = 1469598103934665603u64;
    for byte in value.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_choice_stays_in_bounds() {
        let profiles = vec![
            AcquisitionProfile {
                acq_id: "a".to_string(),
                power_id: 1,
                rarity_weight: 10,
            },
            AcquisitionProfile {
                acq_id: "b".to_string(),
                power_id: 2,
                rarity_weight: 1,
            },
        ];
        let mut seed = 42u64;
        let idx = weighted_choice_index(&profiles, &mut seed);
        assert!(idx < profiles.len());
    }
}
