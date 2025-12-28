use std::collections::{HashMap, HashSet};

use crate::content::{PowerId, PowerRepository};
use crate::data::omni_powers::OmniPowerCatalog;
use crate::simulation::alien::AlienSpeciesProfile;
use crate::simulation::cosmic::{OmniPowerRegistry, OmniRollConfig};

const DEFAULT_EXPRESSION_MIN: u32 = 1;
const DEFAULT_EXPRESSION_MAX: u32 = 2;
const DEFAULT_PARENT_MUTANT_CHANCE: u32 = 2;
const DEFAULT_PARENT_HOMOZYGOUS_CHANCE: u32 = 4;
const DEFAULT_OMEGA_PARENT_CHANCE: u32 = 2;
const DEFAULT_PARENT_POWERS_SINGLE_MIN: u32 = 1;
const DEFAULT_PARENT_POWERS_SINGLE_MAX: u32 = 2;
const DEFAULT_PARENT_POWERS_DUAL_MIN: u32 = 2;
const DEFAULT_PARENT_POWERS_DUAL_MAX: u32 = 4;

#[derive(Debug, Clone)]
pub struct PowerAssignmentConfig {
    pub expression_min: u32,
    pub expression_max: u32,
    pub omni_roll: OmniRollConfig,
}

impl Default for PowerAssignmentConfig {
    fn default() -> Self {
        Self {
            expression_min: DEFAULT_EXPRESSION_MIN,
            expression_max: DEFAULT_EXPRESSION_MAX,
            omni_roll: OmniRollConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MutantInheritanceProfile {
    pub parent_powers: Vec<PowerId>,
    pub omega_parent: bool,
}

#[derive(Debug, Clone)]
pub struct MutantLineageOutcome {
    pub mutant_gene: bool,
    pub inheritance: Option<MutantInheritanceProfile>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PowerAssignmentResult {
    pub baseline: Option<PowerId>,
    pub expressions: Vec<PowerId>,
    pub omni: Vec<PowerId>,
    pub notes: Vec<String>,
}

#[derive(Debug)]
pub enum PowerAssignmentError {
    Repository(String),
    InvalidConfig(String),
}

impl std::fmt::Display for PowerAssignmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowerAssignmentError::Repository(message) => write!(f, "repository error: {}", message),
            PowerAssignmentError::InvalidConfig(message) => write!(f, "{}", message),
        }
    }
}

impl std::error::Error for PowerAssignmentError {}

pub fn assign_alien_powers(
    repo: &dyn PowerRepository,
    profile: &AlienSpeciesProfile,
    omni_catalog: &OmniPowerCatalog,
    omni_registry: &mut OmniPowerRegistry,
    holder_id: &str,
    universe_id: &str,
    seed: u64,
    config: PowerAssignmentConfig,
) -> Result<PowerAssignmentResult, PowerAssignmentError> {
    validate_config(&config)?;
    let mut rng = seed ^ hash_seed("alien_power_assignment");
    let mut notes = Vec::new();
    let affinity = derive_alien_affinity(profile);
    notes.extend(affinity.notes);

    let mut baseline = pick_power(repo, &affinity.baseline_tags, &affinity.blocked_tags, &mut rng)?;
    if baseline.is_none() {
        baseline =
            pick_power(repo, &affinity.affinity_tags, &affinity.blocked_tags, &mut rng)?;
    }
    if baseline.is_none() {
        notes.push("No baseline power found for species tags.".to_string());
    }

    let mut selected: HashSet<PowerId> = HashSet::new();
    if let Some(id) = baseline {
        selected.insert(id);
    }

    let target_count = roll_range(&mut rng, config.expression_min, config.expression_max);
    let expressions = pick_expression_powers(
        repo,
        &affinity.affinity_tags,
        &affinity.blocked_tags,
        &mut rng,
        target_count,
        &mut selected,
    )?;

    let omni_assignment = omni_registry
        .assign_omni_powers(
            repo,
            omni_catalog,
            holder_id,
            universe_id,
            &mut rng,
            config.omni_roll,
        )
        .map_err(|err| PowerAssignmentError::Repository(err.to_string()))?;

    for note in omni_assignment.notes {
        notes.push(note);
    }
    let omni: Vec<PowerId> = omni_assignment
        .assigned
        .iter()
        .map(|entry| entry.power_id)
        .collect();

    Ok(PowerAssignmentResult {
        baseline,
        expressions,
        omni,
        notes,
    })
}

pub fn assign_mutant_powers(
    repo: &dyn PowerRepository,
    omni_catalog: &OmniPowerCatalog,
    omni_registry: &mut OmniPowerRegistry,
    holder_id: &str,
    universe_id: &str,
    seed: u64,
    config: PowerAssignmentConfig,
    inheritance: Option<MutantInheritanceProfile>,
) -> Result<PowerAssignmentResult, PowerAssignmentError> {
    validate_config(&config)?;
    let mut rng = seed ^ hash_seed("mutant_power_assignment");
    let mut notes = Vec::new();

    let mut affinity = PowerAffinity::default();
    affinity.affinity_tags.extend(mutant_affinity_tags());

    let mut inherited: Vec<PowerId> = Vec::new();
    let mut omega = false;
    if let Some(heritage) = inheritance {
        omega = heritage.omega_parent;
        let inherit_chance = if omega { 70 } else { 50 };
        for parent_power in heritage.parent_powers {
            if roll_percent(&mut rng, inherit_chance) {
                inherited.push(parent_power);
            }
            if let Ok(tags) = repo.power_tags(parent_power) {
                for tag in tags {
                    affinity.affinity_tags.push(tag.to_ascii_lowercase());
                }
            }
        }
        if omega {
            notes.push("Omega inheritance increases power count.".to_string());
        }
    }
    affinity.blocked_tags.extend(mutant_blocked_tags(omega));
    affinity.affinity_tags = dedupe_lowercase(affinity.affinity_tags);
    affinity.blocked_tags = dedupe_lowercase(affinity.blocked_tags);

    let mut baseline = inherited.get(0).copied();
    if baseline.is_none() {
        baseline =
            pick_power(repo, &affinity.affinity_tags, &affinity.blocked_tags, &mut rng)?;
    }
    if baseline.is_none() {
        notes.push("No baseline power found for mutant tags.".to_string());
    }

    let mut selected: HashSet<PowerId> = HashSet::new();
    if let Some(id) = baseline {
        selected.insert(id);
    }
    for power in &inherited {
        selected.insert(*power);
    }

    let mut target_count = roll_range(&mut rng, config.expression_min, config.expression_max);
    if omega {
        target_count += 1;
        if roll_percent(&mut rng, 25) {
            target_count += 1;
        }
    }
    let mut expressions = inherited;
    let extra = pick_expression_powers(
        repo,
        &affinity.affinity_tags,
        &affinity.blocked_tags,
        &mut rng,
        target_count.saturating_sub(expressions.len() as u32),
        &mut selected,
    )?;
    expressions.extend(extra);

    let omni_assignment = omni_registry
        .assign_omni_powers(
            repo,
            omni_catalog,
            holder_id,
            universe_id,
            &mut rng,
            config.omni_roll,
        )
        .map_err(|err| PowerAssignmentError::Repository(err.to_string()))?;
    for note in omni_assignment.notes {
        notes.push(note);
    }
    let omni: Vec<PowerId> = omni_assignment
        .assigned
        .iter()
        .map(|entry| entry.power_id)
        .collect();

    Ok(PowerAssignmentResult {
        baseline,
        expressions,
        omni,
        notes,
    })
}

pub fn roll_mutant_lineage(
    repo: &dyn PowerRepository,
    seed: u64,
) -> Result<MutantLineageOutcome, PowerAssignmentError> {
    let mut rng = seed ^ hash_seed("mutant_lineage");
    let mut notes = Vec::new();
    let parent_a = roll_parent_genotype(&mut rng);
    let parent_b = roll_parent_genotype(&mut rng);
    let mutant_parents = parent_a.is_mutant() as u8 + parent_b.is_mutant() as u8;
    if mutant_parents == 0 {
        notes.push("Lineage roll: no mutant parents.".to_string());
        return Ok(MutantLineageOutcome {
            mutant_gene: false,
            inheritance: None,
            notes,
        });
    }

    let mutant_gene = child_inherits_mutation(&mut rng, parent_a, parent_b);
    if !mutant_gene {
        notes.push("Lineage roll: mutation not inherited.".to_string());
        return Ok(MutantLineageOutcome {
            mutant_gene: false,
            inheritance: None,
            notes,
        });
    }

    let inheritance = seed_mutant_inheritance(repo, seed, mutant_parents)?;
    if inheritance.omega_parent {
        notes.push("Lineage roll: omega parent detected.".to_string());
    }

    Ok(MutantLineageOutcome {
        mutant_gene: true,
        inheritance: Some(inheritance),
        notes,
    })
}

pub fn seed_mutant_inheritance(
    repo: &dyn PowerRepository,
    seed: u64,
    mutant_parent_count: u8,
) -> Result<MutantInheritanceProfile, PowerAssignmentError> {
    if mutant_parent_count == 0 {
        return Ok(MutantInheritanceProfile {
            parent_powers: Vec::new(),
            omega_parent: false,
        });
    }
    let mut rng = seed ^ hash_seed("mutant_inheritance");
    let mut omega_parent = false;
    for _ in 0..mutant_parent_count {
        if roll_percent(&mut rng, DEFAULT_OMEGA_PARENT_CHANCE) {
            omega_parent = true;
            break;
        }
    }

    let power_target = roll_parent_power_count(&mut rng, mutant_parent_count, omega_parent);
    let mut selected = HashSet::new();
    let mut parent_powers = pick_expression_powers(
        repo,
        &mutant_affinity_tags(),
        &mutant_blocked_tags(omega_parent),
        &mut rng,
        power_target,
        &mut selected,
    )?;

    if parent_powers.is_empty() && power_target > 0 {
        if let Some(power) = pick_power(
            repo,
            &mutant_affinity_tags(),
            &mutant_blocked_tags(omega_parent),
            &mut rng,
        )? {
            parent_powers.push(power);
        }
    }

    Ok(MutantInheritanceProfile {
        parent_powers,
        omega_parent,
    })
}

pub fn classify_mutant_tier(
    repo: &dyn PowerRepository,
    baseline: Option<PowerId>,
    expressions: &[PowerId],
    omni: &[PowerId],
    omega_parent: bool,
) -> Result<u8, PowerAssignmentError> {
    let mut powers: HashSet<PowerId> = HashSet::new();
    if let Some(id) = baseline {
        powers.insert(id);
    }
    for power in expressions {
        powers.insert(*power);
    }
    for power in omni {
        powers.insert(*power);
    }

    let mut tags: HashSet<String> = HashSet::new();
    for power_id in &powers {
        let power_tags = repo
            .power_tags(*power_id)
            .map_err(|err| PowerAssignmentError::Repository(err.to_string()))?;
        for tag in power_tags {
            tags.insert(tag.to_ascii_lowercase());
        }
    }

    if !omni.is_empty() || tags.contains("tier:apex") || tags.contains("omni") {
        return Ok(7);
    }
    if tags.contains("tier:cosmic") || tags.contains("cosmic") {
        return Ok(6);
    }
    if omega_parent {
        return Ok(6);
    }

    let count = powers.len();
    let tier = match count {
        0 => 0,
        1 | 2 => 1,
        3 => 2,
        4 => 3,
        5 => 4,
        _ => 5,
    };
    Ok(tier)
}

#[derive(Debug, Default, Clone)]
struct PowerAffinity {
    baseline_tags: Vec<String>,
    affinity_tags: Vec<String>,
    blocked_tags: Vec<String>,
    notes: Vec<String>,
}

fn derive_alien_affinity(profile: &AlienSpeciesProfile) -> PowerAffinity {
    let mut affinity = PowerAffinity::default();
    affinity.baseline_tags = extract_power_tags(&profile.capabilities.signature_gift.tags);
    affinity.affinity_tags.extend(affinity.baseline_tags.clone());

    let mut allowed_domains: HashSet<String> = HashSet::new();
    let mut has_cosmic = false;
    let mut has_apex = false;

    for tag in &profile.tags {
        let tag = tag.to_ascii_lowercase();
        if let Some(domain) = tag.strip_prefix("domain:") {
            allowed_domains.insert(domain.to_string());
        }
        if tag == "tier:cosmic" {
            has_cosmic = true;
        }
        if tag == "tier:apex" {
            has_cosmic = true;
            has_apex = true;
        }

        match tag.as_str() {
            "gravity:low" => affinity.affinity_tags.extend(vec![
                "speed".to_string(),
                "flight".to_string(),
            ]),
            "gravity:high" => affinity.affinity_tags.extend(vec![
                "strength".to_string(),
                "durability".to_string(),
            ]),
            "gravity:extreme" => affinity.affinity_tags.extend(vec![
                "strength".to_string(),
                "durability".to_string(),
                "density".to_string(),
            ]),
            "gravity:variable" => affinity.affinity_tags.push("gravity".to_string()),
            "phys:strengthened" => affinity.affinity_tags.extend(vec![
                "strength".to_string(),
                "powerful".to_string(),
                "durability".to_string(),
            ]),
            "phys:massive" => affinity.affinity_tags.extend(vec![
                "strength".to_string(),
                "durability".to_string(),
                "density".to_string(),
            ]),
            "phys:fragile" => affinity.affinity_tags.extend(vec![
                "speed".to_string(),
                "agility".to_string(),
            ]),
            "body:lithic" => affinity.affinity_tags.extend(vec![
                "durability".to_string(),
                "density".to_string(),
            ]),
            "body:energy" => affinity.affinity_tags.extend(vec![
                "energy".to_string(),
                "light".to_string(),
            ]),
            _ => {}
        }

        if let Some(trait_tag) = tag.strip_prefix("gravitic:") {
            match trait_tag {
                "dense_muscle" => affinity.affinity_tags.extend(vec![
                    "strength".to_string(),
                    "durability".to_string(),
                ]),
                "impact_resistant" | "shockproof" => {
                    affinity.affinity_tags.push("durability".to_string())
                }
                "lightframe" => affinity.affinity_tags.extend(vec![
                    "speed".to_string(),
                    "flight".to_string(),
                ]),
                _ => affinity.affinity_tags.push("gravity".to_string()),
            }
        }
    }

    for (domain, tags) in domain_power_tags() {
        if allowed_domains.contains(domain) {
            affinity.affinity_tags.extend(tags.iter().map(|tag| tag.to_string()));
        } else {
            affinity
                .blocked_tags
                .extend(tags.iter().map(|tag| tag.to_string()));
        }
    }

    if !has_cosmic {
        affinity.blocked_tags.extend(vec![
            "cosmic".to_string(),
            "tier:cosmic".to_string(),
            "tier:apex".to_string(),
            "universal".to_string(),
            "universe".to_string(),
            "multiverse".to_string(),
            "godlike".to_string(),
        ]);
    }
    if !has_apex {
        affinity
            .blocked_tags
            .extend(vec!["omni".to_string(), "tier:apex".to_string()]);
    }

    affinity.affinity_tags = dedupe_lowercase(affinity.affinity_tags);
    affinity.baseline_tags = dedupe_lowercase(affinity.baseline_tags);
    affinity.blocked_tags = dedupe_lowercase(affinity.blocked_tags);
    affinity
}

fn extract_power_tags(tags: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for tag in tags {
        if let Some(value) = tag.strip_prefix("power:") {
            out.push(value.to_ascii_lowercase());
        } else if let Some(value) = tag.strip_prefix("tag:") {
            out.push(value.to_ascii_lowercase());
        }
    }
    out
}

fn domain_power_tags() -> HashMap<&'static str, Vec<&'static str>> {
    let mut map = HashMap::new();
    map.insert(
        "time",
        vec!["domain:time", "time", "temporal", "chrono", "chronal"],
    );
    map.insert(
        "space",
        vec![
            "domain:space",
            "space",
            "teleport",
            "teleportation",
            "portal",
            "warp",
        ],
    );
    map.insert(
        "reality",
        vec![
            "domain:reality",
            "reality",
            "dimension",
            "dimensional",
            "universe",
            "multiverse",
            "omniverse",
        ],
    );
    map.insert(
        "divine",
        vec![
            "domain:divine",
            "divine",
            "god",
            "godlike",
            "angel",
            "demon",
            "holy",
            "sacred",
            "celestial",
        ],
    );
    map.insert(
        "psionic",
        vec![
            "domain:psionic",
            "psychic",
            "mind",
            "mental",
            "psionic",
            "telepathy",
            "telekinesis",
        ],
    );
    map.insert(
        "energy",
        vec![
            "domain:energy",
            "energy",
            "light",
            "fire",
            "heat",
            "electricity",
            "electric",
            "radiation",
            "plasma",
        ],
    );
    map
}

fn validate_config(config: &PowerAssignmentConfig) -> Result<(), PowerAssignmentError> {
    if config.expression_min == 0 || config.expression_max == 0 {
        return Err(PowerAssignmentError::InvalidConfig(
            "expression count must be >= 1".to_string(),
        ));
    }
    if config.expression_min > config.expression_max {
        return Err(PowerAssignmentError::InvalidConfig(
            "expression_min cannot exceed expression_max".to_string(),
        ));
    }
    Ok(())
}

fn pick_power(
    repo: &dyn PowerRepository,
    tags_any: &[String],
    tags_none: &[String],
    rng: &mut u64,
) -> Result<Option<PowerId>, PowerAssignmentError> {
    if tags_any.is_empty() {
        return Ok(None);
    }
    let candidates = repo
        .power_ids_by_tags(tags_any, &[], tags_none)
        .map_err(|err| PowerAssignmentError::Repository(err.to_string()))?;
    if candidates.is_empty() {
        return Ok(None);
    }
    let idx = (next_u64(rng) as usize) % candidates.len();
    Ok(candidates.get(idx).copied())
}

fn pick_expression_powers(
    repo: &dyn PowerRepository,
    tags_any: &[String],
    tags_none: &[String],
    rng: &mut u64,
    count: u32,
    selected: &mut HashSet<PowerId>,
) -> Result<Vec<PowerId>, PowerAssignmentError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    let mut candidates = repo
        .power_ids_by_tags(tags_any, &[], tags_none)
        .map_err(|err| PowerAssignmentError::Repository(err.to_string()))?;
    candidates.retain(|id| !selected.contains(id));
    let mut out = Vec::new();
    let mut remaining = count.min(candidates.len() as u32);
    while remaining > 0 && !candidates.is_empty() {
        let idx = (next_u64(rng) as usize) % candidates.len();
        let power = candidates.swap_remove(idx);
        if selected.insert(power) {
            out.push(power);
            remaining -= 1;
        }
    }
    Ok(out)
}

