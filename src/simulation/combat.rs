use crate::rules::power::ExpressionId;
use crate::rules::signature::SignatureInstance;
use crate::simulation::city::LocationId;
use crate::simulation::pressure::PressureState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatScale {
    Street,
    District,
    City,
    National,
    Cosmic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatIntent {
    Attack,
    Escape,
    Hold,
    Capture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatSide {
    Player,
    Opponent,
    Ally,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatEnd {
    PlayerEscaped,
    PlayerDefeated,
    OpponentsDefeated,
    Resolved,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CombatPressureDelta {
    pub temporal: f32,
    pub identity: f32,
    pub institutional: f32,
    pub moral: f32,
    pub resource: f32,
    pub psychological: f32,
}

impl CombatPressureDelta {
    pub fn apply(self, pressure: &mut PressureState) {
        pressure.temporal = (pressure.temporal + self.temporal).clamp(0.0, 100.0);
        pressure.identity = (pressure.identity + self.identity).clamp(0.0, 100.0);
        pressure.institutional = (pressure.institutional + self.institutional).clamp(0.0, 100.0);
        pressure.moral = (pressure.moral + self.moral).clamp(0.0, 100.0);
        pressure.resource = (pressure.resource + self.resource).clamp(0.0, 100.0);
        pressure.psychological = (pressure.psychological + self.psychological).clamp(0.0, 100.0);
    }
}

#[derive(Debug, Clone)]
pub struct CombatOutcome {
    pub end: CombatEnd,
    pub signatures: Vec<SignatureInstance>,
    pub pressure_delta: CombatPressureDelta,
    pub witness_bonus: u32,
}

#[derive(Debug, Clone)]
pub struct Combatant {
    pub id: u32,
    pub name: String,
    pub side: CombatSide,
    pub stress: i32,
    pub intent: CombatIntent,
    pub is_player: bool,
}

#[derive(Debug, Clone)]
pub struct CombatState {
    pub active: bool,
    pub source: String,
    pub location_id: LocationId,
    pub scale: CombatScale,
    pub tick: u64,
    pub log: Vec<String>,
    pub combatants: Vec<Combatant>,
    pub pending_player_expression: Option<ExpressionId>,
    pub escape_progress: u8,
    pub last_outcome: Option<CombatOutcome>,
}

impl Default for CombatState {
    fn default() -> Self {
        Self {
            active: false,
            source: String::new(),
            location_id: LocationId(0),
            scale: CombatScale::Street,
            tick: 0,
            log: Vec::new(),
            combatants: Vec::new(),
            pending_player_expression: None,
            escape_progress: 0,
            last_outcome: None,
        }
    }
}

impl CombatState {
    pub fn player(&self) -> Option<&Combatant> {
        self.combatants.iter().find(|c| c.is_player)
    }

    pub fn player_mut(&mut self) -> Option<&mut Combatant> {
        self.combatants.iter_mut().find(|c| c.is_player)
    }

    pub fn opponents_mut(&mut self) -> impl Iterator<Item = &mut Combatant> {
        self.combatants
            .iter_mut()
            .filter(|c| c.side == CombatSide::Opponent)
    }

    pub fn active_opponent_count(&self) -> usize {
        self.combatants
            .iter()
            .filter(|c| c.side == CombatSide::Opponent && c.stress < 100)
            .count()
    }
}
