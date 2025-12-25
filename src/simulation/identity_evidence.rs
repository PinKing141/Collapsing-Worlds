use bevy_ecs::prelude::*;

use crate::rules::signature::SignatureType;
use crate::simulation::city::LocationId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonaHint {
    Civilian,
    Masked,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct IdentityEvidenceItem {
    pub evidence_id: u32,
    pub location_id: LocationId,
    pub time_tick: u64,
    pub signatures: Vec<SignatureType>,
    pub witness_count: u32,
    pub visual_quality: u8,
    pub suspect_features: Vec<String>,
    pub persona_hint: PersonaHint,
}

#[derive(Resource, Debug, Default)]
pub struct IdentityEvidenceStore {
    pub items: Vec<IdentityEvidenceItem>,
    next_id: u32,
}

impl IdentityEvidenceStore {
    pub fn record(
        &mut self,
        location_id: LocationId,
        time_tick: u64,
        signatures: Vec<SignatureType>,
        witness_count: u32,
        visual_quality: u8,
        persona_hint: PersonaHint,
        suspect_features: Vec<String>,
    ) -> u32 {
        let evidence_id = self.next_id + 1;
        self.next_id = evidence_id;
        self.items.push(IdentityEvidenceItem {
            evidence_id,
            location_id,
            time_tick,
            signatures,
            witness_count,
            visual_quality,
            suspect_features,
            persona_hint,
        });
        evidence_id
    }
}
