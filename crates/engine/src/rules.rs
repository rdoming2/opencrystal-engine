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
    pub render: RenderRules,
    pub stats: StatsRules,
    #[serde(default)]
    pub job_system: JobSystemRules,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StartLocation {
    pub world: String,
    pub map: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GameRules {
    pub title: String,
    pub party_size: usize,
    pub party_reserve_size: usize,
    pub battle_mode: BattleMode,
    pub magic_system: MagicSystem,
    #[serde(default = "default_magic_acquisition")]
    pub magic_acquisition: MagicAcquisition,
    pub start_event: Option<String>,
    pub start_location: StartLocation,
    pub currency: Currency,
    #[serde(default = "default_atb_speed")]
    pub atb_speed: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Currency {
    pub id: String,
    pub name: String,
    pub symbol: String,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JobSystemRules {
    #[serde(default = "default_job_progression_mode")]
    pub progression_mode: JobProgressionMode,
    #[serde(default = "default_secondary_job_enabled")]
    pub secondary_jobs: bool,
    #[serde(default)]
    pub job_exp_curve: ExpCurveRules,
}

#[derive(Clone, Debug)]
pub struct Ruleset {
    pub title: String,
    pub party_size: usize,
    pub party_reserve_size: usize,
    pub battle_mode: BattleMode,
    pub magic_system: MagicSystem,
    pub magic_acquisition: MagicAcquisition,
    pub start_event: Option<String>,
    pub start_location: StartLocation,
    pub party_mode: PartyMode,
    pub party_create: PartyCreateRules,
    pub exp_curve: ExpCurveRules,
    pub inventory: InventoryRules,
    pub systems: HashMap<String, bool>,
    pub atb_speed: f32,
    pub job_system: JobSystemRules,
}

impl Ruleset {
    pub fn demo() -> Self {
        Self {
            title: "OpenCrystal".to_string(),
            party_size: 4,
            party_reserve_size: 4,
            battle_mode: BattleMode::Turn,
            magic_system: MagicSystem::Mp,
            magic_acquisition: MagicAcquisition::Level,
            start_event: Some("intro_cutscene".to_string()),
            start_location: StartLocation {
                world: "gaia".to_string(),
                map: "overworld_gaia".to_string(),
                x: 20,
                y: 14,
            },
            party_mode: PartyMode::Create,
            party_create: PartyCreateRules::default(),
            exp_curve: ExpCurveRules::default(),
            inventory: InventoryRules::default(),
            systems: HashMap::new(),
            atb_speed: 2.0,
            job_system: JobSystemRules::default(),
        }
    }

    pub fn from_file(file: RulesFile) -> Self {
        Self {
            title: file.game.title,
            party_size: file.game.party_size,
            party_reserve_size: file.game.party_reserve_size,
            battle_mode: file.game.battle_mode,
            magic_system: file.game.magic_system,
            magic_acquisition: file.game.magic_acquisition,
            start_event: file.game.start_event,
            start_location: file.game.start_location,
            party_mode: file.party_mode,
            party_create: file.party_create,
            exp_curve: file.exp_curve,
            inventory: file.inventory,
            systems: file.systems,
            atb_speed: file.game.atb_speed,
            job_system: file.job_system,
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
pub enum BattleMode {
    Turn,
    DynamicWait,
    Dynamic,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MagicSystem {
    Mp,
    TierCharges,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MagicAcquisition {
    Level,
    Item,
    Equip,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PartyMode {
    Create,
    Preset,
    PresetRename,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JobProgressionMode {
    Character,
    Job,
    JobPoints,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PartyCreateRules {
    #[serde(default = "default_party_level")]
    pub starting_level: u32,
    #[serde(default = "default_name_length")]
    pub name_length: usize,
}

impl Default for PartyCreateRules {
    fn default() -> Self {
        Self {
            starting_level: default_party_level(),
            name_length: default_name_length(),
        }
    }
}

fn default_party_mode() -> PartyMode {
    PartyMode::Create
}

fn default_magic_acquisition() -> MagicAcquisition {
    MagicAcquisition::Level
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

fn default_atb_speed() -> f32 {
    2.0
}

fn default_job_progression_mode() -> JobProgressionMode {
    JobProgressionMode::Character
}

fn default_secondary_job_enabled() -> bool {
    false
}

impl Default for JobSystemRules {
    fn default() -> Self {
        Self {
            progression_mode: default_job_progression_mode(),
            secondary_jobs: default_secondary_job_enabled(),
            job_exp_curve: ExpCurveRules::default(),
        }
    }
}
