use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::rules::signature::SignatureType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionTypeCatalog {
    pub schema_version: u32,
    pub faction_types: Vec<FactionTypeDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionInstanceCatalog {
    pub schema_version: u32,
    pub factions: Vec<FactionInstanceDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionTypeDefinition {
    pub id: String,
    pub name: String,
    pub domain: FactionDomain,
    pub default_jurisdiction: Jurisdiction,
    pub detection: DetectionProfile,
    pub response: ResponseProfile,
    #[serde(default)]
    pub narrative_pressure: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactionInstanceDefinition {
    pub id: String,
    pub type_id: String,
    pub jurisdiction: Jurisdiction,
    #[serde(default)]
    pub scope: FactionScope,
    #[serde(default)]
    pub influence: Vec<LocationInfluence>,
    #[serde(default)]
    pub detection_override: Option<DetectionProfile>,
    #[serde(default)]
    pub response_override: Option<ResponseProfile>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FactionScope {
    #[serde(default)]
    pub location_ids: Vec<u32>,
    #[serde(default)]
    pub location_tags: Vec<LocationTag>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LocationInfluence {
    pub location_id: u32,
    pub influence: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionProfile {
    #[serde(default)]
    pub signature_types: Vec<SignatureType>,
    #[serde(default)]
    pub heat_min: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseProfile {
    #[serde(default)]
    pub doctrine: Vec<ResponseDoctrine>,
    #[serde(default)]
    pub thresholds: Vec<ResponseThreshold>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseThreshold {
    pub heat: u32,
    pub level: String,
    #[serde(default)]
    pub actions: Vec<ResponseAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseAction {
    pub kind: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug)]
pub enum DataError {
    Io { path: String, source: std::io::Error },
    Json { path: String, source: serde_json::Error },
    Validation(String),
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
            DataError::Validation(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for DataError {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FactionDomain {
    Law,
    Military,
    Corporate,
    Criminal,
    Occult,
    Civic,
    Cosmic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Jurisdiction {
    Local,
    Regional,
    National,
    Global,
    Planetary,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResponseDoctrine {
    Arrest,
    Suppress,
    Exploit,
    Study,
    Eliminate,
    Contain,
    Disrupt,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LocationTag {
    Public,
    Residential,
    Industrial,
    HighSecurity,
}

pub struct FactionData {
    pub types: FactionTypeCatalog,
    pub instances: FactionInstanceCatalog,
}

pub fn load_faction_type_catalog(path: impl AsRef<Path>) -> Result<FactionTypeCatalog, DataError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| DataError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let catalog: FactionTypeCatalog =
        serde_json::from_str(&raw).map_err(|source| DataError::Json {
            path: path.display().to_string(),
            source,
        })?;
    catalog.validate()?;
    Ok(catalog)
}

pub fn load_faction_instance_catalog(
    path: impl AsRef<Path>,
) -> Result<FactionInstanceCatalog, DataError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| DataError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let catalog: FactionInstanceCatalog =
        serde_json::from_str(&raw).map_err(|source| DataError::Json {
            path: path.display().to_string(),
            source,
        })?;
    catalog.validate()?;
    Ok(catalog)
}

pub fn load_faction_data(
    types_path: impl AsRef<Path>,
    instances_path: impl AsRef<Path>,
) -> Result<FactionData, DataError> {
    let types = load_faction_type_catalog(types_path)?;
    let instances = load_faction_instance_catalog(instances_path)?;
    instances.validate_against(&types)?;
    Ok(FactionData { types, instances })
}

impl FactionTypeCatalog {
    pub fn validate(&self) -> Result<(), DataError> {
        ensure_unique_ids(
            "faction type",
            self.faction_types.iter().map(|def| def.id.as_str()),
        )?;
        for def in &self.faction_types {
            if def.id.trim().is_empty() {
                return Err(DataError::Validation(
                    "faction type id cannot be empty".to_string(),
                ));
            }
            if def.name.trim().is_empty() {
                return Err(DataError::Validation(format!(
                    "faction type {} has empty name",
                    def.id
                )));
            }
        }
        Ok(())
    }
}

impl FactionInstanceCatalog {
    pub fn validate(&self) -> Result<(), DataError> {
        ensure_unique_ids(
            "faction instance",
            self.factions.iter().map(|def| def.id.as_str()),
        )?;
        for def in &self.factions {
            if def.id.trim().is_empty() {
                return Err(DataError::Validation(
                    "faction instance id cannot be empty".to_string(),
                ));
            }
            if def.type_id.trim().is_empty() {
                return Err(DataError::Validation(format!(
                    "faction instance {} has empty type_id",
                    def.id
                )));
            }
        }
        Ok(())
    }

    pub fn validate_against(&self, types: &FactionTypeCatalog) -> Result<(), DataError> {
        let type_ids: HashSet<&str> = types
            .faction_types
            .iter()
            .map(|def| def.id.as_str())
            .collect();
        for instance in &self.factions {
            if !type_ids.contains(instance.type_id.as_str()) {
                return Err(DataError::Validation(format!(
                    "faction instance {} references unknown type_id {}",
                    instance.id, instance.type_id
                )));
            }
        }
        Ok(())
    }
}

fn ensure_unique_ids<'a>(
    label: &str,
    ids: impl Iterator<Item = &'a str>,
) -> Result<(), DataError> {
    let mut seen = HashSet::new();
    for id in ids {
        if !seen.insert(id) {
            return Err(DataError::Validation(format!(
                "duplicate {} id {}",
                label, id
            )));
        }
    }
    Ok(())
}
