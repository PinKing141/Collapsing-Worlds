use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::simulation::economy::{
    clamp_liquidity, default_liquidity_for_tier, lifestyle_upkeep, EconomyTickResult, Wealth,
    WealthProfile, WealthTier,
};
use crate::simulation::time::GameTime;

#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct CivilianState {
    #[serde(default)]
    pub life: LifeState,
    pub job_status: JobStatus,
    pub job: CivilianJob,
    #[serde(default)]
    pub routine: RoutineSchedule,
    #[serde(default)]
    pub education: EducationTrack,
    #[serde(default)]
    pub health: CivilianHealth,
    #[serde(default)]
    pub mistake_risk: i32,
    #[serde(default)]
    pub pending_death: Option<DeathRecord>,
    #[serde(default)]
    pub legacy: Vec<LegacyRecord>,
    pub finances: CivilianFinances,
    #[serde(default)]
    pub housing: HousingState,
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
    #[serde(default)]
    pub event_settings: CivilianEventSettings,
    #[serde(default)]
    pub auto_choices: AutoChoicePreferences,
    #[serde(default)]
    pub event_history: HashMap<String, u32>,
    pub last_day: u32,
    pub last_work_day: u32,
    pub last_relationship_day: u32,
    #[serde(default)]
    pub last_mistake_day: u32,
    #[serde(default)]
    pub last_reputation_heat_day: u32,
    #[serde(default)]
    pub last_hobby_day: u32,
    #[serde(default)]
    pub last_health_day: u32,
    pub last_economy_day: u32,
    pub last_job_offer_day: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Employed,
    PartTime,
    Unemployed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifeStage {
    Child,
    Teen,
    YoungAdult,
    Adult,
    Mature,
    Elder,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifeState {
    pub age_years: u32,
    pub life_stage: LifeStage,
    #[serde(alias = "last_birthday_day")]
    pub birth_day: u32,
    #[serde(default)]
    pub mortality_risk: i32,
    #[serde(default)]
    pub mutant_gene: bool,
    #[serde(default)]
    pub mutation_ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EducationLevel {
    None,
    Primary,
    Secondary,
    Tertiary,
    Graduate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EducationTrack {
    pub level: EducationLevel,
    pub credits: i32,
    pub attendance: i32,
    pub progress: i32,
    #[serde(default)]
    pub dropout_risk: i32,
    pub is_enrolled: bool,
    pub last_school_day: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutineActivity {
    Work,
    School,
    Hobby,
    Social,
    Rest,
    Errands,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutineBlock {
    pub start_hour: u8,
    pub duration: u8,
    pub activity: RoutineActivity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutineSchedule {
    pub blocks: Vec<RoutineBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilianHealth {
    pub fitness: i32,
    pub sleep_debt: i32,
    pub stress: i32,
    pub injuries: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyRecord {
    pub day: u32,
    pub age_years: u32,
    pub life_stage: LifeStage,
    pub alignment: String,
    pub wealth_tier: WealthTier,
    pub career_role: JobRole,
    pub career_level: i32,
    pub reputation: ReputationTrack,
    pub achievements: Vec<String>,
    pub death_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeathRecord {
    pub day: u32,
    pub age_years: u32,
    pub reason: String,
}

impl LifeState {
    pub fn new(age_years: u32, current_day: u32) -> Self {
        let life_stage = life_stage_for_age(age_years);
        let mutation_ready = false;
        Self {
            age_years,
            life_stage,
            birth_day: current_day,
            mortality_risk: 0,
            mutant_gene: false,
            mutation_ready,
        }
    }
}

impl Default for LifeState {
    fn default() -> Self {
        LifeState::new(DEFAULT_START_AGE, 1)
    }
}

impl Default for HousingState {
    fn default() -> Self {
        let profile = neighborhood_profile("midtown").unwrap_or(&NEIGHBORHOOD_CATALOG[0]);
        Self {
            neighborhood_id: profile.id.to_string(),
            rent: profile.rent,
            stability: profile.stability,
            safety: profile.safety,
            privacy: profile.privacy,
            relocation_cooldown: 0,
        }
    }
}

impl Default for EducationTrack {
    fn default() -> Self {
        Self {
            level: EducationLevel::None,
            credits: 0,
            attendance: 50,
            progress: 0,
            dropout_risk: 0,
            is_enrolled: false,
            last_school_day: 0,
        }
    }
}

impl EducationTrack {
    fn for_age(age_years: u32) -> Self {
        let mut track = EducationTrack::default();
        if age_years < 6 {
            track.level = EducationLevel::None;
            track.is_enrolled = false;
        } else if age_years < 11 {
            track.level = EducationLevel::Primary;
            track.is_enrolled = true;
        } else if age_years < 18 {
            track.level = EducationLevel::Secondary;
            track.is_enrolled = true;
        } else if age_years < 23 {
            track.level = EducationLevel::Tertiary;
            track.is_enrolled = true;
        } else {
            track.level = EducationLevel::Graduate;
            track.is_enrolled = false;
        }
        track
    }
}

impl Default for RoutineSchedule {
    fn default() -> Self {
        Self { blocks: Vec::new() }
    }
}

impl RoutineSchedule {
    fn activity_at(&self, hour: u8) -> RoutineActivity {
        let mut selected = None;
        for block in &self.blocks {
            if block.contains(hour) {
                let priority = block.activity.priority();
                if selected
                    .map(|(_, best)| priority > best)
                    .unwrap_or(true)
                {
                    selected = Some((block.activity, priority));
                }
            }
        }
        selected.map(|(activity, _)| activity).unwrap_or(RoutineActivity::Rest)
    }

    fn first_hour_for(&self, activity: RoutineActivity) -> Option<u8> {
        self.blocks
            .iter()
            .filter(|block| block.activity == activity)
            .map(|block| block.start_hour)
            .min()
    }

    fn push_block(&mut self, start_hour: u8, duration: u8, activity: RoutineActivity) {
        if duration == 0 {
            return;
        }
        self.blocks.push(RoutineBlock {
            start_hour,
            duration,
            activity,
        });
    }
}

impl RoutineBlock {
    fn contains(&self, hour: u8) -> bool {
        let end = self.start_hour.saturating_add(self.duration);
        hour >= self.start_hour && hour < end
    }
}

impl RoutineActivity {
    fn priority(self) -> u8 {
        match self {
            RoutineActivity::Work => 5,
            RoutineActivity::School => 4,
            RoutineActivity::Social => 3,
            RoutineActivity::Hobby => 2,
            RoutineActivity::Errands => 1,
            RoutineActivity::Rest => 0,
        }
    }
}

impl Default for CivilianHealth {
    fn default() -> Self {
        Self {
            fitness: 50,
            sleep_debt: 0,
            stress: 15,
            injuries: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilianJob {
    pub role: JobRole,
    pub level: i32,
    pub satisfaction: i32,
    pub stability: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilianFinances {
    pub cash: i32,
    pub debt: i32,
    pub rent: i32,
    pub rent_due_in: i32,
    pub wage: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HousingState {
    pub neighborhood_id: String,
    pub rent: i32,
    pub stability: i32,
    pub safety: i32,
    pub privacy: i32,
    pub relocation_cooldown: i32,
}

struct NeighborhoodProfile {
    id: &'static str,
    label: &'static str,
    rent: i32,
    stability: i32,
    safety: i32,
    privacy: i32,
}

const NEIGHBORHOOD_CATALOG: [NeighborhoodProfile; 4] = [
    NeighborhoodProfile {
        id: "midtown",
        label: "Midtown",
        rent: 80,
        stability: 62,
        safety: 55,
        privacy: 50,
    },
    NeighborhoodProfile {
        id: "uptown",
        label: "Uptown",
        rent: 120,
        stability: 75,
        safety: 78,
        privacy: 70,
    },
    NeighborhoodProfile {
        id: "edge",
        label: "Edge District",
        rent: 60,
        stability: 55,
        safety: 45,
        privacy: 40,
    },
    NeighborhoodProfile {
        id: "harbor",
        label: "Harbor Ward",
        rent: 70,
        stability: 58,
        safety: 50,
        privacy: 55,
    },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialTies {
    pub support: i32,
    pub strain: i32,
    pub obligation: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationTrack {
    pub career: i32,
    pub community: i32,
    pub media: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactDomain {
    Professional,
    Community,
    Media,
    Underground,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SocialWeb {
    pub professional: i32,
    pub community: i32,
    pub media: i32,
    pub underground: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

impl Default for RelationType {
    fn default() -> Self {
        RelationType::Peer
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipLevel {
    Stranger,
    Acquaintance,
    Friend,
    Confidant,
    Ally,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    Family,
    Mentor,
    Rival,
    Romance,
    Colleague,
    Peer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub name: String,
    pub level: RelationshipLevel,
    pub domain: ContactDomain,
    #[serde(default)]
    pub relation_type: RelationType,
    pub bond: i32,
    pub influence: i32,
    #[serde(default)]
    pub last_interaction_day: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilianEvent {
    pub storylet_id: String,
    pub created_tick: u64,
    #[serde(default)]
    pub contact_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CivilianEventCategory {
    Work,
    School,
    Health,
    Housing,
    Social,
    Opportunity,
    Crime,
    Routine,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilianEventSettings {
    #[serde(default)]
    pub suppress_repeat_days: u32,
    #[serde(default)]
    pub min_effect_magnitude: i32,
    #[serde(default)]
    pub muted_categories: Vec<CivilianEventCategory>,
}

impl Default for CivilianEventSettings {
    fn default() -> Self {
        Self {
            suppress_repeat_days: 0,
            min_effect_magnitude: 0,
            muted_categories: Vec::new(),
        }
    }
}

impl CivilianEventSettings {
    pub fn is_muted(&self, category: CivilianEventCategory) -> bool {
        self.muted_categories.contains(&category)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoChoicePreferences {
    #[serde(default)]
    pub pay_rent_if_cash_at_least: Option<i32>,
    #[serde(default)]
    pub attend_school: bool,
    #[serde(default)]
    pub rest_if_sleep_debt_at_least: Option<i32>,
}

impl Default for AutoChoicePreferences {
    fn default() -> Self {
        Self {
            pay_rent_if_cash_at_least: None,
            attend_school: false,
            rest_if_sleep_debt_at_least: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
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
const DAYS_PER_YEAR: u32 = 336;
const DEFAULT_START_AGE: u32 = 16;

impl Default for CivilianState {
    fn default() -> Self {
        let cash = 120;
        let life = LifeState::new(DEFAULT_START_AGE, 1);
        let education = EducationTrack::for_age(life.age_years);
        let job_status = if education.is_enrolled {
            JobStatus::PartTime
        } else {
            JobStatus::Employed
        };
        let housing = HousingState::default();
        let wage = career_wage(JobRole::Journalist, 1, job_status);
        let mut wealth = Wealth::new(cash as i64);
        wealth.income_per_tick = wage as i64;
        wealth.upkeep_per_tick = lifestyle_upkeep(wealth.tier);
        wealth.liquidity = clamp_liquidity(wealth.liquidity);
        let routine = build_routine_schedule(job_status, &education, &life);
        Self {
            life,
            job_status,
            job: CivilianJob {
                role: JobRole::Journalist,
                level: 1,
                satisfaction: 52,
                stability: 48,
            },
            routine,
            education,
            health: CivilianHealth::default(),
            mistake_risk: 0,
            pending_death: None,
            legacy: Vec::new(),
            finances: CivilianFinances {
                cash,
                debt: 0,
                rent: housing.rent,
                rent_due_in: 5,
                wage,
            },
            housing,
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
            event_settings: CivilianEventSettings::default(),
            auto_choices: AutoChoicePreferences::default(),
            event_history: HashMap::new(),
            last_day: 0,
            last_work_day: 0,
            last_relationship_day: 0,
            last_mistake_day: 0,
            last_reputation_heat_day: 0,
            last_hobby_day: 0,
            last_health_day: 0,
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

    pub fn last_event_seen_day(&self, event_id: &str) -> Option<u32> {
        self.event_history.get(event_id).copied()
    }

    pub fn mark_event_seen(&mut self, event_id: &str, day: u32) {
        self.event_history.insert(event_id.to_string(), day);
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
        if self.housing.stability < 50 {
            temporal += (50 - self.housing.stability) as f32 * 0.4;
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
        if self.housing.safety < 50 {
            moral += (50 - self.housing.safety) as f32 * 0.3;
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
        if self.housing.privacy < 55 {
            identity += (55 - self.housing.privacy) as f32 * 0.4;
        }
        identity = identity.clamp(0.0, 100.0);

        CivilianPressure {
            temporal,
            resource,
            moral,
            identity,
        }
    }

    pub fn tech_access_score(&self) -> i32 {
        let rewards = self.effective_rewards();
        let tier_rank = self.wealth.tier.rank() as i32;
        let base = tier_rank * 12 + rewards.access * 3 + rewards.favors * 2;
        clamp_metric(base)
    }

    pub fn public_reputation_score(&self) -> i32 {
        self.reputation.media
    }

    pub fn social_leverage_score(&self) -> i32 {
        let rewards = self.effective_rewards();
        let base = rewards.favors * 10
            + (self.social_web.professional / 3)
            + (self.social_web.media / 4);
        clamp_metric(base)
    }

    pub fn social_protection_score(&self) -> i32 {
        let rewards = self.effective_rewards();
        let base =
            rewards.safehouse * 12 + (self.social.support / 2) + (self.housing.privacy / 4);
        clamp_metric(base)
    }

    pub fn social_vulnerability_score(&self) -> i32 {
        let rivals = self
            .contacts
            .iter()
            .filter(|contact| contact.relation_type == RelationType::Rival)
            .count() as i32;
        let mut base = (self.social.strain + self.social.obligation) / 2 + rivals * 10;
        if self.housing.stability < 45 {
            base += (45 - self.housing.stability) / 2;
        }
        clamp_metric(base)
    }

    pub fn routine_summary(&self) -> String {
        if self.routine.blocks.is_empty() {
            return "none".to_string();
        }
        let mut blocks: Vec<&RoutineBlock> = self.routine.blocks.iter().collect();
        blocks.sort_by_key(|block| block.start_hour);
        let parts: Vec<String> = blocks
            .iter()
            .map(|block| {
                let end = block.start_hour.saturating_add(block.duration);
                format!(
                    "{:02}-{:02} {:?}",
                    block.start_hour, end, block.activity
                )
            })
            .collect();
        parts.join(", ")
    }

    pub fn record_legacy(
        &mut self,
        alignment_label: &str,
        reason: &str,
        day: u32,
    ) -> LegacyRecord {
        let achievements = legacy_achievements(self);
        let record = LegacyRecord {
            day,
            age_years: self.life.age_years,
            life_stage: self.life.life_stage,
            alignment: alignment_label.to_string(),
            wealth_tier: self.wealth.tier,
            career_role: self.job.role,
            career_level: self.job.level,
            reputation: self.reputation.clone(),
            achievements,
            death_reason: reason.to_string(),
        };
        self.legacy.push(record.clone());
        record
    }
}

pub fn tick_civilian_life(state: &mut CivilianState, time: &GameTime) {
    if time.day != state.last_day {
        state.last_day = time.day;
        update_age_and_life_stage(state, time);
        update_routine_schedule(state);
        apply_age_decline(state, time.day);
        apply_daily_health(state);
        update_mortality_risk(state);
        check_for_death(state, time.day);
        sync_housing_rent(state);
        state.finances.rent_due_in -= 1;
        if state.finances.rent_due_in <= 0 {
            queue_event(state, "civilian_rent_due", time.tick);
        }
        if should_queue_crime_opportunity(state) {
            queue_event(state, "civilian_crime_quick_hit", time.tick);
        }
        update_housing_state(state, time);
        update_social_web(state);
        update_civilian_tier(state);
        update_network_rewards(state);
        update_mistake_risk(state);
        if should_queue_job_offer(state, time.day) {
            queue_event(state, "civilian_job_offer", time.tick);
            state.last_job_offer_day = time.day;
        }
    }

    let activity = state.routine.activity_at(time.hour);
    apply_routine_activity(state, activity, time);
    check_for_death(state, time.day);

    if let Some(start_hour) = state.routine.first_hour_for(RoutineActivity::Work) {
        if matches!(state.job_status, JobStatus::Employed | JobStatus::PartTime)
            && time.hour == start_hour
            && state.last_work_day != time.day
        {
            state.last_work_day = time.day;
            queue_event(state, "civilian_work_shift", time.tick);
            record_work_shift(state);
            apply_career_progression(state, time.day);
        }
    }

    if state.education.is_enrolled {
        if let Some(start_hour) = state.routine.first_hour_for(RoutineActivity::School) {
            if time.hour == start_hour && state.education.last_school_day != time.day {
                queue_event(state, "civilian_school_day", time.tick);
                state.education.last_school_day = time.day;
                record_school_session(state);
            }
        }
    }

    if let Some(start_hour) = state.routine.first_hour_for(RoutineActivity::Hobby) {
        if time.hour == start_hour && state.last_hobby_day != time.day {
            state.last_hobby_day = time.day;
            queue_event(state, "civilian_hobby_session", time.tick);
        }
    }

    let social_hour = state
        .routine
        .first_hour_for(RoutineActivity::Social)
        .unwrap_or(19);
    if time.hour == social_hour && state.last_relationship_day != time.day {
        state.last_relationship_day = time.day;
        if let Some((event_id, contact_name)) = choose_relationship_event(state) {
            queue_event_with_contact(state, event_id, time.tick, contact_name);
        } else if state.social.support < 60 || state.social.strain > 15 {
            queue_event(state, "civilian_relationship_checkin", time.tick);
        }
    }

    if (state.health.stress >= 65 || state.health.sleep_debt >= 16)
        && time.hour == 20
        && state.last_health_day != time.day
    {
        state.last_health_day = time.day;
        queue_event(state, "civilian_health_checkin", time.tick);
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
    let mut housing_changed = false;
    let mut life_changed = false;
    let mut education_changed = false;
    let mut routine_changed = false;
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
            "housing_rent" => {
                apply_delta_at(
                    &mut state.housing.rent,
                    parts.get(1),
                    &mut applied,
                    "housing_rent",
                );
                state.housing.rent = state.housing.rent.max(0);
                housing_changed = true;
            }
            "housing_stability" => {
                apply_delta_at(
                    &mut state.housing.stability,
                    parts.get(1),
                    &mut applied,
                    "housing_stability",
                );
                state.housing.stability = clamp_metric(state.housing.stability);
                housing_changed = true;
            }
            "housing_safety" => {
                apply_delta_at(
                    &mut state.housing.safety,
                    parts.get(1),
                    &mut applied,
                    "housing_safety",
                );
                state.housing.safety = clamp_metric(state.housing.safety);
                housing_changed = true;
            }
            "housing_privacy" => {
                apply_delta_at(
                    &mut state.housing.privacy,
                    parts.get(1),
                    &mut applied,
                    "housing_privacy",
                );
                state.housing.privacy = clamp_metric(state.housing.privacy);
                housing_changed = true;
            }
            "housing_neighborhood" => {
                if let Some(value) = parts.get(1).map(|value| value.trim()) {
                    if !value.is_empty() {
                        if apply_neighborhood_profile(state, value, &mut applied) {
                            housing_changed = true;
                        }
                    }
                }
            }
            "relocation_cooldown" => {
                apply_delta_at(
                    &mut state.housing.relocation_cooldown,
                    parts.get(1),
                    &mut applied,
                    "relocation_cooldown",
                );
                state.housing.relocation_cooldown = state.housing.relocation_cooldown.max(0);
            }
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
            "age" | "age_years" => {
                apply_delta_at_u32(
                    &mut state.life.age_years,
                    parts.get(1),
                    &mut applied,
                    "age_years",
                );
                life_changed = true;
            }
            "life_stage" => {
                if let Some(stage) = parts.get(1).and_then(|value| parse_life_stage(value)) {
                    state.life.life_stage = stage;
                    applied.push(format!("life_stage -> {:?}", stage));
                    life_changed = true;
                }
            }
            "mutant_gene" => {
                if let Some(value) = parts.get(1).and_then(|value| parse_bool_flag(value)) {
                    state.life.mutant_gene = value;
                    applied.push(format!("mutant_gene -> {}", value));
                    life_changed = true;
                }
            }
            "mutation_ready" => {
                if let Some(value) = parts.get(1).and_then(|value| parse_bool_flag(value)) {
                    state.life.mutation_ready = value;
                    applied.push(format!("mutation_ready -> {}", value));
                    life_changed = true;
                }
            }
            "education_level" => {
                if let Some(level) = parts.get(1).and_then(|value| parse_education_level(value)) {
                    state.education.level = level;
                    applied.push(format!("education level -> {:?}", level));
                    education_changed = true;
                }
            }
            "education_progress" => {
                apply_delta_at(
                    &mut state.education.progress,
                    parts.get(1),
                    &mut applied,
                    "education_progress",
                );
                state.education.progress = clamp_metric(state.education.progress);
                education_changed = true;
            }
            "education_attendance" => {
                apply_delta_at(
                    &mut state.education.attendance,
                    parts.get(1),
                    &mut applied,
                    "education_attendance",
                );
                state.education.attendance = clamp_metric(state.education.attendance);
                education_changed = true;
            }
            "education_dropout_risk" => {
                apply_delta_at(
                    &mut state.education.dropout_risk,
                    parts.get(1),
                    &mut applied,
                    "education_dropout_risk",
                );
                state.education.dropout_risk = clamp_metric(state.education.dropout_risk);
                education_changed = true;
            }
            "education_credits" => {
                apply_delta_at(
                    &mut state.education.credits,
                    parts.get(1),
                    &mut applied,
                    "education_credits",
                );
                education_changed = true;
            }
            "education_enrolled" => {
                if let Some(value) = parts.get(1).and_then(|value| parse_bool_flag(value)) {
                    state.education.is_enrolled = value;
                    applied.push(format!("education enrolled -> {}", value));
                    education_changed = true;
                }
            }
            "health_stress" => {
                apply_delta_at(
                    &mut state.health.stress,
                    parts.get(1),
                    &mut applied,
                    "health_stress",
                );
                state.health.stress = clamp_metric(state.health.stress);
                routine_changed = true;
            }
            "health_sleep_debt" => {
                apply_delta_at(
                    &mut state.health.sleep_debt,
                    parts.get(1),
                    &mut applied,
                    "health_sleep_debt",
                );
                state.health.sleep_debt = clamp_metric(state.health.sleep_debt);
                routine_changed = true;
            }
            "health_fitness" => {
                apply_delta_at(
                    &mut state.health.fitness,
                    parts.get(1),
                    &mut applied,
                    "health_fitness",
                );
                state.health.fitness = clamp_metric(state.health.fitness);
                routine_changed = true;
            }
            "health_injuries" => {
                apply_delta_at(
                    &mut state.health.injuries,
                    parts.get(1),
                    &mut applied,
                    "health_injuries",
                );
                state.health.injuries = clamp_metric(state.health.injuries);
                routine_changed = true;
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
                    routine_changed = true;
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
                        let mut level = None;
                        let mut domain = None;
                        let mut relation_type = None;
                        for value in parts.iter().skip(2).take(3) {
                            let value = value.trim();
                            if value.is_empty() {
                                continue;
                            }
                            if level.is_none() {
                                if let Some(parsed) = parse_relationship_level(value) {
                                    level = Some(parsed);
                                    continue;
                                }
                            }
                            if domain.is_none() {
                                if let Some(parsed) = parse_contact_domain(value) {
                                    domain = Some(parsed);
                                    continue;
                                }
                            }
                            if relation_type.is_none() {
                                if let Some(parsed) = parse_relation_type(value) {
                                    relation_type = Some(parsed);
                                }
                            }
                        }
                        let level = level.unwrap_or(RelationshipLevel::Acquaintance);
                        state.upsert_contact(name, level, domain, relation_type, &mut applied);
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
            "relation_type" | "contact_relation" => {
                if let (Some(name), Some(type_raw)) = (parts.get(1), parts.get(2)) {
                    if let Some(relation_type) = parse_relation_type(type_raw.trim()) {
                        state.set_relation_type(name.trim(), relation_type, &mut applied);
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
        routine_changed = true;
    }
    if wealth_changed {
        sync_finances_from_wealth(state);
        refresh_wealth_tier(state);
    } else if cash_changed {
        sync_wealth_from_finances(state);
    } else if debt_changed {
        refresh_wealth_tier(state);
    }
    if housing_changed {
        sync_housing_rent(state);
    }
    if life_changed {
        state.life.life_stage = life_stage_for_age(state.life.age_years);
        align_education_with_life_stage(state);
        update_mutation_status(state);
        routine_changed = true;
    }
    if education_changed {
        advance_education_if_ready(state);
        routine_changed = true;
    }
    if routine_changed {
        update_routine_schedule(state);
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

fn update_housing_state(state: &mut CivilianState, time: &GameTime) {
    if state.housing.relocation_cooldown > 0 {
        state.housing.relocation_cooldown -= 1;
    }

    let rent_overdue = state.finances.rent_due_in <= 0;
    if rent_overdue {
        let penalty = if state.finances.cash < state.housing.rent {
            4
        } else {
            2
        };
        state.housing.stability = clamp_metric(state.housing.stability - penalty);
        state.social.strain = clamp_metric(state.social.strain + 1);
    } else if state.finances.rent_due_in > 7 && state.housing.stability < 82 {
        state.housing.stability = clamp_metric(state.housing.stability + 1);
    }

    if state.housing.stability <= 40 && state.housing.relocation_cooldown <= 0 {
        queue_event(state, "civilian_relocation_offer", time.tick);
    }
}

fn update_mistake_risk(state: &mut CivilianState) {
    let mut risk = 0;
    if state.job.stability < 45 {
        risk += (45 - state.job.stability) / 2;
    }
    if state.job.satisfaction < 40 {
        risk += (40 - state.job.satisfaction) / 2;
    }
    if state.finances.rent_due_in <= 0 {
        risk += 12;
    } else if state.finances.rent_due_in <= 2 {
        risk += 6;
    }
    if state.health.stress > 70 {
        risk += (state.health.stress - 70) / 2;
    }
    if state.health.sleep_debt > 18 {
        risk += (state.health.sleep_debt - 18) / 2;
    }
    if state.education.dropout_risk > 60 {
        risk += (state.education.dropout_risk - 60) / 2;
    }
    if state.housing.stability < 45 {
        risk += (45 - state.housing.stability) / 2;
    }
    if state.social.strain > 60 {
        risk += (state.social.strain - 60) / 2;
    }
    if state.social.obligation > 65 {
        risk += (state.social.obligation - 65) / 2;
    }
    state.mistake_risk = clamp_metric(risk);
}

fn has_relation_type(state: &CivilianState, relation_type: RelationType) -> bool {
    state
        .contacts
        .iter()
        .any(|contact| contact.relation_type == relation_type)
}

fn pick_contact_for_event(state: &CivilianState) -> Option<&Contact> {
    state
        .contacts
        .iter()
        .min_by_key(|contact| contact.last_interaction_day)
}

fn event_for_contact(state: &CivilianState, contact: &Contact) -> &'static str {
    match contact.relation_type {
        RelationType::Family => "civilian_contact_family",
        RelationType::Mentor => "civilian_contact_mentor",
        RelationType::Romance if state.life.age_years >= 16 => "civilian_contact_romance",
        RelationType::Rival if state.life.age_years >= 15 => "civilian_contact_rival",
        RelationType::Colleague
            if matches!(state.job_status, JobStatus::Employed | JobStatus::PartTime) =>
        {
            "civilian_contact_colleague"
        }
        _ => "civilian_contact_checkin",
    }
}

fn choose_relationship_event(state: &CivilianState) -> Option<(&'static str, Option<String>)> {
    if let Some(contact) = pick_contact_for_event(state) {
        let event_id = event_for_contact(state, contact);
        return Some((event_id, Some(contact.name.clone())));
    }

    let age = state.life.age_years;
    if age >= 12 && !has_relation_type(state, RelationType::Family) {
        return Some(("civilian_family_obligation", None));
    }
    if !has_relation_type(state, RelationType::Mentor)
        && (matches!(
            state.education.level,
            EducationLevel::Secondary | EducationLevel::Tertiary | EducationLevel::Graduate
        ) || state.job.level >= 2)
    {
        return Some(("civilian_mentor_session", None));
    }
    if matches!(state.job_status, JobStatus::Employed | JobStatus::PartTime)
        && !has_relation_type(state, RelationType::Colleague)
    {
        return Some(("civilian_colleague_project", None));
    }
    if age >= 17 && !has_relation_type(state, RelationType::Romance) {
        return Some(("civilian_romance_date", None));
    }
    if age >= 18 && state.job.level >= 2 && !has_relation_type(state, RelationType::Rival) {
        return Some(("civilian_rival_runin", None));
    }
    None
}

fn legacy_achievements(state: &CivilianState) -> Vec<String> {
    let mut out = Vec::new();
    if state.job.level >= 4 {
        out.push(format!("Career level {}", state.job.level));
    }
    match state.civilian_tier {
        CivilianTier::Influential => out.push("Local influence".to_string()),
        CivilianTier::PowerBroker => out.push("Power broker".to_string()),
        _ => {}
    }
    if state.wealth.tier.rank() >= WealthTier::Wealthy.rank() {
        out.push(format!("Wealth {}", state.wealth.tier.label()));
    }
    if state.reputation.career >= 70 {
        out.push("Respected professional".to_string());
    }
    if state.reputation.community >= 70 {
        out.push("Community pillar".to_string());
    }
    if state.reputation.media >= 70 {
        out.push("Media figure".to_string());
    }
    if state.contacts.len() >= 6 {
        out.push("Wide social web".to_string());
    }
    if out.is_empty() {
        out.push("Quiet life".to_string());
    }
    out
}

fn should_queue_crime_opportunity(state: &CivilianState) -> bool {
    if state.life.age_years < 16 {
        return false;
    }
    if state.finances.rent_due_in > 1 {
        return false;
    }
    state.finances.cash < state.housing.rent
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

fn sync_housing_rent(state: &mut CivilianState) {
    if state.finances.rent != state.housing.rent {
        state.finances.rent = state.housing.rent;
    }
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
    let relation_bonus = match contact.relation_type {
        RelationType::Family => 4,
        RelationType::Mentor => 3,
        RelationType::Romance => 3,
        RelationType::Colleague => 1,
        RelationType::Peer => 0,
        RelationType::Rival => -2,
    };
    target += relation_bonus;
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
    queue_event_with_contact(state, storylet_id, created_tick, None);
}

fn queue_event_with_contact(
    state: &mut CivilianState,
    storylet_id: &str,
    created_tick: u64,
    contact_name: Option<String>,
) {
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
        contact_name,
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

fn apply_delta_at_u32(
    target: &mut u32,
    value: Option<&&str>,
    applied: &mut Vec<String>,
    label: &str,
) {
    let Some(value) = value else {
        return;
    };
    if let Ok(delta) = value.trim().parse::<i32>() {
        let next = (*target as i32 + delta).max(0) as u32;
        *target = next;
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

fn update_age_and_life_stage(state: &mut CivilianState, time: &GameTime) {
    let mut age_changed = false;
    while time.day >= state.life.birth_day.saturating_add(DAYS_PER_YEAR) {
        state.life.birth_day = state.life.birth_day.saturating_add(DAYS_PER_YEAR);
        state.life.age_years = state.life.age_years.saturating_add(1);
        age_changed = true;
    }
    if age_changed {
        let new_stage = life_stage_for_age(state.life.age_years);
        if new_stage != state.life.life_stage {
            state.life.life_stage = new_stage;
            align_education_with_life_stage(state);
        }
    }
    update_mutation_status(state);
}

fn apply_age_decline(state: &mut CivilianState, day: u32) {
    if day % 28 != 0 {
        return;
    }
    if state.life.age_years >= 45 {
        state.health.fitness = clamp_metric(state.health.fitness - 1);
    }
    if state.life.age_years >= 60 {
        state.health.stress = clamp_metric(state.health.stress + 1);
    }
    if state.life.age_years >= 70 {
        state.health.injuries = clamp_metric(state.health.injuries + 1);
    }
    if state.life.age_years >= 80 {
        state.health.sleep_debt = clamp_metric(state.health.sleep_debt + 1);
    }
}

fn update_mortality_risk(state: &mut CivilianState) {
    let age = state.life.age_years as i32;
    let mut risk = if age < 40 {
        0
    } else if age < 60 {
        age - 40
    } else if age < 80 {
        20 + (age - 60) * 2
    } else {
        60 + (age - 80) * 3
    };

    let injuries = state.health.injuries;
    if injuries > 60 {
        risk += (injuries - 60) / 2;
    }
    let stress = state.health.stress;
    if stress > 70 {
        risk += (stress - 70) / 3;
    }
    let sleep_debt = state.health.sleep_debt;
    if sleep_debt > 75 {
        risk += (sleep_debt - 75) / 4;
    }
    let fitness = state.health.fitness;
    if fitness < 30 {
        risk += (30 - fitness) / 2;
    }
    state.life.mortality_risk = risk.clamp(0, 150);
}

fn check_for_death(state: &mut CivilianState, day: u32) {
    if state.pending_death.is_some() {
        return;
    }
    let reason = if state.health.injuries >= 95 {
        Some("critical injuries")
    } else if state.health.stress >= 95 && state.health.sleep_debt >= 85 {
        Some("collapse")
    } else if state.life.mortality_risk >= 100 {
        Some("mortality")
    } else {
        None
    };
    if let Some(reason) = reason {
        state.pending_death = Some(DeathRecord {
            day,
            age_years: state.life.age_years,
            reason: reason.to_string(),
        });
    }
}

fn update_mutation_status(state: &mut CivilianState) {
    if state.life.mutant_gene
        && !state.life.mutation_ready
        && state.life.age_years >= 13
        && state.life.age_years <= 19
    {
        state.life.mutation_ready = true;
    }
}

fn align_education_with_life_stage(state: &mut CivilianState) {
    match state.life.life_stage {
        LifeStage::Child => {
            state.education.level = EducationLevel::Primary;
            state.education.is_enrolled = true;
        }
        LifeStage::Teen => {
            state.education.level = EducationLevel::Secondary;
            state.education.is_enrolled = true;
        }
        LifeStage::YoungAdult => {
            if state.education.level == EducationLevel::Secondary
                && state.education.progress >= 60
            {
                state.education.level = EducationLevel::Tertiary;
                state.education.progress = 0;
                state.education.credits = 0;
            }
            state.education.is_enrolled = state.education.level == EducationLevel::Tertiary;
        }
        LifeStage::Adult | LifeStage::Mature | LifeStage::Elder => {
            if state.education.level == EducationLevel::Tertiary
                && state.education.progress >= 80
            {
                state.education.level = EducationLevel::Graduate;
                state.education.progress = 0;
                state.education.credits = 0;
            }
            state.education.is_enrolled = false;
        }
    }
}

fn update_routine_schedule(state: &mut CivilianState) {
    state.routine = build_routine_schedule(state.job_status, &state.education, &state.life);
}

fn build_routine_schedule(
    job_status: JobStatus,
    education: &EducationTrack,
    life: &LifeState,
) -> RoutineSchedule {
    let mut schedule = RoutineSchedule::default();
    schedule.push_block(0, 6, RoutineActivity::Rest);
    schedule.push_block(6, 1, RoutineActivity::Errands);
    schedule.push_block(7, 1, RoutineActivity::Rest);

    if education.is_enrolled {
        match education.level {
            EducationLevel::Primary | EducationLevel::Secondary => {
                schedule.push_block(8, 6, RoutineActivity::School);
            }
            EducationLevel::Tertiary => {
                schedule.push_block(10, 4, RoutineActivity::School);
            }
            _ => {}
        }
    }

    match job_status {
        JobStatus::Employed => {
            let (start, duration) = if education.is_enrolled
                || matches!(life.life_stage, LifeStage::Child | LifeStage::Teen)
            {
                (16, 4)
            } else {
                (9, 8)
            };
            schedule.push_block(start, duration, RoutineActivity::Work);
        }
        JobStatus::PartTime => {
            let start = if education.is_enrolled { 16 } else { 12 };
            schedule.push_block(start, 4, RoutineActivity::Work);
        }
        JobStatus::Unemployed => {}
    }

    schedule.push_block(19, 2, RoutineActivity::Hobby);
    schedule.push_block(21, 1, RoutineActivity::Social);
    schedule.push_block(22, 2, RoutineActivity::Rest);
    schedule
}

fn apply_daily_health(state: &mut CivilianState) {
    if state.health.sleep_debt > 16 {
        state.health.stress = clamp_metric(state.health.stress + 2);
    }
    if state.health.stress > 70 {
        state.health.fitness = clamp_metric(state.health.fitness - 1);
    }
    if state.health.sleep_debt > 20 {
        state.health.sleep_debt = clamp_metric(state.health.sleep_debt - 1);
    }
}

fn apply_routine_activity(state: &mut CivilianState, activity: RoutineActivity, time: &GameTime) {
    match activity {
        RoutineActivity::Work => {
            state.health.stress = clamp_metric(state.health.stress + 1);
            state.health.sleep_debt = clamp_metric(state.health.sleep_debt + 1);
            if state.job.satisfaction < 45 {
                state.social.strain = clamp_metric(state.social.strain + 1);
            }
        }
        RoutineActivity::School => {
            state.health.stress = clamp_metric(state.health.stress + 1);
            state.health.sleep_debt = clamp_metric(state.health.sleep_debt + 1);
            if state.education.attendance < 45 {
                state.social.strain = clamp_metric(state.social.strain + 1);
            }
        }
        RoutineActivity::Hobby => {
            state.health.stress = clamp_metric(state.health.stress - 2);
            state.health.sleep_debt = clamp_metric(state.health.sleep_debt - 1);
            state.social.support = clamp_metric(state.social.support + 1);
        }
        RoutineActivity::Social => {
            state.health.stress = clamp_metric(state.health.stress - 1);
            state.social.support = clamp_metric(state.social.support + 1);
            state.social.strain = clamp_metric(state.social.strain - 1);
        }
        RoutineActivity::Rest => {
            state.health.sleep_debt = clamp_metric(state.health.sleep_debt - 2);
            state.health.stress = clamp_metric(state.health.stress - 1);
            if time.is_day && state.health.fitness < 60 {
                state.health.fitness = clamp_metric(state.health.fitness + 1);
            }
        }
        RoutineActivity::Errands => {
            state.health.stress = clamp_metric(state.health.stress + 1);
        }
    }
}

fn record_school_session(state: &mut CivilianState) {
    let attendance_gain = if state.health.sleep_debt > 12 { 1 } else { 2 };
    state.education.attendance = clamp_metric(state.education.attendance + attendance_gain);
    let progress_gain = (2 + (state.education.attendance - 50) / 25).max(1);
    state.education.progress = clamp_metric(state.education.progress + progress_gain);
    state.education.credits = state.education.credits.saturating_add(1);
    let mut dropout_delta = 0;
    if state.education.attendance < 40 {
        dropout_delta += 2;
    }
    if state.education.attendance < 25 {
        dropout_delta += 2;
    }
    if state.health.stress > 70 {
        dropout_delta += 1;
    }
    if state.health.sleep_debt > 18 {
        dropout_delta += 1;
    }
    if state.education.attendance > 65 {
        dropout_delta -= 2;
    }
    if state.education.attendance > 80 {
        dropout_delta -= 2;
    }
    state.education.dropout_risk = clamp_metric(state.education.dropout_risk + dropout_delta);
    advance_education_if_ready(state);
}

fn advance_education_if_ready(state: &mut CivilianState) {
    if state.education.progress >= 100 {
        state.education.progress = 0;
        state.education.credits = 0;
        state.education.level = match state.education.level {
            EducationLevel::Primary => EducationLevel::Secondary,
            EducationLevel::Secondary => EducationLevel::Tertiary,
            EducationLevel::Tertiary => EducationLevel::Graduate,
            EducationLevel::Graduate | EducationLevel::None => EducationLevel::Graduate,
        };
        if state.education.level == EducationLevel::Graduate {
            state.education.is_enrolled = false;
        }
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

fn parse_life_stage(value: &str) -> Option<LifeStage> {
    match value.to_ascii_lowercase().as_str() {
        "child" => Some(LifeStage::Child),
        "teen" => Some(LifeStage::Teen),
        "young_adult" | "youngadult" => Some(LifeStage::YoungAdult),
        "adult" => Some(LifeStage::Adult),
        "mature" => Some(LifeStage::Mature),
        "elder" => Some(LifeStage::Elder),
        _ => None,
    }
}

fn parse_education_level(value: &str) -> Option<EducationLevel> {
    match value.to_ascii_lowercase().as_str() {
        "none" => Some(EducationLevel::None),
        "primary" => Some(EducationLevel::Primary),
        "secondary" => Some(EducationLevel::Secondary),
        "tertiary" => Some(EducationLevel::Tertiary),
        "graduate" => Some(EducationLevel::Graduate),
        _ => None,
    }
}

fn parse_bool_flag(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "1" => Some(true),
        "false" | "no" | "0" => Some(false),
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

fn parse_relation_type(value: &str) -> Option<RelationType> {
    match value.to_ascii_lowercase().as_str() {
        "family" => Some(RelationType::Family),
        "mentor" => Some(RelationType::Mentor),
        "rival" => Some(RelationType::Rival),
        "romance" | "romantic" => Some(RelationType::Romance),
        "colleague" | "coworker" => Some(RelationType::Colleague),
        "peer" | "friend" => Some(RelationType::Peer),
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

fn default_relation_type(domain: ContactDomain) -> RelationType {
    match domain {
        ContactDomain::Professional => RelationType::Colleague,
        _ => RelationType::Peer,
    }
}

fn neighborhood_profile(id: &str) -> Option<&'static NeighborhoodProfile> {
    NEIGHBORHOOD_CATALOG
        .iter()
        .find(|profile| profile.id.eq_ignore_ascii_case(id))
}

fn apply_neighborhood_profile(
    state: &mut CivilianState,
    id: &str,
    applied: &mut Vec<String>,
) -> bool {
    let Some(profile) = neighborhood_profile(id) else {
        state.housing.neighborhood_id = id.to_string();
        applied.push(format!("housing neighborhood -> {}", id));
        return true;
    };
    state.housing.neighborhood_id = profile.id.to_string();
    state.housing.rent = profile.rent.max(0);
    state.housing.stability = clamp_metric(profile.stability);
    state.housing.safety = clamp_metric(profile.safety);
    state.housing.privacy = clamp_metric(profile.privacy);
    applied.push(format!(
        "housing neighborhood -> {} ({})",
        profile.id, profile.label
    ));
    true
}

fn life_stage_for_age(age_years: u32) -> LifeStage {
    match age_years {
        0..=12 => LifeStage::Child,
        13..=17 => LifeStage::Teen,
        18..=25 => LifeStage::YoungAdult,
        26..=40 => LifeStage::Adult,
        41..=60 => LifeStage::Mature,
        _ => LifeStage::Elder,
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
        relation_type: Option<RelationType>,
        applied: &mut Vec<String>,
    ) {
        if let Some(contact) = self.contacts.iter_mut().find(|entry| entry.name == name) {
            contact.level = level;
            if let Some(domain) = domain {
                contact.domain = domain;
                applied.push(format!("contact {} domain -> {:?}", name, domain));
            }
            if let Some(relation_type) = relation_type {
                contact.relation_type = relation_type;
                applied.push(format!("contact {} relation -> {:?}", name, relation_type));
            }
            contact.last_interaction_day = self.last_day;
            applied.push(format!("contact {} -> {:?}", name, level));
            return;
        }
        let domain = domain.unwrap_or_else(|| default_contact_domain(self.job.role));
        let relation_type = relation_type.unwrap_or_else(|| default_relation_type(domain));
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
            relation_type,
            bond,
            influence: 10,
            last_interaction_day: self.last_day,
        });
        applied.push(format!("contact added {} ({:?})", name, level));
    }

    fn adjust_relationship(&mut self, name: &str, delta: i32, applied: &mut Vec<String>) {
        let contact = match self.contacts.iter_mut().find(|entry| entry.name == name) {
            Some(contact) => contact,
            None => {
                self.upsert_contact(name, RelationshipLevel::Acquaintance, None, None, applied);
                self.contacts
                    .iter_mut()
                    .find(|entry| entry.name == name)
                    .expect("contact inserted")
            }
        };
        contact.bond = clamp_metric(contact.bond + delta);
        contact.level = relationship_level_from_bond(contact.bond);
        contact.last_interaction_day = self.last_day;
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
                self.upsert_contact(name, level, None, None, applied);
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
        contact.last_interaction_day = self.last_day;
        applied.push(format!("relationship level {} -> {:?}", name, level));
    }

    fn adjust_contact_influence(&mut self, name: &str, delta: i32, applied: &mut Vec<String>) {
        let contact = match self.contacts.iter_mut().find(|entry| entry.name == name) {
            Some(contact) => contact,
            None => {
                self.upsert_contact(name, RelationshipLevel::Acquaintance, None, None, applied);
                self.contacts
                    .iter_mut()
                    .find(|entry| entry.name == name)
                    .expect("contact inserted")
            }
        };
        contact.influence = clamp_metric(contact.influence + delta);
        contact.last_interaction_day = self.last_day;
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
                self.upsert_contact(
                    name,
                    RelationshipLevel::Acquaintance,
                    Some(domain),
                    None,
                    applied,
                );
                return;
            }
        };
        contact.domain = domain;
        contact.last_interaction_day = self.last_day;
        applied.push(format!("contact {} domain -> {:?}", name, domain));
    }

    fn set_relation_type(
        &mut self,
        name: &str,
        relation_type: RelationType,
        applied: &mut Vec<String>,
    ) {
        let contact = match self.contacts.iter_mut().find(|entry| entry.name == name) {
            Some(contact) => contact,
            None => {
                self.upsert_contact(
                    name,
                    RelationshipLevel::Acquaintance,
                    None,
                    Some(relation_type),
                    applied,
                );
                return;
            }
        };
        contact.relation_type = relation_type;
        contact.last_interaction_day = self.last_day;
        applied.push(format!("contact {} relation -> {:?}", name, relation_type));
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
