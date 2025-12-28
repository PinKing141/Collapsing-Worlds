use crate::data::alien_generation::{AlienGenerationCatalog, AlienTableEntry};

#[derive(Debug, Clone)]
pub struct AlienSpeciesProfile {
    pub seed: u64,
    pub species_name: String,
    pub demonym: String,
    pub homeworld: AlienHomeworldProfile,
    pub physiology: AlienPhysiologyProfile,
    pub society: AlienSocietyProfile,
    pub capabilities: AlienCapabilityProfile,
    pub weaknesses: Vec<AlienTableEntry>,
    pub tags: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AlienHomeworldProfile {
    pub name: String,
    pub star_archetype: AlienTableEntry,
    pub origin: AlienTableEntry,
    pub gravity: AlienTableEntry,
    pub atmosphere: AlienTableEntry,
    pub climate: AlienTableEntry,
    pub terrain: AlienTableEntry,
}

#[derive(Debug, Clone)]
pub struct AlienPhysiologyProfile {
    pub body_plan: AlienTableEntry,
    pub scale: AlienTableEntry,
    pub covering: AlienTableEntry,
    pub pigmentation: AlienTableEntry,
    pub feature: AlienTableEntry,
    pub senses: AlienTableEntry,
    pub locomotion: AlienTableEntry,
    pub adaptation: AlienTableEntry,
}

#[derive(Debug, Clone)]
pub struct AlienSocietyProfile {
    pub society: AlienTableEntry,
    pub values: AlienTableEntry,
    pub technology: AlienTableEntry,
    pub interstellar_role: AlienTableEntry,
    pub governing_body: String,
}

#[derive(Debug, Clone)]
pub struct AlienCapabilityProfile {
    pub cosmic_tier: AlienTableEntry,
    pub power_source: AlienTableEntry,
    pub signature_gift: AlienTableEntry,
    pub flight_style: AlienTableEntry,
    pub gravitic_traits: Vec<String>,
}

pub fn generate_alien_species(
    catalog: &AlienGenerationCatalog,
    seed: u64,
) -> AlienSpeciesProfile {
    let mut rng = seed ^ hash_seed("alien_generation");
    let naming_style = roll_table_d6(&catalog.tables.naming_style, &mut rng, 0);
    let (species_name, demonym, homeworld_name) =
        build_species_names(catalog, &mut rng, &naming_style);
    let governing_body = build_governing_body(catalog, &mut rng, &homeworld_name);

    let star_archetype = roll_table_d6(&catalog.tables.star_archetype, &mut rng, 0);
    let origin = roll_table_d6(&catalog.tables.origin, &mut rng, 0);
    let gravity = roll_table_2d6(&catalog.tables.gravity, &mut rng, 0);
    let atmosphere = roll_table_2d6(&catalog.tables.atmosphere, &mut rng, 0);
    let climate = roll_table_2d6(&catalog.tables.climate, &mut rng, 0);
    let terrain = roll_table_d66(&catalog.tables.terrain, &mut rng, 0);

    let body_plan = roll_table_d6(&catalog.tables.body_plan, &mut rng, 0);
    let scale = roll_table_2d6(&catalog.tables.scale, &mut rng, 0);
    let covering = roll_table_d6(&catalog.tables.covering, &mut rng, 0);
    let pigmentation = roll_table_d6(&catalog.tables.pigmentation, &mut rng, 0);
    let feature = roll_table_d6(&catalog.tables.feature, &mut rng, 0);
    let senses = roll_table_d66(&catalog.tables.senses, &mut rng, 0);
    let locomotion = roll_table_d6(&catalog.tables.locomotion, &mut rng, 0);
    let adaptation = roll_table_d6(&catalog.tables.adaptation, &mut rng, 0);

    let society = roll_table_2d6(&catalog.tables.society, &mut rng, 0);
    let values = roll_table_2d6(&catalog.tables.values, &mut rng, 0);
    let technology = roll_table_2d6(&catalog.tables.technology, &mut rng, 0);
    let interstellar_role = roll_table_d6(&catalog.tables.interstellar_role, &mut rng, 0);

    let cosmic_tier = roll_table_d6(&catalog.tables.cosmic_tier, &mut rng, 0);
    let power_source = roll_table_d6(&catalog.tables.power_source, &mut rng, 0);
    let signature_gift = roll_table_d6(&catalog.tables.signature_gift, &mut rng, 0);
    let flight_mod = flight_mod_from_gravity(&gravity);
    let flight_style = roll_table_d6(&catalog.tables.flight_style, &mut rng, flight_mod);
    let gravitic_traits = derive_gravity_traits(&gravity);

    let weakness = roll_table_d66(&catalog.tables.weakness, &mut rng, 0);

    let homeworld = AlienHomeworldProfile {
        name: homeworld_name,
        star_archetype,
        origin,
        gravity,
        atmosphere,
        climate,
        terrain,
    };

    let physiology = AlienPhysiologyProfile {
        body_plan,
        scale,
        covering,
        pigmentation,
        feature,
        senses,
        locomotion,
        adaptation,
    };

    let society_profile = AlienSocietyProfile {
        society,
        values,
        technology,
        interstellar_role,
        governing_body,
    };

    let capabilities = AlienCapabilityProfile {
        cosmic_tier,
        power_source,
        signature_gift,
        flight_style,
        gravitic_traits,
    };

    let mut tags = Vec::new();
    let notes = Vec::new();
    collect_entry_tags(&mut tags, &naming_style);
    collect_entry_tags(&mut tags, &homeworld.star_archetype);
    collect_entry_tags(&mut tags, &homeworld.origin);
    collect_entry_tags(&mut tags, &homeworld.gravity);
    collect_entry_tags(&mut tags, &homeworld.atmosphere);
    collect_entry_tags(&mut tags, &homeworld.climate);
    collect_entry_tags(&mut tags, &homeworld.terrain);
    collect_entry_tags(&mut tags, &physiology.body_plan);
    collect_entry_tags(&mut tags, &physiology.scale);
    collect_entry_tags(&mut tags, &physiology.covering);
    collect_entry_tags(&mut tags, &physiology.pigmentation);
    collect_entry_tags(&mut tags, &physiology.feature);
    collect_entry_tags(&mut tags, &physiology.senses);
    collect_entry_tags(&mut tags, &physiology.locomotion);
    collect_entry_tags(&mut tags, &physiology.adaptation);
    collect_entry_tags(&mut tags, &society_profile.society);
    collect_entry_tags(&mut tags, &society_profile.values);
    collect_entry_tags(&mut tags, &society_profile.technology);
    collect_entry_tags(&mut tags, &society_profile.interstellar_role);
    collect_entry_tags(&mut tags, &capabilities.cosmic_tier);
    collect_entry_tags(&mut tags, &capabilities.power_source);
    collect_entry_tags(&mut tags, &capabilities.signature_gift);
    collect_entry_tags(&mut tags, &capabilities.flight_style);
    for gravitic_trait in &capabilities.gravitic_traits {
        push_unique(&mut tags, format!("gravitic:{}", gravitic_trait));
    }

    for weakness_tag in &weakness.tags {
        push_unique(&mut tags, weakness_tag.clone());
    }

    AlienSpeciesProfile {
        seed,
        species_name,
        demonym,
        homeworld,
        physiology,
        society: society_profile,
        capabilities,
        weaknesses: vec![weakness],
        tags,
        notes,
    }
}

pub fn format_alien_profile(profile: &AlienSpeciesProfile, detail: bool) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "Alien species: {} ({})",
        profile.species_name, profile.demonym
    ));
    lines.push(format!(
        "Homeworld: {} | star: {} | gravity: {} | climate: {}",
        profile.homeworld.name,
        profile.homeworld.star_archetype.label,
        profile.homeworld.gravity.label,
        profile.homeworld.climate.label
    ));
    lines.push(format!(
        "Physiology: {} | {} | {} | {}",
        profile.physiology.body_plan.label,
        profile.physiology.scale.label,
        profile.physiology.covering.label,
        profile.physiology.senses.label
    ));
    lines.push(format!(
        "Culture: {} | values: {} | tech: {} | role: {}",
        profile.society.society.label,
        profile.society.values.label,
        profile.society.technology.label,
        profile.society.interstellar_role.label
    ));
    lines.push(format!(
        "Capabilities: {} | gift: {} | flight: {} | tier: {}",
        profile.capabilities.power_source.label,
        profile.capabilities.signature_gift.label,
        profile.capabilities.flight_style.label,
        profile.capabilities.cosmic_tier.label
    ));
    if let Some(weakness) = profile.weaknesses.first() {
        lines.push(format!("Weakness: {}", weakness.label));
    }
    if !profile.tags.is_empty() {
        lines.push(format!("Tags: {}", profile.tags.join(", ")));
    }

    if detail {
        lines.push(format!(
            "Origin: {}",
            describe_entry(&profile.homeworld.origin)
        ));
        lines.push(format!(
            "Atmosphere: {}",
            describe_entry(&profile.homeworld.atmosphere)
        ));
        lines.push(format!(
            "Terrain: {}",
            describe_entry(&profile.homeworld.terrain)
        ));
        lines.push(format!(
            "Pigmentation: {}",
            describe_entry(&profile.physiology.pigmentation)
        ));
        lines.push(format!(
            "Distinguishing feature: {}",
            describe_entry(&profile.physiology.feature)
        ));
        lines.push(format!(
            "Locomotion: {}",
            describe_entry(&profile.physiology.locomotion)
        ));
        lines.push(format!(
            "Adaptation: {}",
            describe_entry(&profile.physiology.adaptation)
        ));
        lines.push(format!(
            "Governing body: {}",
            profile.society.governing_body
        ));
        if !profile.capabilities.gravitic_traits.is_empty() {
            lines.push(format!(
                "Gravitic traits: {}",
                profile.capabilities.gravitic_traits.join(", ")
            ));
        }
        if let Some(weakness) = profile.weaknesses.first() {
            if let Some(detail) = weakness.detail.as_deref() {
                lines.push(format!("Weakness detail: {}", detail));
            }
        }
        for note in &profile.notes {
            lines.push(format!("Note: {}", note));
        }
    }

    lines
}

