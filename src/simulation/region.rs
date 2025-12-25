use std::collections::HashMap;

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::simulation::city::{CityEvent, CityEventKind, CityId, CityState, LocationId};
use crate::simulation::pressure::PressureState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegionId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CountryId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContinentId(pub u32);

#[derive(Resource, Debug, Clone)]
pub struct RegionState {
    pub continents: HashMap<ContinentId, ContinentProfile>,
    pub countries: HashMap<CountryId, CountryProfile>,
    pub regions: HashMap<RegionId, RegionProfile>,
    pub global_pressure: GlobalPressure,
}

#[derive(Debug, Clone)]
pub struct RegionProfile {
    pub id: RegionId,
    pub name: String,
    pub country_id: CountryId,
    pub continent_id: ContinentId,
    pub city_ids: Vec<CityId>,
    pub heat_average: f32,
    pub crime_pressure_average: f32,
    pub escalation: RegionEscalation,
}

#[derive(Debug, Clone)]
pub struct CountryProfile {
    pub id: CountryId,
    pub name: String,
    pub continent_id: ContinentId,
    pub region_ids: Vec<RegionId>,
    pub heat_average: f32,
    pub crime_pressure_average: f32,
    pub escalation: RegionEscalation,
}

#[derive(Debug, Clone)]
pub struct ContinentProfile {
    pub id: ContinentId,
    pub name: String,
    pub country_ids: Vec<CountryId>,
    pub heat_average: f32,
    pub crime_pressure_average: f32,
    pub escalation: RegionEscalation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionEscalation {
    Stable,
    Alert,
    Emergency,
}

impl RegionEscalation {
    pub fn rank(self) -> u8 {
        match self {
            RegionEscalation::Stable => 0,
            RegionEscalation::Alert => 1,
            RegionEscalation::Emergency => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalEscalation {
    Stable,
    Tense,
    Crisis,
    Cosmic,
}

impl GlobalEscalation {
    pub fn rank(self) -> u8 {
        match self {
            GlobalEscalation::Stable => 0,
            GlobalEscalation::Tense => 1,
            GlobalEscalation::Crisis => 2,
            GlobalEscalation::Cosmic => 3,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GlobalPressure {
    pub total: f32,
    pub escalation: GlobalEscalation,
}

#[derive(Resource, Debug, Default)]
pub struct RegionEventLog(pub Vec<RegionEvent>);

#[derive(Debug, Clone)]
pub struct RegionEvent {
    pub region_id: RegionId,
    pub city_id: CityId,
    pub location_id: LocationId,
    pub kind: RegionEventKind,
}

#[derive(Debug, Clone)]
pub enum RegionEventKind {
    CityHeatResponseChanged { response: crate::simulation::city::HeatResponse },
}

impl Default for RegionState {
    fn default() -> Self {
        let mut continents = HashMap::new();
        continents.insert(
            ContinentId(1),
            ContinentProfile {
                id: ContinentId(1),
                name: "Central Continent".to_string(),
                country_ids: vec![CountryId(1)],
                heat_average: 0.0,
                crime_pressure_average: 0.0,
                escalation: RegionEscalation::Stable,
            },
        );

        let mut countries = HashMap::new();
        countries.insert(
            CountryId(1),
            CountryProfile {
                id: CountryId(1),
                name: "Metro Nation".to_string(),
                continent_id: ContinentId(1),
                region_ids: vec![RegionId(1)],
                heat_average: 0.0,
                crime_pressure_average: 0.0,
                escalation: RegionEscalation::Stable,
            },
        );

        let mut regions = HashMap::new();
        regions.insert(
            RegionId(1),
            RegionProfile {
                id: RegionId(1),
                name: "Metro Region".to_string(),
                country_id: CountryId(1),
                continent_id: ContinentId(1),
                city_ids: vec![CityId(1)],
                heat_average: 0.0,
                crime_pressure_average: 0.0,
                escalation: RegionEscalation::Stable,
            },
        );

        Self {
            continents,
            countries,
            regions,
            global_pressure: GlobalPressure {
                total: 0.0,
                escalation: GlobalEscalation::Stable,
            },
        }
    }
}

impl RegionState {
    pub fn update_from_city(&mut self, city: &CityState) {
        let region_entry = self
            .regions
            .entry(city.region_id)
            .or_insert_with(|| RegionProfile {
                id: city.region_id,
                name: format!("Region {}", city.region_id.0),
                country_id: city.country_id,
                continent_id: city.continent_id,
                city_ids: vec![city.city_id],
                heat_average: 0.0,
                crime_pressure_average: 0.0,
                escalation: RegionEscalation::Stable,
            });

        region_entry.country_id = city.country_id;
        region_entry.continent_id = city.continent_id;
        if !region_entry.city_ids.contains(&city.city_id) {
            region_entry.city_ids.push(city.city_id);
        }

        let mut heat_total = 0.0;
        let mut crime_total = 0.0;
        let mut count = 0.0;
        for location in city.locations.values() {
            heat_total += location.heat as f32;
            crime_total += location.crime_pressure as f32;
            count += 1.0;
        }

        if count > 0.0 {
            region_entry.heat_average = heat_total / count;
            region_entry.crime_pressure_average = crime_total / count;
        }

        region_entry.escalation = region_escalation_for(
            region_entry.heat_average,
            region_entry.crime_pressure_average,
        );

        let country_entry = self
            .countries
            .entry(city.country_id)
            .or_insert_with(|| CountryProfile {
                id: city.country_id,
                name: format!("Country {}", city.country_id.0),
                continent_id: city.continent_id,
                region_ids: vec![city.region_id],
                heat_average: 0.0,
                crime_pressure_average: 0.0,
                escalation: RegionEscalation::Stable,
            });
        country_entry.continent_id = city.continent_id;
        if !country_entry.region_ids.contains(&city.region_id) {
            country_entry.region_ids.push(city.region_id);
        }

        let mut country_heat = 0.0;
        let mut country_crime = 0.0;
        let mut country_count = 0.0;
        for region_id in &country_entry.region_ids {
            if let Some(profile) = self.regions.get(region_id) {
                country_heat += profile.heat_average;
                country_crime += profile.crime_pressure_average;
                country_count += 1.0;
            }
        }
        if country_count > 0.0 {
            country_entry.heat_average = country_heat / country_count;
            country_entry.crime_pressure_average = country_crime / country_count;
        }
        country_entry.escalation = region_escalation_for(
            country_entry.heat_average,
            country_entry.crime_pressure_average,
        );

        let continent_entry = self
            .continents
            .entry(city.continent_id)
            .or_insert_with(|| ContinentProfile {
                id: city.continent_id,
                name: format!("Continent {}", city.continent_id.0),
                country_ids: vec![city.country_id],
                heat_average: 0.0,
                crime_pressure_average: 0.0,
                escalation: RegionEscalation::Stable,
            });
        if !continent_entry.country_ids.contains(&city.country_id) {
            continent_entry.country_ids.push(city.country_id);
        }

        let mut continent_heat = 0.0;
        let mut continent_crime = 0.0;
        let mut continent_count = 0.0;
        for country_id in &continent_entry.country_ids {
            if let Some(profile) = self.countries.get(country_id) {
                continent_heat += profile.heat_average;
                continent_crime += profile.crime_pressure_average;
                continent_count += 1.0;
            }
        }
        if continent_count > 0.0 {
            continent_entry.heat_average = continent_heat / continent_count;
            continent_entry.crime_pressure_average = continent_crime / continent_count;
        }
        continent_entry.escalation = region_escalation_for(
            continent_entry.heat_average,
            continent_entry.crime_pressure_average,
        );
    }

    pub fn update_global_pressure(&mut self, pressure: &PressureState) {
        let total = (pressure.temporal
            + pressure.identity
            + pressure.institutional
            + pressure.moral
            + pressure.resource
            + pressure.psychological)
            / 6.0;
        let escalation = global_escalation_for(total);
        self.global_pressure = GlobalPressure { total, escalation };
    }
}

pub fn region_escalation_for(heat_average: f32, crime_pressure_average: f32) -> RegionEscalation {
    if heat_average >= 60.0 || crime_pressure_average >= 70.0 {
        RegionEscalation::Emergency
    } else if heat_average >= 35.0 || crime_pressure_average >= 45.0 {
        RegionEscalation::Alert
    } else {
        RegionEscalation::Stable
    }
}

pub fn global_escalation_for(global_pressure: f32) -> GlobalEscalation {
    if global_pressure >= 80.0 {
        GlobalEscalation::Cosmic
    } else if global_pressure >= 60.0 {
        GlobalEscalation::Crisis
    } else if global_pressure >= 40.0 {
        GlobalEscalation::Tense
    } else {
        GlobalEscalation::Stable
    }
}

pub fn propagate_city_event(
    event: CityEvent,
    region_id: RegionId,
) -> RegionEvent {
    let kind = match event.kind {
        CityEventKind::HeatResponseChanged { response } => {
            RegionEventKind::CityHeatResponseChanged { response }
        }
    };

    RegionEvent {
        region_id,
        city_id: event.city_id,
        location_id: event.location_id,
        kind,
    }
}
