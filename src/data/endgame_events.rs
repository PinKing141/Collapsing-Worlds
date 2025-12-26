use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndgameEventCatalog {
    pub schema_version: u32,
    pub events: Vec<EndgameEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndgameEvent {
    pub id: String,
    pub phase: EndgamePhase,
    pub title: String,
    pub text_stub: String,
    pub choices: Vec<EndgameChoice>,
    #[serde(default)]
    pub effects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndgameChoice {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub effects: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EndgamePhase {
    Exposed,
    Registration,
    CosmicJudgement,
    Ascension,
    Exile,
}

#[derive(Debug)]
pub enum EndgameEventDataError {
    Io { path: String, source: std::io::Error },
    Json { path: String, source: serde_json::Error },
    Validation(String),
}

impl std::fmt::Display for EndgameEventDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EndgameEventDataError::Io { path, source } => {
                write!(f, "failed to read {}: {}", path, source)
            }
            EndgameEventDataError::Json { path, source } => {
                write!(f, "failed to parse {}: {}", path, source)
            }
            EndgameEventDataError::Validation(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for EndgameEventDataError {}

pub fn load_endgame_event_catalog(
    path: impl AsRef<Path>,
) -> Result<EndgameEventCatalog, EndgameEventDataError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| EndgameEventDataError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let catalog: EndgameEventCatalog =
        serde_json::from_str(&raw).map_err(|source| EndgameEventDataError::Json {
            path: path.display().to_string(),
            source,
        })?;
    catalog.validate()?;
    Ok(catalog)
}

impl EndgameEventCatalog {
    pub fn validate(&self) -> Result<(), EndgameEventDataError> {
        let mut ids = HashSet::new();
        for event in &self.events {
            if event.id.trim().is_empty() {
                return Err(EndgameEventDataError::Validation(
                    "endgame event id cannot be empty".to_string(),
                ));
            }
            if !ids.insert(event.id.clone()) {
                return Err(EndgameEventDataError::Validation(format!(
                    "duplicate endgame event id {}",
                    event.id
                )));
            }
            if event.title.trim().is_empty() {
                return Err(EndgameEventDataError::Validation(format!(
                    "endgame event {} missing title",
                    event.id
                )));
            }
            if event.text_stub.trim().is_empty() {
                return Err(EndgameEventDataError::Validation(format!(
                    "endgame event {} missing text_stub",
                    event.id
                )));
            }
            if event.choices.is_empty() {
                return Err(EndgameEventDataError::Validation(format!(
                    "endgame event {} has no choices",
                    event.id
                )));
            }
        }
        Ok(())
    }
}
