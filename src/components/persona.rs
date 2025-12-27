use serde::{Deserialize, Serialize};

use bevy_ecs::prelude::*;

use crate::simulation::city::LocationTag;

#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct PersonaStack {
    pub personas: Vec<Persona>,
    pub active_persona_id: String,
    pub next_switch_tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    pub persona_id: String,
    pub persona_type: PersonaType,
    pub label: String,
    pub location_rules: LocationRules,
    pub allowed_actions: Vec<PersonaAction>,
    pub visibility: VisibilityProfile,
    pub risk_modifiers: RiskModifiers,
    pub suspicion: PersonaSuspicion,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PersonaType {
    Civilian,
    Masked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PersonaAction {
    Work,
    Social,
    Patrol,
    PowerUse,
    Combat,
    Investigate,
    Rest,
    SwitchPersona,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocationRules {
    #[serde(default)]
    pub allowed_tags: Vec<LocationTag>,
    #[serde(default)]
    pub restricted_tags: Vec<LocationTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibilityProfile {
    pub public_visibility: u8,
    pub surveillance_visibility: u8,
    pub witness_visibility: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskModifiers {
    pub public_suspicion: f32,
    pub civilian_suspicion: f32,
    pub wanted_level: f32,
    pub exposure_risk: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersonaSuspicion {
    pub public_suspicion: u8,
    pub civilian_suspicion: u8,
    pub wanted_level: u8,
    pub exposure_risk: u8,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Alignment {
    Neutral,
    Hero,
    Vigilante,
    Villain,
}

impl PersonaStack {
    pub fn active_persona(&self) -> Option<&Persona> {
        self.personas
            .iter()
            .find(|persona| persona.persona_id == self.active_persona_id)
    }

    pub fn active_persona_mut(&mut self) -> Option<&mut Persona> {
        self.personas
            .iter_mut()
            .find(|persona| persona.persona_id == self.active_persona_id)
    }

    pub fn persona_mut(&mut self, persona_id: &str) -> Option<&mut Persona> {
        self.personas
            .iter_mut()
            .find(|persona| persona.persona_id == persona_id)
    }

    pub fn can_switch_to(&self, persona_id: &str, tags: &[LocationTag]) -> bool {
        let Some(persona) = self.personas.iter().find(|p| p.persona_id == persona_id) else {
            return false;
        };
        if persona.location_rules.restricted_tags.iter().any(|tag| tags.contains(tag)) {
            return false;
        }
        if persona.location_rules.allowed_tags.is_empty() {
            return true;
        }
        persona
            .location_rules
            .allowed_tags
            .iter()
            .any(|tag| tags.contains(tag))
    }
}

impl Alignment {
    pub fn suspicion_multiplier(self) -> RiskModifiers {
        match self {
            Alignment::Neutral => RiskModifiers {
                public_suspicion: 1.0,
                civilian_suspicion: 1.0,
                wanted_level: 1.0,
                exposure_risk: 1.0,
            },
            Alignment::Hero => RiskModifiers {
                public_suspicion: 1.0,
                civilian_suspicion: 1.1,
                wanted_level: 0.9,
                exposure_risk: 1.0,
            },
            Alignment::Vigilante => RiskModifiers {
                public_suspicion: 1.2,
                civilian_suspicion: 1.0,
                wanted_level: 1.2,
                exposure_risk: 1.1,
            },
            Alignment::Villain => RiskModifiers {
                public_suspicion: 0.9,
                civilian_suspicion: 0.7,
                wanted_level: 1.3,
                exposure_risk: 0.8,
            },
        }
    }
}

pub fn neutral_persona_stack() -> PersonaStack {
    let mut stack = hero_persona_stack();
    stack.personas.retain(|persona| persona.persona_type == PersonaType::Civilian);
    if let Some(civilian) = stack.personas.get_mut(0) {
        civilian.label = "Civilian".to_string();
        civilian.risk_modifiers = RiskModifiers {
            public_suspicion: 1.0,
            civilian_suspicion: 1.0,
            wanted_level: 1.0,
            exposure_risk: 1.0,
        };
    }
    stack.active_persona_id = "civilian".to_string();
    stack
}

pub fn hero_persona_stack() -> PersonaStack {
    PersonaStack {
        personas: vec![
            Persona {
                persona_id: "civilian".to_string(),
                persona_type: PersonaType::Civilian,
                label: "Civilian".to_string(),
                location_rules: LocationRules {
                    allowed_tags: vec![
                        LocationTag::Public,
                        LocationTag::Residential,
                    ],
                    restricted_tags: vec![LocationTag::HighSecurity],
                },
                allowed_actions: vec![
                    PersonaAction::Work,
                    PersonaAction::Social,
                    PersonaAction::Rest,
                ],
                visibility: VisibilityProfile {
                    public_visibility: 40,
                    surveillance_visibility: 30,
                    witness_visibility: 35,
                },
                risk_modifiers: RiskModifiers {
                    public_suspicion: 0.8,
                    civilian_suspicion: 1.1,
                    wanted_level: 0.7,
                    exposure_risk: 1.0,
                },
                suspicion: PersonaSuspicion::default(),
            },
            Persona {
                persona_id: "masked".to_string(),
                persona_type: PersonaType::Masked,
                label: "Masked".to_string(),
                location_rules: LocationRules {
                    allowed_tags: vec![
                        LocationTag::Public,
                        LocationTag::Industrial,
                        LocationTag::HighSecurity,
                    ],
                    restricted_tags: vec![],
                },
                allowed_actions: vec![
                    PersonaAction::Patrol,
                    PersonaAction::PowerUse,
                    PersonaAction::Combat,
                    PersonaAction::Rest,
                ],
                visibility: VisibilityProfile {
                    public_visibility: 65,
                    surveillance_visibility: 70,
                    witness_visibility: 60,
                },
                risk_modifiers: RiskModifiers {
                    public_suspicion: 1.1,
                    civilian_suspicion: 0.9,
                    wanted_level: 1.0,
                    exposure_risk: 1.0,
                },
                suspicion: PersonaSuspicion::default(),
            },
        ],
        active_persona_id: "civilian".to_string(),
        next_switch_tick: 0,
    }
}

pub fn vigilante_persona_stack() -> PersonaStack {
    let mut stack = hero_persona_stack();
    for persona in stack.personas.iter_mut() {
        match persona.persona_type {
            PersonaType::Civilian => {
                persona.risk_modifiers.public_suspicion = 1.0;
                persona.risk_modifiers.civilian_suspicion = 1.1;
                persona.risk_modifiers.wanted_level = 0.9;
                persona.label = "Civilian".to_string();
            }
            PersonaType::Masked => {
                persona.risk_modifiers.public_suspicion = 1.2;
                persona.risk_modifiers.wanted_level = 1.3;
                persona.label = "Vigilante".to_string();
            }
        }
    }
    stack
}

pub fn villain_persona_stack() -> PersonaStack {
    let mut stack = hero_persona_stack();
    for persona in stack.personas.iter_mut() {
        if persona.persona_type == PersonaType::Masked {
            persona.label = "Villain".to_string();
            persona.risk_modifiers.public_suspicion = 0.9;
            persona.risk_modifiers.wanted_level = 1.3;
            persona.risk_modifiers.exposure_risk = 0.8;
        }
    }
    stack
}

impl PersonaSuspicion {
    pub fn apply_delta(&mut self, delta: &SuspicionDelta) {
        self.public_suspicion = apply_delta(self.public_suspicion, delta.public_suspicion);
        self.civilian_suspicion = apply_delta(self.civilian_suspicion, delta.civilian_suspicion);
        self.wanted_level = apply_delta(self.wanted_level, delta.wanted_level);
        self.exposure_risk = apply_delta(self.exposure_risk, delta.exposure_risk);
    }
}

#[derive(Debug, Clone, Default)]
pub struct SuspicionDelta {
    pub public_suspicion: i32,
    pub civilian_suspicion: i32,
    pub wanted_level: i32,
    pub exposure_risk: i32,
}

fn apply_delta(current: u8, delta: i32) -> u8 {
    let next = current as i32 + delta;
    next.clamp(0, 100) as u8
}
