pub const CURRENCY_CODE: &str = "CR";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WealthTier {
    Destitute,
    Poor,
    Working,
    Middle,
    Affluent,
    Wealthy,
    UltraWealthy,
    Elite,
    Titan,
}

impl WealthTier {
    pub fn rank(self) -> u8 {
        match self {
            WealthTier::Destitute => 0,
            WealthTier::Poor => 1,
            WealthTier::Working => 2,
            WealthTier::Middle => 3,
            WealthTier::Affluent => 4,
            WealthTier::Wealthy => 5,
            WealthTier::UltraWealthy => 6,
            WealthTier::Elite => 7,
            WealthTier::Titan => 8,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            WealthTier::Destitute => "DESTITUTE",
            WealthTier::Poor => "POOR",
            WealthTier::Working => "WORKING",
            WealthTier::Middle => "MIDDLE",
            WealthTier::Affluent => "AFFLUENT",
            WealthTier::Wealthy => "WEALTHY",
            WealthTier::UltraWealthy => "ULTRA_WEALTHY",
            WealthTier::Elite => "ELITE",
            WealthTier::Titan => "TITAN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WealthProfile {
    Balanced,
    Vigilante,
    Corporate,
}

#[derive(Debug, Clone, Copy)]
pub struct WealthProfileModifiers {
    pub income_scale: f32,
    pub upkeep_scale: f32,
    pub liquidity_bonus: f32,
    pub liquidity_cap: f32,
}

impl WealthProfile {
    pub fn label(self) -> &'static str {
        match self {
            WealthProfile::Balanced => "BALANCED",
            WealthProfile::Vigilante => "VIGILANTE",
            WealthProfile::Corporate => "CORPORATE",
        }
    }

    pub fn modifiers(self) -> WealthProfileModifiers {
        match self {
            WealthProfile::Balanced => WealthProfileModifiers {
                income_scale: 1.0,
                upkeep_scale: 1.0,
                liquidity_bonus: 0.0,
                liquidity_cap: 1.0,
            },
            WealthProfile::Vigilante => WealthProfileModifiers {
                income_scale: 1.0,
                upkeep_scale: 1.4,
                liquidity_bonus: 0.08,
                liquidity_cap: 0.75,
            },
            WealthProfile::Corporate => WealthProfileModifiers {
                income_scale: 1.15,
                upkeep_scale: 0.8,
                liquidity_bonus: -0.12,
                liquidity_cap: 0.45,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Wealth {
    pub current_cr: i64,
    pub tier: WealthTier,
    pub income_per_tick: i64,
    pub upkeep_per_tick: i64,
    pub liquidity: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct EconomyTickResult {
    pub income_cr: i64,
    pub upkeep_cr: i64,
    pub net_cr: i64,
    pub balance_cr: i64,
    pub tier: WealthTier,
}

impl Default for Wealth {
    fn default() -> Self {
        Self::new(0)
    }
}

impl Wealth {
    pub fn new(current_cr: i64) -> Self {
        let tier = wealth_tier_for(current_cr);
        Self {
            current_cr,
            tier,
            income_per_tick: 0,
            upkeep_per_tick: 0,
            liquidity: default_liquidity_for_tier(tier),
        }
    }

    pub fn net_worth(&self, debt_cr: i64) -> i64 {
        self.current_cr.saturating_sub(debt_cr.max(0))
    }

    pub fn refresh_tier(&mut self, debt_cr: i64) -> WealthTier {
        let net = self.net_worth(debt_cr);
        self.tier = wealth_tier_for(net);
        self.tier
    }

    pub fn available_liquidity(&self) -> i64 {
        let ratio = clamp_liquidity(self.liquidity);
        ((self.current_cr as f64) * (ratio as f64)).round() as i64
    }

    pub fn can_spend(&self, cost_cr: i64) -> bool {
        cost_cr <= self.available_liquidity()
    }

    pub fn spend(&mut self, cost_cr: i64) -> bool {
        if cost_cr <= 0 {
            return true;
        }
        if !self.can_spend(cost_cr) {
            return false;
        }
        self.current_cr = (self.current_cr - cost_cr).max(0);
        true
    }

    pub fn apply_tick(&mut self, debt_cr: i64) -> EconomyTickResult {
        let income = self.income_per_tick.max(0);
        let upkeep = self.upkeep_per_tick.max(0);
        let net = income - upkeep;
        self.current_cr = (self.current_cr + net).max(0);
        let tier = self.refresh_tier(debt_cr);
        EconomyTickResult {
            income_cr: income,
            upkeep_cr: upkeep,
            net_cr: net,
            balance_cr: self.current_cr,
            tier,
        }
    }
}

pub fn wealth_tier_for(net_worth_cr: i64) -> WealthTier {
    match net_worth_cr {
        value if value < 100 => WealthTier::Destitute,
        value if value < 1_000 => WealthTier::Poor,
        value if value < 10_000 => WealthTier::Working,
        value if value < 100_000 => WealthTier::Middle,
        value if value < 1_000_000 => WealthTier::Affluent,
        value if value < 10_000_000 => WealthTier::Wealthy,
        value if value < 100_000_000 => WealthTier::UltraWealthy,
        value if value < 1_000_000_000 => WealthTier::Elite,
        _ => WealthTier::Titan,
    }
}

pub fn lifestyle_upkeep(tier: WealthTier) -> i64 {
    match tier {
        WealthTier::Destitute => 0,
        WealthTier::Poor => 10,
        WealthTier::Working => 40,
        WealthTier::Middle => 150,
        WealthTier::Affluent => 600,
        WealthTier::Wealthy => 2_000,
        WealthTier::UltraWealthy => 5_000,
        WealthTier::Elite => 10_000,
        WealthTier::Titan => 25_000,
    }
}

pub fn default_liquidity_for_tier(tier: WealthTier) -> f32 {
    match tier {
        WealthTier::Destitute => 0.9,
        WealthTier::Poor => 0.85,
        WealthTier::Working => 0.8,
        WealthTier::Middle => 0.7,
        WealthTier::Affluent => 0.6,
        WealthTier::Wealthy => 0.5,
        WealthTier::UltraWealthy => 0.4,
        WealthTier::Elite => 0.3,
        WealthTier::Titan => 0.2,
    }
}

pub fn clamp_liquidity(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

pub fn can_fund_gadget(wealth: &Wealth, min_tier: WealthTier, cost_cr: i64) -> bool {
    wealth.tier.rank() >= min_tier.rank() && wealth.can_spend(cost_cr)
}
