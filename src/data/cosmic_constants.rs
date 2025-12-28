use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

const DEFAULT_COSMIC_CONSTANTS_PATH: &str = "./assets/data/cosmic_constants.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmicConstantsCatalog {
    pub schema_version: u32,
    pub constants: Vec<CosmicConstantDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmicConstantDefinition {
    pub id: String,
    pub label: String,
    pub role: String,
    pub order: u32,
    pub body_count: u32,
    pub hive_mind: bool,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug)]
pub enum CosmicConstantsError {
    Io { path: String, source: std::io::Error },
    Json { path: String, source: serde_json::Error },
    Validation(String),
}

impl std::fmt::Display for CosmicConstantsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CosmicConstantsError::Io { path, source } => {
                write!(f, "failed to read {}: {}", path, source)
            }
            CosmicConstantsError::Json { path, source } => {
                write!(f, "failed to parse {}: {}", path, source)
            }
            CosmicConstantsError::Validation(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for CosmicConstantsError {}

pub fn load_default_cosmic_constants() -> Result<CosmicConstantsCatalog, CosmicConstantsError> {
    load_cosmic_constants(DEFAULT_COSMIC_CONSTANTS_PATH)
}

pub fn load_cosmic_constants(
    path: impl AsRef<Path>,
) -> Result<CosmicConstantsCatalog, CosmicConstantsError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| CosmicConstantsError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let catalog: CosmicConstantsCatalog =
        serde_json::from_str(&raw).map_err(|source| CosmicConstantsError::Json {
            path: path.display().to_string(),
            source,
        })?;
    catalog.validate()?;
    Ok(catalog)
}

impl CosmicConstantsCatalog {
    pub fn validate(&self) -> Result<(), CosmicConstantsError> {
        if self.schema_version == 0 {
            return Err(CosmicConstantsError::Validation(
                "cosmic constants schema_version must be >= 1".to_string(),
            ));
        }
        if self.constants.is_empty() {
            return Err(CosmicConstantsError::Validation(
                "cosmic constants cannot be empty".to_string(),
            ));
        }
        let mut ids = HashSet::new();
        for constant in &self.constants {
            if constant.id.trim().is_empty() {
                return Err(CosmicConstantsError::Validation(
                    "cosmic constant id cannot be empty".to_string(),
                ));
            }
            if !ids.insert(constant.id.clone()) {
                return Err(CosmicConstantsError::Validation(format!(
                    "duplicate cosmic constant id {}",
                    constant.id
                )));
            }
            if constant.label.trim().is_empty() {
                return Err(CosmicConstantsError::Validation(format!(
                    "cosmic constant {} missing label",
                    constant.id
                )));
            }
        }
        Ok(())
    }
}

