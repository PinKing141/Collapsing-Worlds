use std::collections::HashMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::world::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocationId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LocationTag {
    Public,
    Residential,
    Industrial,
    HighSecurity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeatResponse {
    None,
    PolicePatrol,
    Investigation,
    FactionAttention,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationState {
    pub id: LocationId,
    pub tags: Vec<LocationTag>,
    pub heat: i32,
    pub crime_pressure: i32,
    pub police_presence: i32,
    pub surveillance_level: i32,
    pub lockdown_level: i32,
    pub police_units: u8,
    pub investigators: u8,
    pub gang_units: u8,
    pub faction_influence: HashMap<String, u16>,
    pub response: HeatResponse,
}

#[derive(Resource, Debug, Clone)]
pub struct CityState {
    pub locations: HashMap<LocationId, LocationState>,
    pub active_location: LocationId,
}

impl CityState {
    pub fn location_for_position(&self, pos: &Position) -> LocationId {
        match (pos.x >= 0, pos.y >= 0) {
            (true, true) => LocationId(1),
            (false, true) => LocationId(2),
            (true, false) => LocationId(3),
            (false, false) => LocationId(4),
        }
    }
}

impl Default for CityState {
    fn default() -> Self {
        let mut locations = HashMap::new();

        locations.insert(
            LocationId(1),
            LocationState {
                id: LocationId(1),
                tags: vec![LocationTag::Public],
                heat: 0,
                crime_pressure: 8,
                police_presence: 20,
                surveillance_level: 15,
                lockdown_level: 0,
                police_units: 1,
                investigators: 0,
                gang_units: 0,
                faction_influence: HashMap::new(),
                response: HeatResponse::None,
            },
        );

        locations.insert(
            LocationId(2),
            LocationState {
                id: LocationId(2),
                tags: vec![LocationTag::Residential],
                heat: 0,
                crime_pressure: 4,
                police_presence: 15,
                surveillance_level: 10,
                lockdown_level: 0,
                police_units: 1,
                investigators: 0,
                gang_units: 0,
                faction_influence: HashMap::new(),
                response: HeatResponse::None,
            },
        );

        locations.insert(
            LocationId(3),
            LocationState {
                id: LocationId(3),
                tags: vec![LocationTag::Industrial],
                heat: 0,
                crime_pressure: 10,
                police_presence: 10,
                surveillance_level: 5,
                lockdown_level: 0,
                police_units: 0,
                investigators: 0,
                gang_units: 1,
                faction_influence: HashMap::new(),
                response: HeatResponse::None,
            },
        );

        locations.insert(
            LocationId(4),
            LocationState {
                id: LocationId(4),
                tags: vec![LocationTag::HighSecurity],
                heat: 0,
                crime_pressure: 12,
                police_presence: 35,
                surveillance_level: 40,
                lockdown_level: 10,
                police_units: 2,
                investigators: 0,
                gang_units: 0,
                faction_influence: HashMap::new(),
                response: HeatResponse::None,
            },
        );

        CityState {
            locations,
            active_location: LocationId(1),
        }
    }
}
