use bevy_ecs::prelude::*;

use crate::components::persona::Alignment;
use crate::data::storylets::{Storylet, StoryletCategory};

#[derive(Resource, Debug, Default, Clone)]
pub struct StoryletLibrary {
    pub hero: Vec<Storylet>,
    pub vigilante: Vec<Storylet>,
    pub villain: Vec<Storylet>,
}

impl StoryletLibrary {
    pub fn for_alignment(&self, alignment: Alignment) -> &[Storylet] {
        match alignment {
            Alignment::Neutral => &self.hero,
            Alignment::Hero => &self.hero,
            Alignment::Vigilante => &self.vigilante,
            Alignment::Villain => &self.villain,
        }
    }
}

pub fn is_punctuation_storylet(storylet: &Storylet) -> bool {
    if storylet
        .tags
        .iter()
        .any(|tag| tag.eq_ignore_ascii_case("punctuation"))
    {
        return true;
    }
    matches!(
        storylet.category,
        StoryletCategory::CivilianLife | StoryletCategory::MaskedLife
    )
}

pub fn storylet_has_gate_requirements(storylet: &Storylet) -> bool {
    !storylet.tags.is_empty()
        || storylet_has_thresholds(storylet)
        || storylet_has_state_gate(storylet)
}

pub fn storylet_has_thresholds(storylet: &Storylet) -> bool {
    storylet
        .preconditions
        .iter()
        .any(|condition| condition_has_threshold(condition))
}

pub fn storylet_threshold_keys(storylet: &Storylet) -> Vec<String> {
    storylet
        .preconditions
        .iter()
        .filter_map(|condition| threshold_key(condition))
        .collect()
}

fn condition_has_threshold(condition: &str) -> bool {
    threshold_key(condition).is_some()
}

fn storylet_has_state_gate(storylet: &Storylet) -> bool {
    storylet.preconditions.iter().any(|condition| {
        let cond = condition.trim();
        cond.starts_with("flag.") || cond.starts_with("endgame.state")
    })
}

fn threshold_key(condition: &str) -> Option<String> {
    let cond = condition.trim();
    if cond.is_empty() {
        return None;
    }
    if cond == "time.is_day" || cond == "time.is_night" || cond == "signatures.visible" {
        return None;
    }

    let parts: Vec<&str> = cond.split_whitespace().collect();
    if parts.len() != 3 {
        return None;
    }
    let left = parts[0];
    let op = parts[1];
    let right = parts[2];

    if matches!(left, "alignment" | "persona") {
        return None;
    }
    if !matches!(op, ">=" | "<=" | ">" | "<" | "==" | "!=") {
        return None;
    }
    if right.parse::<i32>().is_err() {
        return None;
    }
    Some(left.to_string())
}