fn describe_entry(entry: &AlienTableEntry) -> String {
    if let Some(detail) = entry.detail.as_deref() {
        format!("{} ({})", entry.label, detail)
    } else {
        entry.label.clone()
    }
}

fn build_species_names(
    catalog: &AlienGenerationCatalog,
    rng: &mut u64,
    naming_style: &AlienTableEntry,
) -> (String, String, String) {
    let root_primary = build_root(&catalog.naming, rng);
    let root_secondary = build_root(&catalog.naming, rng);
    let homeworld_name = build_world_name(&catalog.naming, rng, &root_primary);
    let style_key = entry_key(naming_style);
    let species_suffix = pick_fragment(&catalog.naming.species_suffixes, rng);

    match style_key.as_str() {
        "homeworld_demonym" => {
            let demonym = build_demonym(&catalog.naming, rng, &homeworld_name);
            (demonym.clone(), demonym, homeworld_name)
        }
        "dual_root" => {
            let species_name = build_species_name(&root_secondary, species_suffix);
            let demonym = build_demonym(&catalog.naming, rng, &homeworld_name);
            (species_name, demonym, homeworld_name)
        }
        "house_title" => {
            let species_name = build_species_name(&root_primary, species_suffix);
            let demonym = build_demonym(&catalog.naming, rng, &homeworld_name);
            (species_name, demonym, homeworld_name)
        }
        "cosmic_epithet" => {
            let species_name = build_species_name(&root_primary, species_suffix);
            (species_name.clone(), species_name, homeworld_name)
        }
        _ => {
            let species_name = build_species_name(&root_primary, species_suffix);
            let demonym = build_demonym(&catalog.naming, rng, &homeworld_name);
            (species_name, demonym, homeworld_name)
        }
    }
}