fn dedupe_lowercase(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for value in values {
        let norm = value.to_ascii_lowercase();
        if seen.insert(norm.clone()) {
            out.push(norm);
        }
    }
    out
}

fn roll_range(rng: &mut u64, min: u32, max: u32) -> u32 {
    if min >= max {
        return min;
    }
    let span = max - min + 1;
    min + (next_u64(rng) % (span as u64)) as u32
}

fn roll_percent(rng: &mut u64, percent: u32) -> bool {
    if percent == 0 {
        return false;
    }
    let roll = (next_u64(rng) % 100) as u32 + 1;
    roll <= percent.min(100)
}

fn next_u64(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1);
    *state
}

fn hash_seed(value: &str) -> u64 {
    let mut hash = 1469598103934665603u64;
    for byte in value.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
}

fn mutant_affinity_tags() -> Vec<String> {
    vec![
        "mutation".to_string(),
        "mutant".to_string(),
        "genetic".to_string(),
        "dna".to_string(),
    ]
}

fn mutant_blocked_tags(omega_parent: bool) -> Vec<String> {
    let mut blocked = vec!["tier:apex".to_string(), "omni".to_string()];
    if !omega_parent {
        blocked.extend(vec![
            "cosmic".to_string(),
            "tier:cosmic".to_string(),
            "universal".to_string(),
            "universe".to_string(),
            "multiverse".to_string(),
            "godlike".to_string(),
        ]);
    }
    blocked
}

