use bevy_ecs::prelude::*;

use crate::rules::signature::SignatureInstance;
use crate::simulation::city::LocationId;

#[derive(Debug, Clone)]
pub struct SignatureEvent {
    pub location_id: LocationId,
    pub signature: SignatureInstance,
    pub is_new: bool,
}

#[derive(Resource, Debug, Default)]
pub struct WorldEvidence {
    pub signatures: Vec<SignatureEvent>,
}

impl WorldEvidence {
    pub fn emit(&mut self, location_id: LocationId, signatures: &[SignatureInstance]) {
        for sig in signatures {
            let mut instance = sig.clone();
            if instance.remaining_turns <= 0 {
                instance.remaining_turns = 5;
            }
            self.signatures.push(SignatureEvent {
                location_id,
                signature: instance,
                is_new: true,
            });
        }
    }

    pub fn tick_decay(&mut self) {
        for item in self.signatures.iter_mut() {
            if item.signature.remaining_turns > 0 {
                item.signature.remaining_turns -= 1;
            }
        }
        self.signatures
            .retain(|s| s.signature.remaining_turns > 0);
    }
}

pub trait EvidenceSink {
    fn emit(&mut self, where_: LocationId, sigs: &[SignatureInstance]);
}

impl EvidenceSink for WorldEvidence {
    fn emit(&mut self, where_: LocationId, sigs: &[SignatureInstance]) {
        WorldEvidence::emit(self, where_, sigs);
    }
}
