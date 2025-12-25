use bevy_ecs::prelude::*;

use crate::simulation::time::GameTime;

#[derive(Resource, Debug, Clone)]
pub struct CivilianState {
    pub job_status: JobStatus,
    pub finances: CivilianFinances,
    pub social: SocialTies,
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
pub struct CivilianEvent {
    pub storylet_id: String,
    pub created_tick: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CivilianPressure {
    pub temporal: f32,
    pub resource: f32,
    pub moral: f32,
}

impl Default for CivilianState {
    fn default() -> Self {
        Self {
            job_status: JobStatus::Employed,
            finances: CivilianFinances {
                cash: 120,
                debt: 0,
                rent: 80,
                rent_due_in: 5,
                wage: 45,
            },
            social: SocialTies {
                support: 55,
                strain: 10,
                obligation: 12,
            },
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

        let temporal = (base_time + rent_urgency + self.social.obligation as f32 * 0.6)
            .clamp(0.0, 100.0);

        let mut resource = (self.finances.debt.max(0) as f32 * 0.8).clamp(0.0, 60.0);
        if self.finances.cash < self.finances.rent {
            resource += 18.0;
        }
        if self.finances.rent_due_in <= 2 {
            resource += 12.0;
        }
        resource = resource.clamp(0.0, 100.0);

        let mut moral = (self.social.strain as f32 * 1.4).clamp(0.0, 60.0);
        if self.social.support < 35 {
            moral += 12.0;
        }
        moral = moral.clamp(0.0, 100.0);

        CivilianPressure {
            temporal,
            resource,
            moral,
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
        if let Some((key, value)) = effect.split_once(':') {
            match key.trim() {
                "cash" => apply_delta(&mut state.finances.cash, value, &mut applied, "cash"),
                "debt" => apply_delta(&mut state.finances.debt, value, &mut applied, "debt"),
                "rent_due_in" => {
                    apply_delta(&mut state.finances.rent_due_in, value, &mut applied, "rent_due_in")
                }
                "support" => {
                    apply_delta(&mut state.social.support, value, &mut applied, "support");
                    state.social.support = clamp_metric(state.social.support);
                }
                "strain" => {
                    apply_delta(&mut state.social.strain, value, &mut applied, "strain");
                    state.social.strain = clamp_metric(state.social.strain);
                }
                "obligation" => {
                    apply_delta(&mut state.social.obligation, value, &mut applied, "obligation");
                    state.social.obligation = clamp_metric(state.social.obligation);
                }
                "job" => {
                    if let Some(job) = parse_job_status(value.trim()) {
                        state.job_status = job;
                        applied.push(format!("job -> {:?}", job));
                    }
                }
                _ => {}
            }
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

fn apply_delta(target: &mut i32, value: &str, applied: &mut Vec<String>, label: &str) {
    if let Ok(delta) = value.trim().parse::<i32>() {
        *target += delta;
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

fn clamp_metric(value: i32) -> i32 {
    value.clamp(0, 100)
}
