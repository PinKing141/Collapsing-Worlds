use bevy_ecs::prelude::*;

use crate::simulation::case::{CaseEventLog, CaseRegistry, CaseStatus, CaseTargetType};
use crate::simulation::city::CityState;
use crate::simulation::evidence::WorldEvidence;
use crate::simulation::identity_evidence::IdentityEvidenceStore;

pub fn case_progress_system(
    mut cases: ResMut<CaseRegistry>,
    city: Res<CityState>,
    evidence: Res<WorldEvidence>,
    identity: Res<IdentityEvidenceStore>,
    mut log: ResMut<CaseEventLog>,
) {
    update_cases(&mut cases, &city, &evidence, &identity, &mut log);
}

pub fn update_cases(
    cases: &mut CaseRegistry,
    city: &CityState,
    evidence: &WorldEvidence,
    identity: &IdentityEvidenceStore,
    log: &mut CaseEventLog,
) {
    for case in cases.cases.iter_mut() {
        if case.status != CaseStatus::Active {
            continue;
        }

        let investigators = city
            .locations
            .get(&case.location_id)
            .map(|loc| loc.investigators as u32)
            .unwrap_or(0);

        let mut delta = investigators * 2;
        let matches = count_matching_signatures(evidence, case);
        if matches > 0 {
            delta += matches.min(3) as u32 * 2;
        }

        let evidence_hits = count_matching_evidence(identity, case);
        if evidence_hits > 0 {
            delta += evidence_hits.min(3) as u32 * 2;
        }

        if delta == 0 {
            continue;
        }

        case.progress = (case.progress + delta).min(100);
        update_case_milestones(case, log);
    }
}

fn count_matching_signatures(evidence: &WorldEvidence, case: &crate::simulation::case::Case) -> usize {
    evidence
        .signatures
        .iter()
        .filter(|event| {
            event.location_id == case.location_id
                && case
                    .signature_pattern
                    .contains(&event.signature.signature.signature_type)
        })
        .count()
}

fn count_matching_evidence(
    identity: &IdentityEvidenceStore,
    case: &crate::simulation::case::Case,
) -> usize {
    identity
        .items
        .iter()
        .filter(|item| item.location_id == case.location_id)
        .filter(|item| {
            let signature_match = item
                .signatures
                .iter()
                .any(|sig| case.signature_pattern.contains(sig));
            if !signature_match {
                return false;
            }
            match case.target_type {
                CaseTargetType::UnknownMasked => item.persona_hint != crate::simulation::identity_evidence::PersonaHint::Civilian,
                CaseTargetType::KnownMasked => true,
                CaseTargetType::CivilianLink => item.persona_hint != crate::simulation::identity_evidence::PersonaHint::Masked,
            }
        })
        .count()
}

fn update_case_milestones(case: &mut crate::simulation::case::Case, log: &mut CaseEventLog) {
    if case.progress >= 30 && case.milestone < 1 {
        case.milestone = 1;
        case.pressure_actions.push("PROFILE_FORMED".to_string());
        log.0.push(format!(
            "Case {}: suspect profile built",
            case.case_id
        ));
    }
    if case.progress >= 60 && case.milestone < 2 {
        case.milestone = 2;
        if case.target_type == CaseTargetType::UnknownMasked {
            case.target_type = CaseTargetType::KnownMasked;
        }
        case.pressure_actions.push("ACTIVE_OPERATIONS".to_string());
        log.0.push(format!("Case {}: search warrant ready", case.case_id));
    }
    if case.progress >= 85 && case.milestone < 3 {
        case.milestone = 3;
        if case.target_type != CaseTargetType::CivilianLink {
            case.target_type = CaseTargetType::CivilianLink;
        }
        case.pressure_actions.push("LINKAGE_ATTEMPT".to_string());
        log.0.push(format!(
            "Case {}: identity pressure increases",
            case.case_id
        ));
    }
    if case.progress >= 100 && case.status == CaseStatus::Active {
        case.status = CaseStatus::Resolved;
        case.pressure_actions.push("CONVERGENCE".to_string());
        log.0.push(format!("Case {}: resolved", case.case_id));
    }
}
