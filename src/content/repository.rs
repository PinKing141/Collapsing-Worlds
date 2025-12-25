use serde_json::Value;

use crate::rules::expression::ExpressionDef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PowerId(pub i64);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExpressionId(pub String);

#[derive(Debug, Clone)]
pub struct PersonaExpression {
    pub persona_id: String,
    pub mastery_level: i64,
    pub modifiers: Value,
    pub is_unlocked: bool,
    pub expression: ExpressionDef,
}

#[derive(Debug, Clone)]
pub struct PowerInfo {
    pub id: PowerId,
    pub name: String,
    pub overview: Option<String>,
    pub description: Option<String>,
    pub text_short: Option<String>,
    pub text_mechanical: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct PowerStats {
    pub power_count: i64,
    pub expression_count: i64,
    pub acquisition_count: i64,
}

pub trait PowerRepository {
    fn stats(&self) -> Result<PowerStats, Box<dyn std::error::Error>>;
    fn expression(&self, expr_id: &ExpressionId) -> Result<ExpressionDef, Box<dyn std::error::Error>>;
    fn expressions_for_power(
        &self,
        power_id: PowerId,
    ) -> Result<Vec<ExpressionDef>, Box<dyn std::error::Error>>;
    fn power_info(
        &self,
        power_id: PowerId,
    ) -> Result<Option<PowerInfo>, Box<dyn std::error::Error>>;
    fn expressions_for_persona(
        &self,
        persona_id: &str,
    ) -> Result<Vec<PersonaExpression>, Box<dyn std::error::Error>>;
}
