use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const DAYS_PER_YEAR: u32 = 336;
const DEFAULT_START_YEAR: i32 = 2040;
const RETIREMENT_AGE: i32 = 65;
const MANDATORY_RETIREMENT_AGE: i32 = 75;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ImportanceTier {
    A,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentCharacter {
    pub character_id: String,
    pub scope_id: String,
    pub first_name: String,
    pub last_name: String,
    pub birth_year: Option<i32>,
    pub ancestry: Option<String>,
    pub nationality: Option<String>,
    pub importance_tier: ImportanceTier,
    pub created_at_tick: u64,
    pub roles: Vec<CharacterRole>,
    pub personas: Vec<CharacterPersona>,
    pub powers: Vec<CharacterPower>,
    pub relationships: Vec<CharacterRelationship>,
}

#[derive(Debug, Default, Clone)]
pub struct CastAgingReport {
    pub retired: Vec<String>,
    pub deceased: Vec<String>,
    pub assigned_birth_year: Vec<String>,
    pub changed_ids: HashSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRole {
    pub role_type: String,
    pub faction_id: Option<String>,
    pub rank: Option<String>,
    pub start_tick: u64,
    pub end_tick: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterPersona {
    pub persona_id: String,
    pub persona_kind: String,
    pub label: String,
    pub is_active_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterPower {
    pub power_id: i64,
    pub expression_id: Option<String>,
    pub acq_id: Option<String>,
    pub mastery: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterRelationship {
    pub other_character_id: String,
    pub relation_type: String,
    pub trust: i32,
    pub fear: i32,
    pub resentment: i32,
    pub is_public: bool,
    pub start_tick: u64,
    pub end_tick: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromotionReason {
    Nemesis,
    FactionRole,
    NarrativeBinding,
    MediaRecognition,
    Recurrence,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionCandidate {
    pub scope_id: String,
    pub first_name: String,
    pub last_name: String,
    pub role_type: String,
    pub faction_id: Option<String>,
    pub rank: Option<String>,
    pub persona_kind: Option<String>,
    pub persona_label: Option<String>,
    pub reason: PromotionReason,
}

impl PromotionCandidate {
    pub fn to_character(&self, character_id: String, created_at_tick: u64) -> PersistentCharacter {
        let role = CharacterRole {
            role_type: self.role_type.clone(),
            faction_id: self.faction_id.clone(),
            rank: self.rank.clone(),
            start_tick: created_at_tick,
            end_tick: None,
        };
        let personas = self
            .persona_kind
            .as_ref()
            .map(|kind| CharacterPersona {
                persona_id: "default".to_string(),
                persona_kind: kind.clone(),
                label: self
                    .persona_label
                    .clone()
                    .unwrap_or_else(|| kind.to_string()),
                is_active_default: true,
            })
            .into_iter()
            .collect();
        PersistentCharacter {
            character_id,
            scope_id: self.scope_id.clone(),
            first_name: self.first_name.clone(),
            last_name: self.last_name.clone(),
            birth_year: None,
            ancestry: None,
            nationality: None,
            importance_tier: ImportanceTier::A,
            created_at_tick,
            roles: vec![role],
            personas,
            powers: Vec::new(),
            relationships: Vec::new(),
        }
    }
}

pub fn current_year_from_day(day: u32) -> i32 {
    let years = day.saturating_sub(1) / DAYS_PER_YEAR;
    DEFAULT_START_YEAR + years as i32
}

pub fn tick_cast_aging(
    characters: &mut Vec<PersistentCharacter>,
    current_year: i32,
    current_tick: u64,
) -> CastAgingReport {
    let mut report = CastAgingReport::default();
    for character in characters.iter_mut() {
        let birth_year = ensure_birth_year(character, current_year, &mut report);
        let age_years = current_year.saturating_sub(birth_year);

        if has_active_role(character, "DECEASED") {
            continue;
        }

        if should_mark_deceased(character, current_year, age_years) {
            mark_character_status(character, "DECEASED", current_tick);
            report.deceased.push(character.character_id.clone());
            report.changed_ids.insert(character.character_id.clone());
            continue;
        }

        if !has_active_role(character, "RETIRED")
            && should_mark_retired(character, current_year, age_years)
        {
            mark_character_status(character, "RETIRED", current_tick);
            report.retired.push(character.character_id.clone());
            report.changed_ids.insert(character.character_id.clone());
        }
    }
    report
}

fn ensure_birth_year(
    character: &mut PersistentCharacter,
    current_year: i32,
    report: &mut CastAgingReport,
) -> i32 {
    if let Some(year) = character.birth_year {
        return year;
    }
    let seed = stable_seed_for_character(character);
    let age = 24 + (seed % 31) as i32;
    let birth_year = current_year - age;
    character.birth_year = Some(birth_year);
    report.assigned_birth_year.push(character.character_id.clone());
    report.changed_ids.insert(character.character_id.clone());
    birth_year
}

fn should_mark_retired(character: &PersistentCharacter, current_year: i32, age: i32) -> bool {
    if age >= MANDATORY_RETIREMENT_AGE {
        return true;
    }
    if age < RETIREMENT_AGE {
        return false;
    }
    let roll = deterministic_roll(character, current_year, 31);
    let base = 5 + (age - RETIREMENT_AGE) * 2;
    roll < base.clamp(5, 35) as u32
}

fn should_mark_deceased(character: &PersistentCharacter, current_year: i32, age: i32) -> bool {
    let risk = match age {
        i32::MIN..=69 => 0,
        70..=79 => 3,
        80..=84 => 6,
        85..=89 => 12,
        90..=94 => 25,
        _ => 45,
    };
    if risk == 0 {
        return false;
    }
    let roll = deterministic_roll(character, current_year, 97);
    roll < risk as u32
}

fn deterministic_roll(character: &PersistentCharacter, current_year: i32, salt: u32) -> u32 {
    let seed = stable_seed_for_character(character);
    let mixed = seed ^ (current_year as u32).wrapping_mul(2654435761) ^ salt;
    mixed % 100
}

fn stable_seed_for_character(character: &PersistentCharacter) -> u32 {
    let mut seed = 0u32;
    for byte in character.character_id.as_bytes() {
        seed = seed.wrapping_add(*byte as u32).wrapping_mul(1664525);
    }
    for byte in character.first_name.as_bytes() {
        seed = seed.wrapping_add(*byte as u32).wrapping_mul(1013904223);
    }
    for byte in character.last_name.as_bytes() {
        seed = seed.wrapping_add(*byte as u32).wrapping_mul(22695477);
    }
    seed
}

fn has_active_role(character: &PersistentCharacter, role: &str) -> bool {
    character
        .roles
        .iter()
        .any(|entry| entry.end_tick.is_none() && entry.role_type.eq_ignore_ascii_case(role))
}

fn mark_character_status(character: &mut PersistentCharacter, status: &str, current_tick: u64) {
    for role in character.roles.iter_mut() {
        if role.end_tick.is_none() {
            role.end_tick = Some(current_tick);
        }
    }
    character.roles.push(CharacterRole {
        role_type: status.to_string(),
        faction_id: None,
        rank: None,
        start_tick: current_tick,
        end_tick: None,
    });
}
