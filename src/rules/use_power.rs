use std::collections::{HashMap, HashSet};

use crate::rules::cost::{CostSpec, CostType};
use crate::rules::expression::ExpressionDef;
use crate::rules::mastery::MasteryStage;
use crate::rules::power::ExpressionId;
use crate::rules::signature::{SignatureInstance, SignatureSpec};

#[derive(Debug, Default)]
pub struct ActorState {
    pub stamina: i64,
    pub focus: i64,
    pub resources: HashMap<String, i64>,
    pub cooldowns: HashMap<ExpressionId, i64>,
}

#[derive(Debug, Default)]
pub struct WorldState {
    pub turn: u64,
}

#[derive(Debug)]
pub struct TargetContext {
    pub distance_m: Option<i64>,
    pub has_line_of_sight: bool,
    pub has_contact: bool,
    pub in_public: bool,
    pub witnesses: u32,
}

#[derive(Debug)]
pub struct UseContext<'a> {
    pub actor: &'a mut ActorState,
    pub world: &'a WorldState,
    pub mastery: Option<MasteryStage>,
    pub unlocked: Option<&'a HashSet<ExpressionId>>,
}

#[derive(Debug)]
pub enum UseError {
    Locked,
    OnCooldown,
    NotEnoughStamina,
    NotEnoughFocus,
    MissingResource,
    ConstraintFailed(&'static str),
}

#[derive(Debug)]
pub struct UseResult {
    pub applied_costs: Vec<CostSpec>,
    pub emitted_signatures: Vec<SignatureInstance>,
    pub cooldown_turns: Option<i64>,
    pub mastery_stage: MasteryStage,
}

pub fn can_use(ctx: &UseContext, expr: &ExpressionDef, target: &TargetContext) -> Result<(), UseError> {
    let mastery_stage = ctx.mastery.unwrap_or(MasteryStage::Raw);
    let costs = apply_mastery_costs(&expr.costs, mastery_stage);
    if let Some(unlocked) = ctx.unlocked {
        if !unlocked.contains(&expr.id) {
            return Err(UseError::Locked);
        }
    }
    if expr.constraints.requires_contact && !target.has_contact {
        return Err(UseError::ConstraintFailed("requires_contact"));
    }
    if expr.constraints.requires_los && !target.has_line_of_sight {
        return Err(UseError::ConstraintFailed("requires_los"));
    }
    if let (Some(range), Some(distance)) = (expr.constraints.range_m, target.distance_m) {
        if distance > range {
            return Err(UseError::ConstraintFailed("range"));
        }
    }

    if let Some(turns) = ctx.actor.cooldowns.get(&expr.id) {
        if *turns > 0 {
            return Err(UseError::OnCooldown);
        }
    }

    let stamina_cost = sum_costs(&costs, CostType::Stamina);
    if ctx.actor.stamina < stamina_cost {
        return Err(UseError::NotEnoughStamina);
    }

    let focus_cost = sum_costs(&costs, CostType::Focus);
    if ctx.actor.focus < focus_cost {
        return Err(UseError::NotEnoughFocus);
    }

    let resource_cost = sum_costs(&costs, CostType::Resource);
    if resource_cost > 0 {
        let available = ctx.actor.resources.get("resource").copied().unwrap_or(0);
        if available < resource_cost {
            return Err(UseError::MissingResource);
        }
    }

    Ok(())
}

pub fn use_power(
    ctx: &mut UseContext,
    expr: &ExpressionDef,
    target: &TargetContext,
) -> Result<UseResult, UseError> {
    can_use(ctx, expr, target)?;
    let mastery_stage = ctx.mastery.unwrap_or(MasteryStage::Raw);
    let costs = apply_mastery_costs(&expr.costs, mastery_stage);

    let stamina_cost = sum_costs(&costs, CostType::Stamina);
    let focus_cost = sum_costs(&costs, CostType::Focus);
    let resource_cost = sum_costs(&costs, CostType::Resource);

    if stamina_cost > 0 {
        ctx.actor.stamina -= stamina_cost;
    }
    if focus_cost > 0 {
        ctx.actor.focus -= focus_cost;
    }
    if resource_cost > 0 {
        let entry = ctx.actor.resources.entry("resource".to_string()).or_insert(0);
        *entry -= resource_cost;
    }

    let cooldown_turns = max_cost(&costs, CostType::Cooldown);
    if let Some(turns) = cooldown_turns {
        ctx.actor.cooldowns.insert(expr.id.clone(), turns);
    }

    let emitted_signatures = apply_mastery_signatures(&expr.signatures, mastery_stage)
        .iter()
        .map(SignatureSpec::to_instance)
        .collect();

    Ok(UseResult {
        applied_costs: costs,
        emitted_signatures,
        cooldown_turns,
        mastery_stage,
    })
}

fn sum_costs(costs: &[CostSpec], cost_type: CostType) -> i64 {
    costs
        .iter()
        .filter(|c| c.cost_type == cost_type)
        .filter_map(|c| c.value)
        .sum()
}

fn max_cost(costs: &[CostSpec], cost_type: CostType) -> Option<i64> {
    costs
        .iter()
        .filter(|c| c.cost_type == cost_type)
        .filter_map(|c| c.value)
        .max()
}

fn apply_mastery_costs(costs: &[CostSpec], stage: MasteryStage) -> Vec<CostSpec> {
    let (cost_num, cost_den, risk_num, risk_den) = mastery_cost_factors(stage);
    costs
        .iter()
        .map(|cost| {
            let value = cost.value.map(|v| scale_cost(v, cost_num, cost_den));
            let risk_chance = cost.risk_chance.map(|v| scale_risk(v, risk_num, risk_den));
            CostSpec {
                cost_type: cost.cost_type,
                value,
                risk_type: cost.risk_type.clone(),
                risk_chance,
            }
        })
        .collect()
}

fn apply_mastery_signatures(
    signatures: &[SignatureSpec],
    stage: MasteryStage,
) -> Vec<SignatureSpec> {
    let (strength_num, strength_den, persistence_delta) = mastery_signature_factors(stage);
    signatures
        .iter()
        .map(|sig| SignatureSpec {
            signature_type: sig.signature_type,
            strength: scale_cost(sig.strength, strength_num, strength_den),
            persistence_turns: (sig.persistence_turns + persistence_delta).max(1),
        })
        .collect()
}

fn mastery_cost_factors(stage: MasteryStage) -> (i64, i64, i64, i64) {
    match stage {
        MasteryStage::Raw => (100, 100, 100, 100),
        MasteryStage::Controlled => (95, 100, 90, 100),
        MasteryStage::Precise => (90, 100, 85, 100),
        MasteryStage::Silent => (85, 100, 70, 100),
        MasteryStage::Iconic => (80, 100, 65, 100),
    }
}

fn mastery_signature_factors(stage: MasteryStage) -> (i64, i64, i64) {
    match stage {
        MasteryStage::Raw => (100, 100, 0),
        MasteryStage::Controlled => (90, 100, 0),
        MasteryStage::Precise => (80, 100, -1),
        MasteryStage::Silent => (65, 100, -2),
        MasteryStage::Iconic => (60, 100, -2),
    }
}

fn scale_cost(value: i64, num: i64, den: i64) -> i64 {
    if value <= 0 {
        return value;
    }
    let scaled = (value * num + den - 1) / den;
    scaled.max(1)
}

fn scale_risk(value: f64, num: i64, den: i64) -> f64 {
    let scale = num as f64 / den as f64;
    value * scale
}
