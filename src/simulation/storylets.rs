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
