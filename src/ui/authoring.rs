use std::collections::HashMap;

use crate::data::civilian_events::CivilianStorylet;
use crate::data::endgame_events::{EndgameEvent, EndgamePhase};
use crate::data::global_events::GlobalEventDefinition;
use crate::data::nemesis::NemesisActionCatalog;
use crate::data::storylets::StoryletCategory;
use crate::simulation::endgame::EndgameState;
use crate::simulation::origin::{OriginCatalog, OriginPathCatalog};
use crate::simulation::region::{GlobalEventState, RegionState};
use crate::simulation::storylets::{
    is_punctuation_storylet, storylet_has_gate_requirements, storylet_threshold_keys,
    StoryletLibrary,
};

pub fn render_authoring_dashboard(
    storylets: &StoryletLibrary,
    civilian_events: &[CivilianStorylet],
    endgame_events: &[EndgameEvent],
    endgame_state: &EndgameState,
    origins: &OriginCatalog,
    origin_paths: &OriginPathCatalog,
    nemesis_actions: &NemesisActionCatalog,
    region: &RegionState,
    global_events: &[GlobalEventDefinition],
    global_event_state: &GlobalEventState,
) -> String {
    let mut output = String::new();
    output.push_str("=== Authoring Console ===\n");

    let hero_count = storylets.hero.len();
    let vigilante_count = storylets.vigilante.len();
    let villain_count = storylets.villain.len();
    let total_storylets = hero_count + vigilante_count + villain_count;

    let mut category_counts: HashMap<StoryletCategory, usize> = HashMap::new();
    let mut punctuation_count = 0usize;
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    let mut missing_tags = 0usize;
    let mut threshold_counts: HashMap<String, usize> = HashMap::new();
    let mut threshold_storylets = 0usize;
    let mut ungated_storylets = 0usize;
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
        if storylet.tags.is_empty() {
            missing_tags += 1;
        } else {
            for tag in &storylet.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        let thresholds = storylet_threshold_keys(storylet);
        if thresholds.is_empty() {
            if !storylet_has_gate_requirements(storylet) {
                ungated_storylets += 1;
            }
        } else {
            threshold_storylets += 1;
            for key in thresholds {
                *threshold_counts.entry(key).or_insert(0) += 1;
            }
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
    output.push_str(&format!("  Untagged: {}\n", missing_tags));
    if !tag_counts.is_empty() {
        output.push_str("  Tags:\n");
        let mut tags: Vec<(String, usize)> = tag_counts.into_iter().collect();
        tags.sort_by(|a, b| a.0.cmp(&b.0));
        for (tag, count) in tags {
            output.push_str(&format!("    {}: {}\n", tag, count));
        }
    }
    if !threshold_counts.is_empty() {
        output.push_str(&format!(
            "  Threshold-gated storylets: {}\n",
            threshold_storylets
        ));
        output.push_str("  Threshold keys:\n");
        let mut keys: Vec<(String, usize)> = threshold_counts.into_iter().collect();
        keys.sort_by(|a, b| a.0.cmp(&b.0));
        for (key, count) in keys {
            output.push_str(&format!("    {}: {}\n", key, count));
        }
    }
    if ungated_storylets > 0 {
        output.push_str(&format!(
            "  Ungated storylets (missing tags/thresholds): {}\n",
            ungated_storylets
        ));
    }

    output.push_str("\nCivilian Events\n");
    output.push_str(&format!("  Total: {}\n", civilian_events.len()));

    output.push_str("\nEndgame\n");
    output.push_str(&format!("  Current state: {}\n", endgame_state.label()));
    output.push_str(&format!("  Events: {}\n", endgame_events.len()));
    let mut phase_counts: HashMap<EndgamePhase, usize> = HashMap::new();
    for event in endgame_events {
        *phase_counts.entry(event.phase).or_insert(0) += 1;
    }
    if !phase_counts.is_empty() {
        output.push_str("  Phases:\n");
        let mut phases: Vec<(EndgamePhase, usize)> = phase_counts.into_iter().collect();
        phases.sort_by_key(|(phase, _)| format!("{:?}", phase));
        for (phase, count) in phases {
            output.push_str(&format!("    {:?}: {}\n", phase, count));
        }
    }

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

    output.push_str("\nEscalation\n");
    output.push_str(&format!(
        "  Global pressure: {:.1}\n",
        region.global_pressure.total
    ));
    output.push_str(&format!(
        "  Global escalation: {:?}\n",
        region.global_pressure.escalation
    ));

    output.push_str("\nGlobal Events\n");
    output.push_str(&format!("  Total: {}\n", global_events.len()));
    output.push_str(&format!(
        "  Pending: {}\n",
        global_event_state.pending.len()
    ));
    if !global_events.is_empty() {
        let mut escalation_counts: HashMap<String, usize> = HashMap::new();
        let mut tag_counts: HashMap<String, usize> = HashMap::new();
        for event in global_events {
            *escalation_counts
                .entry(format!("{:?}", event.min_escalation))
                .or_insert(0) += 1;
            if event.tags.is_empty() {
                *tag_counts.entry("untagged".to_string()).or_insert(0) += 1;
            } else {
                for tag in &event.tags {
                    *tag_counts.entry(tag.clone()).or_insert(0) += 1;
                }
            }
        }
        output.push_str("  Escalation gates:\n");
        let mut escalations: Vec<(String, usize)> = escalation_counts.into_iter().collect();
        escalations.sort_by(|a, b| a.0.cmp(&b.0));
        for (label, count) in escalations {
            output.push_str(&format!("    {}: {}\n", label, count));
        }
        output.push_str("  Tags:\n");
        let mut tags: Vec<(String, usize)> = tag_counts.into_iter().collect();
        tags.sort_by(|a, b| a.0.cmp(&b.0));
        for (tag, count) in tags {
            output.push_str(&format!("    {}: {}\n", tag, count));
        }
    }

    output
}
