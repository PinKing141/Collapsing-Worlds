#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasteryStage {
    Raw,
    Controlled,
    Precise,
    Silent,
    Iconic,
}

pub fn stage_from_uses(uses: u32) -> MasteryStage {
    if uses >= 40 {
        MasteryStage::Iconic
    } else if uses >= 24 {
        MasteryStage::Silent
    } else if uses >= 12 {
        MasteryStage::Precise
    } else if uses >= 5 {
        MasteryStage::Controlled
    } else {
        MasteryStage::Raw
    }
}
