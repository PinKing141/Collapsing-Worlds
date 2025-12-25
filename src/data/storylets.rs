use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryletCatalog {
    pub schema_version: u32,
    pub storylets: Vec<Storylet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storylet {
    pub id: String,
    pub category: StoryletCategory,
    #[serde(default)]
    pub preconditions: Vec<String>,
    pub text_stub: String,
    pub choices: Vec<StoryletChoice>,
    #[serde(default)]
    pub effects: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryletChoice {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub effects: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StoryletCategory {
    CivilianLife,
    MaskedLife,
    InstitutionalPressure,
    VillainOpportunities,
}

#[derive(Debug)]
pub enum StoryletDataError {
    Io { path: String, source: std::io::Error },
    Json { path: String, source: serde_json::Error },
    Validation(String),
}

impl std::fmt::Display for StoryletDataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoryletDataError::Io { path, source } => {
                write!(f, "failed to read {}: {}", path, source)
            }
            StoryletDataError::Json { path, source } => {
                write!(f, "failed to parse {}: {}", path, source)
            }
            StoryletDataError::Validation(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for StoryletDataError {}

pub fn load_storylet_catalog(path: impl AsRef<Path>) -> Result<StoryletCatalog, StoryletDataError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| StoryletDataError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let catalog: StoryletCatalog =
        serde_json::from_str(&raw).map_err(|source| StoryletDataError::Json {
            path: path.display().to_string(),
            source,
        })?;
    catalog.validate()?;
    Ok(catalog)
}

impl StoryletCatalog {
    pub fn validate(&self) -> Result<(), StoryletDataError> {
        let mut ids = HashSet::new();
        for storylet in &self.storylets {
            if storylet.id.trim().is_empty() {
                return Err(StoryletDataError::Validation(
                    "storylet id cannot be empty".to_string(),
                ));
            }
            if !ids.insert(storylet.id.clone()) {
                return Err(StoryletDataError::Validation(format!(
                    "duplicate storylet id {}",
                    storylet.id
                )));
            }
            if storylet.text_stub.trim().is_empty() {
                return Err(StoryletDataError::Validation(format!(
                    "storylet {} missing text_stub",
                    storylet.id
                )));
            }
            if storylet.choices.is_empty() {
                return Err(StoryletDataError::Validation(format!(
                    "storylet {} has no choices",
                    storylet.id
                )));
            }
        }
        Ok(())
    }
}
