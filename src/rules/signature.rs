use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::rules::expression::ParseEnumError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SignatureType {
    VisualAnomaly,
    EmSpike,
    ThermalBloom,
    AcousticShock,
    ChemicalResidue,
    PsychicEcho,
    RadiationTrace,
    BioMarker,
    DimensionalResidue,
    GraviticDisturbance,
    ArcaneResonance,
    CausalImprint,
    KineticStress,
}

#[derive(Debug, Clone)]
pub struct SignatureSpec {
    pub signature_type: SignatureType,
    pub strength: i64,
    pub persistence_turns: i64,
}

#[derive(Debug, Clone)]
pub struct SignatureInstance {
    pub signature: SignatureSpec,
    pub remaining_turns: i64,
}

const DEFAULT_SIGNATURE_PERSISTENCE: i64 = 5;

impl SignatureSpec {
    pub fn to_instance(&self) -> SignatureInstance {
        let remaining_turns = if self.persistence_turns > 0 {
            self.persistence_turns
        } else {
            DEFAULT_SIGNATURE_PERSISTENCE
        };
        SignatureInstance {
            signature: self.clone(),
            remaining_turns,
        }
    }
}

impl FromStr for SignatureType {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "VISUAL_ANOMALY" => Ok(SignatureType::VisualAnomaly),
            "EM_SPIKE" => Ok(SignatureType::EmSpike),
            "THERMAL_BLOOM" => Ok(SignatureType::ThermalBloom),
            "ACOUSTIC_SHOCK" => Ok(SignatureType::AcousticShock),
            "CHEMICAL_RESIDUE" => Ok(SignatureType::ChemicalResidue),
            "PSYCHIC_ECHO" => Ok(SignatureType::PsychicEcho),
            "RADIATION_TRACE" => Ok(SignatureType::RadiationTrace),
            "BIO_MARKER" => Ok(SignatureType::BioMarker),
            "DIMENSIONAL_RESIDUE" => Ok(SignatureType::DimensionalResidue),
            "GRAVITIC_DISTURBANCE" => Ok(SignatureType::GraviticDisturbance),
            "ARCANE_RESONANCE" => Ok(SignatureType::ArcaneResonance),
            "CAUSAL_IMPRINT" => Ok(SignatureType::CausalImprint),
            "KINETIC_STRESS" => Ok(SignatureType::KineticStress),
            _ => Err(ParseEnumError {
                value: s.to_string(),
            }),
        }
    }
}
