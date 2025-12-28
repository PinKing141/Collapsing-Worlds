use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

const DEFAULT_OMNI_POWERS_PATH: &str = "./assets/data/omni_powers.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniPowerCatalog {
    pub schema_version: u32,
    pub powers: Vec<OmniPowerDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmniPowerDefinition {
    pub id: String,
    pub label: String,
    pub power_name: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug)]
pub enum OmniPowerError {
    Io { path: String, source: std::io::Error },
    Json { path: String, source: serde_json::Error },
    Validation(String),
}

impl std::fmt::Display for OmniPowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OmniPowerError::Io { path, source } => {
                write!(f, "failed to read {}: {}", path, source)
            }
            OmniPowerError::Json { path, source } => {
                write!(f, "failed to parse {}: {}", path, source)
            }
            OmniPowerError::Validation(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for OmniPowerError {}

pub fn load_default_omni_powers() -> Result<OmniPowerCatalog, OmniPowerError> {
    load_omni_powers(DEFAULT_OMNI_POWERS_PATH)
}

pub fn load_omni_powers(path: impl AsRef<Path>) -> Result<OmniPowerCatalog, OmniPowerError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| OmniPowerError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let catalog: OmniPowerCatalog =
        serde_json::from_str(&raw).map_err(|source| OmniPowerError::Json {
            path: path.display().to_string(),
            source,
        })?;
    catalog.validate()?;
    Ok(catalog)
}

impl OmniPowerCatalog {
    pub fn validate(&self) -> Result<(), OmniPowerError> {
        if self.schema_version == 0 {
            return Err(OmniPowerError::Validation(
                "omni power schema_version must be >= 1".to_string(),
            ));
        }
        if self.powers.is_empty() {
            return Err(OmniPowerError::Validation(
                "omni power list cannot be empty".to_string(),
            ));
        }
        let mut ids = HashSet::new();
        for power in &self.powers {
            if power.id.trim().is_empty() {
                return Err(OmniPowerError::Validation(
                    "omni power id cannot be empty".to_string(),
                ));
            }
            if !ids.insert(power.id.clone()) {
                return Err(OmniPowerError::Validation(format!(
                    "duplicate omni power id {}",
                    power.id
                )));
            }
            if power.label.trim().is_empty() {
                return Err(OmniPowerError::Validation(format!(
                    "omni power {} missing label",
                    power.id
                )));
            }
            if power.power_name.trim().is_empty() {
                return Err(OmniPowerError::Validation(format!(
                    "omni power {} missing power_name",
                    power.id
                )));
            }
        }
        Ok(())
    }
}

