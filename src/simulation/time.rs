use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

/// Global resource tracking the simulation timeline.
#[derive(Resource, Debug, Serialize, Deserialize, Clone)]
pub struct GameTime {
    pub tick: u64,
    pub day: u32,
    pub hour: u8,
    pub week: u32,
    pub month: u32,
    pub is_day: bool,
}

impl Default for GameTime {
    fn default() -> Self {
        let hour = 8;
        Self {
            tick: 0,
            day: 1,
            hour,
            week: 1,
            month: 1,
            is_day: hour >= 6 && hour < 18,
        }
    }
}

impl GameTime {
    pub fn to_string(&self) -> String {
        let phase = if self.is_day { "Day" } else { "Night" };
        format!(
            "Day {}, Week {}, Month {}, {:02}:00 ({})",
            self.day, self.week, self.month, self.hour, phase
        )
    }

    pub fn advance(&mut self) {
        self.tick += 1;
        self.hour += 1;

        if self.hour >= 24 {
            self.hour = 0;
            self.day += 1;
            if self.day % 7 == 0 {
                self.week += 1;
            }
            if self.day % 28 == 0 {
                self.month += 1;
            }
        }

        self.is_day = self.hour >= 6 && self.hour < 18;
    }
}

/// System: Advances the clock.
/// 1 Tick = 1 Hour for this prototype (can be scaled later).
pub fn advance_time_system(mut time: ResMut<GameTime>) {
    time.advance();
}
