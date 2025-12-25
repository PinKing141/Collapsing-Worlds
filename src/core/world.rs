use std::path::Path;

use bevy_ecs::prelude::*;

use crate::components::combat::Health;
use crate::components::faction::Faction;
use crate::components::identity::{CivilianIdentity, Name, SuperIdentity};
use crate::components::persona::{hero_persona_stack, Alignment};
use crate::components::world::{EntityId, Player, Position};
use crate::core::ecs::{create_schedule, create_world};
use crate::core::serialization::{
    apply_state_to_world, extract_state_from_world, load_state_from_path, save_state_to_path, SaveState,
};
use crate::simulation::time::GameTime;
use crate::simulation::origin::assign_origin_for_player;
use crate::systems::combat::CombatLog;

/// Intent-driven commands fed into the ECS each tick.
#[derive(Debug, Clone)]
pub enum ActionIntent {
    Move { entity_id: u32, dx: i32, dy: i32 },
    Interact { entity_id: u32 },
    Rest { entity_id: u32 },
    Attack { attacker_id: u32, target_id: Option<u32> },
    SwitchPersona { entity_id: u32, persona_id: String },
    Wait,
}

/// Resource storing the intents for the next tick.
#[derive(Resource, Default, Debug)]
pub struct ActionQueue(pub Vec<ActionIntent>);

/// Data snapshot returned to the UI layer after each tick.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub time_str: String,
    pub player_name: String,
    pub suspicion: u8,
    pub player_health: (i32, i32),
    pub player_pos: (i32, i32),
    pub combat_log: Vec<String>,
    pub entities: Vec<EntitySummary>,
}

#[derive(Debug, Clone)]
pub struct EntitySummary {
    pub id: u32,
    pub name: String,
    pub position: (i32, i32),
    pub health: Option<(i32, i32)>,
}

/// Wrapper around the ECS world and schedule.
pub struct Game {
    world: World,
    schedule: Schedule,
    player: Entity,
    player_uid: u32,
    seed: u64,
}

impl Game {
    /// Create a new game world using the provided seed.
    pub fn new(seed: u64) -> Self {
        let mut world = create_world(seed);
        let player_uid = allocate_entity_id(&mut world);
        let player = spawn_player(&mut world, player_uid);
        assign_origin_for_player(&mut world, player, seed);
        spawn_demo_npcs(&mut world);
        let schedule = create_schedule();

        Self {
            world,
            schedule,
            player,
            player_uid,
            seed,
        }
    }

    /// Run a simulation tick with the provided intents and return a snapshot for rendering.
    pub fn tick(&mut self, intents: Vec<ActionIntent>) -> Snapshot {
        {
            let mut queue = self.world.resource_mut::<ActionQueue>();
            queue.0 = intents;
        }

        self.schedule.run(&mut self.world);
        Snapshot::capture(self.player, &self.world)
    }

    /// Expose the player's entity index for intent addressing.
    pub fn get_player_id(&self) -> u32 {
        self.player_uid
    }

    /// Extract a serializable save state from the current world.
    pub fn save_state(&self) -> SaveState {
        extract_state_from_world(&self.world, self.player, self.seed)
    }

    /// Apply a saved state back into the live world.
    pub fn load_state(&mut self, state: SaveState) {
        self.seed = state.seed;
        apply_state_to_world(state, &mut self.world, self.player);
        self.player_uid = self
            .world
            .get::<EntityId>(self.player)
            .map(|id| id.0)
            .unwrap_or(self.player_uid);
    }

    /// Save state directly to a file path.
    pub fn save_to_path<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        save_state_to_path(&self.save_state(), path)
    }

    /// Load state directly from a file path.
    pub fn load_from_path<P: AsRef<Path>>(&mut self, path: P) -> std::io::Result<()> {
        let state = load_state_from_path(path)?;
        self.load_state(state);
        Ok(())
    }
}

#[derive(Resource, Debug)]
pub struct IdAllocator {
    next: u32,
}

impl Default for IdAllocator {
    fn default() -> Self {
        Self { next: 1 }
    }
}

impl IdAllocator {
    pub fn alloc(&mut self) -> u32 {
        let id = self.next;
        self.next += 1;
        id
    }

    pub fn bump_to_at_least(&mut self, min_next: u32) {
        if self.next < min_next {
            self.next = min_next;
        }
    }
}

fn allocate_entity_id(world: &mut World) -> u32 {
    let mut alloc = world.resource_mut::<IdAllocator>();
    alloc.alloc()
}

fn spawn_player(world: &mut World, uid: u32) -> Entity {
    let persona_stack = hero_persona_stack();

    world
        .spawn((
            Player,
            EntityId(uid),
            Name("You".to_string()),
            Position { x: 0, y: 0 },
            Health::new(100),
            CivilianIdentity {
                job_title: "Office Worker".to_string(),
                salary: 42_000,
                suspicion_meter: 0,
            },
            SuperIdentity {
                hero_name: "Unmasked".to_string(),
                is_masked: false,
                reputation: 0,
            },
            Faction {
                name: "Independent".to_string(),
            },
            persona_stack,
            Alignment::Hero,
        ))
        .id()
}

fn spawn_demo_npcs(world: &mut World) {
    // Simple set of test NPCs with different factions/health for targeting.
    let mut spawn_npc = |name: &str, pos: (i32, i32), hp: i32, faction: &str| {
        let uid = allocate_entity_id(world);
        world.spawn((
            EntityId(uid),
            Name(name.to_string()),
            Position { x: pos.0, y: pos.1 },
            Health::new(hp),
            Faction {
                name: faction.to_string(),
            },
        ));
    };

    let roster = [
        ("Training Dummy", (1, 0), 30, "Neutral"),
        ("Street Thug", (2, 0), 40, "Gang"),
        ("Gang Lieutenant", (3, 1), 60, "Gang"),
        ("Agent", (-1, 0), 50, "Agency"),
        ("Detective", (-2, -1), 55, "Police"),
    ];

    for (name, pos, hp, faction) in roster {
        spawn_npc(name, pos, hp, faction);
    }
}

impl Snapshot {
    fn capture(player: Entity, world: &World) -> Self {
        let time = world.resource::<GameTime>();
        let time_str = time.to_string();

        let player_name = world
            .get::<Name>(player)
            .map(|n| n.0.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let suspicion = world
            .get::<CivilianIdentity>(player)
            .map(|civ| civ.suspicion_meter)
            .unwrap_or(0);

        let player_pos = world
            .get::<Position>(player)
            .map(|pos| (pos.x, pos.y))
            .unwrap_or((0, 0));

        let player_health = world
            .get::<Health>(player)
            .map(|hp| (hp.current, hp.max))
            .unwrap_or((0, 0));

        let entities = world
            .iter_entities()
            .filter(|e| e.id() != player)
            .filter_map(|e| {
                let id = e.get::<EntityId>()?.0;
                let name = e
                    .get::<Name>()
                    .map(|n| n.0.clone())
                    .unwrap_or_else(|| format!("Entity {}", id));
                let position = e
                    .get::<Position>()
                    .map(|p| (p.x, p.y))
                    .unwrap_or((0, 0));
                let health = e.get::<Health>().map(|hp| (hp.current, hp.max));
                Some(EntitySummary {
                    id,
                    name,
                    position,
                    health,
                })
            })
            .collect();

        let combat_log = world
            .get_resource::<CombatLog>()
            .map(|log| log.0.clone())
            .unwrap_or_default();

        Snapshot {
            time_str,
            player_name,
            suspicion,
            player_health,
            player_pos,
            combat_log,
            entities,
        }
    }
}