fn build_root(naming: &crate::data::alien_generation::AlienNamingCatalog, rng: &mut u64) -> String {
    let prefix = pick_fragment(&naming.root_prefixes, rng);
    let core = pick_fragment(&naming.root_cores, rng);
    let suffix = pick_fragment(&naming.root_suffixes, rng);
    let merged = merge_fragments(&merge_fragments(prefix, core), suffix);
    capitalize_first(&merged)
}

fn build_world_name(
    naming: &crate::data::alien_generation::AlienNamingCatalog,
    rng: &mut u64,
    root: &str,
) -> String {
    let suffix = pick_fragment(&naming.world_suffixes, rng);
    let merged = merge_fragments(root, suffix);
    capitalize_first(&merged)
}

fn build_species_name(root: &str, suffix: &str) -> String {
    let merged = merge_fragments(root, suffix);
    capitalize_first(&merged)
}

fn build_demonym(
    naming: &crate::data::alien_generation::AlienNamingCatalog,
    rng: &mut u64,
    homeworld: &str,
) -> String {
    let suffix = pick_fragment(&naming.demonym_suffixes, rng);
    let merged = merge_fragments(homeworld, suffix);
    capitalize_first(&merged)
}

fn build_governing_body(
    catalog: &AlienGenerationCatalog,
    rng: &mut u64,
    root: &str,
) -> String {
    let prefix = pick_fragment(&catalog.naming.house_prefixes, rng);
    let suffix = pick_fragment(&catalog.naming.house_suffixes, rng);
    let merged = format!("{} {} {}", prefix, root, suffix);
    capitalize_first(merged.trim())
}

fn merge_fragments(left: &str, right: &str) -> String {
    if left.is_empty() {
        return right.to_string();
    }
    if right.is_empty() {
        return left.to_string();
    }
    let left_last = left.chars().last().unwrap_or(' ');
    let right_first = right.chars().next().unwrap_or(' ');
    if left_last.to_ascii_lowercase() == right_first.to_ascii_lowercase() {
        let mut merged = left.to_string();
        merged.pop();
        merged.push_str(right);
        merged
    } else {
        format!("{}{}", left, right)
    }
}

