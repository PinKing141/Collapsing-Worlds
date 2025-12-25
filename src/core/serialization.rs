use std::fs;
use std::path::Path;

use bevy_ecs::prelude::*;
use bevy_ecs::query::Without;
use serde::{Deserialize, Serialize};

use crate::components::combat::Health;
use crate::components::faction::Faction;
use crate::components::identity::{CivilianIdentity, Name, SuperIdentity};
use crate::components::persona::{
    hero_persona_stack, vigilante_persona_stack, villain_persona_stack, Alignment, PersonaStack,
};
use crate::components::world::{EntityId, Player, Position};
use crate::core::world::IdAllocator;
use crate::simulation::storylet_state::StoryletState;
use crate::simulation::time::GameTime;

/// Save state capturing core world data (time, seed, player, NPCs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveState {
    #[serde(default = "default_save_version")]
    pub version: u32,
    pub seed: u64,
    pub time: GameTime,
    pub player: SavedPlayer,
    pub npcs: Vec<SavedActor>,
    #[serde(default)]
    pub storylet_state: StoryletState,
}

fn default_save_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedHealth {
    pub current: i32,
    pub max: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedCivilian {
    pub job_title: String,
    pub salary: u32,
    pub suspicion: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSuperIdentity {
    pub hero_name: String,
    pub is_masked: bool,
    pub reputation: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPlayer {
    pub uid: u32,
    pub name: String,
    pub suspicion: u8,
    pub position: (i32, i32),
    pub health: SavedHealth,
    pub faction: Option<String>,
    pub civilian: Option<SavedCivilian>,
    pub super_identity: Option<SavedSuperIdentity>,
    #[serde(default)]
    pub persona_stack: Option<PersonaStack>,
    #[serde(default)]
    pub alignment: Option<Alignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedActor {
    pub uid: u32,
    pub name: Option<String>,
    pub position: Option<(i32, i32)>,
    pub civilian: Option<SavedCivilian>,
    pub super_identity: Option<SavedSuperIdentity>,
    pub health: Option<SavedHealth>,
    pub faction: Option<String>,
    #[serde(default)]
    pub persona_stack: Option<PersonaStack>,
    #[serde(default)]
    pub alignment: Option<Alignment>,
}

/// Extract a serializable snapshot of the world.
pub fn extract_state_from_world(world: &World, player: Entity, seed: u64) -> SaveState {
    let time = world.resource::<GameTime>().clone();
    let storylet_state = world
        .get_resource::<StoryletState>()
        .cloned()
        .unwrap_or_default();

    let player_uid = world.get::<EntityId>(player).map(|id| id.0).unwrap_or(0);

    let player_name = world
        .get::<Name>(player)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| format!("Player {}", player_uid));

    let civilian = world
        .get::<CivilianIdentity>(player)
        .map(|c| SavedCivilian {
            job_title: c.job_title.clone(),
            salary: c.salary,
            suspicion: c.suspicion_meter,
        });

    let super_identity = world.get::<SuperIdentity>(player).map(|s| SavedSuperIdentity {
        hero_name: s.hero_name.clone(),
        is_masked: s.is_masked,
        reputation: s.reputation,
    });
    let persona_stack = world.get::<PersonaStack>(player).cloned();
    let alignment = world.get::<Alignment>(player).copied();

    let suspicion = civilian.as_ref().map(|c| c.suspicion).unwrap_or(0);

    let position = world
        .get::<Position>(player)
        .map(|pos| (pos.x, pos.y))
        .unwrap_or((0, 0));

    let health = world.get::<Health>(player).map_or(
        SavedHealth {
            current: 0,
            max: 0,
        },
        |h| SavedHealth {
            current: h.current,
            max: h.max,
        },
    );

    let faction = world.get::<Faction>(player).map(|f| f.name.clone());

    let npcs = world
        .iter_entities()
        .filter(|entity_ref| entity_ref.id() != player)
        .map(|entity_ref| {
            let uid = entity_ref
                .get::<EntityId>()
                .map(|id| id.0)
                .unwrap_or(entity_ref.id().index());
            let name = entity_ref.get::<Name>().map(|n| n.0.clone());
            let position = entity_ref
                .get::<Position>()
                .map(|p| (p.x, p.y));
            let civilian = entity_ref
                .get::<CivilianIdentity>()
                .map(|c| SavedCivilian {
                    job_title: c.job_title.clone(),
                    salary: c.salary,
                    suspicion: c.suspicion_meter,
                });
            let super_identity = entity_ref
                .get::<SuperIdentity>()
                .map(|s| SavedSuperIdentity {
                    hero_name: s.hero_name.clone(),
                    is_masked: s.is_masked,
                    reputation: s.reputation,
                });
            let persona_stack = entity_ref.get::<PersonaStack>().cloned();
            let alignment = entity_ref.get::<Alignment>().copied();
            let health = entity_ref.get::<Health>().map(|h| SavedHealth {
                current: h.current,
                max: h.max,
            });
            let faction = entity_ref.get::<Faction>().map(|f| f.name.clone());

            SavedActor {
                uid,
                name,
                position,
                civilian,
                super_identity,
                health,
                faction,
                persona_stack,
                alignment,
            }
        })
        .collect();

    SaveState {
        version: default_save_version(),
        seed,
        time,
        player: SavedPlayer {
            uid: player_uid,
            name: player_name,
            suspicion,
            position,
            health,
            faction,
            civilian,
            super_identity,
            persona_stack,
            alignment,
        },
        npcs,
        storylet_state,
    }
}

/// Apply a saved snapshot back into the world.
pub fn apply_state_to_world(state: SaveState, world: &mut World, player: Entity) {
    if let Some(mut time) = world.get_resource_mut::<GameTime>() {
        *time = state.time.clone();
    }
    if let Some(mut storylets) = world.get_resource_mut::<StoryletState>() {
        *storylets = state.storylet_state.clone();
    } else {
        world.insert_resource(state.storylet_state.clone());
    }

    if let Some(mut ent_id) = world.get_mut::<EntityId>(player) {
        ent_id.0 = state.player.uid;
    } else if let Some(mut ent) = world.get_entity_mut(player) {
        ent.insert(EntityId(state.player.uid));
    }

    if let Some(mut name) = world.get_mut::<Name>(player) {
        name.0 = state.player.name.clone();
    } else if let Some(mut ent) = world.get_entity_mut(player) {
        ent.insert(Name(state.player.name.clone()));
    }

    match world.get_mut::<Position>(player) {
        Some(mut pos) => {
            pos.x = state.player.position.0;
            pos.y = state.player.position.1;
        }
        None => {
            if let Some(mut ent) = world.get_entity_mut(player) {
                ent.insert(Position {
                    x: state.player.position.0,
                    y: state.player.position.1,
                });
            }
        }
    }

    if let Some(saved_civ) = &state.player.civilian {
        match world.get_mut::<CivilianIdentity>(player) {
            Some(mut civ) => {
                civ.job_title = saved_civ.job_title.clone();
                civ.salary = saved_civ.salary;
                civ.suspicion_meter = saved_civ.suspicion;
            }
            None => {
                if let Some(mut ent) = world.get_entity_mut(player) {
                    ent.insert(CivilianIdentity {
                        job_title: saved_civ.job_title.clone(),
                        salary: saved_civ.salary,
                        suspicion_meter: saved_civ.suspicion,
                    });
                }
            }
        }
    }

    if let Some(saved_super) = &state.player.super_identity {
        match world.get_mut::<SuperIdentity>(player) {
            Some(mut sup) => {
                sup.hero_name = saved_super.hero_name.clone();
                sup.is_masked = saved_super.is_masked;
                sup.reputation = saved_super.reputation;
            }
            None => {
                if let Some(mut ent) = world.get_entity_mut(player) {
                    ent.insert(SuperIdentity {
                        hero_name: saved_super.hero_name.clone(),
                        is_masked: saved_super.is_masked,
                        reputation: saved_super.reputation,
                    });
                }
            }
        }
    }

    match world.get_mut::<Health>(player) {
        Some(mut hp) => {
            hp.current = state.player.health.current;
            hp.max = state.player.health.max;
        }
        None => {
            if let Some(mut ent) = world.get_entity_mut(player) {
                ent.insert(Health {
                    current: state.player.health.current,
                    max: state.player.health.max,
                });
            }
        }
    }

    match state.player.faction {
        Some(name) => match world.get_mut::<Faction>(player) {
            Some(mut faction) => faction.name = name.clone(),
            None => {
                if let Some(mut ent) = world.get_entity_mut(player) {
                    ent.insert(Faction { name });
                }
            }
        },
        None => {}
    }

    if let Some(saved_persona) = &state.player.persona_stack {
        match world.get_mut::<PersonaStack>(player) {
            Some(mut persona) => *persona = saved_persona.clone(),
            None => {
                if let Some(mut ent) = world.get_entity_mut(player) {
                    ent.insert(saved_persona.clone());
                }
            }
        }
    }

    if let Some(saved_alignment) = &state.player.alignment {
        match world.get_mut::<Alignment>(player) {
            Some(mut alignment) => *alignment = *saved_alignment,
            None => {
                if let Some(mut ent) = world.get_entity_mut(player) {
                    ent.insert(*saved_alignment);
                }
            }
        }
    }

    if state.player.persona_stack.is_none() {
        let alignment = state.player.alignment.unwrap_or(Alignment::Hero);
        let default_stack = match alignment {
            Alignment::Hero => hero_persona_stack(),
            Alignment::Vigilante => vigilante_persona_stack(),
            Alignment::Villain => villain_persona_stack(),
        };
        if let Some(mut ent) = world.get_entity_mut(player) {
            ent.insert(default_stack);
        }
    }

    // Clear existing non-player entities.
    let to_remove: Vec<Entity> = world
        .query_filtered::<Entity, Without<Player>>()
        .iter(world)
        .collect();
    for entity in to_remove {
        let _ = world.despawn(entity);
    }

    // Spawn saved NPCs.
    for saved in state.npcs.iter() {
        let mut ent = world.spawn_empty();

        ent.insert(EntityId(saved.uid));
        if let Some(name) = saved.name.clone() {
            ent.insert(Name(name));
        }
        if let Some((x, y)) = saved.position {
            ent.insert(Position { x, y });
        }
        if let Some(civ) = saved.civilian.clone() {
            ent.insert(CivilianIdentity {
                job_title: civ.job_title,
                salary: civ.salary,
                suspicion_meter: civ.suspicion,
            });
        }
        if let Some(super_id) = saved.super_identity.clone() {
            ent.insert(SuperIdentity {
                hero_name: super_id.hero_name,
                is_masked: super_id.is_masked,
                reputation: super_id.reputation,
            });
        }
        if let Some(hp) = saved.health.clone() {
            ent.insert(Health {
                current: hp.current,
                max: hp.max,
            });
        }
        if let Some(name) = saved.faction.clone() {
            ent.insert(Faction { name });
        }
        if let Some(persona) = saved.persona_stack.clone() {
            ent.insert(persona);
        }
        if let Some(alignment) = saved.alignment {
            ent.insert(alignment);
        }
    }

    // Update allocator to avoid collisions.
    let max_uid = state
        .npcs
        .iter()
        .map(|s| s.uid)
        .chain(std::iter::once(state.player.uid))
        .max()
        .unwrap_or(0);
    if let Some(mut alloc) = world.get_resource_mut::<IdAllocator>() {
        alloc.bump_to_at_least(max_uid + 1);
    }
}

/// Serialize a save state into JSON for persistence.
pub fn save_state_to_json(state: &SaveState) -> serde_json::Result<String> {
    serde_json::to_string_pretty(state)
}

/// Deserialize JSON back into a save state.
pub fn load_state_from_json(data: &str) -> serde_json::Result<SaveState> {
    serde_json::from_str(data)
}

/// Write a save state to a file path.
pub fn save_state_to_path<P: AsRef<Path>>(state: &SaveState, path: P) -> std::io::Result<()> {
    let json = save_state_to_json(state).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    fs::write(path, json)
}

/// Read a save state from a file path.
pub fn load_state_from_path<P: AsRef<Path>>(path: P) -> std::io::Result<SaveState> {
    let data = fs::read_to_string(&path)?;
    load_state_from_json(&data).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
