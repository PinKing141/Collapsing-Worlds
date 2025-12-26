use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::data::factions::{FactionDomain, Jurisdiction};
use crate::simulation::city::{CityEventLog, CityState};
use crate::simulation::pressure::PressureState;
use crate::simulation::region::{
    propagate_city_event, GlobalEscalation, RegionEscalation, RegionEventLog, RegionId, RegionState,
};

#[derive(Resource, Debug)]
pub struct GlobalFactionDirector {
    definitions: Vec<GlobalFactionDefinition>,
    last_levels: HashMap<(String, Option<u32>), String>,
}

#[derive(Resource, Debug, Default)]
pub struct GlobalFactionEventLog(pub Vec<GlobalFactionEvent>);

#[derive(Debug, Clone)]
pub struct GlobalFactionEvent {
    pub faction_id: String,
    pub name: String,
    pub domain: FactionDomain,
    pub jurisdiction: Jurisdiction,
    pub scope: GlobalFactionScope,
    pub level: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalFactionScope {
    Global,
    Region(RegionId),
}

#[derive(Debug, Clone)]
pub struct GlobalFactionDefinition {
    pub id: String,
    pub name: String,
    pub domain: FactionDomain,
    pub jurisdiction: Jurisdiction,
    pub global_thresholds: Vec<GlobalThreshold>,
    pub region_thresholds: Vec<RegionThreshold>,
}

#[derive(Debug, Clone)]
pub struct GlobalThreshold {
    pub min_pressure: f32,
    pub level: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RegionThreshold {
    pub min_escalation: RegionEscalation,
    pub level: String,
    pub actions: Vec<String>,
}

impl Default for GlobalFactionDirector {
    fn default() -> Self {
        Self::load_default()
    }
}

impl GlobalFactionDirector {
    pub fn load_default() -> Self {
        let definitions = vec![
            GlobalFactionDefinition {
                id: "cosmic_convergence".to_string(),
                name: "Cosmic Convergence Accord".to_string(),
                domain: FactionDomain::Cosmic,
                jurisdiction: Jurisdiction::Global,
                global_thresholds: vec![
                    GlobalThreshold {
                        min_pressure: 60.0,
                        level: "OMEN".to_string(),
                        actions: vec!["TRACK_ANOMALY".to_string()],
                    },
                    GlobalThreshold {
                        min_pressure: 80.0,
                        level: "COSMIC_ALERT".to_string(),
                        actions: vec!["OPEN_DIMENSIONAL_WARD".to_string()],
                    },
                ],
                region_thresholds: Vec::new(),
            },
            GlobalFactionDefinition {
                id: "regional_taskforce".to_string(),
                name: "Regional Taskforce".to_string(),
                domain: FactionDomain::Military,
                jurisdiction: Jurisdiction::Regional,
                global_thresholds: Vec::new(),
                region_thresholds: vec![
                    RegionThreshold {
                        min_escalation: RegionEscalation::Alert,
                        level: "MOBILIZE".to_string(),
                        actions: vec!["DEPLOY_COMMAND".to_string()],
                    },
                    RegionThreshold {
                        min_escalation: RegionEscalation::Emergency,
                        level: "INTERVENE".to_string(),
                        actions: vec!["DECLARE_MARTIAL".to_string()],
                    },
                ],
            },
        ];

        Self {
            definitions,
            last_levels: HashMap::new(),
        }
    }
}

pub fn region_system(
    mut region: ResMut<RegionState>,
    city: Res<CityState>,
    pressure: Res<PressureState>,
    mut city_events: ResMut<CityEventLog>,
    mut region_events: ResMut<RegionEventLog>,
) {
    run_region_update(
        &mut region,
        &city,
        &pressure,
        &mut city_events,
        &mut region_events,
    );
}

pub fn run_region_update(
    region: &mut RegionState,
    city: &CityState,
    pressure: &PressureState,
    city_events: &mut CityEventLog,
    region_events: &mut RegionEventLog,
) {
    region.update_from_city(city);
    region.update_global_pressure(pressure);

    region_events.0.clear();
    let region_id = city.region_id;
    for event in city_events.0.drain(..) {
        region_events
            .0
            .push(propagate_city_event(event, region_id));
    }
}

pub fn global_faction_system(
    mut director: ResMut<GlobalFactionDirector>,
    region: Res<RegionState>,
    mut log: ResMut<GlobalFactionEventLog>,
) {
    run_global_faction_director(&mut director, &region, &mut log);
}

pub fn run_global_faction_director(
    director: &mut GlobalFactionDirector,
    region: &RegionState,
    log: &mut GlobalFactionEventLog,
) {
    log.0.clear();
    if director.definitions.is_empty() {
        director.last_levels.clear();
        return;
    }

    let definitions = director.definitions.clone();
    for def in definitions.iter() {
        if let Some(threshold) = select_global_threshold(&def.global_thresholds, region) {
            push_global_event(
                director,
                log,
                def,
                GlobalFactionScope::Global,
                &threshold.level,
                &threshold.actions,
            );
        } else {
            director.last_levels.remove(&(def.id.clone(), None));
        }

        for (region_id, profile) in region.regions.iter() {
            if let Some(threshold) =
                select_region_threshold(&def.region_thresholds, profile.escalation)
            {
                push_global_event(
                    director,
                    log,
                    def,
                    GlobalFactionScope::Region(*region_id),
                    &threshold.level,
                    &threshold.actions,
                );
            } else {
                director
                    .last_levels
                    .remove(&(def.id.clone(), Some(region_id.0)));
            }
        }
    }
}

fn select_global_threshold<'a>(
    thresholds: &'a [GlobalThreshold],
    region: &RegionState,
) -> Option<&'a GlobalThreshold> {
    thresholds
        .iter()
        .filter(|threshold| region.global_pressure.total >= threshold.min_pressure)
        .max_by(|a, b| a.min_pressure.partial_cmp(&b.min_pressure).unwrap_or(std::cmp::Ordering::Equal))
}

fn select_region_threshold<'a>(
    thresholds: &'a [RegionThreshold],
    escalation: RegionEscalation,
) -> Option<&'a RegionThreshold> {
    thresholds
        .iter()
        .filter(|threshold| escalation.rank() >= threshold.min_escalation.rank())
        .max_by_key(|threshold| threshold.min_escalation.rank())
}

fn push_global_event(
    director: &mut GlobalFactionDirector,
    log: &mut GlobalFactionEventLog,
    def: &GlobalFactionDefinition,
    scope: GlobalFactionScope,
    level: &str,
    actions: &[String],
) {
    let scope_key = match scope {
        GlobalFactionScope::Global => None,
        GlobalFactionScope::Region(region_id) => Some(region_id.0),
    };
    let key = (def.id.clone(), scope_key);
    if director
        .last_levels
        .get(&key)
        .map(|cached| cached == level)
        .unwrap_or(false)
    {
        return;
    }

    director.last_levels.insert(key, level.to_string());
    log.0.push(GlobalFactionEvent {
        faction_id: def.id.clone(),
        name: def.name.clone(),
        domain: def.domain,
        jurisdiction: def.jurisdiction,
        scope,
        level: level.to_string(),
        actions: actions.to_vec(),
    });
}

pub fn global_escalation_triggered(
    region: &RegionState,
    escalation: GlobalEscalation,
) -> bool {
    region.global_pressure.escalation.rank() >= escalation.rank()
}
