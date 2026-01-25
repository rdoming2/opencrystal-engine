use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::inventory::InventoryStack;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RulesFile {
    pub version: u32,
    pub game: GameRules,
    #[serde(default = "default_party_mode")]
    pub party_mode: PartyMode,
    #[serde(default)]
    pub party_create: PartyCreateRules,
    #[serde(default)]
    pub exp_curve: ExpCurveRules,
    #[serde(default)]
    pub inventory: InventoryRules,
    #[serde(default)]
    pub systems: HashMap<String, bool>,
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
    pub party_mode: PartyMode,
    pub party_create: PartyCreateRules,
    pub exp_curve: ExpCurveRules,
    pub inventory: InventoryRules,
    pub systems: HashMap<String, bool>,
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
            party_mode: PartyMode::Predefined,
            party_create: PartyCreateRules::default(),
            exp_curve: ExpCurveRules::default(),
            inventory: InventoryRules::default(),
            systems: HashMap::new(),
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
            party_mode: file.party_mode,
            party_create: file.party_create,
            exp_curve: file.exp_curve,
            inventory: file.inventory,
            systems: file.systems,
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PartyMode {
    Create,
    Predefined,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyCreateRules {
    pub default_job: String,
    #[serde(default = "default_party_level")]
    pub starting_level: u32,
    #[serde(default = "default_name_length")]
    pub name_length: usize,
    #[serde(default)]
    pub starting_equipment: HashMap<String, String>,
}

impl Default for PartyCreateRules {
    fn default() -> Self {
        Self {
            default_job: "shallot_knight".to_string(),
            starting_level: default_party_level(),
            name_length: default_name_length(),
            starting_equipment: HashMap::new(),
        }
    }
}

fn default_party_mode() -> PartyMode {
    PartyMode::Predefined
}

fn default_party_level() -> u32 {
    1
}

fn default_name_length() -> usize {
    12
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExpCurveRules {
    pub mode: String,
    #[serde(default)]
    pub table: Vec<i32>,
    #[serde(default)]
    pub formula: Option<String>,
    #[serde(default = "default_max_level")]
    pub max_level: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InventoryRules {
    #[serde(default = "default_inventory_stack")]
    pub max_stack: i32,
    #[serde(default)]
    pub items: Vec<InventoryStack>,
    #[serde(default)]
    pub equipment: Vec<InventoryStack>,
}

impl Default for InventoryRules {
    fn default() -> Self {
        Self {
            max_stack: default_inventory_stack(),
            items: Vec::new(),
            equipment: Vec::new(),
        }
    }
}

impl Default for ExpCurveRules {
    fn default() -> Self {
        Self {
            mode: "table".to_string(),
            table: vec![0, 10, 30, 60, 100],
            formula: None,
            max_level: default_max_level(),
        }
    }
}

fn default_max_level() -> u32 {
    99
}

fn default_inventory_stack() -> i32 {
    99
}
