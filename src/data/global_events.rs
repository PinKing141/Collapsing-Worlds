use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::simulation::region::GlobalEscalation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalEventCatalog {
    pub schema_version: u32,
    pub events: Vec<GlobalEventDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalEventDefinition {
    pub id: String,
    pub title: String,
    pub text_stub: String,
    pub min_escalation: GlobalEscalation,
    #[serde(default)]
    pub max_escalation: Option<GlobalEscalation>,
    #[serde(default)]
    pub cooldown_days: u32,
    pub choices: Vec<GlobalEventChoice>,
    #[serde(default)]
    pub effects: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalEventChoice {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub effects: Vec<String>,
}

#[derive(Debug)]
pub enum GlobalEventDataError {
    Io { path: String, source: std::io::Error },
    Json { path: String, source: serde_json::Error },
    Validation(String),
}

impl std::fmt::Display for GlobalEventDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlobalEventDataError::Io { path, source } => {
                write!(f, "failed to read {}: {}", path, source)
            }
            GlobalEventDataError::Json { path, source } => {
                write!(f, "failed to parse {}: {}", path, source)
            }
            GlobalEventDataError::Validation(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for GlobalEventDataError {}

pub fn load_global_event_catalog(
    path: impl AsRef<Path>,
) -> Result<GlobalEventCatalog, GlobalEventDataError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| GlobalEventDataError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let catalog: GlobalEventCatalog =
        serde_json::from_str(&raw).map_err(|source| GlobalEventDataError::Json {
            path: path.display().to_string(),
            source,
        })?;
    catalog.validate()?;
    Ok(catalog)
}

impl GlobalEventCatalog {
    pub fn validate(&self) -> Result<(), GlobalEventDataError> {
        let mut ids = HashSet::new();
        for event in &self.events {
            if event.id.trim().is_empty() {
                return Err(GlobalEventDataError::Validation(
                    "global event id cannot be empty".to_string(),
                ));
            }
            if !ids.insert(event.id.clone()) {
                return Err(GlobalEventDataError::Validation(format!(
                    "duplicate global event id {}",
                    event.id
                )));
            }
            if event.title.trim().is_empty() {
                return Err(GlobalEventDataError::Validation(format!(
                    "global event {} missing title",
                    event.id
                )));
            }
            if event.text_stub.trim().is_empty() {
                return Err(GlobalEventDataError::Validation(format!(
                    "global event {} missing text_stub",
                    event.id
                )));
            }
            if event.choices.is_empty() {
                return Err(GlobalEventDataError::Validation(format!(
                    "global event {} has no choices",
                    event.id
                )));
            }
        }
        Ok(())
    }
}
