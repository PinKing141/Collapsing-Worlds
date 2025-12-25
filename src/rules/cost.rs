use std::str::FromStr;

use crate::rules::expression::ParseEnumError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostType {
    Stamina,
    Focus,
    Resource,
    Cooldown,
    Risk,
}

#[derive(Debug, Clone)]
pub struct CostSpec {
    pub cost_type: CostType,
    pub value: Option<i64>,
    pub risk_type: Option<String>,
    pub risk_chance: Option<f64>,
}

impl FromStr for CostType {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "STAMINA" => Ok(CostType::Stamina),
            "FOCUS" => Ok(CostType::Focus),
            "RESOURCE" => Ok(CostType::Resource),
            "COOLDOWN" => Ok(CostType::Cooldown),
            "RISK" => Ok(CostType::Risk),
            _ => Err(ParseEnumError {
                value: s.to_string(),
            }),
        }
    }
}
