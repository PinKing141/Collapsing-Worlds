use bevy_ecs::prelude::*;

use crate::simulation::economy::{EconomyState, Money};
use crate::simulation::time::GameTime;

#[derive(Resource, Debug, Clone)]
pub struct CivilianState {
    pub job_status: JobStatus,
    pub job: CivilianJob,
    pub finances: CivilianFinances,
    pub economy: EconomyState,
    pub social: SocialTies,
    pub reputation: ReputationTrack,
    pub rewards: CivilianRewards,
    pub contacts: Vec<Contact>,
    pub pending_events: Vec<CivilianEvent>,
    pub last_day: u32,
    pub last_work_day: u32,
    pub last_relationship_day: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Employed,
    PartTime,
    Unemployed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobRole {
    Lawyer,
    Journalist,
    Chef,
    Photographer,
    Scientist,
    Artist,
    Engineer,
    Nurse,
    Teacher,
    Mechanic,
    Analyst,
    Contractor,
}

#[derive(Debug, Clone)]
pub struct CivilianJob {
    pub role: JobRole,
    pub level: i32,
    pub satisfaction: i32,
    pub stability: i32,
}

#[derive(Debug, Clone)]
pub struct CivilianFinances {
    pub cash: i32,
    pub debt: i32,
    pub rent: i32,
    pub rent_due_in: i32,
    pub wage: i32,
}

#[derive(Debug, Clone)]
pub struct SocialTies {
    pub support: i32,
    pub strain: i32,
    pub obligation: i32,
}

#[derive(Debug, Clone)]
pub struct ReputationTrack {
    pub career: i32,
    pub community: i32,
    pub media: i32,
}

#[derive(Debug, Clone)]
pub struct CivilianRewards {
    pub income_boost: i32,
    pub safehouse: i32,
    pub access: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipLevel {
    Stranger,
    Acquaintance,
    Friend,
    Confidant,
    Ally,
}

#[derive(Debug, Clone)]
pub struct Contact {
    pub name: String,
    pub level: RelationshipLevel,
    pub bond: i32,
    pub influence: i32,
}

#[derive(Debug, Clone)]
pub struct CivilianEvent {
    pub storylet_id: String,
    pub created_tick: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CivilianPressure {
    pub temporal: f32,
    pub resource: f32,
    pub moral: f32,
    pub identity: f32,
}

impl Default for CivilianState {
    fn default() -> Self {
        let job_status = JobStatus::Employed;
        let job = CivilianJob {
            role: JobRole::Journalist,
            level: 1,
            satisfaction: 52,
            stability: 48,
        };
        let finances = CivilianFinances {
            cash: 120,
            debt: 0,
            rent: 80,
            rent_due_in: 5,
            wage: 45,
        };
        let mut economy = EconomyState::default();
        economy.update_from_job(&job, job_status);
        economy.update_monthly_expenses(finances.rent);

        Self {
            job_status,
            job,
            finances,
            economy,
            social: SocialTies {
                support: 55,
                strain: 10,
                obligation: 12,
            },
            reputation: ReputationTrack {
                career: 42,
                community: 38,
                media: 25,
            },
            rewards: CivilianRewards {
                income_boost: 0,
                safehouse: 0,
                access: 0,
            },
            contacts: Vec::new(),
            pending_events: Vec::new(),
            last_day: 0,
            last_work_day: 0,
            last_relationship_day: 0,
        }
    }
}

impl CivilianState {
    pub fn pressure_targets(&self) -> CivilianPressure {
        let base_time = match self.job_status {
            JobStatus::Employed => 18.0,
            JobStatus::PartTime => 10.0,
            JobStatus::Unemployed => 6.0,
        };

        let rent_urgency = if self.finances.rent_due_in <= 0 {
            18.0
        } else if self.finances.rent_due_in <= 3 {
            10.0
        } else {
            0.0
        };

        let mut temporal =
            (base_time + rent_urgency + self.social.obligation as f32 * 0.6).clamp(0.0, 100.0);
        if self.rewards.access > 0 {
            temporal -= self.rewards.access as f32 * 0.4;
        }
        temporal = temporal.clamp(0.0, 100.0);

        let liquid = self.economy.liquid.as_dollars() as f32;
        let monthly_expenses = self.economy.monthly_expenses.as_dollars() as f32;
        let liabilities = self.economy.liabilities.as_dollars() as f32;

        let mut resource = (liabilities.max(0.0) * 0.08).clamp(0.0, 60.0);
        if liquid < monthly_expenses {
            resource += 18.0;
        }
        if self.finances.rent_due_in <= 2 {
            resource += 12.0;
        }
        if self.rewards.income_boost > 0 {
            resource -= self.rewards.income_boost as f32 * 0.6;
        }
        resource = resource.clamp(0.0, 100.0);

        let mut moral = (self.social.strain as f32 * 1.4).clamp(0.0, 60.0);
        if self.social.support < 35 {
            moral += 12.0;
        }
        if self.reputation.community < 40 {
            moral += (40 - self.reputation.community) as f32 * 0.25;
        }
        moral = moral.clamp(0.0, 100.0);

        let mut identity = (self.reputation.media as f32 * 0.7).clamp(0.0, 70.0);
        identity += (self.contacts.len() as f32 * 2.0).clamp(0.0, 20.0);
        if self.rewards.safehouse > 0 {
            identity -= self.rewards.safehouse as f32 * 6.0;
        }
        if self.rewards.access > 0 {
            identity -= self.rewards.access as f32 * 2.5;
        }
        identity = identity.clamp(0.0, 100.0);

        CivilianPressure {
            temporal,
            resource,
            moral,
            identity,
        }
    }
}

pub fn tick_civilian_life(state: &mut CivilianState, time: &GameTime) {
    if time.day != state.last_day {
        state.last_day = time.day;
        state.finances.rent_due_in -= 1;
        if state.finances.rent_due_in <= 0 {
            queue_event(state, "civilian_rent_due", time.tick);
        }
        state.economy.update_from_job(&state.job, state.job_status);
        state.economy.update_monthly_expenses(state.finances.rent);
        state.economy.tick_daily(time.day);
    }

    if matches!(state.job_status, JobStatus::Employed | JobStatus::PartTime)
        && time.hour == 9
        && state.last_work_day != time.day
    {
        state.last_work_day = time.day;
        queue_event(state, "civilian_work_shift", time.tick);
    }

    if time.hour == 19 && state.last_relationship_day != time.day {
        state.last_relationship_day = time.day;
        if state.social.support < 60 || state.social.strain > 15 {
            queue_event(state, "civilian_relationship_checkin", time.tick);
        }
    }
}

pub fn apply_civilian_effects(state: &mut CivilianState, effects: &[String]) -> Vec<String> {
    let mut applied = Vec::new();
    for effect in effects {
        let parts: Vec<&str> = effect.split(':').collect();
        if parts.is_empty() {
            continue;
        }
        let key = parts[0].trim();
        match key {
            "cash" => apply_delta_at(&mut state.finances.cash, parts.get(1), &mut applied, "cash"),
            "debt" => apply_delta_at(&mut state.finances.debt, parts.get(1), &mut applied, "debt"),
            "rent_due_in" => apply_delta_at(
                &mut state.finances.rent_due_in,
                parts.get(1),
                &mut applied,
                "rent_due_in",
            ),
            "wage" => apply_delta_at(&mut state.finances.wage, parts.get(1), &mut applied, "wage"),
            "liquid" => apply_money_delta(
                &mut state.economy.liquid,
                parts.get(1),
                &mut applied,
                "liquid",
            ),
            "savings" => apply_money_delta(
                &mut state.economy.savings,
                parts.get(1),
                &mut applied,
                "savings",
            ),
            "investments" => apply_money_delta(
                &mut state.economy.investments,
                parts.get(1),
                &mut applied,
                "investments",
            ),
            "assets" => apply_money_delta(
                &mut state.economy.assets,
                parts.get(1),
                &mut applied,
                "assets",
            ),
            "liabilities" => apply_money_delta(
                &mut state.economy.liabilities,
                parts.get(1),
                &mut applied,
                "liabilities",
            ),
            "gadget_fund" => apply_money_delta(
                &mut state.economy.gadget_fund,
                parts.get(1),
                &mut applied,
                "gadget_fund",
            ),
            "support" => {
                apply_delta_at(
                    &mut state.social.support,
                    parts.get(1),
                    &mut applied,
                    "support",
                );
                state.social.support = clamp_metric(state.social.support);
            }
            "strain" => {
                apply_delta_at(
                    &mut state.social.strain,
                    parts.get(1),
                    &mut applied,
                    "strain",
                );
                state.social.strain = clamp_metric(state.social.strain);
            }
            "obligation" => {
                apply_delta_at(
                    &mut state.social.obligation,
                    parts.get(1),
                    &mut applied,
                    "obligation",
                );
                state.social.obligation = clamp_metric(state.social.obligation);
            }
            "career" => {
                apply_delta_at(
                    &mut state.reputation.career,
                    parts.get(1),
                    &mut applied,
                    "career",
                );
                state.reputation.career = clamp_metric(state.reputation.career);
            }
            "community" => {
                apply_delta_at(
                    &mut state.reputation.community,
                    parts.get(1),
                    &mut applied,
                    "community",
                );
                state.reputation.community = clamp_metric(state.reputation.community);
            }
            "media" => {
                apply_delta_at(
                    &mut state.reputation.media,
                    parts.get(1),
                    &mut applied,
                    "media",
                );
                state.reputation.media = clamp_metric(state.reputation.media);
            }
            "income_boost" => {
                apply_delta_at(
                    &mut state.rewards.income_boost,
                    parts.get(1),
                    &mut applied,
                    "income_boost",
                );
                state.rewards.income_boost = clamp_metric(state.rewards.income_boost);
            }
            "safehouse" => {
                apply_delta_at(
                    &mut state.rewards.safehouse,
                    parts.get(1),
                    &mut applied,
                    "safehouse",
                );
                state.rewards.safehouse = clamp_metric(state.rewards.safehouse);
            }
            "access" => {
                apply_delta_at(
                    &mut state.rewards.access,
                    parts.get(1),
                    &mut applied,
                    "access",
                );
                state.rewards.access = clamp_metric(state.rewards.access);
            }
            "job" => {
                if let Some(job) = parts
                    .get(1)
                    .and_then(|value| parse_job_status(value.trim()))
                {
                    state.job_status = job;
                    applied.push(format!("job -> {:?}", job));
                }
            }
            "job_role" => {
                if let Some(role) = parts.get(1).and_then(|value| parse_job_role(value.trim())) {
                    state.job.role = role;
                    applied.push(format!("job role -> {:?}", role));
                }
            }
            "job_level" => {
                apply_delta_at(
                    &mut state.job.level,
                    parts.get(1),
                    &mut applied,
                    "job_level",
                );
                state.job.level = state.job.level.clamp(0, 10);
            }
            "job_satisfaction" => {
                apply_delta_at(
                    &mut state.job.satisfaction,
                    parts.get(1),
                    &mut applied,
                    "job_satisfaction",
                );
                state.job.satisfaction = clamp_metric(state.job.satisfaction);
            }
            "job_stability" => {
                apply_delta_at(
                    &mut state.job.stability,
                    parts.get(1),
                    &mut applied,
                    "job_stability",
                );
                state.job.stability = clamp_metric(state.job.stability);
            }
            "contact" => {
                if let Some(name) = parts.get(1).map(|value| value.trim()) {
                    if !name.is_empty() {
                        let level = parts
                            .get(2)
                            .and_then(|value| parse_relationship_level(value.trim()))
                            .unwrap_or(RelationshipLevel::Acquaintance);
                        state.upsert_contact(name, level, &mut applied);
                    }
                }
            }
            "relationship" => {
                if let (Some(name), Some(delta_raw)) = (parts.get(1), parts.get(2)) {
                    if let Ok(delta) = delta_raw.trim().parse::<i32>() {
                        state.adjust_relationship(name.trim(), delta, &mut applied);
                    }
                }
            }
            "relationship_level" => {
                if let (Some(name), Some(level_raw)) = (parts.get(1), parts.get(2)) {
                    if let Some(level) = parse_relationship_level(level_raw.trim()) {
                        state.set_relationship_level(name.trim(), level, &mut applied);
                    }
                }
            }
            "contact_influence" => {
                if let (Some(name), Some(delta_raw)) = (parts.get(1), parts.get(2)) {
                    if let Ok(delta) = delta_raw.trim().parse::<i32>() {
                        state.adjust_contact_influence(name.trim(), delta, &mut applied);
                    }
                }
            }
            _ => {}
        }
    }
    applied
}

fn queue_event(state: &mut CivilianState, storylet_id: &str, created_tick: u64) {
    if state
        .pending_events
        .iter()
        .any(|event| event.storylet_id == storylet_id)
    {
        return;
    }
    state.pending_events.push(CivilianEvent {
        storylet_id: storylet_id.to_string(),
        created_tick,
    });
}

fn apply_delta_at(target: &mut i32, value: Option<&&str>, applied: &mut Vec<String>, label: &str) {
    let Some(value) = value else {
        return;
    };
    if let Ok(delta) = value.trim().parse::<i32>() {
        *target += delta;
        applied.push(format!("{} {:+}", label, delta));
    }
}

fn apply_money_delta(
    target: &mut Money,
    value: Option<&&str>,
    applied: &mut Vec<String>,
    label: &str,
) {
    let Some(value) = value else {
        return;
    };
    if let Ok(delta) = value.trim().parse::<i64>() {
        *target = target.add(Money::from_dollars(delta));
        applied.push(format!("{} {:+}", label, delta));
    }
}

fn parse_job_status(value: &str) -> Option<JobStatus> {
    match value.to_ascii_lowercase().as_str() {
        "employed" => Some(JobStatus::Employed),
        "part_time" => Some(JobStatus::PartTime),
        "unemployed" => Some(JobStatus::Unemployed),
        _ => None,
    }
}

fn parse_job_role(value: &str) -> Option<JobRole> {
    match value.to_ascii_lowercase().as_str() {
        "lawyer" => Some(JobRole::Lawyer),
        "journalist" => Some(JobRole::Journalist),
        "chef" => Some(JobRole::Chef),
        "photographer" => Some(JobRole::Photographer),
        "scientist" => Some(JobRole::Scientist),
        "artist" => Some(JobRole::Artist),
        "engineer" => Some(JobRole::Engineer),
        "nurse" => Some(JobRole::Nurse),
        "teacher" => Some(JobRole::Teacher),
        "mechanic" => Some(JobRole::Mechanic),
        "analyst" => Some(JobRole::Analyst),
        "contractor" => Some(JobRole::Contractor),
        _ => None,
    }
}

fn parse_relationship_level(value: &str) -> Option<RelationshipLevel> {
    match value.to_ascii_lowercase().as_str() {
        "stranger" => Some(RelationshipLevel::Stranger),
        "acquaintance" => Some(RelationshipLevel::Acquaintance),
        "friend" => Some(RelationshipLevel::Friend),
        "confidant" => Some(RelationshipLevel::Confidant),
        "ally" => Some(RelationshipLevel::Ally),
        _ => None,
    }
}

fn clamp_metric(value: i32) -> i32 {
    value.clamp(0, 100)
}

impl CivilianState {
    fn upsert_contact(&mut self, name: &str, level: RelationshipLevel, applied: &mut Vec<String>) {
        if let Some(contact) = self.contacts.iter_mut().find(|entry| entry.name == name) {
            contact.level = level;
            applied.push(format!("contact {} -> {:?}", name, level));
            return;
        }
        let bond = match level {
            RelationshipLevel::Stranger => 15,
            RelationshipLevel::Acquaintance => 35,
            RelationshipLevel::Friend => 55,
            RelationshipLevel::Confidant => 70,
            RelationshipLevel::Ally => 85,
        };
        self.contacts.push(Contact {
            name: name.to_string(),
            level,
            bond,
            influence: 10,
        });
        applied.push(format!("contact added {} ({:?})", name, level));
    }

