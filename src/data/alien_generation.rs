use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

const DEFAULT_ALIEN_GENERATION_PATH: &str = "./assets/data/alien_generation.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlienGenerationCatalog {
    pub schema_version: u32,
    pub naming: AlienNamingCatalog,
    pub tables: AlienGenerationTables,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlienNamingCatalog {
    pub root_prefixes: Vec<String>,
    pub root_cores: Vec<String>,
    pub root_suffixes: Vec<String>,
    pub world_suffixes: Vec<String>,
    pub species_suffixes: Vec<String>,
    pub demonym_suffixes: Vec<String>,
    pub house_prefixes: Vec<String>,
    pub house_suffixes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlienGenerationTables {
    pub naming_style: Vec<AlienTableEntry>,
    pub star_archetype: Vec<AlienTableEntry>,
    pub origin: Vec<AlienTableEntry>,
    pub gravity: Vec<AlienTableEntry>,
    pub atmosphere: Vec<AlienTableEntry>,
    pub climate: Vec<AlienTableEntry>,
    pub terrain: Vec<AlienTableEntry>,
    pub body_plan: Vec<AlienTableEntry>,
    pub scale: Vec<AlienTableEntry>,
    pub covering: Vec<AlienTableEntry>,
    pub pigmentation: Vec<AlienTableEntry>,
    pub feature: Vec<AlienTableEntry>,
    pub senses: Vec<AlienTableEntry>,
    pub locomotion: Vec<AlienTableEntry>,
    pub adaptation: Vec<AlienTableEntry>,
    pub society: Vec<AlienTableEntry>,
    pub values: Vec<AlienTableEntry>,
    pub technology: Vec<AlienTableEntry>,
    pub interstellar_role: Vec<AlienTableEntry>,
    pub cosmic_tier: Vec<AlienTableEntry>,
    pub power_source: Vec<AlienTableEntry>,
    pub signature_gift: Vec<AlienTableEntry>,
    pub flight_style: Vec<AlienTableEntry>,
    pub weakness: Vec<AlienTableEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlienTableEntry {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "d6")]
    pub d6: Option<u32>,
    #[serde(rename = "2d6")]
    pub d2d6: Option<u32>,
    #[serde(rename = "d66")]
    pub d66: Option<u32>,
    pub label: String,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug)]
pub enum AlienGenerationError {
    Io { path: String, source: std::io::Error },
    Json { path: String, source: serde_json::Error },
    Validation(String),
}

impl std::fmt::Display for AlienGenerationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlienGenerationError::Io { path, source } => {
                write!(f, "failed to read {}: {}", path, source)
            }
            AlienGenerationError::Json { path, source } => {
                write!(f, "failed to parse {}: {}", path, source)
            }
            AlienGenerationError::Validation(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for AlienGenerationError {}

pub fn load_default_alien_generation_catalog() -> Result<AlienGenerationCatalog, AlienGenerationError> {
    load_alien_generation_catalog(DEFAULT_ALIEN_GENERATION_PATH)
}

pub fn load_alien_generation_catalog(
    path: impl AsRef<Path>,
) -> Result<AlienGenerationCatalog, AlienGenerationError> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path).map_err(|source| AlienGenerationError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let catalog: AlienGenerationCatalog =
        serde_json::from_str(&raw).map_err(|source| AlienGenerationError::Json {
            path: path.display().to_string(),
            source,
        })?;
    catalog.validate()?;
    Ok(catalog)
}

impl AlienGenerationCatalog {
    pub fn validate(&self) -> Result<(), AlienGenerationError> {
        if self.schema_version == 0 {
            return Err(AlienGenerationError::Validation(
                "alien generation schema_version must be >= 1".to_string(),
            ));
        }
        self.naming.validate()?;
        self.tables.validate()?;
        Ok(())
    }
}

impl AlienNamingCatalog {
    fn validate(&self) -> Result<(), AlienGenerationError> {
        if self.root_prefixes.is_empty()
            || self.root_cores.is_empty()
            || self.root_suffixes.is_empty()
            || self.world_suffixes.is_empty()
            || self.species_suffixes.is_empty()
            || self.demonym_suffixes.is_empty()
        {
            return Err(AlienGenerationError::Validation(
                "alien naming fragments cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

impl AlienGenerationTables {
    fn validate(&self) -> Result<(), AlienGenerationError> {
        let tables = [
            ("naming_style", &self.naming_style),
            ("star_archetype", &self.star_archetype),
            ("origin", &self.origin),
            ("gravity", &self.gravity),
            ("atmosphere", &self.atmosphere),
            ("climate", &self.climate),
            ("terrain", &self.terrain),
            ("body_plan", &self.body_plan),
            ("scale", &self.scale),
            ("covering", &self.covering),
            ("pigmentation", &self.pigmentation),
            ("feature", &self.feature),
            ("senses", &self.senses),
            ("locomotion", &self.locomotion),
            ("adaptation", &self.adaptation),
            ("society", &self.society),
            ("values", &self.values),
            ("technology", &self.technology),
            ("interstellar_role", &self.interstellar_role),
            ("cosmic_tier", &self.cosmic_tier),
            ("power_source", &self.power_source),
            ("signature_gift", &self.signature_gift),
            ("flight_style", &self.flight_style),
            ("weakness", &self.weakness),
        ];
        for (label, table) in tables {
            if table.is_empty() {
                return Err(AlienGenerationError::Validation(format!(
                    "alien table {} cannot be empty",
                    label
                )));
            }
        }
        Ok(())
    }
}

