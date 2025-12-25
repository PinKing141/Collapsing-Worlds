use std::str::FromStr;

use serde_json::Value;

use crate::rules::cost::CostSpec;
use crate::rules::power::{ExpressionId, PowerId};
use crate::rules::signature::SignatureSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionForm {
    Beam,
    Projectile,
    Touch,
    Aura,
    Zone,
    Construct,
    Summon,
    Passive,
    Movement,
    Sense,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    Instant,
    Channeled,
    Toggled,
    Charged,
    Triggered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scale {
    Street,
    Block,
    District,
    City,
    Regional,
    Global,
    Cosmic,
}

#[derive(Debug, Clone)]
pub struct ExpressionText {
    pub ui_name: String,
    pub tooltip_short: String,
}

#[derive(Debug, Clone)]
pub struct Constraints {
    pub requires_contact: bool,
    pub requires_los: bool,
    pub range_m: Option<i64>,
    pub radius_m: Option<i64>,
    pub cooldown: Option<i64>,
    pub duration_turns: Option<i64>,
}

impl Constraints {
    pub fn from_json(value: &Value) -> Self {
        Self {
            requires_contact: value
                .get("requires_contact")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            requires_los: value
                .get("requires_los")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            range_m: value.get("range_m").and_then(Value::as_i64),
            radius_m: value.get("radius_m").and_then(Value::as_i64),
            cooldown: value.get("cooldown").and_then(Value::as_i64),
            duration_turns: value.get("duration_turns").and_then(Value::as_i64),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExpressionDef {
    pub id: ExpressionId,
    pub power_id: PowerId,
    pub form: ExpressionForm,
    pub delivery: Delivery,
    pub scale: Scale,
    pub constraints: Constraints,
    pub text: ExpressionText,
    pub costs: Vec<CostSpec>,
    pub signatures: Vec<SignatureSpec>,
}

#[derive(Debug)]
pub enum ExpressionError {
    MissingCosts(ExpressionId),
    MissingSignatures(ExpressionId),
}

impl std::fmt::Display for ExpressionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpressionError::MissingCosts(id) => write!(f, "expression {} missing costs", id.0),
            ExpressionError::MissingSignatures(id) => {
                write!(f, "expression {} missing signatures", id.0)
            }
        }
    }
}

impl std::error::Error for ExpressionError {}

impl ExpressionDef {
    pub fn validate_defaults(&self) -> Result<(), ExpressionError> {
        if self.form != ExpressionForm::Passive && self.costs.is_empty() {
            return Err(ExpressionError::MissingCosts(self.id.clone()));
        }
        if self.form != ExpressionForm::Passive && self.signatures.is_empty() {
            return Err(ExpressionError::MissingSignatures(self.id.clone()));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ParseEnumError {
    pub value: String,
}

impl std::fmt::Display for ParseEnumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown enum value: {}", self.value)
    }
}

impl std::error::Error for ParseEnumError {}

impl FromStr for ExpressionForm {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BEAM" => Ok(ExpressionForm::Beam),
            "PROJECTILE" => Ok(ExpressionForm::Projectile),
            "TOUCH" => Ok(ExpressionForm::Touch),
            "AURA" => Ok(ExpressionForm::Aura),
            "ZONE" => Ok(ExpressionForm::Zone),
            "CONSTRUCT" => Ok(ExpressionForm::Construct),
            "SUMMON" => Ok(ExpressionForm::Summon),
            "PASSIVE" => Ok(ExpressionForm::Passive),
            "MOVEMENT" => Ok(ExpressionForm::Movement),
            "SENSE" => Ok(ExpressionForm::Sense),
            _ => Err(ParseEnumError {
                value: s.to_string(),
            }),
        }
    }
}

impl FromStr for Delivery {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "INSTANT" => Ok(Delivery::Instant),
            "CHANNELED" => Ok(Delivery::Channeled),
            "TOGGLED" => Ok(Delivery::Toggled),
            "CHARGED" => Ok(Delivery::Charged),
            "TRIGGERED" => Ok(Delivery::Triggered),
            _ => Err(ParseEnumError {
                value: s.to_string(),
            }),
        }
    }
}

impl FromStr for Scale {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "STREET" => Ok(Scale::Street),
            "BLOCK" => Ok(Scale::Block),
            "DISTRICT" => Ok(Scale::District),
            "CITY" => Ok(Scale::City),
            "REGIONAL" => Ok(Scale::Regional),
            "GLOBAL" => Ok(Scale::Global),
            "COSMIC" => Ok(Scale::Cosmic),
            _ => Err(ParseEnumError {
                value: s.to_string(),
            }),
        }
    }
}
