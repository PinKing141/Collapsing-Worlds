use std::fmt;

use crate::simulation::civilian::{CivilianJob, JobRole, JobStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    Credits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Money {
    cents: i64,
}

impl Money {
    pub fn zero() -> Self {
        Self { cents: 0 }
    }

    pub fn from_dollars(dollars: i64) -> Self {
        Self {
            cents: dollars.saturating_mul(100),
        }
    }

    pub fn as_dollars(self) -> i64 {
        self.cents / 100
    }

    pub fn add(self, other: Money) -> Self {
        Self {
            cents: self.cents.saturating_add(other.cents),
        }
    }

    pub fn sub(self, other: Money) -> Self {
        Self {
            cents: self.cents.saturating_sub(other.cents),
        }
    }

    pub fn scale(self, factor: f64) -> Self {
        let scaled = (self.cents as f64 * factor).round() as i64;
        Self { cents: scaled }
    }

    pub fn per_day(self, days: i64) -> Self {
        if days <= 0 {
            return self;
        }
        Self {
            cents: self.cents / days,
        }
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dollars = self.cents / 100;
        let cents = (self.cents.abs() % 100) as i64;
        let sign = if self.cents < 0 { "-" } else { "" };
        let formatted = format_dollars(dollars.abs());
        write!(f, "{}${}.{:02}", sign, formatted, cents)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WealthTier {
    Struggling,
    Stable,
    Comfortable,
    Affluent,
    Wealthy,
    UltraWealthy,
    Billionaire,
}

impl fmt::Display for WealthTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            WealthTier::Struggling => "Struggling",
            WealthTier::Stable => "Stable",
            WealthTier::Comfortable => "Comfortable",
            WealthTier::Affluent => "Affluent",
            WealthTier::Wealthy => "Wealthy",
            WealthTier::UltraWealthy => "Ultra-wealthy",
            WealthTier::Billionaire => "Billionaire",
        };
        write!(f, "{}", label)
    }
}

#[derive(Debug, Clone)]
pub struct EconomyState {
    pub currency: Currency,
    pub liquid: Money,
    pub savings: Money,
    pub investments: Money,
    pub assets: Money,
    pub liabilities: Money,
    pub monthly_income: Money,
    pub monthly_expenses: Money,
    pub gadget_fund: Money,
    pub last_income_day: u32,
    pub last_expense_day: u32,
    pub last_gadget_day: u32,
}

impl Default for EconomyState {
    fn default() -> Self {
        Self {
            currency: Currency::Credits,
            liquid: Money::from_dollars(120),
            savings: Money::from_dollars(600),
            investments: Money::from_dollars(2_500),
            assets: Money::from_dollars(8_000),
            liabilities: Money::zero(),
            monthly_income: Money::from_dollars(3_200),
            monthly_expenses: Money::from_dollars(2_100),
            gadget_fund: Money::from_dollars(150),
            last_income_day: 0,
            last_expense_day: 0,
            last_gadget_day: 0,
        }
    }
}

impl EconomyState {
    pub fn net_worth(&self) -> Money {
        self.liquid
            .add(self.savings)
            .add(self.investments)
            .add(self.assets)
            .sub(self.liabilities)
    }

    pub fn wealth_tier(&self) -> WealthTier {
        let net = self.net_worth().as_dollars();
        match net {
            ..=0 => WealthTier::Struggling,
            1..=25_000 => WealthTier::Stable,
            25_001..=150_000 => WealthTier::Comfortable,
            150_001..=1_000_000 => WealthTier::Affluent,
            1_000_001..=25_000_000 => WealthTier::Wealthy,
            25_000_001..=999_999_999 => WealthTier::UltraWealthy,
            _ => WealthTier::Billionaire,
        }
    }

    pub fn update_from_job(&mut self, job: &CivilianJob, status: JobStatus) {
        let comp = job_compensation(job, status);
        self.monthly_income = comp.monthly_income;
    }

    pub fn update_monthly_expenses(&mut self, rent: i32) {
        let rent_cost = Money::from_dollars(rent as i64);
        let lifestyle = self
            .monthly_income
            .scale(0.35)
            .add(Money::from_dollars(350));
        self.monthly_expenses = rent_cost.add(lifestyle);
    }

    pub fn tick_daily(&mut self, day: u32) {
        if self.last_income_day != day {
            self.last_income_day = day;
            let daily_income = self.monthly_income.per_day(30);
            self.liquid = self.liquid.add(daily_income);
        }
        if self.last_expense_day != day {
            self.last_expense_day = day;
            let daily_expenses = self.monthly_expenses.per_day(30);
            self.liquid = self.liquid.sub(daily_expenses);
        }
        let investment_growth = self.investments.scale(0.05 / 365.0);
        self.investments = self.investments.add(investment_growth);

        if self.last_gadget_day != day {
            self.last_gadget_day = day;
            let allocation = match self.wealth_tier() {
                WealthTier::Billionaire => self.net_worth().scale(0.002).per_day(30),
                WealthTier::UltraWealthy => self.net_worth().scale(0.001).per_day(30),
                WealthTier::Wealthy => self.net_worth().scale(0.0005).per_day(30),
                _ => self.monthly_income.scale(0.02).per_day(30),
            };
            self.gadget_fund = self.gadget_fund.add(allocation);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct JobCompensation {
    pub annual_salary: Money,
    pub annual_bonus: Money,
    pub monthly_income: Money,
}

pub fn job_compensation(job: &CivilianJob, status: JobStatus) -> JobCompensation {
    let base_salary = base_salary_for(job.role);
    let level_multiplier = 1.0 + (job.level.max(0) as f64 * 0.12);
    let employment_factor = match status {
        JobStatus::Employed => 1.0,
        JobStatus::PartTime => 0.55,
        JobStatus::Unemployed => 0.0,
    };
    let annual_salary =
        Money::from_dollars((base_salary as f64 * level_multiplier * employment_factor) as i64);
    let bonus = annual_salary.scale(0.08);
    let monthly_income = annual_salary.add(bonus).per_day(365).scale(365.0 / 12.0);
    JobCompensation {
        annual_salary,
        annual_bonus: bonus,
        monthly_income,
    }
}

fn base_salary_for(role: JobRole) -> i64 {
    match role {
        JobRole::Lawyer => 115_000,
        JobRole::Journalist => 58_000,
        JobRole::Chef => 52_000,
        JobRole::Photographer => 48_000,
        JobRole::Scientist => 112_000,
        JobRole::Artist => 45_000,
        JobRole::Engineer => 98_000,
        JobRole::Nurse => 72_000,
        JobRole::Teacher => 55_000,
        JobRole::Mechanic => 60_000,
        JobRole::Analyst => 82_000,
        JobRole::Contractor => 88_000,
    }
}

fn format_dollars(mut value: i64) -> String {
    if value == 0 {
        return "0".to_string();
    }
    let mut parts = Vec::new();
    while value > 0 {
        parts.push(format!("{:03}", value % 1000));
        value /= 1000;
    }
    if let Some(last) = parts.last_mut() {
        *last = last.trim_start_matches('0').to_string();
        if last.is_empty() {
            *last = "0".to_string();
        }
    }
    parts.reverse();
    parts.join(",")
}
