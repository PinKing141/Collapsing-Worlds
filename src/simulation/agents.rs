use std::collections::HashMap;
use std::fs;
use std::path::Path;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::rules::signature::{SignatureInstance, SignatureSpec};
use crate::simulation::city::{CityState, LocationId};
use crate::simulation::time::GameTime;

const DEFAULT_AGENTS_PATH: &str = "./assets/data/agents.json";
const DAYS_PER_YEAR: u32 = 336;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCatalog {
    pub schema_version: u32,
    pub roles: Vec<AgentRole>,
    pub templates: Vec<AgentTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRole {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub incident_chance: f32,
    #[serde(default)]
    pub incident_cooldown: u64,
    #[serde(default)]
    pub incident_signatures: Vec<SignatureSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    pub id: String,
    pub name: String,
    pub role_id: String,
    pub home_location: u32,
    pub haunt_location: u32,
    #[serde(default)]
    pub move_interval: u64,
    #[serde(default)]
    pub age_years: Option<u32>,
    #[serde(default)]
    pub age_min: Option<u32>,
    #[serde(default)]
    pub age_max: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub id: u32,
    pub name: String,
    pub role_id: String,
}

#[derive(Debug, Clone)]
pub struct AgentGoal {
    pub description: String,
    pub target_location: LocationId,
    pub priority: u8,
    pub created_tick: u64,
}

#[derive(Debug, Clone)]
pub struct AgentSchedule {
    pub home_location: LocationId,
    pub haunt_location: LocationId,
    pub move_interval: u64,
    pub next_move_tick: u64,
}

#[derive(Debug, Clone)]
pub struct AgentState {
    pub agent: Agent,
    pub goal: AgentGoal,
    pub schedule: AgentSchedule,
    pub current_location: LocationId,
    pub last_incident_tick: u64,
    pub age_years: u32,
    pub birth_day: u32,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct AgentRegistry {
    pub roles: HashMap<String, AgentRole>,
    pub agents: Vec<AgentState>,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct AgentEventLog(pub Vec<AgentEvent>);

#[derive(Debug, Clone)]
pub enum AgentEvent {
    EnteredLocation {
        agent_id: u32,
        location_id: LocationId,
    },
    LeftLocation {
        agent_id: u32,
        location_id: LocationId,
    },
    Incident {
        agent_id: u32,
        role_id: String,
        location_id: LocationId,
        signatures: Vec<SignatureInstance>,
    },
}

#[derive(Debug)]
pub enum AgentLoadError {
    Io(std::io::Error),
    Parse(serde_json::Error),
}

impl std::fmt::Display for AgentLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentLoadError::Io(err) => write!(f, "I/O error: {}", err),
            AgentLoadError::Parse(err) => write!(f, "Parse error: {}", err),
        }
    }
}

impl std::error::Error for AgentLoadError {}

impl AgentRegistry {
    pub fn load_default() -> Result<Self, AgentLoadError> {
        Self::load_from_path(Path::new(DEFAULT_AGENTS_PATH))
    }

    pub fn load_from_path(path: &Path) -> Result<Self, AgentLoadError> {
        let data = fs::read_to_string(path).map_err(AgentLoadError::Io)?;
        let catalog: AgentCatalog = serde_json::from_str(&data).map_err(AgentLoadError::Parse)?;
        Ok(Self::from_catalog(catalog))
    }

