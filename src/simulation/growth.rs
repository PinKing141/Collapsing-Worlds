use std::collections::{HashMap, HashSet};

use crate::rules::expression::ExpressionForm;
use crate::rules::mastery::{stage_from_uses, MasteryStage};
use crate::rules::power::ExpressionId;
use crate::rules::ExpressionDef;

#[derive(Debug, Clone)]
pub struct ExpressionMastery {
    pub stage: MasteryStage,
    pub uses: u32,
}

#[derive(Debug, Clone)]
pub struct Reputation {
    pub trust: i32,
    pub fear: i32,
    pub infamy: i32,
    pub symbolism: i32,
}

#[derive(Debug, Clone)]
pub struct GrowthState {
    pub mastery: HashMap<ExpressionId, ExpressionMastery>,
    pub unlocked_expressions: HashSet<ExpressionId>,
    pub reputation: Reputation,
    pub pressure_resistance: i32,
}

impl Default for Reputation {
    fn default() -> Self {
        Self {
            trust: 0,
            fear: 0,
            infamy: 0,
            symbolism: 0,
        }
    }
}

impl Default for GrowthState {
    fn default() -> Self {
        Self {
            mastery: HashMap::new(),
            unlocked_expressions: HashSet::new(),
            reputation: Reputation::default(),
            pressure_resistance: 0,
        }
    }
}

pub fn record_expression_use(growth: &mut GrowthState, expr: &ExpressionDef) -> Option<MasteryStage> {
    growth.unlocked_expressions.insert(expr.id.clone());

    let entry = growth
        .mastery
        .entry(expr.id.clone())
        .or_insert(ExpressionMastery {
            stage: MasteryStage::Raw,
            uses: 0,
        });
    entry.uses = entry.uses.saturating_add(1);
    let next_stage = stage_from_uses(entry.uses);
    if next_stage != entry.stage {
        entry.stage = next_stage;
        return Some(next_stage);
    }
    None
}

pub fn select_evolution_candidate(
    expr: &ExpressionDef,
    candidates: &[ExpressionDef],
    unlocked: &HashSet<ExpressionId>,
) -> Option<ExpressionId> {
    let preferred = preferred_forms(expr.form);
    let mut choices: Vec<&ExpressionDef> = candidates
        .iter()
        .filter(|candidate| candidate.id != expr.id)
        .filter(|candidate| !unlocked.contains(&candidate.id))
        .collect();

    if choices.is_empty() {
        return None;
    }

    if !preferred.is_empty() {
        let mut preferred_choices: Vec<&ExpressionDef> = choices
            .iter()
            .copied()
            .filter(|candidate| preferred.contains(&candidate.form))
            .collect();
        if !preferred_choices.is_empty() {
            preferred_choices.sort_by(|a, b| a.id.0.cmp(&b.id.0));
            return Some(preferred_choices[0].id.clone());
        }
    }

    choices.sort_by(|a, b| a.id.0.cmp(&b.id.0));
    Some(choices[0].id.clone())
}

fn preferred_forms(form: ExpressionForm) -> Vec<ExpressionForm> {
    match form {
        ExpressionForm::Beam | ExpressionForm::Projectile => {
            vec![ExpressionForm::Zone, ExpressionForm::Aura, ExpressionForm::Touch]
        }
        ExpressionForm::Zone | ExpressionForm::Aura => {
            vec![ExpressionForm::Projectile, ExpressionForm::Beam]
        }
        ExpressionForm::Touch => vec![ExpressionForm::Sense, ExpressionForm::Aura],
        ExpressionForm::Sense => vec![ExpressionForm::Touch],
        ExpressionForm::Movement => vec![ExpressionForm::Touch, ExpressionForm::Aura],
        ExpressionForm::Summon => vec![ExpressionForm::Zone, ExpressionForm::Aura],
        ExpressionForm::Construct => vec![ExpressionForm::Projectile, ExpressionForm::Zone],
        ExpressionForm::Passive => vec![ExpressionForm::Touch],
    }
}
