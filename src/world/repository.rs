use crate::simulation::cast::{PersistentCharacter, PromotionCandidate};
use crate::world::sqlite::WorldDbState;

pub trait WorldRepository {
    fn load_or_init(&mut self) -> Result<WorldDbState, Box<dyn std::error::Error>>;
    fn save_state(&mut self, state: &WorldDbState) -> Result<(), Box<dyn std::error::Error>>;
    fn load_characters(&self) -> Result<Vec<PersistentCharacter>, Box<dyn std::error::Error>>;
    fn upsert_character(
        &mut self,
        character: &PersistentCharacter,
    ) -> Result<(), Box<dyn std::error::Error>>;
    fn promote_candidate(
        &mut self,
        candidate: &PromotionCandidate,
        created_at_tick: u64,
    ) -> Result<PersistentCharacter, Box<dyn std::error::Error>>;
}
