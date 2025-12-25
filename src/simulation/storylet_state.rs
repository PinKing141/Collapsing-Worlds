use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryletPunctuationState {
    pub only: bool,
    pub remaining_turns: i32,
}

impl Default for StoryletPunctuationState {
    fn default() -> Self {
        Self {
            only: false,
            remaining_turns: 0,
        }
    }
}

impl StoryletPunctuationState {
    pub fn activate(&mut self, turns: i32) {
        let turns = turns.max(1);
        self.only = true;
        self.remaining_turns = turns;
    }

    pub fn clear(&mut self) {
        self.only = false;
        self.remaining_turns = 0;
    }

    pub fn tick(&mut self) {
        if self.only && self.remaining_turns > 0 {
            self.remaining_turns -= 1;
            if self.remaining_turns <= 0 {
                self.clear();
            }
        }
    }
}

#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoryletState {
    #[serde(default)]
    pub fired: HashSet<String>,
    #[serde(default)]
    pub cooldowns: HashMap<String, i32>,
    #[serde(default)]
    pub flags: HashMap<String, bool>,
    #[serde(default)]
    pub punctuation: StoryletPunctuationState,
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
        self.punctuation.tick();
    }
}
