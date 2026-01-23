#[derive(Clone, Debug)]
pub struct Ruleset {
    pub title: String,
    pub start_mode: StartMode,
    pub party_size: usize,
    pub party_reserve_size: usize,
    pub battle_mode: BattleMode,
    pub magic_system: MagicSystem,
}

impl Ruleset {
    pub fn demo() -> Self {
        Self {
            title: "OpenCrystal".to_string(),
            start_mode: StartMode::Ff1,
            party_size: 4,
            party_reserve_size: 4,
            battle_mode: BattleMode::Turn,
            magic_system: MagicSystem::Mp,
        }
    }
}

#[derive(Clone, Debug)]
pub enum StartMode {
    Ff1,
    Preset,
}

#[derive(Clone, Debug)]
pub enum BattleMode {
    Turn,
    AtbWait,
    AtbActive,
}

#[derive(Clone, Debug)]
pub enum MagicSystem {
    Mp,
    TierCharges,
}
