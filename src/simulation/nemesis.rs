use std::collections::HashMap;

use bevy_ecs::prelude::*;

use crate::rules::signature::SignatureType;
use crate::simulation::city::LocationId;

#[derive(Debug, Clone, Default)]
pub struct NemesisSignaturePattern {
    pub pattern: Vec<SignatureType>,
    pub count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct NemesisMemory {
    pub signatures: Vec<SignatureType>,
    pub signature_counts: HashMap<SignatureType, u32>,
    pub signature_patterns: Vec<NemesisSignaturePattern>,
    pub forms: Vec<String>,
    pub form_counts: HashMap<String, u32>,
    pub identity_traits: Vec<String>,
}

impl NemesisMemory {
    pub fn record_signature(&mut self, signature: SignatureType) {
        if !self.signatures.contains(&signature) {
            self.signatures.push(signature);
        }
        *self.signature_counts.entry(signature).or_insert(0) += 1;
    }

    pub fn record_signature_pattern(&mut self, pattern: Vec<SignatureType>) {
        if pattern.is_empty() {
            return;
        }
        if let Some(entry) = self
            .signature_patterns
            .iter_mut()
            .find(|entry| entry.pattern == pattern)
        {
            entry.count = entry.count.saturating_add(1);
        } else {
            self.signature_patterns.push(NemesisSignaturePattern { pattern, count: 1 });
        }
    }

    pub fn record_form(&mut self, form: String) {
        if !self.forms.contains(&form) {
            self.forms.push(form);
        }
        *self.form_counts.entry(form).or_insert(0) += 1;
    }

    pub fn record_identity_trait(&mut self, trait_name: String) {
        if !self.identity_traits.contains(&trait_name) {
            self.identity_traits.push(trait_name);
        }
    }

    pub fn most_common_signature(&self, min_count: u32) -> Option<SignatureType> {
        self.signature_counts
            .iter()
            .filter(|(_, count)| **count >= min_count)
            .max_by_key(|(_, count)| *count)
            .map(|(signature, _)| *signature)
    }

    pub fn most_common_form(&self, min_count: u32) -> Option<String> {
        self.form_counts
            .iter()
            .filter(|(_, count)| **count >= min_count)
            .max_by_key(|(_, count)| *count)
            .map(|(form, _)| form.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NemesisPersonaArc {
    PublicThreat,
    SecretHunt,
}

#[derive(Debug, Clone)]
pub struct NemesisCandidate {
    pub faction_id: String,
    pub location_id: LocationId,
    pub heat: i32,
    pub case_progress: u32,
    pub memory: NemesisMemory,
    pub adaptation_level: u8,
    pub persona_arc: NemesisPersonaArc,
    pub is_nemesis: bool,
    pub last_action_tick: u64,
    pub last_storylet_tick: u64,
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
