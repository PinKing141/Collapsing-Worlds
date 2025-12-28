use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttributeKind {
    Strength,
    Agility,
    Endurance,
    Intellect,
    Will,
    Charisma,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Attributes {
    pub strength: i32,
    pub agility: i32,
    pub endurance: i32,
    pub intellect: i32,
    pub will: i32,
    pub charisma: i32,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            strength: 50,
            agility: 50,
            endurance: 50,
            intellect: 50,
            will: 50,
            charisma: 50,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AttributeCaps {
    pub min: i32,
    pub max: i32,
}

impl Attributes {
    pub fn get(&self, kind: AttributeKind) -> i32 {
        match kind {
            AttributeKind::Strength => self.strength,
            AttributeKind::Agility => self.agility,
            AttributeKind::Endurance => self.endurance,
            AttributeKind::Intellect => self.intellect,
            AttributeKind::Will => self.will,
            AttributeKind::Charisma => self.charisma,
        }
    }

    pub fn average(&self) -> i32 {
        let sum = self.strength
            + self.agility
            + self.endurance
            + self.intellect
            + self.will
            + self.charisma;
        sum / 6
    }

    pub fn clamp(self, caps: AttributeCaps) -> Self {
        Self {
            strength: self.strength.clamp(caps.min, caps.max),
            agility: self.agility.clamp(caps.min, caps.max),
            endurance: self.endurance.clamp(caps.min, caps.max),
            intellect: self.intellect.clamp(caps.min, caps.max),
            will: self.will.clamp(caps.min, caps.max),
            charisma: self.charisma.clamp(caps.min, caps.max),
        }
    }
}