fn capitalize_first(input: &str) -> String {
    let mut chars = input.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let rest: String = chars.collect();
    format!("{}{}", first.to_ascii_uppercase(), rest)
}

fn pick_fragment<'a>(list: &'a [String], rng: &mut u64) -> &'a str {
    if list.is_empty() {
        return "";
    }
    let idx = (next_u64(rng) as usize) % list.len();
    list[idx].as_str()
}

fn entry_key(entry: &AlienTableEntry) -> String {
    entry
        .id
        .clone()
        .unwrap_or_else(|| entry.label.to_ascii_lowercase().replace(' ', "_"))
}

fn collect_entry_tags(tags: &mut Vec<String>, entry: &AlienTableEntry) {
    for tag in &entry.tags {
        push_unique(tags, tag.clone());
    }
}

fn derive_gravity_traits(gravity: &AlienTableEntry) -> Vec<String> {
    let mut traits = Vec::new();
    if has_tag(gravity, "gravity:low") {
        traits.push("lightframe".to_string());
        traits.push("longstride".to_string());
        traits.push("glide".to_string());
    }
    if has_tag(gravity, "gravity:high") {
        traits.push("dense_muscle".to_string());
        traits.push("impact_resistant".to_string());
        traits.push("burst_flight".to_string());
    }
    if has_tag(gravity, "gravity:extreme") {
        traits.push("massive_bone".to_string());
        traits.push("shockproof".to_string());
        traits.push("impulse_flight".to_string());
    }
    if has_tag(gravity, "gravity:variable") {
        traits.push("adaptive_balance".to_string());
        traits.push("vector_reflex".to_string());
    }
    if traits.is_empty() {
        traits.push("baseline".to_string());
    }
    traits
}

fn flight_mod_from_gravity(gravity: &AlienTableEntry) -> i32 {
    if has_tag(gravity, "gravity:extreme") {
        2
    } else if has_tag(gravity, "gravity:high") {
        1
    } else if has_tag(gravity, "gravity:low") {
        -1
    } else {
        0
    }
}

fn has_tag(entry: &AlienTableEntry, tag: &str) -> bool {
    entry.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
}

fn roll_table_d6(entries: &[AlienTableEntry], rng: &mut u64, modifier: i32) -> AlienTableEntry {
    let roll = roll_d6(rng, modifier);
    select_by_threshold(entries, roll, |entry| entry.d6)
}

fn roll_table_2d6(entries: &[AlienTableEntry], rng: &mut u64, modifier: i32) -> AlienTableEntry {
    let roll = roll_2d6(rng, modifier);
    select_by_threshold(entries, roll, |entry| entry.d2d6)
}

fn roll_table_d66(entries: &[AlienTableEntry], rng: &mut u64, tens_mod: i32) -> AlienTableEntry {
    let roll = roll_d66(rng, tens_mod);
    select_by_threshold(entries, roll, |entry| entry.d66)
}

fn select_by_threshold<F>(entries: &[AlienTableEntry], roll: u32, field: F) -> AlienTableEntry
where
    F: Fn(&AlienTableEntry) -> Option<u32>,
{
    for entry in entries {
        if let Some(max) = field(entry) {
            if roll <= max {
                return entry.clone();
            }
        }
    }
    entries
        .last()
        .cloned()
        .unwrap_or_else(|| AlienTableEntry {
            id: None,
            d6: None,
            d2d6: None,
            d66: None,
            label: "Unknown".to_string(),
            detail: None,
            tags: Vec::new(),
        })
}

fn roll_d6(rng: &mut u64, modifier: i32) -> u32 {
    let roll = (next_u64(rng) % 6) as i32 + 1 + modifier;
    roll.clamp(1, 6) as u32
}

fn roll_2d6(rng: &mut u64, modifier: i32) -> u32 {
    let roll = roll_d6(rng, 0) as i32 + roll_d6(rng, 0) as i32 + modifier;
    roll.clamp(2, 12) as u32
}

fn roll_d66(rng: &mut u64, tens_mod: i32) -> u32 {
    let tens = roll_d6(rng, 0) as i32 + tens_mod;
    let tens = tens.clamp(1, 6);
    let ones = roll_d6(rng, 0) as i32;
    (tens * 10 + ones) as u32
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

fn push_unique(target: &mut Vec<String>, value: String) {
    if target.iter().any(|entry| entry == &value) {
        return;
    }
    target.push(value);
}
