use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilianEventCatalog {
    pub schema_version: u32,
    pub events: Vec<CivilianStorylet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilianStorylet {
    pub id: String,
    pub title: String,
    pub text_stub: String,
    pub choices: Vec<CivilianChoice>,
    #[serde(default)]
    pub effects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivilianChoice {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub effects: Vec<String>,
}

#[derive(Debug)]
pub enum CivilianEventDataError {
    Io { path: String, source: std::io::Error },
    Json { path: String, source: serde_json::Error },
    Validation(String),
}

impl std::fmt::Display for CivilianEventDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CivilianEventDataError::Io { path, source } => {
                write!(f, "failed to read {}: {}", path, source)
            }
            CivilianEventDataError::Json { path, source } => {
                write!(f, "failed to parse {}: {}", path, source)
            }
            CivilianEventDataError::Validation(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for CivilianEventDataError {}

pub fn load_civilian_event_catalog(
    path: impl AsRef<Path>,
) -> Result<CivilianEventCatalog, CivilianEventDataError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| CivilianEventDataError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let catalog: CivilianEventCatalog =
        serde_json::from_str(&raw).map_err(|source| CivilianEventDataError::Json {
            path: path.display().to_string(),
            source,
        })?;
    catalog.validate()?;
    Ok(catalog)
}

impl CivilianEventCatalog {
    pub fn validate(&self) -> Result<(), CivilianEventDataError> {
        let mut ids = HashSet::new();
        for event in &self.events {
            if event.id.trim().is_empty() {
                return Err(CivilianEventDataError::Validation(
                    "civilian event id cannot be empty".to_string(),
                ));
            }
            if !ids.insert(event.id.clone()) {
                return Err(CivilianEventDataError::Validation(format!(
                    "duplicate civilian event id {}",
                    event.id
                )));
            }
            if event.title.trim().is_empty() {
                return Err(CivilianEventDataError::Validation(format!(
                    "civilian event {} missing title",
                    event.id
                )));
            }
            if event.text_stub.trim().is_empty() {
                return Err(CivilianEventDataError::Validation(format!(
                    "civilian event {} missing text_stub",
                    event.id
                )));
            }
            if event.choices.is_empty() {
                return Err(CivilianEventDataError::Validation(format!(
                    "civilian event {} has no choices",
                    event.id
                )));
            }
        }
        Ok(())
    }
}