#[derive(Debug, Clone, Copy)]
enum ParentGenotype {
    Normal,
    Hetero,
    Homo,
}

impl ParentGenotype {
    fn is_mutant(self) -> bool {
        !matches!(self, ParentGenotype::Normal)
    }
}

fn roll_parent_genotype(rng: &mut u64) -> ParentGenotype {
    if !roll_percent(rng, DEFAULT_PARENT_MUTANT_CHANCE) {
        return ParentGenotype::Normal;
    }
    if roll_percent(rng, DEFAULT_PARENT_HOMOZYGOUS_CHANCE) {
        ParentGenotype::Homo
    } else {
        ParentGenotype::Hetero
    }
}

fn child_inherits_mutation(rng: &mut u64, a: ParentGenotype, b: ParentGenotype) -> bool {
    match (a, b) {
        (ParentGenotype::Homo, _) | (_, ParentGenotype::Homo) => true,
        (ParentGenotype::Hetero, ParentGenotype::Hetero) => roll_percent(rng, 75),
        (ParentGenotype::Hetero, ParentGenotype::Normal)
        | (ParentGenotype::Normal, ParentGenotype::Hetero) => roll_percent(rng, 50),
        _ => false,
    }
}

fn roll_parent_power_count(
    rng: &mut u64,
    mutant_parent_count: u8,
    omega_parent: bool,
) -> u32 {
    let (min, max) = if mutant_parent_count >= 2 {
        (DEFAULT_PARENT_POWERS_DUAL_MIN, DEFAULT_PARENT_POWERS_DUAL_MAX)
    } else {
        (DEFAULT_PARENT_POWERS_SINGLE_MIN, DEFAULT_PARENT_POWERS_SINGLE_MAX)
    };
    let mut count = roll_range(rng, min, max);
    if omega_parent {
        count += 1;
        if roll_percent(rng, 35) {
            count += 1;
        }
    }
    count
}
