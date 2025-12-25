use bevy_ecs::prelude::*;

use crate::simulation::city::{CityState, LocationId};

pub fn unit_movement_system(mut city: ResMut<CityState>) {
    update_units(&mut city);
}

pub fn update_units(city: &mut CityState) {
    apply_presence_effects(city);
    shift_gang_units(city);
}

fn apply_presence_effects(city: &mut CityState) {
    for location in city.locations.values_mut() {
        let police_effect = location.police_units as i32 + (location.police_presence / 20);
        let gang_effect = location.gang_units as i32;
        let delta = gang_effect - police_effect;
        location.crime_pressure = (location.crime_pressure + delta).clamp(0, 100);
    }
}

fn shift_gang_units(city: &mut CityState) {
    let Some((&target_id, _)) = city
        .locations
        .iter()
        .min_by_key(|(_, loc)| loc.police_presence)
    else {
        return;
    };

    let mut moves: Vec<(LocationId, LocationId)> = Vec::new();
    for (id, location) in city.locations.iter() {
        if location.gang_units > 0 && location.police_presence > 30 && *id != target_id {
            moves.push((*id, target_id));
        }
    }

    for (from_id, to_id) in moves {
        if let Some(from) = city.locations.get_mut(&from_id) {
            if from.gang_units > 0 {
                from.gang_units -= 1;
            }
        }
        if let Some(to) = city.locations.get_mut(&to_id) {
            to.gang_units = to.gang_units.saturating_add(1);
        }
    }
}
