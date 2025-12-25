use std::collections::HashMap;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::rules::mastery::MasteryStage;
use crate::rules::power::ExpressionId;
use crate::rules::signature::SignatureType;
use crate::simulation::case::{Case, CaseRegistry, CaseStatus, CaseTargetType};
use crate::simulation::cast::{
    CharacterPersona, CharacterPower, CharacterRelationship, CharacterRole, PersistentCharacter,
    PromotionCandidate,
};
use crate::simulation::city::{
    CityId, CityState, HeatResponse, LocationId, LocationState, LocationTag,
};
use crate::simulation::combat::{CombatIntent, CombatScale, CombatSide, CombatState, Combatant};
use crate::simulation::growth::{ExpressionMastery, GrowthState, Reputation};
use crate::simulation::region::{ContinentId, CountryId, RegionId};
use crate::simulation::storylet_state::StoryletState;
use crate::simulation::time::GameTime;

const WORLD_SCHEMA_VERSION: i64 = 4;
const WORLD_SAVE_VERSION: i64 = 1;

const WORLD_DB_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS world_meta (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  schema_version INTEGER NOT NULL,
  save_version INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS world_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  world_turn INTEGER NOT NULL,
  active_location INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS world_time (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  tick INTEGER NOT NULL,
  day INTEGER NOT NULL,
  hour INTEGER NOT NULL,
  week INTEGER NOT NULL,
  month INTEGER NOT NULL,
  is_day INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS locations (
  location_id INTEGER PRIMARY KEY,
  heat INTEGER NOT NULL,
  crime_pressure INTEGER NOT NULL,
  police_presence INTEGER NOT NULL,
  surveillance_level INTEGER NOT NULL,
  lockdown_level INTEGER NOT NULL,
  police_units INTEGER NOT NULL,
  investigators INTEGER NOT NULL,
  gang_units INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS location_tags (
  location_id INTEGER NOT NULL,
  tag TEXT NOT NULL,
  PRIMARY KEY (location_id, tag)
);

CREATE TABLE IF NOT EXISTS location_faction_influence (
  location_id INTEGER NOT NULL,
  faction_id TEXT NOT NULL,
  influence INTEGER NOT NULL,
  PRIMARY KEY (location_id, faction_id)
);

CREATE TABLE IF NOT EXISTS cases (
  case_id INTEGER PRIMARY KEY,
  faction_id TEXT NOT NULL,
  location_id INTEGER NOT NULL,
  target_type TEXT NOT NULL,
  progress INTEGER NOT NULL,
  heat_lock INTEGER NOT NULL,
  status TEXT NOT NULL,
  milestone INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS case_signatures (
  case_id INTEGER NOT NULL,
  signature_type TEXT NOT NULL,
  PRIMARY KEY (case_id, signature_type)
);

CREATE TABLE IF NOT EXISTS case_pressure_actions (
  case_id INTEGER NOT NULL,
  action TEXT NOT NULL,
  PRIMARY KEY (case_id, action)
);

CREATE TABLE IF NOT EXISTS storylet_fired (
  storylet_id TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS storylet_cooldowns (
  storylet_id TEXT PRIMARY KEY,
  turns INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS storylet_flags (
  flag_key TEXT PRIMARY KEY,
  flag_value INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS storylet_punctuation (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  only INTEGER NOT NULL,
  turns INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS growth_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  pressure_resistance INTEGER NOT NULL,
  trust INTEGER NOT NULL,
  fear INTEGER NOT NULL,
  infamy INTEGER NOT NULL,
  symbolism INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS expression_mastery (
  expression_id TEXT PRIMARY KEY,
  stage TEXT NOT NULL,
  uses INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS unlocked_expressions (
  expression_id TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS combat_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  active INTEGER NOT NULL,
  source TEXT NOT NULL,
  location_id INTEGER NOT NULL,
  scale TEXT NOT NULL,
  tick INTEGER NOT NULL,
  escape_progress INTEGER NOT NULL,
  pending_expression_id TEXT
);

CREATE TABLE IF NOT EXISTS combatants (
  combat_id INTEGER NOT NULL,
  combatant_id INTEGER NOT NULL,
  name TEXT NOT NULL,
  side TEXT NOT NULL,
  stress INTEGER NOT NULL,
  intent TEXT NOT NULL,
  is_player INTEGER NOT NULL,
  PRIMARY KEY (combat_id, combatant_id)
);

CREATE TABLE IF NOT EXISTS characters (
  character_id TEXT PRIMARY KEY,
  scope_id TEXT NOT NULL,
  first_name TEXT NOT NULL,
  last_name TEXT NOT NULL,
  birth_year INTEGER,
  ancestry TEXT,
  nationality TEXT,
  importance_tier TEXT NOT NULL,
  created_at_tick INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS character_roles (
  character_id TEXT NOT NULL,
  role_type TEXT NOT NULL,
  faction_id TEXT,
  rank TEXT,
  start_tick INTEGER NOT NULL,
  end_tick INTEGER
);

CREATE TABLE IF NOT EXISTS character_personas (
  character_id TEXT NOT NULL,
  persona_id TEXT NOT NULL,
  persona_kind TEXT NOT NULL,
  label TEXT NOT NULL,
  is_active_default INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS character_powers (
  character_id TEXT NOT NULL,
  power_id INTEGER NOT NULL,
  expression_id TEXT,
  acq_id TEXT,
  mastery INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS character_relationships (
  a_character_id TEXT NOT NULL,
  b_character_id TEXT NOT NULL,
  relation_type TEXT NOT NULL,
  trust INTEGER NOT NULL,
  fear INTEGER NOT NULL,
  resentment INTEGER NOT NULL,
  is_public INTEGER NOT NULL,
  start_tick INTEGER NOT NULL,
  end_tick INTEGER
);
"#;

#[derive(Debug)]
pub enum WorldDbError {
    Sqlite(rusqlite::Error),
    InvalidData(String),
}

fn response_for_heat(heat: i32) -> HeatResponse {
    if heat >= 70 {
        HeatResponse::FactionAttention
    } else if heat >= 50 {
        HeatResponse::Investigation
    } else if heat >= 30 {
        HeatResponse::PolicePatrol
    } else {
        HeatResponse::None
    }
}

fn location_tag_to_str(tag: LocationTag) -> &'static str {
    match tag {
        LocationTag::Public => "PUBLIC",
        LocationTag::Residential => "RESIDENTIAL",
        LocationTag::Industrial => "INDUSTRIAL",
        LocationTag::HighSecurity => "HIGH_SECURITY",
    }
}

fn location_tag_from_str(tag: &str) -> Option<LocationTag> {
    match tag {
        "PUBLIC" => Some(LocationTag::Public),
        "RESIDENTIAL" => Some(LocationTag::Residential),
        "INDUSTRIAL" => Some(LocationTag::Industrial),
        "HIGH_SECURITY" => Some(LocationTag::HighSecurity),
        _ => None,
    }
}

fn case_status_to_str(status: CaseStatus) -> &'static str {
    match status {
        CaseStatus::Active => "ACTIVE",
        CaseStatus::Resolved => "RESOLVED",
    }
}

fn case_status_from_str(value: &str) -> Result<CaseStatus, WorldDbError> {
    match value {
        "ACTIVE" => Ok(CaseStatus::Active),
        "RESOLVED" => Ok(CaseStatus::Resolved),
        _ => Err(WorldDbError::InvalidData(format!(
            "unknown case status {}",
            value
        ))),
    }
}

fn case_target_to_str(target: CaseTargetType) -> &'static str {
    match target {
        CaseTargetType::UnknownMasked => "UNKNOWN_MASKED",
        CaseTargetType::KnownMasked => "KNOWN_MASKED",
        CaseTargetType::CivilianLink => "CIVILIAN_LINK",
    }
}

fn case_target_from_str(value: &str) -> Result<CaseTargetType, WorldDbError> {
    match value {
        "UNKNOWN_MASKED" => Ok(CaseTargetType::UnknownMasked),
        "KNOWN_MASKED" => Ok(CaseTargetType::KnownMasked),
        "CIVILIAN_LINK" => Ok(CaseTargetType::CivilianLink),
        _ => Err(WorldDbError::InvalidData(format!(
            "unknown case target {}",
            value
        ))),
    }
}

fn signature_type_to_str(sig: SignatureType) -> &'static str {
    match sig {
        SignatureType::VisualAnomaly => "VISUAL_ANOMALY",
        SignatureType::EmSpike => "EM_SPIKE",
        SignatureType::ThermalBloom => "THERMAL_BLOOM",
        SignatureType::AcousticShock => "ACOUSTIC_SHOCK",
        SignatureType::ChemicalResidue => "CHEMICAL_RESIDUE",
        SignatureType::BioMarker => "BIO_MARKER",
        SignatureType::PsychicEcho => "PSYCHIC_ECHO",
        SignatureType::DimensionalResidue => "DIMENSIONAL_RESIDUE",
        SignatureType::GraviticDisturbance => "GRAVITIC_DISTURBANCE",
        SignatureType::ArcaneResonance => "ARCANE_RESONANCE",
        SignatureType::CausalImprint => "CAUSAL_IMPRINT",
        SignatureType::KineticStress => "KINETIC_STRESS",
        SignatureType::RadiationTrace => "RADIATION_TRACE",
    }
}

fn signature_type_from_str(value: &str) -> Option<SignatureType> {
    match value {
        "VISUAL_ANOMALY" => Some(SignatureType::VisualAnomaly),
        "EM_SPIKE" => Some(SignatureType::EmSpike),
        "THERMAL_BLOOM" => Some(SignatureType::ThermalBloom),
        "ACOUSTIC_SHOCK" => Some(SignatureType::AcousticShock),
        "CHEMICAL_RESIDUE" => Some(SignatureType::ChemicalResidue),
        "BIO_MARKER" => Some(SignatureType::BioMarker),
        "PSYCHIC_ECHO" => Some(SignatureType::PsychicEcho),
        "DIMENSIONAL_RESIDUE" => Some(SignatureType::DimensionalResidue),
        "GRAVITIC_DISTURBANCE" => Some(SignatureType::GraviticDisturbance),
        "ARCANE_RESONANCE" => Some(SignatureType::ArcaneResonance),
        "CAUSAL_IMPRINT" => Some(SignatureType::CausalImprint),
        "KINETIC_STRESS" => Some(SignatureType::KineticStress),
        "RADIATION_TRACE" => Some(SignatureType::RadiationTrace),
        _ => None,
    }
}

fn importance_to_str(tier: crate::simulation::cast::ImportanceTier) -> &'static str {
    match tier {
        crate::simulation::cast::ImportanceTier::A => "A",
    }
}

fn importance_from_str(
    value: &str,
) -> Result<crate::simulation::cast::ImportanceTier, WorldDbError> {
    match value {
        "A" => Ok(crate::simulation::cast::ImportanceTier::A),
        _ => Err(WorldDbError::InvalidData(format!(
            "unknown importance tier {}",
            value
        ))),
    }
}

fn mastery_stage_to_str(stage: MasteryStage) -> &'static str {
    match stage {
        MasteryStage::Raw => "RAW",
        MasteryStage::Controlled => "CONTROLLED",
        MasteryStage::Precise => "PRECISE",
        MasteryStage::Silent => "SILENT",
        MasteryStage::Iconic => "ICONIC",
    }
}

fn mastery_stage_from_str(value: &str) -> Result<MasteryStage, WorldDbError> {
    match value {
        "RAW" => Ok(MasteryStage::Raw),
        "CONTROLLED" => Ok(MasteryStage::Controlled),
        "PRECISE" => Ok(MasteryStage::Precise),
        "SILENT" => Ok(MasteryStage::Silent),
        "ICONIC" => Ok(MasteryStage::Iconic),
        _ => Err(WorldDbError::InvalidData(format!(
            "unknown mastery stage {}",
            value
        ))),
    }
}

fn combat_scale_to_str(scale: CombatScale) -> &'static str {
    match scale {
        CombatScale::Street => "STREET",
        CombatScale::District => "DISTRICT",
        CombatScale::City => "CITY",
        CombatScale::National => "NATIONAL",
        CombatScale::Cosmic => "COSMIC",
    }
}

fn combat_scale_from_str(value: &str) -> Result<CombatScale, WorldDbError> {
    match value {
        "STREET" => Ok(CombatScale::Street),
        "DISTRICT" => Ok(CombatScale::District),
        "CITY" => Ok(CombatScale::City),
        "NATIONAL" => Ok(CombatScale::National),
        "COSMIC" => Ok(CombatScale::Cosmic),
        _ => Err(WorldDbError::InvalidData(format!(
            "unknown combat scale {}",
            value
        ))),
    }
}

fn combat_intent_to_str(intent: CombatIntent) -> &'static str {
    match intent {
        CombatIntent::Attack => "ATTACK",
        CombatIntent::Escape => "ESCAPE",
        CombatIntent::Hold => "HOLD",
        CombatIntent::Capture => "CAPTURE",
    }
}

fn combat_intent_from_str(value: &str) -> Result<CombatIntent, WorldDbError> {
    match value {
        "ATTACK" => Ok(CombatIntent::Attack),
        "ESCAPE" => Ok(CombatIntent::Escape),
        "HOLD" => Ok(CombatIntent::Hold),
        "CAPTURE" => Ok(CombatIntent::Capture),
        _ => Err(WorldDbError::InvalidData(format!(
            "unknown combat intent {}",
            value
        ))),
    }
}

fn combat_side_to_str(side: CombatSide) -> &'static str {
    match side {
        CombatSide::Player => "PLAYER",
        CombatSide::Opponent => "OPPONENT",
        CombatSide::Ally => "ALLY",
    }
}

fn combat_side_from_str(value: &str) -> Result<CombatSide, WorldDbError> {
    match value {
        "PLAYER" => Ok(CombatSide::Player),
        "OPPONENT" => Ok(CombatSide::Opponent),
        "ALLY" => Ok(CombatSide::Ally),
        _ => Err(WorldDbError::InvalidData(format!(
            "unknown combat side {}",
            value
        ))),
    }
}

impl std::fmt::Display for WorldDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorldDbError::Sqlite(err) => write!(f, "sqlite error: {}", err),
            WorldDbError::InvalidData(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for WorldDbError {}

impl From<rusqlite::Error> for WorldDbError {
    fn from(err: rusqlite::Error) -> Self {
        Self::Sqlite(err)
    }
}

#[derive(Debug, Clone)]
pub struct WorldDbState {
    pub world_turn: u64,
    pub game_time: GameTime,
    pub city: CityState,
    pub cases: CaseRegistry,
    pub combat: CombatState,
    pub growth: GrowthState,
    pub storylet_state: StoryletState,
}

impl Default for WorldDbState {
    fn default() -> Self {
        let city = CityState::default();
        let mut combat = CombatState::default();
        combat.location_id = city.active_location;
        Self {
            world_turn: 0,
            game_time: GameTime::default(),
            city,
            cases: CaseRegistry::default(),
            combat,
            growth: GrowthState::default(),
            storylet_state: StoryletState::default(),
        }
    }
}

pub struct WorldDb {
    conn: Connection,
}

impl WorldDb {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, WorldDbError> {
        let conn = Connection::open(path)?;
        let mut db = Self { conn };
        db.conn.execute_batch(WORLD_DB_SCHEMA)?;
        db.ensure_world_meta()?;
        Ok(db)
    }

    pub fn load_or_init(&mut self) -> Result<WorldDbState, WorldDbError> {
        if let Some(state) = self.load_state()? {
            Ok(state)
        } else {
            let state = WorldDbState::default();
            self.save_state(&state)?;
            Ok(state)
        }
    }

    pub fn load_state(&self) -> Result<Option<WorldDbState>, WorldDbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT world_turn, active_location FROM world_state WHERE id = 1")?;
        let mut rows = stmt.query([])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        let world_turn: u64 = row.get::<_, i64>(0)? as u64;
        let active_location = LocationId(row.get::<_, i64>(1)? as u32);

        let game_time = self.load_game_time()?;
        let mut city = self.load_city()?;
        city.active_location = active_location;
        let cases = self.load_cases()?;
        let combat = self.load_combat_state(active_location)?;
        let growth = self.load_growth_state()?;
        let storylet_state = self.load_storylet_state()?;

        Ok(Some(WorldDbState {
            world_turn,
            game_time,
            city,
            cases,
            combat,
            growth,
            storylet_state,
        }))
    }

    pub fn save_state(&mut self, state: &WorldDbState) -> Result<(), WorldDbError> {
        let tx = self.conn.transaction()?;

        tx.execute("DELETE FROM world_state", [])?;
        tx.execute(
            "INSERT INTO world_state (id, world_turn, active_location) VALUES (1, ?1, ?2)",
            params![state.world_turn as i64, state.city.active_location.0 as i64],
        )?;

        tx.execute("DELETE FROM world_time", [])?;
        tx.execute(
            "INSERT INTO world_time (id, tick, day, hour, week, month, is_day) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                state.game_time.tick as i64,
                state.game_time.day as i64,
                state.game_time.hour as i64,
                state.game_time.week as i64,
                state.game_time.month as i64,
                if state.game_time.is_day { 1 } else { 0 }
            ],
        )?;

        tx.execute("DELETE FROM locations", [])?;
        tx.execute("DELETE FROM location_tags", [])?;
        tx.execute("DELETE FROM location_faction_influence", [])?;
        for location in state.city.locations.values() {
            tx.execute(
                "INSERT INTO locations (location_id, heat, crime_pressure, police_presence, surveillance_level, lockdown_level, police_units, investigators, gang_units) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    location.id.0 as i64,
                    location.heat,
                    location.crime_pressure,
                    location.police_presence,
                    location.surveillance_level,
                    location.lockdown_level,
                    location.police_units as i64,
                    location.investigators as i64,
                    location.gang_units as i64
                ],
            )?;
            for tag in &location.tags {
                tx.execute(
                    "INSERT INTO location_tags (location_id, tag) VALUES (?1, ?2)",
                    params![location.id.0 as i64, location_tag_to_str(*tag)],
                )?;
            }
            for (faction_id, influence) in &location.faction_influence {
                tx.execute(
                    "INSERT INTO location_faction_influence (location_id, faction_id, influence) VALUES (?1, ?2, ?3)",
                    params![location.id.0 as i64, faction_id, *influence as i64],
                )?;
            }
        }

        tx.execute("DELETE FROM cases", [])?;
        tx.execute("DELETE FROM case_signatures", [])?;
        tx.execute("DELETE FROM case_pressure_actions", [])?;
        for case in &state.cases.cases {
            tx.execute(
                "INSERT INTO cases (case_id, faction_id, location_id, target_type, progress, heat_lock, status, milestone) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    case.case_id as i64,
                    case.faction_id,
                    case.location_id.0 as i64,
                    case_target_to_str(case.target_type),
                    case.progress as i64,
                    if case.heat_lock { 1 } else { 0 },
                    case_status_to_str(case.status),
                    case.milestone as i64
                ],
            )?;
            for sig in &case.signature_pattern {
                tx.execute(
                    "INSERT INTO case_signatures (case_id, signature_type) VALUES (?1, ?2)",
                    params![case.case_id as i64, signature_type_to_str(*sig)],
                )?;
            }
            for action in &case.pressure_actions {
                tx.execute(
                    "INSERT INTO case_pressure_actions (case_id, action) VALUES (?1, ?2)",
                    params![case.case_id as i64, action],
                )?;
            }
        }

        tx.execute("DELETE FROM storylet_fired", [])?;
        tx.execute("DELETE FROM storylet_cooldowns", [])?;
        tx.execute("DELETE FROM storylet_flags", [])?;
        tx.execute("DELETE FROM storylet_punctuation", [])?;
        for storylet_id in &state.storylet_state.fired {
            tx.execute(
                "INSERT INTO storylet_fired (storylet_id) VALUES (?1)",
                params![storylet_id],
            )?;
        }
        for (storylet_id, turns) in &state.storylet_state.cooldowns {
            tx.execute(
                "INSERT INTO storylet_cooldowns (storylet_id, turns) VALUES (?1, ?2)",
                params![storylet_id, *turns as i64],
            )?;
        }
        for (flag, value) in &state.storylet_state.flags {
            tx.execute(
                "INSERT INTO storylet_flags (flag_key, flag_value) VALUES (?1, ?2)",
                params![flag, if *value { 1 } else { 0 }],
            )?;
        }
        tx.execute(
            "INSERT INTO storylet_punctuation (id, only, turns) VALUES (1, ?1, ?2)",
            params![
                if state.storylet_state.punctuation.only {
                    1
                } else {
                    0
                },
                state.storylet_state.punctuation.remaining_turns as i64
            ],
        )?;

        tx.execute("DELETE FROM growth_state", [])?;
        tx.execute("DELETE FROM expression_mastery", [])?;
        tx.execute("DELETE FROM unlocked_expressions", [])?;
        tx.execute(
            "INSERT INTO growth_state (id, pressure_resistance, trust, fear, infamy, symbolism) VALUES (1, ?1, ?2, ?3, ?4, ?5)",
            params![
                state.growth.pressure_resistance,
                state.growth.reputation.trust,
                state.growth.reputation.fear,
                state.growth.reputation.infamy,
                state.growth.reputation.symbolism,
            ],
        )?;
        for (expr_id, mastery) in &state.growth.mastery {
            tx.execute(
                "INSERT INTO expression_mastery (expression_id, stage, uses) VALUES (?1, ?2, ?3)",
                params![
                    expr_id.0.as_str(),
                    mastery_stage_to_str(mastery.stage),
                    mastery.uses as i64,
                ],
            )?;
        }
        for expr_id in &state.growth.unlocked_expressions {
            tx.execute(
                "INSERT INTO unlocked_expressions (expression_id) VALUES (?1)",
                params![expr_id.0.as_str()],
            )?;
        }

        tx.execute("DELETE FROM combat_state", [])?;
        tx.execute("DELETE FROM combatants", [])?;
        tx.execute(
            "INSERT INTO combat_state (id, active, source, location_id, scale, tick, escape_progress, pending_expression_id) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                if state.combat.active { 1 } else { 0 },
                state.combat.source.as_str(),
                state.combat.location_id.0 as i64,
                combat_scale_to_str(state.combat.scale),
                state.combat.tick as i64,
                state.combat.escape_progress as i64,
                state
                    .combat
                    .pending_player_expression
                    .as_ref()
                    .map(|id| id.0.clone()),
            ],
        )?;
        for combatant in &state.combat.combatants {
            tx.execute(
                "INSERT INTO combatants (combat_id, combatant_id, name, side, stress, intent, is_player) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    combatant.id as i64,
                    combatant.name.as_str(),
                    combat_side_to_str(combatant.side),
                    combatant.stress,
                    combat_intent_to_str(combatant.intent),
                    if combatant.is_player { 1 } else { 0 },
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    fn ensure_world_meta(&mut self) -> Result<(), WorldDbError> {
        let meta = self
            .conn
            .query_row(
                "SELECT schema_version, save_version FROM world_meta WHERE id = 1",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?;

        match meta {
            Some((schema_version, save_version)) => {
                if schema_version == WORLD_SCHEMA_VERSION && save_version == WORLD_SAVE_VERSION {
                    return Ok(());
                }
                if (schema_version == 1 || schema_version == 2 || schema_version == 3)
                    && save_version == WORLD_SAVE_VERSION
                {
                    self.conn.execute(
                        "UPDATE world_meta SET schema_version = ?1, save_version = ?2 WHERE id = 1",
                        params![WORLD_SCHEMA_VERSION, WORLD_SAVE_VERSION],
                    )?;
                    return Ok(());
                }
                return Err(WorldDbError::InvalidData(format!(
                    "world_meta version mismatch (schema {}, save {}, expected {}, {})",
                    schema_version, save_version, WORLD_SCHEMA_VERSION, WORLD_SAVE_VERSION
                )));
            }
            None => {
                self.conn.execute(
                    "INSERT INTO world_meta (id, schema_version, save_version) VALUES (1, ?1, ?2)",
                    params![WORLD_SCHEMA_VERSION, WORLD_SAVE_VERSION],
                )?;
            }
        }

        Ok(())
    }

    pub fn load_characters(&self) -> Result<Vec<PersistentCharacter>, WorldDbError> {
        let roles = self.load_roles()?;
        let personas = self.load_personas()?;
        let powers = self.load_powers()?;
        let relationships = self.load_relationships()?;

        let mut stmt = self.conn.prepare(
            "SELECT character_id, scope_id, first_name, last_name, birth_year, ancestry, nationality, importance_tier, created_at_tick FROM characters",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<i32>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, String>(7)?,
                row.get::<_, i64>(8)? as u64,
            ))
        })?;

        let mut characters = Vec::new();
        for row in rows {
            let (
                character_id,
                scope_id,
                first_name,
                last_name,
                birth_year,
                ancestry,
                nationality,
                importance_tier,
                created_at_tick,
            ) = row?;
            let importance_tier = importance_from_str(&importance_tier)?;
            characters.push(PersistentCharacter {
                character_id: character_id.clone(),
                scope_id,
                first_name,
                last_name,
                birth_year,
                ancestry,
                nationality,
                importance_tier,
                created_at_tick,
                roles: roles.get(&character_id).cloned().unwrap_or_default(),
                personas: personas.get(&character_id).cloned().unwrap_or_default(),
                powers: powers.get(&character_id).cloned().unwrap_or_default(),
                relationships: relationships
                    .get(&character_id)
                    .cloned()
                    .unwrap_or_default(),
            });
        }
        Ok(characters)
    }

    pub fn upsert_character(
        &mut self,
        character: &PersistentCharacter,
    ) -> Result<(), WorldDbError> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO characters (character_id, scope_id, first_name, last_name, birth_year, ancestry, nationality, importance_tier, created_at_tick) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                character.character_id,
                character.scope_id,
                character.first_name,
                character.last_name,
                character.birth_year,
                character.ancestry,
                character.nationality,
                importance_to_str(character.importance_tier),
                character.created_at_tick as i64
            ],
        )?;

        tx.execute(
            "DELETE FROM character_roles WHERE character_id = ?1",
            params![character.character_id],
        )?;
        tx.execute(
            "DELETE FROM character_personas WHERE character_id = ?1",
            params![character.character_id],
        )?;
        tx.execute(
            "DELETE FROM character_powers WHERE character_id = ?1",
            params![character.character_id],
        )?;
        tx.execute(
            "DELETE FROM character_relationships WHERE a_character_id = ?1",
            params![character.character_id],
        )?;

        for role in &character.roles {
            tx.execute(
                "INSERT INTO character_roles (character_id, role_type, faction_id, rank, start_tick, end_tick) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    character.character_id,
                    role.role_type,
                    role.faction_id,
                    role.rank,
                    role.start_tick as i64,
                    role.end_tick.map(|v| v as i64),
                ],
            )?;
        }
        for persona in &character.personas {
            tx.execute(
                "INSERT INTO character_personas (character_id, persona_id, persona_kind, label, is_active_default) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    character.character_id,
                    persona.persona_id,
                    persona.persona_kind,
                    persona.label,
                    if persona.is_active_default { 1 } else { 0 }
                ],
            )?;
        }
        for power in &character.powers {
            tx.execute(
                "INSERT INTO character_powers (character_id, power_id, expression_id, acq_id, mastery) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    character.character_id,
                    power.power_id,
                    power.expression_id,
                    power.acq_id,
                    power.mastery
                ],
            )?;
        }
        for relation in &character.relationships {
            tx.execute(
                "INSERT INTO character_relationships (a_character_id, b_character_id, relation_type, trust, fear, resentment, is_public, start_tick, end_tick) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    character.character_id,
                    relation.other_character_id,
                    relation.relation_type,
                    relation.trust,
                    relation.fear,
                    relation.resentment,
                    if relation.is_public { 1 } else { 0 },
                    relation.start_tick as i64,
                    relation.end_tick.map(|v| v as i64),
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn promote_candidate(
        &mut self,
        candidate: &PromotionCandidate,
        created_at_tick: u64,
    ) -> Result<PersistentCharacter, WorldDbError> {
        let character_id = self.next_character_id()?;
        let character = candidate.to_character(character_id, created_at_tick);
        self.upsert_character(&character)?;
        Ok(character)
    }

    fn next_character_id(&self) -> Result<String, WorldDbError> {
        let next_id: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(rowid), 0) + 1 FROM characters",
            [],
            |row| row.get(0),
        )?;
        Ok(format!("char_{}", next_id))
    }

    fn load_game_time(&self) -> Result<GameTime, WorldDbError> {
        let mut stmt = self
            .conn
            .prepare("SELECT tick, day, hour, week, month, is_day FROM world_time WHERE id = 1")?;
        let mut rows = stmt.query([])?;
        let Some(row) = rows.next()? else {
            return Ok(GameTime::default());
        };
        let tick = row.get::<_, i64>(0)? as u64;
        let day = row.get::<_, i64>(1)? as u32;
        let hour = row.get::<_, i64>(2)? as u8;
        let week = row.get::<_, i64>(3)? as u32;
        let month = row.get::<_, i64>(4)? as u32;
        let is_day = row.get::<_, i64>(5)? != 0;
        Ok(GameTime {
            tick,
            day,
            hour,
            week,
            month,
            is_day,
        })
    }

    fn load_growth_state(&self) -> Result<GrowthState, WorldDbError> {
        let row = self
            .conn
            .query_row(
                "SELECT pressure_resistance, trust, fear, infamy, symbolism FROM growth_state WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)? as i32,
                        row.get::<_, i64>(1)? as i32,
                        row.get::<_, i64>(2)? as i32,
                        row.get::<_, i64>(3)? as i32,
                        row.get::<_, i64>(4)? as i32,
                    ))
                },
            )
            .optional()?;

        let mut state = GrowthState::default();
        if let Some((pressure_resistance, trust, fear, infamy, symbolism)) = row {
            state.pressure_resistance = pressure_resistance;
            state.reputation = Reputation {
                trust,
                fear,
                infamy,
                symbolism,
            };
        }
        state.mastery = self.load_expression_mastery()?;
        state.unlocked_expressions = self.load_unlocked_expressions()?;
        Ok(state)
    }

    fn load_expression_mastery(
        &self,
    ) -> Result<HashMap<ExpressionId, ExpressionMastery>, WorldDbError> {
        let mut map = HashMap::new();
        let mut stmt = self
            .conn
            .prepare("SELECT expression_id, stage, uses FROM expression_mastery")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)? as u32,
            ))
        })?;
        for row in rows {
            let (expr_id, stage, uses) = row?;
            map.insert(
                ExpressionId(expr_id),
                ExpressionMastery {
                    stage: mastery_stage_from_str(&stage)?,
                    uses,
                },
            );
        }
        Ok(map)
    }

    fn load_unlocked_expressions(
        &self,
    ) -> Result<std::collections::HashSet<ExpressionId>, WorldDbError> {
        let mut set = std::collections::HashSet::new();
        let mut stmt = self
            .conn
            .prepare("SELECT expression_id FROM unlocked_expressions")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            set.insert(ExpressionId(row?));
        }
        Ok(set)
    }

    fn load_combat_state(&self, active_location: LocationId) -> Result<CombatState, WorldDbError> {
        let row = self
            .conn
            .query_row(
                "SELECT active, source, location_id, scale, tick, escape_progress, pending_expression_id FROM combat_state WHERE id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, i64>(4)?,
                        row.get::<_, i64>(5)?,
                        row.get::<_, Option<String>>(6)?,
                    ))
                },
            )
            .optional()?;

        let mut state = CombatState::default();
        state.location_id = active_location;

        let Some((active, source, location_id, scale, tick, escape_progress, pending_expr)) = row
        else {
            return Ok(state);
        };

        state.active = active != 0;
        state.source = source;
        state.location_id = LocationId(location_id as u32);
        state.scale = combat_scale_from_str(&scale)?;
        state.tick = tick as u64;
        state.escape_progress = escape_progress as u8;
        state.pending_player_expression = pending_expr.map(ExpressionId);
        state.combatants = self.load_combatants()?;
        Ok(state)
    }

    fn load_combatants(&self) -> Result<Vec<Combatant>, WorldDbError> {
        let mut out = Vec::new();
        let mut stmt = self.conn.prepare(
            "SELECT combatant_id, name, side, stress, intent, is_player FROM combatants WHERE combat_id = 1 ORDER BY combatant_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)? as u32,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)? as i32,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)? != 0,
            ))
        })?;
        for row in rows {
            let (id, name, side, stress, intent, is_player) = row?;
            out.push(Combatant {
                id,
                name,
                side: combat_side_from_str(&side)?,
                stress,
                intent: combat_intent_from_str(&intent)?,
                is_player,
            });
        }
        Ok(out)
    }

    fn load_city(&self) -> Result<CityState, WorldDbError> {
        let mut stmt = self.conn.prepare(
            "SELECT location_id, heat, crime_pressure, police_presence, surveillance_level, lockdown_level, police_units, investigators, gang_units FROM locations",
        )?;
        let mut rows = stmt.query([])?;
        let mut locations: HashMap<LocationId, LocationState> = HashMap::new();
        while let Some(row) = rows.next()? {
            let location_id = LocationId(row.get::<_, i64>(0)? as u32);
            let heat = row.get::<_, i64>(1)? as i32;
            let crime_pressure = row.get::<_, i64>(2)? as i32;
            let police_presence = row.get::<_, i64>(3)? as i32;
            let surveillance_level = row.get::<_, i64>(4)? as i32;
            let lockdown_level = row.get::<_, i64>(5)? as i32;
            let police_units = row.get::<_, i64>(6)? as u8;
            let investigators = row.get::<_, i64>(7)? as u8;
            let gang_units = row.get::<_, i64>(8)? as u8;

            let tags = self.load_location_tags(location_id)?;
            let influence = self.load_location_influence(location_id)?;

            locations.insert(
                location_id,
                LocationState {
                    id: location_id,
                    tags,
                    heat,
                    crime_pressure,
                    police_presence,
                    surveillance_level,
                    lockdown_level,
                    police_units,
                    investigators,
                    gang_units,
                    faction_influence: influence,
                    response: response_for_heat(heat),
                },
            );
        }

        if locations.is_empty() {
            return Ok(CityState::default());
        }

        Ok(CityState {
            city_id: CityId(1),
            region_id: RegionId(1),
            country_id: CountryId(1),
            continent_id: ContinentId(1),
            locations,
            active_location: LocationId(1),
        })
    }

    fn load_location_tags(
        &self,
        location_id: LocationId,
    ) -> Result<Vec<LocationTag>, WorldDbError> {
        let mut tags = Vec::new();
        let mut stmt = self
            .conn
            .prepare("SELECT tag FROM location_tags WHERE location_id = ?1")?;
        let rows = stmt.query_map(params![location_id.0 as i64], |row| {
            let tag: String = row.get(0)?;
            Ok(tag)
        })?;
        for row in rows {
            if let Some(tag) = location_tag_from_str(&row?) {
                tags.push(tag);
            }
        }
        Ok(tags)
    }

    fn load_location_influence(
        &self,
        location_id: LocationId,
    ) -> Result<HashMap<String, u16>, WorldDbError> {
        let mut influence = HashMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT faction_id, influence FROM location_faction_influence WHERE location_id = ?1",
        )?;
        let rows = stmt.query_map(params![location_id.0 as i64], |row| {
            let faction_id: String = row.get(0)?;
            let value: i64 = row.get(1)?;
            Ok((faction_id, value as u16))
        })?;
        for row in rows {
            let (faction_id, value) = row?;
            influence.insert(faction_id, value);
        }
        Ok(influence)
    }

    fn load_cases(&self) -> Result<CaseRegistry, WorldDbError> {
        let mut registry = CaseRegistry::default();
        let mut stmt = self.conn.prepare(
            "SELECT case_id, faction_id, location_id, target_type, progress, heat_lock, status, milestone FROM cases",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)? as u32,
                row.get::<_, String>(1)?,
                LocationId(row.get::<_, i64>(2)? as u32),
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)? as u32,
                row.get::<_, i64>(5)? != 0,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)? as u8,
            ))
        })?;

        for row in rows {
            let (
                case_id,
                faction_id,
                location_id,
                target_type,
                progress,
                heat_lock,
                status,
                milestone,
            ) = row?;
            let target_type = case_target_from_str(&target_type)?;
            let status = case_status_from_str(&status)?;
            let signature_pattern = self.load_case_signatures(case_id)?;
            let pressure_actions = self.load_case_actions(case_id)?;
            registry.cases.push(Case {
                case_id,
                faction_id,
                location_id,
                target_type,
                signature_pattern,
                progress,
                heat_lock,
                status,
                milestone,
                pressure_actions,
            });
        }
        registry.sync_next_id();
        Ok(registry)
    }

    fn load_case_signatures(&self, case_id: u32) -> Result<Vec<SignatureType>, WorldDbError> {
        let mut out = Vec::new();
        let mut stmt = self
            .conn
            .prepare("SELECT signature_type FROM case_signatures WHERE case_id = ?1")?;
        let rows = stmt.query_map(params![case_id as i64], |row| {
            let sig: String = row.get(0)?;
            Ok(sig)
        })?;
        for row in rows {
            if let Some(sig) = signature_type_from_str(&row?) {
                out.push(sig);
            }
        }
        Ok(out)
    }

    fn load_case_actions(&self, case_id: u32) -> Result<Vec<String>, WorldDbError> {
        let mut out = Vec::new();
        let mut stmt = self
            .conn
            .prepare("SELECT action FROM case_pressure_actions WHERE case_id = ?1")?;
        let rows = stmt.query_map(params![case_id as i64], |row| row.get(0))?;
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    fn load_storylet_state(&self) -> Result<StoryletState, WorldDbError> {
        let mut state = StoryletState::default();

        let mut stmt = self
            .conn
            .prepare("SELECT storylet_id FROM storylet_fired")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for row in rows {
            state.fired.insert(row?);
        }

        let mut stmt = self
            .conn
            .prepare("SELECT storylet_id, turns FROM storylet_cooldowns")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as i32))
        })?;
        for row in rows {
            let (id, turns) = row?;
            state.cooldowns.insert(id, turns);
        }

        let mut stmt = self
            .conn
            .prepare("SELECT flag_key, flag_value FROM storylet_flags")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? != 0))
        })?;
        for row in rows {
            let (key, value) = row?;
            state.flags.insert(key, value);
        }

        if let Some((only, turns)) = self
            .conn
            .query_row(
                "SELECT only, turns FROM storylet_punctuation WHERE id = 1",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?
        {
            state.punctuation.only = only != 0;
            state.punctuation.remaining_turns = turns as i32;
        }

        Ok(state)
    }

    fn load_roles(&self) -> Result<HashMap<String, Vec<CharacterRole>>, WorldDbError> {
        let mut map: HashMap<String, Vec<CharacterRole>> = HashMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT character_id, role_type, faction_id, rank, start_tick, end_tick FROM character_roles",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                CharacterRole {
                    role_type: row.get(1)?,
                    faction_id: row.get(2)?,
                    rank: row.get(3)?,
                    start_tick: row.get::<_, i64>(4)? as u64,
                    end_tick: row.get::<_, Option<i64>>(5)?.map(|v| v as u64),
                },
            ))
        })?;
        for row in rows {
            let (character_id, role) = row?;
            map.entry(character_id).or_default().push(role);
        }
        Ok(map)
    }

    fn load_personas(&self) -> Result<HashMap<String, Vec<CharacterPersona>>, WorldDbError> {
        let mut map: HashMap<String, Vec<CharacterPersona>> = HashMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT character_id, persona_id, persona_kind, label, is_active_default FROM character_personas",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                CharacterPersona {
                    persona_id: row.get(1)?,
                    persona_kind: row.get(2)?,
                    label: row.get(3)?,
                    is_active_default: row.get::<_, i64>(4)? != 0,
                },
            ))
        })?;
        for row in rows {
            let (character_id, persona) = row?;
            map.entry(character_id).or_default().push(persona);
        }
        Ok(map)
    }

    fn load_powers(&self) -> Result<HashMap<String, Vec<CharacterPower>>, WorldDbError> {
        let mut map: HashMap<String, Vec<CharacterPower>> = HashMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT character_id, power_id, expression_id, acq_id, mastery FROM character_powers",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                CharacterPower {
                    power_id: row.get::<_, i64>(1)?,
                    expression_id: row.get(2)?,
                    acq_id: row.get(3)?,
                    mastery: row.get::<_, i64>(4)? as i32,
                },
            ))
        })?;
        for row in rows {
            let (character_id, power) = row?;
            map.entry(character_id).or_default().push(power);
        }
        Ok(map)
    }

    fn load_relationships(
        &self,
    ) -> Result<HashMap<String, Vec<CharacterRelationship>>, WorldDbError> {
        let mut map: HashMap<String, Vec<CharacterRelationship>> = HashMap::new();
        let mut stmt = self.conn.prepare(
            "SELECT a_character_id, b_character_id, relation_type, trust, fear, resentment, is_public, start_tick, end_tick FROM character_relationships",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                CharacterRelationship {
                    other_character_id: row.get(1)?,
                    relation_type: row.get(2)?,
                    trust: row.get::<_, i64>(3)? as i32,
                    fear: row.get::<_, i64>(4)? as i32,
                    resentment: row.get::<_, i64>(5)? as i32,
                    is_public: row.get::<_, i64>(6)? != 0,
                    start_tick: row.get::<_, i64>(7)? as u64,
                    end_tick: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
                },
            ))
        })?;
        for row in rows {
            let (character_id, relation) = row?;
            map.entry(character_id).or_default().push(relation);
        }
        Ok(map)
    }
}

impl crate::world::repository::WorldRepository for WorldDb {
    fn load_or_init(&mut self) -> Result<WorldDbState, Box<dyn std::error::Error>> {
        Ok(WorldDb::load_or_init(self)?)
    }

    fn save_state(&mut self, state: &WorldDbState) -> Result<(), Box<dyn std::error::Error>> {
        Ok(WorldDb::save_state(self, state)?)
    }

    fn load_characters(&self) -> Result<Vec<PersistentCharacter>, Box<dyn std::error::Error>> {
        Ok(WorldDb::load_characters(self)?)
    }

    fn upsert_character(
        &mut self,
        character: &PersistentCharacter,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(WorldDb::upsert_character(self, character)?)
    }

    fn promote_candidate(
        &mut self,
        candidate: &PromotionCandidate,
        created_at_tick: u64,
    ) -> Result<PersistentCharacter, Box<dyn std::error::Error>> {
        Ok(WorldDb::promote_candidate(
            self,
            candidate,
            created_at_tick,
        )?)
    }
}