    fn adjust_relationship(&mut self, name: &str, delta: i32, applied: &mut Vec<String>) {
        let contact = match self.contacts.iter_mut().find(|entry| entry.name == name) {
            Some(contact) => contact,
            None => {
                self.upsert_contact(name, RelationshipLevel::Acquaintance, applied);
                self.contacts
                    .iter_mut()
                    .find(|entry| entry.name == name)
                    .expect("contact inserted")
            }
        };
        contact.bond = clamp_metric(contact.bond + delta);
        contact.level = relationship_level_from_bond(contact.bond);
        applied.push(format!("relationship {} {:+}", name, delta));
    }

    fn set_relationship_level(
        &mut self,
        name: &str,
        level: RelationshipLevel,
        applied: &mut Vec<String>,
    ) {
        let contact = match self.contacts.iter_mut().find(|entry| entry.name == name) {
            Some(contact) => contact,
            None => {
                self.upsert_contact(name, level, applied);
                return;
            }
        };
        contact.level = level;
        contact.bond = match level {
            RelationshipLevel::Stranger => 15,
            RelationshipLevel::Acquaintance => 35,
            RelationshipLevel::Friend => 55,
            RelationshipLevel::Confidant => 70,
            RelationshipLevel::Ally => 85,
        };
        applied.push(format!("relationship level {} -> {:?}", name, level));
    }

    fn adjust_contact_influence(&mut self, name: &str, delta: i32, applied: &mut Vec<String>) {
        let contact = match self.contacts.iter_mut().find(|entry| entry.name == name) {
            Some(contact) => contact,
            None => {
                self.upsert_contact(name, RelationshipLevel::Acquaintance, applied);
                self.contacts
                    .iter_mut()
                    .find(|entry| entry.name == name)
                    .expect("contact inserted")
            }
        };
        contact.influence = clamp_metric(contact.influence + delta);
        applied.push(format!("contact influence {} {:+}", name, delta));
    }
}

fn relationship_level_from_bond(bond: i32) -> RelationshipLevel {
    match bond {
        0..=24 => RelationshipLevel::Stranger,
        25..=44 => RelationshipLevel::Acquaintance,
        45..=64 => RelationshipLevel::Friend,
        65..=79 => RelationshipLevel::Confidant,
        _ => RelationshipLevel::Ally,
    }
}
