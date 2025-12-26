use std::fs;
use std::path::Path;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::content::{OriginAcquisitionProfile, PowerRepository, SqlitePowerRepository};
use crate::components::identity::SuperIdentity;
use crate::components::world::Position;
use crate::rules::signature::{SignatureInstance, SignatureSpec, SignatureType};
use crate::simulation::cast::CharacterPower;
use crate::simulation::city::CityState;
use crate::simulation::evidence::WorldEvidence;
use crate::simulation::pressure::PressureState;

const DEFAULT_ORIGINS_PATH: &str = "./assets/data/origins.json";
const DEFAULT_ORIGIN_PATHS_PATH: &str = "./assets/data/origin_paths.json";
const DEFAULT_CONTENT_DB_PATH: &str = "./assets/db/content_v1.db";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginCatalog {
    pub origins: Vec<OriginDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OriginPathCatalog {
    pub paths: Vec<OriginPathDefinition>,
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
pub struct OriginPathDefinition {
    pub id: String,
    pub label: String,
    pub summary: String,
    #[serde(default)]
    pub availability: OriginPathAvailability,
    pub stages: Vec<OriginPathStage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OriginPathAvailability {
    #[serde(default = "default_origin_path_weight")]
    pub weight: u32,
    #[serde(default)]
    pub required_origin_ids: Vec<String>,
    #[serde(default)]
    pub required_origin_classes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginPathStage {
    pub id: String,
    pub label: String,
    pub summary: String,
    #[serde(default)]
    pub requirement: OriginStageRequirement,
    #[serde(default)]
    pub reward: OriginStageReward,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OriginStageRequirement {
    #[serde(default)]
    pub progress_needed: u32,
    #[serde(default)]
    pub progress_per_tick: u32,
    #[serde(default)]
    pub event_tags: Vec<String>,
    #[serde(default)]
    pub event_progress: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OriginStageReward {
    #[serde(default)]
    pub reputation_delta: i32,
    #[serde(default)]
    pub pressure_delta: PressureDelta,
    #[serde(default)]
    pub mutation_tags: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
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

#[derive(Resource, Debug, Clone, Default)]
pub struct OriginQuestState {
    pub path_id: Option<String>,
    pub stage_index: usize,
    pub stage_progress: u32,
    pub completed: bool,
    pub completed_stages: Vec<String>,
    pub discovered_paths: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct OriginEffectReport {
    pub event_tags: Vec<String>,
    pub discoveries: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct OriginEffectApplication {
    pub rewards: Vec<OriginStageReward>,
    pub messages: Vec<String>,
}

#[derive(Debug)]
pub enum OriginError {
    Io(std::io::Error),
    Parse(serde_json::Error),
    Repo(String),
    NotFound(String),
}

impl std::fmt::Display for OriginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OriginError::Io(err) => write!(f, "I/O error: {}", err),
            OriginError::Parse(err) => write!(f, "Parse error: {}", err),
            OriginError::Repo(err) => write!(f, "Repository error: {}", err),
            OriginError::NotFound(message) => write!(f, "Not found: {}", message),
        }
    }
}

impl std::error::Error for OriginError {}

pub fn parse_origin_effects(effects: &[String]) -> OriginEffectReport {
    let mut report = OriginEffectReport::default();
    for effect in effects {
        let mut parts = effect.split(':');
        let key = parts.next().unwrap_or("").trim();
        match key {
            "origin.event" => {
                if let Some(tag) = parts.next().map(str::trim).filter(|tag| !tag.is_empty()) {
                    report.event_tags.push(tag.to_string());
                }
            }
            "origin.discover" => {
                if let Some(path_id) = parts.next().map(str::trim).filter(|id| !id.is_empty()) {
                    report.discoveries.push(path_id.to_string());
                }
            }
            _ => {}
        }
    }
    report
}

pub fn apply_origin_effects(
    report: OriginEffectReport,
    quest: &mut OriginQuestState,
    catalog: &OriginPathCatalog,
) -> OriginEffectApplication {
    let mut application = OriginEffectApplication::default();

    for path_id in report.discoveries {
        let Some(path) = catalog.paths.iter().find(|path| path.id == path_id) else {
            application
                .messages
                .push(format!("Origin clue ignored (unknown path): {}", path_id));
            continue;
        };
        if push_unique(&mut quest.discovered_paths, path.id.clone()) {
            application.messages.push(format!(
                "Origin path discovered: {} - {}",
                path.id, path.label
            ));
        }
    }

    if quest.path_id.is_some() {
        for tag in report.event_tags {
            let before_stage = quest.stage_index;
            let before_progress = quest.stage_progress;
            let before_completed = quest.completed_stages.len();
            let mut rewards = register_origin_event(quest, catalog, &tag);
            if before_stage != quest.stage_index
                || before_progress != quest.stage_progress
                || before_completed != quest.completed_stages.len()
            {
                application
                    .messages
                    .push(format!("Origin progress recorded: {}", tag));
            }
            application.rewards.append(&mut rewards);
        }
    }

    application
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
        repo: &dyn PowerRepository,
        seed: u64,
    ) -> Result<(), OriginError> {
        let mut profiles = load_acquisition_profiles(repo, origin)?;
        if profiles.is_empty() {
            return Ok(());
        }

        let mut rng = seed ^ hash_seed(&origin.id);
        let mut remaining = origin.effects.initial_power_count;
        while remaining > 0 && !profiles.is_empty() {
            let idx = weighted_choice_index(&profiles, &mut rng);
            let profile = profiles.swap_remove(idx);
            self.powers.push(CharacterPower {
                power_id: profile.power_id.0,
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
    match SqlitePowerRepository::open(Path::new(DEFAULT_CONTENT_DB_PATH)) {
        Ok(repo) => {
            if let Err(err) = state.award_initial_powers(&origin, &repo, seed) {
                eprintln!("Failed to award origin powers: {}", err);
            }
        }
        Err(err) => {
            eprintln!("Failed to open content DB for origins: {}", err);
        }
    }
    state.apply_effects(world, player, &origin);
    world.insert_resource(state);
}

pub fn load_origin_catalog(path: &str) -> Result<OriginCatalog, OriginError> {
    let data = fs::read_to_string(path).map_err(OriginError::Io)?;
    serde_json::from_str(&data).map_err(OriginError::Parse)
}

pub fn load_origin_path_catalog(path: &str) -> Result<OriginPathCatalog, OriginError> {
    let data = fs::read_to_string(path).map_err(OriginError::Io)?;
    serde_json::from_str(&data).map_err(OriginError::Parse)
}

pub fn load_default_origin_path_catalog() -> Result<OriginPathCatalog, OriginError> {
    load_origin_path_catalog(DEFAULT_ORIGIN_PATHS_PATH)
}

pub fn select_origin_paths(
    catalog: &OriginPathCatalog,
    origin_state: Option<&OriginState>,
    seed: u64,
    count: usize,
) -> Vec<OriginPathDefinition> {
    let mut candidates: Vec<OriginPathDefinition> = catalog
        .paths
        .iter()
        .filter(|path| is_path_available(path, origin_state))
        .cloned()
        .collect();
    if candidates.is_empty() || count == 0 {
        return Vec::new();
    }
    let mut rng = seed ^ hash_seed("origin_paths");
    let mut chosen = Vec::new();
    let mut remaining = count.min(candidates.len());
    while remaining > 0 && !candidates.is_empty() {
        let idx = weighted_origin_path_index(&candidates, &mut rng);
        chosen.push(candidates.swap_remove(idx));
        remaining = remaining.saturating_sub(1);
    }
    chosen
}

pub fn start_origin_path(
    state: &mut OriginQuestState,
    catalog: &OriginPathCatalog,
    path_id: &str,
) -> Result<OriginPathDefinition, OriginError> {
    let Some(path) = catalog.paths.iter().find(|path| path.id == path_id) else {
        return Err(OriginError::NotFound(format!(
            "origin path {}",
            path_id
        )));
    };
    state.path_id = Some(path.id.clone());
    state.stage_index = 0;
    state.stage_progress = 0;
    state.completed = false;
    state.completed_stages.clear();
    Ok(path.clone())
}

pub fn current_origin_stage<'a>(
    state: &OriginQuestState,
    catalog: &'a OriginPathCatalog,
) -> Option<&'a OriginPathStage> {
    let path_id = state.path_id.as_deref()?;
    let path = catalog.paths.iter().find(|path| path.id == path_id)?;
    path.stages.get(state.stage_index)
}

pub fn tick_origin_path(
    state: &mut OriginQuestState,
    catalog: &OriginPathCatalog,
) -> Vec<OriginStageReward> {
    advance_origin_path(state, catalog, OriginAdvance::Tick)
}

pub fn register_origin_event(
    state: &mut OriginQuestState,
    catalog: &OriginPathCatalog,
    event_tag: &str,
) -> Vec<OriginStageReward> {
    advance_origin_path(state, catalog, OriginAdvance::Event(event_tag))
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
    repo: &dyn PowerRepository,
    origin: &OriginDefinition,
) -> Result<Vec<OriginAcquisitionProfile>, OriginError> {
    repo.acquisition_profiles_for_origin(&origin.class_code, &origin.subtype_code)
        .map_err(|err| OriginError::Repo(err.to_string()))
}

fn weighted_choice_index(profiles: &[OriginAcquisitionProfile], rng: &mut u64) -> usize {
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

fn weighted_origin_path_index(paths: &[OriginPathDefinition], rng: &mut u64) -> usize {
    let total_weight: u64 = paths
        .iter()
        .map(|path| path.availability.weight.max(1) as u64)
        .sum();
    if total_weight == 0 {
        return ((*rng as usize) % paths.len()).min(paths.len() - 1);
    }
    let roll = next_u64(rng) % total_weight;
    let mut acc = 0u64;
    for (idx, path) in paths.iter().enumerate() {
        acc += path.availability.weight.max(1) as u64;
        if roll < acc {
            return idx;
        }
    }
    paths.len().saturating_sub(1)
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

fn push_unique(target: &mut Vec<String>, value: String) -> bool {
    if target.iter().any(|entry| entry == &value) {
        return false;
    }
    target.push(value);
    true
}

fn is_path_available(path: &OriginPathDefinition, origin_state: Option<&OriginState>) -> bool {
    let availability = &path.availability;
    if availability.required_origin_ids.is_empty() && availability.required_origin_classes.is_empty()
    {
        return true;
    }
    let Some(origin_state) = origin_state else {
        return true;
    };
    let origin_id = origin_state.origin_id.as_deref();
    let origin_class = origin_state.origin_class.as_deref();
    if !availability.required_origin_ids.is_empty()
        && origin_id
            .map(|id| availability.required_origin_ids.iter().any(|req| req == id))
            .unwrap_or(false)
    {
        return true;
    }
    if !availability.required_origin_classes.is_empty()
        && origin_class
            .map(|class| availability.required_origin_classes.iter().any(|req| req == class))
            .unwrap_or(false)
    {
        return true;
    }
    availability.required_origin_ids.is_empty() && availability.required_origin_classes.is_empty()
}

fn default_origin_path_weight() -> u32 {
    1
}

enum OriginAdvance<'a> {
    Tick,
    Event(&'a str),
}

fn advance_origin_path(
    state: &mut OriginQuestState,
    catalog: &OriginPathCatalog,
    advance: OriginAdvance<'_>,
) -> Vec<OriginStageReward> {
    let mut rewards = Vec::new();
    if state.completed {
        return rewards;
    }
    let path_id = match state.path_id.as_deref() {
        Some(id) => id,
        None => return rewards,
    };
    let Some(path) = catalog.paths.iter().find(|path| path.id == path_id) else {
        return rewards;
    };
    if state.stage_index >= path.stages.len() {
        state.completed = true;
        return rewards;
    }
    let stage = &path.stages[state.stage_index];
    let progress_delta = match advance {
        OriginAdvance::Tick => stage.requirement.progress_per_tick,
        OriginAdvance::Event(tag) => {
            if stage
                .requirement
                .event_tags
                .iter()
                .any(|entry| entry.eq_ignore_ascii_case(tag))
            {
                stage.requirement.event_progress.max(1)
            } else {
                0
            }
        }
    };
    if progress_delta == 0 {
        return rewards;
    }
    state.stage_progress = state.stage_progress.saturating_add(progress_delta);
    loop {
        if state.stage_index >= path.stages.len() {
            state.completed = true;
            break;
        }
        let stage = &path.stages[state.stage_index];
        let needed = stage.requirement.progress_needed.max(1);
        if state.stage_progress < needed {
            break;
        }
        state.completed_stages.push(stage.id.clone());
        rewards.push(stage.reward.clone());
        state.stage_index = state.stage_index.saturating_add(1);
        state.stage_progress = 0;
    }
    rewards
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
