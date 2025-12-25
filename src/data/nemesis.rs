use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::rules::signature::SignatureType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NemesisActionCatalog {
    pub schema_version: u32,
    pub actions: Vec<NemesisActionDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NemesisActionDefinition {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub min_heat: u32,
    #[serde(default)]
    pub min_case_progress: u32,
    #[serde(default)]
    pub signature_traces: Vec<NemesisSignatureTrace>,
    #[serde(default)]
    pub pressure_delta: NemesisPressureDelta,
    #[serde(default)]
    pub case_progress_delta: i32,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NemesisSignatureTrace {
    pub signature_type: SignatureType,
    pub strength: i64,
    pub persistence_turns: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NemesisPressureDelta {
    #[serde(default)]
    pub temporal: f32,
    #[serde(default)]
    pub identity: f32,
    #[serde(default)]
    pub institutional: f32,
    #[serde(default)]
    pub moral: f32,
    #[serde(default)]
    pub resource: f32,
    #[serde(default)]
    pub psychological: f32,
}

#[derive(Debug)]
pub enum DataError {
    Io { path: String, source: std::io::Error },
    Json { path: String, source: serde_json::Error },
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataError::Io { path, source } => {
                write!(f, "failed to read {}: {}", path, source)
            }
            DataError::Json { path, source } => {
                write!(f, "failed to parse {}: {}", path, source)
            }
        }
    }
}

impl std::error::Error for DataError {}

pub fn load_nemesis_action_catalog(path: impl AsRef<Path>) -> Result<NemesisActionCatalog, DataError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| DataError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let catalog: NemesisActionCatalog =
        serde_json::from_str(&raw).map_err(|source| DataError::Json {
            path: path.display().to_string(),
            source,
        })?;
    Ok(catalog)
}
