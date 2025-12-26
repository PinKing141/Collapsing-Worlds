use bevy_ecs::prelude::*;

use crate::simulation::economy::{
    clamp_liquidity, default_liquidity_for_tier, lifestyle_upkeep, EconomyTickResult, Wealth,
    WealthProfile,
};
use crate::simulation::time::GameTime;

#[derive(Resource, Debug, Clone)]
pub struct CivilianState {
    pub job_status: JobStatus,
    pub job: CivilianJob,
    pub finances: CivilianFinances,
    pub wealth: Wealth,
    pub wealth_profile: WealthProfile,
    pub social: SocialTies,
    pub reputation: ReputationTrack,
    pub rewards: CivilianRewards,
    pub network_rewards: CivilianRewards,
    pub social_web: SocialWeb,
    pub civilian_tier: CivilianTier,
    pub career_xp: i32,
    pub last_promotion_day: u32,
    pub contacts: Vec<Contact>,
    pub pending_events: Vec<CivilianEvent>,
    pub last_day: u32,
    pub last_work_day: u32,
    pub last_relationship_day: u32,
    pub last_economy_day: u32,
    pub last_job_offer_day: u32,
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
    Doctor,
    Firefighter,
    PoliceOfficer,
    Electrician,
    SoftwareDeveloper,
    Accountant,
    Pharmacist,
    SocialWorker,
    Architect,
    Pilot,
    Dentist,
    Paramedic,
    Plumber,
    RetailManager,
    Farmer,
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

#[derive(Debug, Clone, Default)]
pub struct CivilianRewards {
    pub income_boost: i32,
    pub safehouse: i32,
    pub access: i32,
    pub intel: i32,
    pub favors: i32,
}

impl CivilianRewards {
    pub fn combined(&self, other: &CivilianRewards) -> CivilianRewards {
        CivilianRewards {
            income_boost: clamp_metric(self.income_boost + other.income_boost),
            safehouse: clamp_metric(self.safehouse + other.safehouse),
            access: clamp_metric(self.access + other.access),
            intel: clamp_metric(self.intel + other.intel),
            favors: clamp_metric(self.favors + other.favors),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.income_boost == 0
            && self.safehouse == 0
            && self.access == 0
            && self.intel == 0
            && self.favors == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactDomain {
    Professional,
    Community,
    Media,
    Underground,
}

#[derive(Debug, Clone, Default)]
pub struct SocialWeb {
    pub professional: i32,
    pub community: i32,
    pub media: i32,
    pub underground: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CivilianTier {
    Local,
    Connected,
    Influential,
    PowerBroker,
}

impl Default for CivilianTier {
    fn default() -> Self {
        CivilianTier::Local
    }
}

impl Default for ContactDomain {
    fn default() -> Self {
        ContactDomain::Community
    }
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
    pub domain: ContactDomain,
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

const CAREER_XP_CAP: i32 = 250;
const PROMOTION_COOLDOWN_DAYS: u32 = 5;
const MAX_JOB_LEVEL: i32 = 6;
const PART_TIME_LEVEL_CAP: i32 = 4;
const INCOME_BOOST_CR: i64 = 25;
const JOB_OFFER_COOLDOWN_DAYS: u32 = 14;
const JOB_OFFER_UNEMPLOYED_COOLDOWN_DAYS: u32 = 7;

impl Default for CivilianState {
    fn default() -> Self {
        let cash = 120;
        let wage = career_wage(JobRole::Journalist, 1, JobStatus::Employed);
        let mut wealth = Wealth::new(cash as i64);
        wealth.income_per_tick = wage as i64;
        wealth.upkeep_per_tick = lifestyle_upkeep(wealth.tier);
        wealth.liquidity = clamp_liquidity(wealth.liquidity);
        Self {
            job_status: JobStatus::Employed,
            job: CivilianJob {
                role: JobRole::Journalist,
                level: 1,
                satisfaction: 52,
                stability: 48,
            },
            finances: CivilianFinances {
                cash,
                debt: 0,
                rent: 80,
                rent_due_in: 5,
                wage,
            },
            wealth,
            wealth_profile: WealthProfile::Balanced,
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
            rewards: CivilianRewards::default(),
            network_rewards: CivilianRewards::default(),
            social_web: SocialWeb::default(),
            civilian_tier: CivilianTier::default(),
            career_xp: 0,
            last_promotion_day: 0,
            contacts: Vec::new(),
            pending_events: Vec::new(),
            last_day: 0,
            last_work_day: 0,
            last_relationship_day: 0,
            last_economy_day: 0,
            last_job_offer_day: 0,
        }
    }
}

impl CivilianState {
    pub fn effective_rewards(&self) -> CivilianRewards {
        self.rewards.combined(&self.network_rewards)
    }

    pub fn net_worth_cr(&self) -> i64 {
        self.wealth.net_worth(self.finances.debt as i64)
    }

    pub fn career_progress(&self) -> (i32, i32) {
        if matches!(self.job_status, JobStatus::Unemployed) {
            return (0, 0);
        }
        (self.career_xp, promotion_threshold(self.job.level))
    }

    pub fn pressure_targets(&self) -> CivilianPressure {
        let rewards = self.effective_rewards();
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

        let mut temporal = (base_time + rent_urgency + self.social.obligation as f32 * 0.6)
            .clamp(0.0, 100.0);
        if rewards.access > 0 {
            temporal -= rewards.access as f32 * 0.4;
        }
        if rewards.intel > 0 {
            temporal -= rewards.intel as f32 * 0.3;
        }
        temporal = temporal.clamp(0.0, 100.0);

        let mut resource = (self.finances.debt.max(0) as f32 * 0.8).clamp(0.0, 60.0);
        if self.finances.cash < self.finances.rent {
            resource += 18.0;
        }
        if self.finances.rent_due_in <= 2 {
            resource += 12.0;
        }
        if rewards.income_boost > 0 {
            resource -= rewards.income_boost as f32 * 0.6;
        }
        if rewards.favors > 0 {
            resource -= rewards.favors as f32 * 0.5;
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
        if rewards.safehouse > 0 {
            identity -= rewards.safehouse as f32 * 6.0;
        }
        if rewards.access > 0 {
            identity -= rewards.access as f32 * 2.5;
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
        update_social_web(state);
        update_civilian_tier(state);
        update_network_rewards(state);
        if should_queue_job_offer(state, time.day) {
            queue_event(state, "civilian_job_offer", time.tick);
            state.last_job_offer_day = time.day;
        }
    }

    if matches!(state.job_status, JobStatus::Employed | JobStatus::PartTime)
        && time.hour == 9
        && state.last_work_day != time.day
    {
        state.last_work_day = time.day;
        queue_event(state, "civilian_work_shift", time.tick);
        record_work_shift(state);
        apply_career_progression(state, time.day);
    }

    if time.hour == 19 && state.last_relationship_day != time.day {
        state.last_relationship_day = time.day;
        if state.social.support < 60 || state.social.strain > 15 {
            queue_event(state, "civilian_relationship_checkin", time.tick);
        }
    }
}

pub fn tick_civilian_economy(
    state: &mut CivilianState,
    time: &GameTime,
) -> Option<EconomyTickResult> {
    if time.day == state.last_economy_day {
        return None;
    }
    state.last_economy_day = time.day;
    refresh_wealth_tier(state);
    update_wealth_profile(state);
    let result = state.wealth.apply_tick(state.finances.debt as i64);
    sync_finances_from_wealth(state);
    Some(result)
}

pub fn apply_civilian_effects(state: &mut CivilianState, effects: &[String]) -> Vec<String> {
    let mut applied = Vec::new();
    let mut career_changed = false;
    let mut job_status_changed = false;
    let mut wage_override = false;
    let mut cash_changed = false;
    let mut debt_changed = false;
    let mut wealth_changed = false;
    for effect in effects {
        let parts: Vec<&str> = effect.split(':').collect();
        if parts.is_empty() {
            continue;
        }
        let key = parts[0].trim();
        match key {
            "cash" => {
                apply_delta_at(&mut state.finances.cash, parts.get(1), &mut applied, "cash");
                cash_changed = true;
            }
            "debt" => {
                apply_delta_at(&mut state.finances.debt, parts.get(1), &mut applied, "debt");
                debt_changed = true;
            }
            "rent_due_in" => apply_delta_at(
                &mut state.finances.rent_due_in,
                parts.get(1),
                &mut applied,
                "rent_due_in",
            ),
            "wage" => {
                wage_override = true;
                apply_delta_at(&mut state.finances.wage, parts.get(1), &mut applied, "wage");
            }
            "wealth" | "cr" => {
                apply_delta_at_i64(
                    &mut state.wealth.current_cr,
                    parts.get(1),
                    &mut applied,
                    "wealth",
                );
                wealth_changed = true;
            }
            "liquidity" => {
                apply_delta_at_f32(
                    &mut state.wealth.liquidity,
                    parts.get(1),
                    &mut applied,
                    "liquidity",
                );
                state.wealth.liquidity = clamp_liquidity(state.wealth.liquidity);
            }
            "wealth_profile" => {
                if let Some(profile) = parts.get(1).and_then(|value| parse_wealth_profile(value)) {
                    state.wealth_profile = profile;
                    applied.push(format!("wealth profile -> {}", profile.label()));
                }
            }
            "support" => {
                apply_delta_at(&mut state.social.support, parts.get(1), &mut applied, "support");
                state.social.support = clamp_metric(state.social.support);
            }
            "strain" => {
                apply_delta_at(&mut state.social.strain, parts.get(1), &mut applied, "strain");
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
            "intel" => {
                apply_delta_at(
                    &mut state.rewards.intel,
                    parts.get(1),
                    &mut applied,
                    "intel",
                );
                state.rewards.intel = clamp_metric(state.rewards.intel);
            }
            "favors" => {
                apply_delta_at(
                    &mut state.rewards.favors,
                    parts.get(1),
                    &mut applied,
                    "favors",
                );
                state.rewards.favors = clamp_metric(state.rewards.favors);
            }
            "job" => {
                if let Some(job) = parts.get(1).and_then(|value| parse_job_status(value.trim())) {
                    state.job_status = job;
                    applied.push(format!("job -> {:?}", job));
                    career_changed = true;
                    job_status_changed = true;
                }
            }
            "job_role" => {
                if let Some(role) = parts.get(1).and_then(|value| parse_job_role(value.trim())) {
                    state.job.role = role;
                    applied.push(format!("job role -> {:?}", role));
                    career_changed = true;
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
                career_changed = true;
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
            "career_xp" => {
                apply_delta_at(
                    &mut state.career_xp,
                    parts.get(1),
                    &mut applied,
                    "career_xp",
                );
                state.career_xp = clamp_progress(state.career_xp);
            }
            "contact" => {
                if let Some(name) = parts.get(1).map(|value| value.trim()) {
                    if !name.is_empty() {
                        let mut level = RelationshipLevel::Acquaintance;
                        let mut domain = None;
                        if let Some(value) = parts.get(2).map(|value| value.trim()) {
                            if let Some(parsed) = parse_relationship_level(value) {
                                level = parsed;
                            } else if let Some(parsed) = parse_contact_domain(value) {
                                domain = Some(parsed);
                            }
                        }
                        if domain.is_none() {
                            domain = parts
                                .get(3)
                                .map(|value| value.trim())
                                .and_then(parse_contact_domain);
                        }
                        state.upsert_contact(name, level, domain, &mut applied);
                    }
                }
            }
            "contact_domain" => {
                if let (Some(name), Some(domain_raw)) = (parts.get(1), parts.get(2)) {
                    if let Some(domain) = parse_contact_domain(domain_raw.trim()) {
                        state.set_contact_domain(name.trim(), domain, &mut applied);
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
    if career_changed && !wage_override {
        sync_career_compensation(state);
    }
    if matches!(state.job_status, JobStatus::Unemployed) {
        state.finances.wage = 0;
    }
    if career_changed
        && !job_status_changed
        && matches!(state.job_status, JobStatus::Unemployed)
    {
        state.job_status = JobStatus::Employed;
        applied.push("job -> Employed".to_string());
        sync_career_compensation(state);
    }
    if wealth_changed {
        sync_finances_from_wealth(state);
        refresh_wealth_tier(state);
    } else if cash_changed {
        sync_wealth_from_finances(state);
    } else if debt_changed {
        refresh_wealth_tier(state);
    }
    apply_career_progression(state, state.last_day);
    rebuild_social_web(state);
    update_civilian_tier(state);
    update_network_rewards(state);
    update_wealth_profile(state);
    applied
}

fn record_work_shift(state: &mut CivilianState) {
    let base_xp = match state.job_status {
        JobStatus::Employed => 4,
        JobStatus::PartTime => 2,
        JobStatus::Unemployed => 0,
    };
    if base_xp == 0 {
        return;
    }

    let satisfaction_bonus = ((state.job.satisfaction - 50) / 15).clamp(-2, 3);
    let stability_bonus = ((state.job.stability - 50) / 20).clamp(-1, 2);
    let support_bonus = if state.social.support > 60 { 1 } else { 0 };
    let strain_penalty = if state.social.strain > 60 { -2 } else { 0 };
    let xp_gain = (base_xp + satisfaction_bonus + stability_bonus + support_bonus + strain_penalty)
        .max(1);

    state.career_xp = clamp_progress(state.career_xp + xp_gain);

    let mut satisfaction_delta = 0;
    if state.social.support > 55 {
        satisfaction_delta += 1;
    }
    if state.social.strain > 55 {
        satisfaction_delta -= 2;
    }
    if matches!(state.job_status, JobStatus::PartTime) {
        satisfaction_delta += 1;
    }
    state.job.satisfaction = clamp_metric(state.job.satisfaction + satisfaction_delta);

    let mut stability_delta = 0;
    if state.job.satisfaction > 60 {
        stability_delta += 1;
    }
    if state.social.strain > 65 {
        stability_delta -= 1;
    }
    state.job.stability = clamp_metric(state.job.stability + stability_delta);
}

fn apply_career_progression(state: &mut CivilianState, day: u32) {
    if matches!(state.job_status, JobStatus::Unemployed) {
        return;
    }
    let cap = max_job_level(state.job_status);
    if state.job.level >= cap {
        return;
    }
    if day < state.last_promotion_day.saturating_add(PROMOTION_COOLDOWN_DAYS) {
        return;
    }
    let threshold = promotion_threshold(state.job.level);
    if state.career_xp < threshold {
        return;
    }

    state.career_xp = clamp_progress(state.career_xp - threshold);
    state.job.level += 1;
    state.last_promotion_day = day;
    state.job.satisfaction = clamp_metric(state.job.satisfaction + 4);
    state.job.stability = clamp_metric(state.job.stability + 3);
    state.reputation.career = clamp_metric(
        state.reputation.career + career_model(state.job.role).reputation_step,
    );
    apply_promotion_rewards(state);
    sync_career_compensation(state);
}

fn apply_promotion_rewards(state: &mut CivilianState) {
    state.rewards.income_boost = clamp_metric(state.rewards.income_boost + 1);
    if state.job.level >= 3 {
        state.rewards.access = clamp_metric(state.rewards.access + 1);
    }
    if state.job.level >= 4 {
        state.rewards.intel = clamp_metric(state.rewards.intel + 1);
    }
    if state.job.level >= 5 {
        state.rewards.favors = clamp_metric(state.rewards.favors + 1);
    }
}

fn update_social_web(state: &mut CivilianState) {
    let bond_delta = daily_bond_delta(state);
    let reputation = state.reputation.clone();
    let job = state.job.clone();
    let social = state.social.clone();
    let finances = state.finances.clone();
    for contact in &mut state.contacts {
        if bond_delta != 0 {
            contact.bond = clamp_metric(contact.bond + bond_delta);
            contact.level = relationship_level_from_bond(contact.bond);
        }
        let target = contact_influence_target(contact, &reputation, &job, &social, &finances);
        if contact.influence < target {
            contact.influence = clamp_metric(contact.influence + 1);
        } else if contact.influence > target {
            contact.influence = clamp_metric(contact.influence - 1);
        }
    }
    rebuild_social_web(state);
}

fn rebuild_social_web(state: &mut CivilianState) {
    let mut professional = 0;
    let mut community = 0;
    let mut media = 0;
    let mut underground = 0;

    for contact in &state.contacts {
        let weight = contact.influence + contact.bond / 2;
        match contact.domain {
            ContactDomain::Professional => professional += weight,
            ContactDomain::Community => community += weight,
            ContactDomain::Media => media += weight,
            ContactDomain::Underground => underground += weight,
        }
    }

    professional += state.reputation.career * 2;
    community += state.reputation.community * 2;
    media += state.reputation.media * 2;
    underground += (state.social.obligation / 2).clamp(0, 50);

    state.social_web.professional = normalize_web_score(professional);
    state.social_web.community = normalize_web_score(community);
    state.social_web.media = normalize_web_score(media);
    state.social_web.underground = normalize_web_score(underground);
}

fn update_civilian_tier(state: &mut CivilianState) {
    let base = (state.social_web.professional
        + state.social_web.community
        + state.social_web.media
        + state.social_web.underground)
        / 4;
    let contact_bonus = state.contacts.len().min(12) as i32 * 3;
    let career_bonus = state.job.level * 4;
    let total = base + contact_bonus + career_bonus;

    state.civilian_tier = if total >= 110 {
        CivilianTier::PowerBroker
    } else if total >= 85 {
        CivilianTier::Influential
    } else if total >= 60 {
        CivilianTier::Connected
    } else {
        CivilianTier::Local
    };
}

fn update_network_rewards(state: &mut CivilianState) {
    let mut rewards = CivilianRewards::default();
    rewards.income_boost = clamp_metric(state.social_web.professional / 18);
    rewards.access = clamp_metric(state.social_web.community / 16);
    rewards.safehouse = clamp_metric(state.social_web.underground / 16);
    rewards.intel = clamp_metric(state.social_web.media / 18);
    rewards.favors = clamp_metric(
        (state.social_web.community + state.social_web.underground) / 32,
    );

    match state.civilian_tier {
        CivilianTier::Connected => {
            rewards.favors = clamp_metric(rewards.favors + 1);
        }
        CivilianTier::Influential => {
            rewards.favors = clamp_metric(rewards.favors + 2);
            rewards.access = clamp_metric(rewards.access + 1);
        }
        CivilianTier::PowerBroker => {
            rewards.favors = clamp_metric(rewards.favors + 3);
            rewards.access = clamp_metric(rewards.access + 2);
            rewards.intel = clamp_metric(rewards.intel + 2);
        }
        CivilianTier::Local => {}
    }

    state.network_rewards = rewards;
}

fn should_queue_job_offer(state: &CivilianState, day: u32) -> bool {
    let cooldown = if matches!(state.job_status, JobStatus::Unemployed) {
        JOB_OFFER_UNEMPLOYED_COOLDOWN_DAYS
    } else {
        JOB_OFFER_COOLDOWN_DAYS
    };
    if day < state.last_job_offer_day.saturating_add(cooldown) {
        return false;
    }
    if matches!(state.job_status, JobStatus::Unemployed) {
        return true;
    }
    state.job.satisfaction < 55 || state.job.stability < 45
}

fn refresh_wealth_tier(state: &mut CivilianState) {
    state.wealth.refresh_tier(state.finances.debt as i64);
}

fn update_wealth_profile(state: &mut CivilianState) {
    let rewards = state.effective_rewards();
    let bonus_income = rewards.income_boost as i64 * INCOME_BOOST_CR;
    let base_income = state.finances.wage as i64 + bonus_income;
    let modifiers = state.wealth_profile.modifiers();
    state.wealth.income_per_tick = ((base_income as f32) * modifiers.income_scale).round() as i64;
    let base_upkeep = lifestyle_upkeep(state.wealth.tier);
    state.wealth.upkeep_per_tick =
        ((base_upkeep as f32) * modifiers.upkeep_scale).round() as i64;
    let base_liquidity = default_liquidity_for_tier(state.wealth.tier);
    let liquidity = clamp_liquidity(base_liquidity + modifiers.liquidity_bonus);
    state.wealth.liquidity = liquidity.min(clamp_liquidity(modifiers.liquidity_cap));
}

fn sync_finances_from_wealth(state: &mut CivilianState) {
    state.finances.cash = clamp_i64_to_i32(state.wealth.current_cr);
}

fn sync_wealth_from_finances(state: &mut CivilianState) {
    state.wealth.current_cr = state.finances.cash as i64;
    refresh_wealth_tier(state);
}

fn clamp_i64_to_i32(value: i64) -> i32 {
    value.clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

fn daily_bond_delta(state: &CivilianState) -> i32 {
    let mut delta = 0;
    if state.social.support > 60 {
        delta += 1;
    }
    if state.social.strain > 60 {
        delta -= 1;
    }
    if state.social.obligation > 65 {
        delta -= 1;
    }
    delta
}

fn contact_influence_target(
    contact: &Contact,
    reputation: &ReputationTrack,
    job: &CivilianJob,
    social: &SocialTies,
    finances: &CivilianFinances,
) -> i32 {
    let level_bonus = match contact.level {
        RelationshipLevel::Stranger => 2,
        RelationshipLevel::Acquaintance => 4,
        RelationshipLevel::Friend => 6,
        RelationshipLevel::Confidant => 8,
        RelationshipLevel::Ally => 10,
    };
    let mut target = contact.bond / 10 + level_bonus;
    match contact.domain {
        ContactDomain::Professional => {
            target += reputation.career / 12;
            target += job.level * 2;
        }
        ContactDomain::Community => {
            target += reputation.community / 12;
            target += social.support / 15;
        }
        ContactDomain::Media => {
            target += reputation.media / 12;
            target += social.strain / 20;
        }
        ContactDomain::Underground => {
            target += social.obligation / 12;
            target += finances.debt.max(0) / 20;
        }
    }
    clamp_metric(target)
}

fn normalize_web_score(score: i32) -> i32 {
    (score / 4).clamp(0, 100)
}

fn promotion_threshold(level: i32) -> i32 {
    let level = level.max(1);
    12 + level * 8
}

fn max_job_level(status: JobStatus) -> i32 {
    match status {
        JobStatus::Employed => MAX_JOB_LEVEL,
        JobStatus::PartTime => PART_TIME_LEVEL_CAP,
        JobStatus::Unemployed => 1,
    }
}

fn sync_career_compensation(state: &mut CivilianState) {
    let cap = max_job_level(state.job_status);
    state.job.level = state.job.level.clamp(0, cap);
    state.finances.wage = career_wage(state.job.role, state.job.level, state.job_status);
    update_wealth_profile(state);
}

fn career_wage(role: JobRole, level: i32, status: JobStatus) -> i32 {
    let model = career_model(role);
    let level = level.max(1);
    let mut wage = model.base_wage + model.wage_step * (level - 1);
    if matches!(status, JobStatus::PartTime) {
        wage = ((wage as f32) * 0.7).round() as i32;
    }
    if matches!(status, JobStatus::Unemployed) {
        wage = 0;
    }
    wage.max(0)
}

struct CareerModel {
    base_wage: i32,
    wage_step: i32,
    reputation_step: i32,
}

fn career_model(role: JobRole) -> CareerModel {
    match role {
        JobRole::Lawyer => CareerModel {
            base_wage: 180,
            wage_step: 70,
            reputation_step: 4,
        },
        JobRole::Journalist => CareerModel {
            base_wage: 70,
            wage_step: 22,
            reputation_step: 3,
        },
        JobRole::Chef => CareerModel {
            base_wage: 50,
            wage_step: 15,
            reputation_step: 2,
        },
        JobRole::Photographer => CareerModel {
            base_wage: 60,
            wage_step: 18,
            reputation_step: 2,
        },
        JobRole::Scientist => CareerModel {
            base_wage: 160,
            wage_step: 60,
            reputation_step: 4,
        },
        JobRole::Artist => CareerModel {
            base_wage: 45,
            wage_step: 14,
            reputation_step: 3,
        },
        JobRole::Engineer => CareerModel {
            base_wage: 140,
            wage_step: 55,
            reputation_step: 3,
        },
        JobRole::Nurse => CareerModel {
            base_wage: 80,
            wage_step: 24,
            reputation_step: 3,
        },
        JobRole::Teacher => CareerModel {
            base_wage: 75,
            wage_step: 22,
            reputation_step: 3,
        },
        JobRole::Mechanic => CareerModel {
            base_wage: 65,
            wage_step: 20,
            reputation_step: 2,
        },
        JobRole::Analyst => CareerModel {
            base_wage: 120,
            wage_step: 45,
            reputation_step: 3,
        },
        JobRole::Contractor => CareerModel {
            base_wage: 110,
            wage_step: 40,
            reputation_step: 2,
        },
        JobRole::Doctor => CareerModel {
            base_wage: 200,
            wage_step: 75,
            reputation_step: 4,
        },
        JobRole::Firefighter => CareerModel {
            base_wage: 85,
            wage_step: 26,
            reputation_step: 3,
        },
        JobRole::PoliceOfficer => CareerModel {
            base_wage: 90,
            wage_step: 28,
            reputation_step: 3,
        },
        JobRole::Electrician => CareerModel {
            base_wage: 95,
            wage_step: 30,
            reputation_step: 2,
        },
        JobRole::SoftwareDeveloper => CareerModel {
            base_wage: 150,
            wage_step: 55,
            reputation_step: 3,
        },
        JobRole::Accountant => CareerModel {
            base_wage: 115,
            wage_step: 38,
            reputation_step: 3,
        },
        JobRole::Pharmacist => CareerModel {
            base_wage: 140,
            wage_step: 45,
            reputation_step: 3,
        },
        JobRole::SocialWorker => CareerModel {
            base_wage: 70,
            wage_step: 22,
            reputation_step: 3,
        },
        JobRole::Architect => CareerModel {
            base_wage: 135,
            wage_step: 50,
            reputation_step: 3,
        },
        JobRole::Pilot => CareerModel {
            base_wage: 170,
            wage_step: 65,
            reputation_step: 4,
        },
        JobRole::Dentist => CareerModel {
            base_wage: 160,
            wage_step: 55,
            reputation_step: 4,
        },
        JobRole::Paramedic => CareerModel {
            base_wage: 80,
            wage_step: 24,
            reputation_step: 3,
        },
        JobRole::Plumber => CareerModel {
            base_wage: 85,
            wage_step: 26,
            reputation_step: 2,
        },
        JobRole::RetailManager => CareerModel {
            base_wage: 75,
            wage_step: 24,
            reputation_step: 2,
        },
        JobRole::Farmer => CareerModel {
            base_wage: 60,
            wage_step: 20,
            reputation_step: 2,
        },
    }
}

fn clamp_progress(value: i32) -> i32 {
    value.clamp(0, CAREER_XP_CAP)
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

fn apply_delta_at(
    target: &mut i32,
    value: Option<&&str>,
    applied: &mut Vec<String>,
    label: &str,
) {
    let Some(value) = value else {
        return;
    };
    if let Ok(delta) = value.trim().parse::<i32>() {
        *target += delta;
        applied.push(format!("{} {:+}", label, delta));
    }
}

fn apply_delta_at_i64(
    target: &mut i64,
    value: Option<&&str>,
    applied: &mut Vec<String>,
    label: &str,
) {
    let Some(value) = value else {
        return;
    };
    if let Ok(delta) = value.trim().parse::<i64>() {
        *target += delta;
        applied.push(format!("{} {:+}", label, delta));
    }
}

fn apply_delta_at_f32(
    target: &mut f32,
    value: Option<&&str>,
    applied: &mut Vec<String>,
    label: &str,
) {
    let Some(value) = value else {
        return;
    };
    if let Ok(delta) = value.trim().parse::<f32>() {
        *target += delta;
        applied.push(format!("{} {:+.2}", label, delta));
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
        "doctor" => Some(JobRole::Doctor),
        "firefighter" => Some(JobRole::Firefighter),
        "police_officer" => Some(JobRole::PoliceOfficer),
        "electrician" => Some(JobRole::Electrician),
        "software_developer" => Some(JobRole::SoftwareDeveloper),
        "accountant" => Some(JobRole::Accountant),
        "pharmacist" => Some(JobRole::Pharmacist),
        "social_worker" => Some(JobRole::SocialWorker),
        "architect" => Some(JobRole::Architect),
        "pilot" => Some(JobRole::Pilot),
        "dentist" => Some(JobRole::Dentist),
        "paramedic" => Some(JobRole::Paramedic),
        "plumber" => Some(JobRole::Plumber),
        "retail_manager" => Some(JobRole::RetailManager),
        "farmer" => Some(JobRole::Farmer),
        _ => None,
    }
}

fn parse_wealth_profile(value: &str) -> Option<WealthProfile> {
    match value.to_ascii_lowercase().as_str() {
        "balanced" => Some(WealthProfile::Balanced),
        "vigilante" => Some(WealthProfile::Vigilante),
        "corporate" => Some(WealthProfile::Corporate),
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

fn parse_contact_domain(value: &str) -> Option<ContactDomain> {
    match value.to_ascii_lowercase().as_str() {
        "professional" => Some(ContactDomain::Professional),
        "community" => Some(ContactDomain::Community),
        "media" => Some(ContactDomain::Media),
        "underground" => Some(ContactDomain::Underground),
        _ => None,
    }
}

fn default_contact_domain(role: JobRole) -> ContactDomain {
    match role {
        JobRole::Lawyer
        | JobRole::Scientist
        | JobRole::Engineer
        | JobRole::Analyst
        | JobRole::Contractor
        | JobRole::Doctor
        | JobRole::Electrician
        | JobRole::SoftwareDeveloper
        | JobRole::Accountant
        | JobRole::Pharmacist
        | JobRole::Architect
        | JobRole::Dentist
        | JobRole::Pilot => ContactDomain::Professional,
        JobRole::Journalist | JobRole::Photographer => ContactDomain::Media,
        JobRole::Artist | JobRole::Farmer => ContactDomain::Community,
        JobRole::Chef
        | JobRole::Nurse
        | JobRole::Teacher
        | JobRole::Mechanic
        | JobRole::Firefighter
        | JobRole::PoliceOfficer
        | JobRole::SocialWorker
        | JobRole::Paramedic
        | JobRole::Plumber
        | JobRole::RetailManager => {
            ContactDomain::Community
        }
    }
}

fn clamp_metric(value: i32) -> i32 {
    value.clamp(0, 100)
}

impl CivilianState {
    fn upsert_contact(
        &mut self,
        name: &str,
        level: RelationshipLevel,
        domain: Option<ContactDomain>,
        applied: &mut Vec<String>,
    ) {
        if let Some(contact) = self.contacts.iter_mut().find(|entry| entry.name == name) {
            contact.level = level;
            if let Some(domain) = domain {
                contact.domain = domain;
                applied.push(format!("contact {} domain -> {:?}", name, domain));
            }
            applied.push(format!("contact {} -> {:?}", name, level));
            return;
        }
        let domain = domain.unwrap_or_else(|| default_contact_domain(self.job.role));
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
            domain,
            bond,
            influence: 10,
        });
        applied.push(format!("contact added {} ({:?})", name, level));
    }

    fn adjust_relationship(&mut self, name: &str, delta: i32, applied: &mut Vec<String>) {
        let contact = match self.contacts.iter_mut().find(|entry| entry.name == name) {
            Some(contact) => contact,
            None => {
                self.upsert_contact(name, RelationshipLevel::Acquaintance, None, applied);
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
                self.upsert_contact(name, level, None, applied);
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
                self.upsert_contact(name, RelationshipLevel::Acquaintance, None, applied);
                self.contacts
                    .iter_mut()
                    .find(|entry| entry.name == name)
                    .expect("contact inserted")
            }
        };
        contact.influence = clamp_metric(contact.influence + delta);
        applied.push(format!("contact influence {} {:+}", name, delta));
    }

    fn set_contact_domain(
        &mut self,
        name: &str,
        domain: ContactDomain,
        applied: &mut Vec<String>,
    ) {
        let contact = match self.contacts.iter_mut().find(|entry| entry.name == name) {
            Some(contact) => contact,
            None => {
                self.upsert_contact(name, RelationshipLevel::Acquaintance, Some(domain), applied);
                return;
            }
        };
        contact.domain = domain;
        applied.push(format!("contact {} domain -> {:?}", name, domain));
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
