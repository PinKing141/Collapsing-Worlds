use bevy_ecs::prelude::*;

use crate::rules::signature::SignatureType;
use crate::simulation::city::LocationId;

#[derive(Debug, Clone, Default)]
pub struct NemesisMemory {
    pub signatures: Vec<SignatureType>,
    pub forms: Vec<String>,
}

impl NemesisMemory {
    pub fn record_signature(&mut self, signature: SignatureType) {
        if !self.signatures.contains(&signature) {
            self.signatures.push(signature);
        }
    }

    pub fn record_form(&mut self, form: String) {
        if !self.forms.contains(&form) {
            self.forms.push(form);
        }
    }
}

#[derive(Debug, Clone)]
pub struct NemesisCandidate {
    pub faction_id: String,
    pub location_id: LocationId,
    pub heat: i32,
    pub case_progress: u32,
    pub memory: NemesisMemory,
    pub adaptation_level: u8,
    pub is_nemesis: bool,
    pub last_action_tick: u64,
}

#[derive(Debug, Clone)]
pub struct NemesisAdaptationThreshold {
    pub level: u8,
    pub min_heat: i32,
    pub min_case_progress: u32,
    pub cooldown: u64,
}

#[derive(Resource, Debug, Clone)]
pub struct NemesisState {
    pub candidates: Vec<NemesisCandidate>,
    pub thresholds: Vec<NemesisAdaptationThreshold>,
}

impl Default for NemesisState {
    fn default() -> Self {
        Self {
            candidates: Vec::new(),
            thresholds: vec![
                NemesisAdaptationThreshold {
                    level: 1,
                    min_heat: 35,
                    min_case_progress: 25,
                    cooldown: 3,
                },
                NemesisAdaptationThreshold {
                    level: 2,
                    min_heat: 55,
                    min_case_progress: 60,
                    cooldown: 2,
                },
                NemesisAdaptationThreshold {
                    level: 3,
                    min_heat: 75,
                    min_case_progress: 85,
                    cooldown: 1,
                },
            ],
        }
    }
}