    pub fn from_catalog(catalog: AgentCatalog) -> Self {
        let mut roles = HashMap::new();
        for role in catalog.roles {
            roles.insert(role.id.clone(), role);
        }

        let mut agents = Vec::new();
        for (idx, template) in catalog.templates.into_iter().enumerate() {
            let Some(role) = roles.get(&template.role_id) else {
                eprintln!("Agent template {} references missing role {}", template.id, template.role_id);
                continue;
            };
            let home_location = LocationId(template.home_location);
            let haunt_location = LocationId(template.haunt_location);
            let move_interval = if template.move_interval > 0 {
                template.move_interval
            } else {
                4
            };
            let age_years = resolve_agent_age_years(&template, idx as u32);
            let target_location = default_target_location(home_location, haunt_location, 0);
            let agent = Agent {
                id: (idx as u32) + 1,
                name: template.name,
                role_id: role.id.clone(),
            };
            let goal = AgentGoal {
                description: format!("Establishing routine near location {}", target_location.0),
                target_location,
                priority: 1,
                created_tick: 0,
            };
            let schedule = AgentSchedule {
                home_location,
                haunt_location,
                move_interval,
                next_move_tick: 0,
            };
            agents.push(AgentState {
                agent,
                goal,
                schedule,
                current_location: home_location,
                last_incident_tick: 0,
                age_years,
                birth_day: 0,
            });
        }

        Self { roles, agents }
    }
}

pub fn tick_agents(
    registry: &mut AgentRegistry,
    _city: &CityState,
    time: &GameTime,
    events: &mut AgentEventLog,
) {
    events.0.clear();

    for state in registry.agents.iter_mut() {
        update_agent_age(state, time.day);
        let target_location = default_target_location(
            state.schedule.home_location,
            state.schedule.haunt_location,
            time.hour,
        );

        if state.goal.target_location != target_location {
            state.goal = AgentGoal {
                description: format!("Heading toward location {}", target_location.0),
                target_location,
                priority: 1,
                created_tick: time.tick,
            };
        }

        if state.current_location != target_location && time.tick >= state.schedule.next_move_tick {
            events.0.push(AgentEvent::LeftLocation {
                agent_id: state.agent.id,
                location_id: state.current_location,
            });
            state.current_location = target_location;
            events.0.push(AgentEvent::EnteredLocation {
                agent_id: state.agent.id,
                location_id: state.current_location,
            });
            state.schedule.next_move_tick = time.tick + state.schedule.move_interval;
        }

        if let Some(role) = registry.roles.get(&state.agent.role_id) {
            let cooldown = role.incident_cooldown.max(1);
            let since_last = time.tick.saturating_sub(state.last_incident_tick);
            let roll = ((time.tick + state.agent.id as u64 * 13) % 100) as f32 / 100.0;
            if roll <= role.incident_chance && since_last >= cooldown {
                let signatures = role
                    .incident_signatures
                    .iter()
                    .map(SignatureSpec::to_instance)
                    .collect();
                events.0.push(AgentEvent::Incident {
                    agent_id: state.agent.id,
                    role_id: role.id.clone(),
                    location_id: state.current_location,
                    signatures,
                });
                state.last_incident_tick = time.tick;
            }
        }
    }
}

pub fn tick_agents_system(
    mut registry: ResMut<AgentRegistry>,
    city: Res<CityState>,
    time: Res<GameTime>,
    mut events: ResMut<AgentEventLog>,
) {
    tick_agents(&mut registry, &city, &time, &mut events);
}

fn default_target_location(home: LocationId, haunt: LocationId, hour: u8) -> LocationId {
    if (8..18).contains(&hour) {
        haunt
    } else {
        home
    }
}

fn resolve_agent_age_years(template: &AgentTemplate, seed: u32) -> u32 {
    if let Some(age) = template.age_years {
        return age.max(18);
    }
    let min_age = template.age_min.unwrap_or(24).max(18);
    let max_age = template.age_max.unwrap_or(55).max(min_age);
    if max_age == min_age {
        return min_age;
    }
    let span = max_age - min_age;
    let offset = seed % (span + 1);
    min_age + offset
}

fn update_agent_age(state: &mut AgentState, current_day: u32) {
    if state.birth_day == 0 {
        let offset = state.age_years.saturating_mul(DAYS_PER_YEAR);
        state.birth_day = current_day.saturating_sub(offset);
    }
    while current_day >= state.birth_day.saturating_add(DAYS_PER_YEAR) {
        state.birth_day = state.birth_day.saturating_add(DAYS_PER_YEAR);
        state.age_years = state.age_years.saturating_add(1);
    }
}
