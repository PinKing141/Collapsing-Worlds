use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SystemSet;

use crate::core::world::ActionQueue;
use crate::core::world::IdAllocator;
use crate::data::storylets::{load_storylet_catalog, Storylet};
use crate::simulation::agents::{AgentEventLog, AgentRegistry};
use crate::simulation::case::{CaseEventLog, CaseRegistry};
use crate::simulation::city::{CityEventLog, CityState};
use crate::simulation::civilian::CivilianState;
use crate::simulation::evidence::WorldEvidence;
use crate::simulation::identity_evidence::IdentityEvidenceStore;
use crate::simulation::nemesis::NemesisState;
use crate::simulation::pressure::PressureState;
use crate::simulation::region::{GlobalEventLog, RegionEventLog, RegionState};
use crate::simulation::storylet_state::StoryletState;
use crate::simulation::storylets::StoryletLibrary;
use crate::simulation::time::{advance_time_system, GameTime};
use crate::systems::case::case_progress_system;
use crate::systems::civilian::civilian_system;
use crate::systems::combat::{combat_system, CombatLog};
use crate::systems::economy::economy_system;
use crate::systems::event_resolver::{event_resolver_system, ResolvedFactionEventLog};
use crate::systems::faction::{faction_director_system, FactionDirector, FactionEventLog};
use crate::systems::heat::{
    heat_decay_system, signature_heat_system, update_active_location_system, WorldEventLog,
};
use crate::systems::movement_system;
use crate::systems::nemesis::{nemesis_system, NemesisDirector, NemesisEventLog};
use crate::systems::persona::{persona_switch_system, PersonaEventLog};
use crate::systems::pressure::pressure_system;
use crate::systems::region::{
    global_faction_system, region_system, GlobalFactionDirector, GlobalFactionEventLog,
};
use crate::systems::suspicion::suspicion_system;
use crate::systems::units::unit_movement_system;

/// Canonical tick ordering for the simulation.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum TickSet {
    Intake,
    Simulation,
    Time,
    Cleanup,
}

/// Build the ECS world with baseline resources.
pub fn create_world(_seed: u64) -> World {
    let mut world = World::new();
    world.insert_resource(GameTime::default());
    world.insert_resource(ActionQueue::default());
    world.insert_resource(CombatLog::default());
    world.insert_resource(IdAllocator::default());
    world.insert_resource(CityState::default());
    world.insert_resource(CityEventLog::default());
    world.insert_resource(WorldEvidence::default());
    world.insert_resource(IdentityEvidenceStore::default());
    world.insert_resource(WorldEventLog::default());
    world.insert_resource(FactionEventLog::default());
    world.insert_resource(ResolvedFactionEventLog::default());
    world.insert_resource(CaseRegistry::default());
    world.insert_resource(CaseEventLog::default());
    world.insert_resource(PersonaEventLog::default());
    world.insert_resource(PressureState::default());
    world.insert_resource(RegionState::default());
    world.insert_resource(RegionEventLog::default());
    world.insert_resource(CivilianState::default());
    world.insert_resource(NemesisState::default());
    world.insert_resource(NemesisEventLog::default());
    world.insert_resource(load_faction_director());
    world.insert_resource(GlobalFactionDirector::load_default());
    world.insert_resource(GlobalFactionEventLog::default());
    world.insert_resource(GlobalEventLog::default());
    world.insert_resource(load_nemesis_director());
    world.insert_resource(load_storylets());
    world.insert_resource(StoryletState::default());
    world.insert_resource(load_agents());
    world.insert_resource(AgentEventLog::default());
    world
}

/// Build the system schedule in the canonical order.
pub fn create_schedule() -> Schedule {
    let mut schedule = Schedule::default();

    schedule.configure_sets(
        (
            TickSet::Intake,
            TickSet::Simulation,
            TickSet::Time,
            TickSet::Cleanup,
        )
            .chain(),
    );

    schedule.add_systems((
        movement_system.in_set(TickSet::Simulation),
        economy_system.in_set(TickSet::Simulation),
        suspicion_system.in_set(TickSet::Simulation),
        update_active_location_system.in_set(TickSet::Simulation),
        signature_heat_system.in_set(TickSet::Simulation),
        persona_switch_system.in_set(TickSet::Simulation),
        faction_director_system.in_set(TickSet::Simulation),
        event_resolver_system.in_set(TickSet::Simulation),
        case_progress_system.in_set(TickSet::Simulation),
        nemesis_system
            .in_set(TickSet::Simulation)
            .after(case_progress_system)
            .before(pressure_system),
        unit_movement_system.in_set(TickSet::Simulation),
        combat_system.in_set(TickSet::Simulation),
        pressure_system.in_set(TickSet::Simulation),
        civilian_system
            .in_set(TickSet::Simulation)
            .after(pressure_system),
        heat_decay_system.in_set(TickSet::Time),
        region_system.in_set(TickSet::Time).after(heat_decay_system),
        global_faction_system
            .in_set(TickSet::Time)
            .after(region_system),
        advance_time_system.in_set(TickSet::Time),
    ));

    schedule
}

fn load_faction_director() -> FactionDirector {
    match FactionDirector::load_default() {
        Ok(director) => director,
        Err(err) => {
            eprintln!("Failed to load faction data: {}", err);
            FactionDirector::default()
        }
    }
}

fn load_nemesis_director() -> NemesisDirector {
    match NemesisDirector::load_default() {
        Ok(director) => director,
        Err(err) => {
            eprintln!("Failed to load nemesis actions: {}", err);
            NemesisDirector::default()
        }
    }
}

fn load_storylets() -> StoryletLibrary {
    StoryletLibrary {
        hero: load_storylet_file("./assets/data/storylets_hero.json"),
        vigilante: load_storylet_file("./assets/data/storylets_vigilante.json"),
        villain: load_storylet_file("./assets/data/storylets_villain.json"),
    }
}

fn load_storylet_file(path: &str) -> Vec<Storylet> {
    match load_storylet_catalog(path) {
        Ok(catalog) => catalog.storylets,
        Err(err) => {
            eprintln!("Failed to load storylets from {}: {}", path, err);
            Vec::new()
        }
    }
}

fn load_agents() -> AgentRegistry {
    match AgentRegistry::load_default() {
        Ok(registry) => registry,
        Err(err) => {
            eprintln!("Failed to load agent data: {}", err);
            AgentRegistry::default()
        }
    }
}
