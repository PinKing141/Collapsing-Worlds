use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoryletState {
    #[serde(default)]
    pub fired: HashSet<String>,
    #[serde(default)]
    pub cooldowns: HashMap<String, i32>,
    #[serde(default)]
    pub flags: HashMap<String, bool>,
}

impl StoryletState {
    pub fn tick(&mut self) {
        let mut to_clear = Vec::new();
        for (id, turns) in self.cooldowns.iter_mut() {
            if *turns > 0 {
                *turns -= 1;
            }
            if *turns <= 0 {
                to_clear.push(id.clone());
            }
        }
        for id in to_clear {
            self.cooldowns.remove(&id);
        }
    }
}
