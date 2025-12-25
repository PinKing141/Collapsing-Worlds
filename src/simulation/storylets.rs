use bevy_ecs::prelude::*;

use crate::components::persona::Alignment;
use crate::data::storylets::Storylet;

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
