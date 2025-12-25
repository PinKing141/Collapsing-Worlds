use serde::{Deserialize, Serialize};

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
