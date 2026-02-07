use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::inventory::InventoryStack;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RulesFile {
    pub version: u32,
    pub game: GameRules,
    #[serde(default)]
    pub battle: BattleRules,
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
    #[serde(default)]
    pub save: SaveRules,
    pub render: RenderRules,
    pub stats: StatsRules,
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
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    pub party_size: usize,
    pub party_reserve_size: usize,
    pub battle_mode: BattleMode,
    pub magic_system: MagicSystem,
    pub magic_acquisition: MagicAcquisition,
    pub ability_acquisition: AbilityAcquisition,
    pub start_event: Option<String>,
    pub start_location: StartLocation,
    pub currency: Currency,
    #[serde(default = "default_readiness_speed")]
    pub readiness_speed: f32,
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
pub struct BattleRules {
    #[serde(default = "default_battle_global_commands")]
    pub global_commands: Vec<String>,
    #[serde(default = "default_battle_commands")]
    pub commands: Vec<BattleCommandDefinition>,
    #[serde(default)]
    pub rows: BattleRowRules,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BattleRowRules {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub allow_battle_switch: bool,
    #[serde(default = "default_back_row_attack_multiplier")]
    pub back_row_attack_multiplier: f32,
    #[serde(default = "default_back_row_defense_multiplier")]
    pub back_row_defense_multiplier: f32,
    #[serde(default)]
    pub ranged_weapon_categories: Vec<String>,
    #[serde(default = "default_back_row_battle_shift")]
    pub battle_shift: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BattleCommandDefinition {
    pub id: String,
    pub label: String,
    pub kind: String,
    #[serde(default = "default_battle_command_sort_order")]
    pub sort_order: i32,
    #[serde(default)]
    pub ability_group: Option<String>,
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
    pub jp_mode: JpMode,
    #[serde(default)]
    pub job_exp_curve: ExpCurveRules,
}

#[derive(Clone, Debug)]
pub struct Ruleset {
    pub title: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub party_size: usize,
    pub party_reserve_size: usize,
    pub battle_mode: BattleMode,
    pub battle: BattleRules,
    pub magic_system: MagicSystem,
    pub magic_acquisition: MagicAcquisition,
    pub ability_acquisition: AbilityAcquisition,
    pub start_event: Option<String>,
    pub start_location: StartLocation,
    pub party_mode: PartyMode,
    pub party_create: PartyCreateRules,
    pub exp_curve: ExpCurveRules,
    pub inventory: InventoryRules,
    pub systems: HashMap<String, bool>,
    pub save: SaveRules,
    pub readiness_speed: f32,
    pub job_system: JobSystemRules,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SaveRules {
    #[serde(default = "default_save_slots_max")]
    pub slots_max: u32,
    #[serde(default)]
    pub autosave_enabled: bool,
}

impl Ruleset {
    pub fn demo() -> Self {
        Self {
            title: "OpenCrystal".to_string(),
            description: None,
            author: None,
            party_size: 4,
            party_reserve_size: 4,
            battle_mode: BattleMode::Turn,
            battle: BattleRules::default(),
            magic_system: MagicSystem::Mp,
            magic_acquisition: MagicAcquisition::Level,
            ability_acquisition: AbilityAcquisition::Level,
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
            save: SaveRules::default(),
            readiness_speed: 2.0,
            job_system: JobSystemRules::default(),
        }
    }

    pub fn from_file(file: RulesFile) -> Self {
        Self {
            title: file.game.title,
            description: file.game.description,
            author: file.game.author,
            party_size: file.game.party_size,
            party_reserve_size: file.game.party_reserve_size,
            battle_mode: file.game.battle_mode,
            battle: file.battle,
            magic_system: file.game.magic_system,
            magic_acquisition: file.game.magic_acquisition,
            ability_acquisition: file.game.ability_acquisition,
            start_event: file.game.start_event,
            start_location: file.game.start_location,
            party_mode: file.party_mode,
            party_create: file.party_create,
            exp_curve: file.exp_curve,
            inventory: file.inventory,
            systems: file.systems,
            save: file.save,
            readiness_speed: file.game.readiness_speed,
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
    Jp,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AbilityAcquisition {
    Level,
    Item,
    Equip,
    Jp,
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum JpMode {
    Spend,
    Earn,
    EarnJobLocked,
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

impl Default for SaveRules {
    fn default() -> Self {
        Self {
            slots_max: default_save_slots_max(),
            autosave_enabled: false,
        }
    }
}

impl Default for BattleRules {
    fn default() -> Self {
        Self {
            global_commands: default_battle_global_commands(),
            commands: default_battle_commands(),
            rows: BattleRowRules::default(),
        }
    }
}

impl Default for BattleRowRules {
    fn default() -> Self {
        Self {
            enabled: false,
            allow_battle_switch: false,
            back_row_attack_multiplier: default_back_row_attack_multiplier(),
            back_row_defense_multiplier: default_back_row_defense_multiplier(),
            ranged_weapon_categories: Vec::new(),
            battle_shift: default_back_row_battle_shift(),
        }
    }
}

fn default_party_mode() -> PartyMode {
    PartyMode::Create
}

fn default_party_level() -> u32 {
    1
}

fn default_name_length() -> usize {
    12
}

fn default_save_slots_max() -> u32 {
    10
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

fn default_readiness_speed() -> f32 {
    2.0
}

fn default_battle_global_commands() -> Vec<String> {
    vec![
        "attack".to_string(),
        "magic".to_string(),
        "abilities".to_string(),
        "items".to_string(),
        "run".to_string(),
        "defend".to_string(),
    ]
}

fn default_battle_commands() -> Vec<BattleCommandDefinition> {
    vec![
        BattleCommandDefinition {
            id: "attack".to_string(),
            label: "Attack".to_string(),
            kind: "attack".to_string(),
            sort_order: 10,
            ability_group: None,
        },
        BattleCommandDefinition {
            id: "magic".to_string(),
            label: "Magic".to_string(),
            kind: "magic".to_string(),
            sort_order: 30,
            ability_group: None,
        },
        BattleCommandDefinition {
            id: "abilities".to_string(),
            label: "Abilities".to_string(),
            kind: "abilities".to_string(),
            sort_order: 40,
            ability_group: None,
        },
        BattleCommandDefinition {
            id: "items".to_string(),
            label: "Items".to_string(),
            kind: "items".to_string(),
            sort_order: 50,
            ability_group: None,
        },
        BattleCommandDefinition {
            id: "defend".to_string(),
            label: "Defend".to_string(),
            kind: "defend".to_string(),
            sort_order: 60,
            ability_group: None,
        },
        BattleCommandDefinition {
            id: "run".to_string(),
            label: "Run".to_string(),
            kind: "run".to_string(),
            sort_order: 70,
            ability_group: None,
        },
    ]
}

fn default_back_row_attack_multiplier() -> f32 {
    0.5
}

fn default_back_row_defense_multiplier() -> f32 {
    0.5
}

fn default_back_row_battle_shift() -> i32 {
    1
}

fn default_battle_command_sort_order() -> i32 {
    100
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
            jp_mode: JpMode::Earn,
            job_exp_curve: ExpCurveRules::default(),
        }
    }
}
