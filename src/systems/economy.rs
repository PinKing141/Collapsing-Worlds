use bevy_ecs::prelude::*;

use crate::simulation::civilian::{tick_civilian_economy, CivilianState};
use crate::simulation::economy::{can_fund_gadget, WealthTier};
use crate::simulation::time::GameTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GadgetTier {
    Basic,
    Advanced,
    Prototype,
    Arsenal,
}

impl GadgetTier {
    pub fn min_tier(self) -> WealthTier {
        match self {
            GadgetTier::Basic => WealthTier::Working,
            GadgetTier::Advanced => WealthTier::Affluent,
            GadgetTier::Prototype => WealthTier::Wealthy,
            GadgetTier::Arsenal => WealthTier::UltraWealthy,
        }
    }

    pub fn cost_cr(self) -> i64 {
        match self {
            GadgetTier::Basic => 200,
            GadgetTier::Advanced => 2_500,
            GadgetTier::Prototype => 12_000,
            GadgetTier::Arsenal => 75_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GadgetPurchaseError {
    InsufficientTier,
    InsufficientLiquidity,
}

pub fn attempt_gadget_purchase(
    civilian: &mut CivilianState,
    tier: GadgetTier,
) -> Result<i64, GadgetPurchaseError> {
    let cost = tier.cost_cr();
    let min_tier = tier.min_tier();
    if !can_fund_gadget(&civilian.wealth, min_tier, cost) {
        if civilian.wealth.tier.rank() < min_tier.rank() {
            return Err(GadgetPurchaseError::InsufficientTier);
        }
        return Err(GadgetPurchaseError::InsufficientLiquidity);
    }
    if !civilian.wealth.spend(cost) {
        return Err(GadgetPurchaseError::InsufficientLiquidity);
    }
    civilian.finances.cash =
        civilian.wealth.current_cr.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    Ok(cost)
}

/// Runs the daily economy tick (income + upkeep) for civilian finances.
pub fn economy_system(mut civilian: ResMut<CivilianState>, time: Res<GameTime>) {
    tick_civilian_economy(&mut civilian, &time);
}
