use std::collections::HashMap;

use crate::data::civilian_events::CivilianStorylet;
use crate::data::nemesis::NemesisActionCatalog;
use crate::data::storylets::StoryletCategory;
use crate::simulation::origin::{OriginCatalog, OriginPathCatalog};
use crate::simulation::storylets::{is_punctuation_storylet, StoryletLibrary};

pub fn render_authoring_dashboard(
    storylets: &StoryletLibrary,
    civilian_events: &[CivilianStorylet],
    origins: &OriginCatalog,
    origin_paths: &OriginPathCatalog,
    nemesis_actions: &NemesisActionCatalog,
) -> String {
    let mut output = String::new();
    output.push_str("=== Authoring Console ===\n");

    let hero_count = storylets.hero.len();
    let vigilante_count = storylets.vigilante.len();
    let villain_count = storylets.villain.len();
    let total_storylets = hero_count + vigilante_count + villain_count;

    let mut category_counts: HashMap<StoryletCategory, usize> = HashMap::new();
    let mut punctuation_count = 0usize;
    for storylet in storylets
        .hero
        .iter()
        .chain(storylets.vigilante.iter())
        .chain(storylets.villain.iter())
    {
        *category_counts.entry(storylet.category).or_insert(0) += 1;
        if is_punctuation_storylet(storylet) {
            punctuation_count += 1;
        }
    }

    output.push_str("Storylets\n");
    output.push_str(&format!(
        "  Total: {} (punctuation: {})\n",
        total_storylets, punctuation_count
    ));
    output.push_str(&format!("  Hero: {}\n", hero_count));
    output.push_str(&format!("  Vigilante: {}\n", vigilante_count));
    output.push_str(&format!("  Villain: {}\n", villain_count));
    output.push_str("  Categories:\n");
    let mut categories: Vec<(StoryletCategory, usize)> = category_counts.into_iter().collect();
    categories.sort_by_key(|(category, _)| format!("{:?}", category));
    for (category, count) in categories {
        output.push_str(&format!("    {:?}: {}\n", category, count));
    }

    output.push_str("\nCivilian Events\n");
    output.push_str(&format!("  Total: {}\n", civilian_events.len()));

    output.push_str("\nOrigins\n");
    output.push_str(&format!("  Total: {}\n", origins.origins.len()));
    let mut class_counts: HashMap<String, usize> = HashMap::new();
    for origin in &origins.origins {
        *class_counts.entry(origin.class_code.clone()).or_insert(0) += 1;
    }
    if !class_counts.is_empty() {
        output.push_str("  Classes:\n");
        let mut classes: Vec<(String, usize)> = class_counts.into_iter().collect();
        classes.sort_by(|a, b| a.0.cmp(&b.0));
        for (class, count) in classes {
            output.push_str(&format!("    {}: {}\n", class, count));
        }
    }

    output.push_str("\nNemesis Actions\n");
    output.push_str(&format!("  Total: {}\n", nemesis_actions.actions.len()));
    let mut heat_gates: HashMap<u32, usize> = HashMap::new();
    for action in &nemesis_actions.actions {
        *heat_gates.entry(action.min_heat).or_insert(0) += 1;
    }
    if !heat_gates.is_empty() {
        output.push_str("  Min heat gates:\n");
        let mut gates: Vec<(u32, usize)> = heat_gates.into_iter().collect();
        gates.sort_by(|a, b| a.0.cmp(&b.0));
        for (heat, count) in gates {
            output.push_str(&format!("    {}+: {}\n", heat, count));
        }
    }

    output.push_str("\nOrigin Paths\n");
    output.push_str(&format!("  Total: {}\n", origin_paths.paths.len()));
    if !origin_paths.paths.is_empty() {
        let mut stage_totals = 0usize;
        for path in &origin_paths.paths {
            stage_totals += path.stages.len();
        }
        output.push_str(&format!("  Stages: {}\n", stage_totals));
    }

    output
}
