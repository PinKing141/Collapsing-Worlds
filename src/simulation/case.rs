use bevy_ecs::prelude::*;

use crate::rules::signature::SignatureType;
use crate::simulation::city::LocationId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseStatus {
    Active,
    Resolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseTargetType {
    UnknownMasked,
    KnownMasked,
    CivilianLink,
}

#[derive(Debug, Clone)]
pub struct Case {
    pub case_id: u32,
    pub faction_id: String,
    pub location_id: LocationId,
    pub target_type: CaseTargetType,
    pub signature_pattern: Vec<SignatureType>,
    pub progress: u32,
    pub heat_lock: bool,
    pub status: CaseStatus,
    pub milestone: u8,
    pub pressure_actions: Vec<String>,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct CaseRegistry {
    pub cases: Vec<Case>,
    next_id: u32,
}

#[derive(Resource, Debug, Default)]
pub struct CaseEventLog(pub Vec<String>);

impl CaseRegistry {
    pub fn create_case(
        &mut self,
        faction_id: String,
        location_id: LocationId,
        signature_pattern: Vec<SignatureType>,
        heat_lock: bool,
    ) -> u32 {
        let case_id = self.next_id + 1;
        self.next_id = case_id;
        self.cases.push(Case {
            case_id,
            faction_id,
            location_id,
            target_type: CaseTargetType::UnknownMasked,
            signature_pattern,
            progress: 0,
            heat_lock,
            status: CaseStatus::Active,
            milestone: 0,
            pressure_actions: Vec::new(),
        });
        case_id
    }

    pub fn find_case_mut(
        &mut self,
        faction_id: &str,
        location_id: LocationId,
    ) -> Option<&mut Case> {
        self.cases
            .iter_mut()
            .find(|case| case.faction_id == faction_id && case.location_id == location_id)
    }

    pub fn has_active_case(&self, faction_id: &str, location_id: LocationId) -> bool {
        self.cases.iter().any(|case| {
            case.faction_id == faction_id
                && case.location_id == location_id
                && case.status == CaseStatus::Active
        })
    }

    pub fn any_heat_lock(&self, location_id: LocationId) -> bool {
        self.cases.iter().any(|case| {
            case.location_id == location_id && case.status == CaseStatus::Active && case.heat_lock
        })
    }

    pub fn sync_next_id(&mut self) {
        let max_id = self.cases.iter().map(|case| case.case_id).max().unwrap_or(0);
        self.next_id = max_id;
    }
}
