use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::content::{PowerId, PowerRepository};
use crate::data::omni_powers::{OmniPowerCatalog, OmniPowerDefinition};

const DEFAULT_OMNI_DENOMINATOR: u64 = 2_000_000;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct OmniPowerRegistry {
    power_holders: HashMap<String, String>,
    holder_powers: HashMap<String, HashSet<String>>,
    universe_holders: HashMap<String, String>,
    omnipresent_holder: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct OmniPowerAssignment {
    pub assigned: Vec<OmniPowerSelection>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OmniPowerSelection {
    pub omni_id: String,
    pub power_id: PowerId,
}

#[derive(Debug, Clone, Copy)]
pub struct OmniRollConfig {
    pub denominator: u64,
}

impl Default for OmniRollConfig {
    fn default() -> Self {
        Self {
            denominator: DEFAULT_OMNI_DENOMINATOR,
        }
    }
}

#[derive(Debug)]
pub enum OmniPowerError {
    Repository(String),
    InvalidConfig(String),
}

impl std::fmt::Display for OmniPowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OmniPowerError::Repository(message) => write!(f, "repository error: {}", message),
            OmniPowerError::InvalidConfig(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for OmniPowerError {}

impl OmniPowerRegistry {
    pub fn holder_for_universe(&self, universe_id: &str) -> Option<&str> {
        self.universe_holders.get(universe_id).map(String::as_str)
    }

    pub fn powers_for_holder(&self, holder_id: &str) -> Vec<String> {
        self.holder_powers
            .get(holder_id)
            .map(|powers| powers.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn holder_ids(&self) -> Vec<String> {
        self.holder_powers.keys().cloned().collect()
    }

    pub fn holder_powers(&self) -> Vec<(String, Vec<String>)> {
        self.holder_powers
            .iter()
            .map(|(holder, powers)| (holder.clone(), powers.iter().cloned().collect()))
            .collect()
    }

    pub fn universe_holders(&self) -> Vec<(String, String)> {
        self.universe_holders
            .iter()
            .map(|(universe, holder)| (universe.clone(), holder.clone()))
            .collect()
    }

    pub fn is_power_claimed(&self, omni_id: &str) -> bool {
        self.power_holders.contains_key(omni_id)
    }

    pub fn mark_holder_dead(&mut self, holder_id: &str) {
        self.holder_powers.remove(holder_id);
        self.universe_holders
            .retain(|_, id| id != holder_id);
        self.power_holders.retain(|_, id| id != holder_id);
        if self
            .omnipresent_holder
            .as_deref()
            .map(|id| id == holder_id)
            .unwrap_or(false)
        {
            self.omnipresent_holder = None;
        }
    }

    pub fn assign_omni_powers(
        &mut self,
        repo: &dyn PowerRepository,
        catalog: &OmniPowerCatalog,
        holder_id: &str,
        universe_id: &str,
        rng: &mut u64,
        config: OmniRollConfig,
    ) -> Result<OmniPowerAssignment, OmniPowerError> {
        if config.denominator == 0 {
            return Err(OmniPowerError::InvalidConfig(
                "omni roll denominator must be > 0".to_string(),
            ));
        }

        if !self.can_assign_in_universe(holder_id, universe_id) {
            return Ok(OmniPowerAssignment {
                assigned: Vec::new(),
                notes: vec![format!(
                    "Universe {} already has a different omni holder.",
                    universe_id
                )],
            });
        }

        let mut assignment = OmniPowerAssignment::default();
        for omni in &catalog.powers {
            if self.power_holders.contains_key(&omni.id) {
                continue;
            }

            if roll_denominator(rng, config.denominator) {
                match repo.power_id_by_name(&omni.power_name) {
                    Ok(Some(power_id)) => {
                        if self.claim_power(omni, holder_id, universe_id) {
                            assignment.assigned.push(OmniPowerSelection {
                                omni_id: omni.id.clone(),
                                power_id,
                            });
                        }
                    }
                    Ok(None) => {
                        assignment.notes.push(format!(
                            "Omni power {} missing from DB.",
                            omni.power_name
                        ));
                    }
                    Err(err) => {
                        return Err(OmniPowerError::Repository(err.to_string()));
                    }
                }
            }
        }

        Ok(assignment)
    }

    fn can_assign_in_universe(&self, holder_id: &str, universe_id: &str) -> bool {
        if let Some(existing) = self.omnipresent_holder.as_deref() {
            return existing == holder_id;
        }
        match self.universe_holders.get(universe_id) {
            None => true,
            Some(existing) => existing == holder_id,
        }
    }

    fn claim_power(
        &mut self,
        omni: &OmniPowerDefinition,
        holder_id: &str,
        universe_id: &str,
    ) -> bool {
        if let Some(existing) = self.power_holders.get(&omni.id) {
            if existing != holder_id {
                return false;
            }
        }

        self.power_holders
            .insert(omni.id.clone(), holder_id.to_string());
        self.holder_powers
            .entry(holder_id.to_string())
            .or_default()
            .insert(omni.id.clone());
        self.universe_holders
            .entry(universe_id.to_string())
            .or_insert_with(|| holder_id.to_string());
        if omni.id == "omnipresence" {
            self.omnipresent_holder = Some(holder_id.to_string());
        }
        true
    }
}

fn roll_denominator(rng: &mut u64, denom: u64) -> bool {
    next_u64(rng) % denom == 0
}

fn next_u64(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1);
    *state
}
