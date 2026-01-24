use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RulesFile {
    pub version: u32,
    pub game: GameRules,
    pub features: FeatureRules,
    pub render: RenderRules,
    pub stats: StatsRules,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameRules {
    pub title: String,
    pub start_mode: StartMode,
    pub party_size: usize,
    pub party_reserve_size: usize,
    pub battle_mode: BattleMode,
    pub magic_system: MagicSystem,
    pub start_event: Option<String>,
    pub job_change_enabled: bool,
    pub job_change_flag: String,
    pub currency: Currency,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Currency {
    pub id: String,
    pub name: String,
    pub symbol: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FeatureRules {
    pub journal: bool,
    pub fast_travel: bool,
    pub overworld_map: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RenderRules {
    pub min_art_width: u16,
    pub min_art_height: u16,
    pub palette: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StatsRules {
    pub track: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Ruleset {
    pub title: String,
    pub start_mode: StartMode,
    pub party_size: usize,
    pub party_reserve_size: usize,
    pub battle_mode: BattleMode,
    pub magic_system: MagicSystem,
    pub start_event: Option<String>,
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
            start_event: Some("intro_cutscene".to_string()),
        }
    }

    pub fn from_file(file: RulesFile) -> Self {
        Self {
            title: file.game.title,
            start_mode: file.game.start_mode,
            party_size: file.game.party_size,
            party_reserve_size: file.game.party_reserve_size,
            battle_mode: file.game.battle_mode,
            magic_system: file.game.magic_system,
            start_event: file.game.start_event,
        }
    }
}

impl RulesFile {
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StartMode {
    Ff1,
    Preset,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleMode {
    Turn,
    AtbWait,
    AtbActive,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MagicSystem {
    Mp,
    TierCharges,
}
