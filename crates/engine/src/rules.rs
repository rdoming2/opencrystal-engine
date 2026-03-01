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
    #[serde(default = "default_progression_mode")]
    pub progression_mode: ProgressionMode,
    #[serde(default)]
    pub activity_progression: ActivityProgressionRules,
    #[serde(default)]
    pub activity_growth: ActivityGrowthRules,
    #[serde(default)]
    pub inventory: InventoryRules,
    #[serde(default)]
    pub systems: HashMap<String, bool>,
    #[serde(default)]
    pub save: SaveRules,
    #[serde(default)]
    pub settings: SettingsRules,
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
    pub currencies: Vec<Currency>,
    #[serde(default = "default_readiness_speed")]
    pub readiness_speed: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Currency {
    pub id: String,
    pub name: String,
    pub symbol: String,
}

impl GameRules {
    pub fn currency(&self, id: &str) -> Option<&Currency> {
        self.currencies.iter().find(|currency| currency.id == id)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RenderRules {
    pub min_art_width: u16,
    pub min_art_height: u16,
    pub palette: String,
    #[serde(default)]
    pub death_markers: DeathMarkerRules,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DeathMarkerRules {
    #[serde(default)]
    pub show_on_map: bool,
    #[serde(default = "default_death_marker_glyph")]
    pub glyph: String,
}

impl Default for DeathMarkerRules {
    fn default() -> Self {
        Self {
            show_on_map: false,
            glyph: default_death_marker_glyph(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BattleRules {
    #[serde(default = "default_battle_exp_for_fallen")]
    pub exp_for_fallen: bool,
    #[serde(default = "default_battle_global_commands")]
    pub global_commands: Vec<String>,
    #[serde(default = "default_battle_commands")]
    pub commands: Vec<BattleCommandDefinition>,
    #[serde(default)]
    pub formulas: BattleFormulaRules,
    #[serde(default)]
    pub rows: BattleRowRules,
    #[serde(default)]
    pub boss_scaling: BossScalingRules,
    #[serde(default)]
    pub difficulty_rewards: DifficultyRewardsRules,
    #[serde(default)]
    pub enemy_ai_defaults: EnemyAiDefaults,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnemyAiDefaults {
    #[serde(default = "default_enemy_ai_reroll")]
    pub palliative_reroll_chance: f32,
    #[serde(default = "default_enemy_ai_cooldown")]
    pub palliative_cooldown_turns: u32,
}

impl Default for EnemyAiDefaults {
    fn default() -> Self {
        Self {
            palliative_reroll_chance: default_enemy_ai_reroll(),
            palliative_cooldown_turns: default_enemy_ai_cooldown(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BattleFormulaRules {
    #[serde(default)]
    pub physical: Option<String>,
    #[serde(default)]
    pub magic: Option<String>,
    #[serde(default)]
    pub heal: Option<String>,
    #[serde(default)]
    pub hit: Option<String>,
    #[serde(default)]
    pub crit: Option<String>,
    #[serde(default = "default_battle_crit_multiplier")]
    pub crit_multiplier: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BossScalingRules {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_boss_hp_multiplier")]
    pub hp_multiplier: f32,
    #[serde(default = "default_boss_stat_multiplier")]
    pub stat_multiplier: f32,
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
pub struct DifficultyRewardsRules {
    #[serde(default)]
    pub exp: bool,
    #[serde(default)]
    pub currency: bool,
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
    #[serde(default)]
    pub ability_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StatsRules {
    pub track: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JobSystemRules {
    #[serde(default = "default_secondary_job_enabled")]
    pub secondary_jobs: bool,
    pub jp_mode: JpMode,
    #[serde(default)]
    pub job_exp_curve: ExpCurveRules,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActivityProgressionRules {
    #[serde(default)]
    pub weapon_gain: ActivityGainRules,
    #[serde(default)]
    pub magic_gain: ActivityGainRules,
    #[serde(default)]
    pub ranks: Vec<ActivityRank>,
    #[serde(default)]
    pub effects: ActivityEffectRules,
    #[serde(default = "default_activity_unarmed_category")]
    pub unarmed_category: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActivityGainRules {
    #[serde(default)]
    pub attack: f32,
    #[serde(default)]
    pub ability: f32,
    #[serde(default)]
    pub cast: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActivityEffectRules {
    #[serde(default = "default_activity_damage_scale")]
    pub damage_scale: f32,
    #[serde(default = "default_activity_hit_bonus")]
    pub hit_bonus: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActivityRank {
    pub min: f32,
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ActivityGrowthRules {
    #[serde(default = "default_activity_growth_base_rate")]
    pub base_rate: f32,
    #[serde(default = "default_activity_growth_min_gain_threshold")]
    pub min_gain_threshold: f32,
    #[serde(default = "default_activity_growth_min_battle_turns")]
    pub min_battle_turns: u32,
    #[serde(default = "default_activity_growth_danger_min")]
    pub danger_factor_min: f32,
    #[serde(default = "default_activity_growth_danger_max")]
    pub danger_factor_max: f32,
    #[serde(default = "default_activity_growth_floor_exponent")]
    pub floor_depth_exponent: f32,
    #[serde(default = "default_activity_growth_status_weight")]
    pub status_effect_weight: f32,
    #[serde(default = "default_activity_growth_initiative_weight")]
    pub initiative_weight: f32,
    #[serde(default = "default_activity_growth_combo_weight")]
    pub combo_weight: f32,
    #[serde(default = "default_activity_growth_survival_bonus")]
    pub survival_bonus: f32,
    #[serde(default)]
    pub soft_caps: HashMap<String, f32>,
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
    pub progression_mode: ProgressionMode,
    pub activity_progression: ActivityProgressionRules,
    pub activity_growth: ActivityGrowthRules,
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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettingsRules {
    #[serde(default)]
    pub autosave_enabled: Option<ToggleSetting>,
    #[serde(default)]
    pub readiness_speed: Option<RangeSetting>,
    #[serde(default)]
    pub difficulty_scale: Option<RangeSetting>,
    #[serde(default)]
    pub battle_mode: Option<ChoiceSetting<BattleMode>>,
    #[serde(default)]
    pub death_markers_visible: Option<ToggleSetting>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ToggleSetting {
    pub value: bool,
    #[serde(default = "default_setting_visible")]
    pub visible: bool,
    #[serde(default = "default_setting_editable")]
    pub editable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RangeSetting {
    pub value: f32,
    #[serde(default = "default_readiness_speed_min")]
    pub min: f32,
    #[serde(default = "default_readiness_speed_max")]
    pub max: f32,
    #[serde(default = "default_readiness_speed_step")]
    pub step: f32,
    #[serde(default = "default_setting_visible")]
    pub visible: bool,
    #[serde(default = "default_setting_editable")]
    pub editable: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ChoiceSetting<T> {
    pub value: T,
    #[serde(default)]
    pub options: Vec<T>,
    #[serde(default = "default_setting_visible")]
    pub visible: bool,
    #[serde(default = "default_setting_editable")]
    pub editable: bool,
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
            progression_mode: default_progression_mode(),
            activity_progression: ActivityProgressionRules::default(),
            activity_growth: ActivityGrowthRules::default(),
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
            progression_mode: file.progression_mode,
            activity_progression: file.activity_progression,
            activity_growth: file.activity_growth,
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BattleMode {
    Turn,
    DynamicWait,
    Dynamic,
}

impl Default for BattleMode {
    fn default() -> Self {
        BattleMode::Turn
    }
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
pub enum ProgressionMode {
    Character,
    Job,
    JobPoints,
    Activity,
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
        }
    }
}

impl Default for SettingsRules {
    fn default() -> Self {
        Self {
            autosave_enabled: None,
            readiness_speed: None,
            difficulty_scale: None,
            battle_mode: None,
            death_markers_visible: None,
        }
    }
}

impl Default for BattleRules {
    fn default() -> Self {
        Self {
            exp_for_fallen: default_battle_exp_for_fallen(),
            global_commands: default_battle_global_commands(),
            commands: default_battle_commands(),
            formulas: BattleFormulaRules::default(),
            rows: BattleRowRules::default(),
            boss_scaling: BossScalingRules::default(),
            difficulty_rewards: DifficultyRewardsRules::default(),
            enemy_ai_defaults: EnemyAiDefaults::default(),
        }
    }
}

impl Default for ActivityProgressionRules {
    fn default() -> Self {
        Self {
            weapon_gain: ActivityGainRules {
                attack: 0.02,
                ability: 0.03,
                cast: 0.0,
            },
            magic_gain: ActivityGainRules {
                attack: 0.0,
                ability: 0.0,
                cast: 0.02,
            },
            ranks: default_activity_ranks(),
            effects: ActivityEffectRules::default(),
            unarmed_category: default_activity_unarmed_category(),
        }
    }
}

impl Default for ActivityGrowthRules {
    fn default() -> Self {
        Self {
            base_rate: default_activity_growth_base_rate(),
            min_gain_threshold: default_activity_growth_min_gain_threshold(),
            min_battle_turns: default_activity_growth_min_battle_turns(),
            danger_factor_min: default_activity_growth_danger_min(),
            danger_factor_max: default_activity_growth_danger_max(),
            floor_depth_exponent: default_activity_growth_floor_exponent(),
            status_effect_weight: default_activity_growth_status_weight(),
            initiative_weight: default_activity_growth_initiative_weight(),
            combo_weight: default_activity_growth_combo_weight(),
            survival_bonus: default_activity_growth_survival_bonus(),
            soft_caps: HashMap::new(),
        }
    }
}

impl Default for ActivityEffectRules {
    fn default() -> Self {
        Self {
            damage_scale: default_activity_damage_scale(),
            hit_bonus: default_activity_hit_bonus(),
        }
    }
}

impl Default for ActivityGainRules {
    fn default() -> Self {
        Self {
            attack: 0.0,
            ability: 0.0,
            cast: 0.0,
        }
    }
}

impl Default for BattleFormulaRules {
    fn default() -> Self {
        Self {
            physical: None,
            magic: None,
            heal: None,
            hit: None,
            crit: None,
            crit_multiplier: default_battle_crit_multiplier(),
        }
    }
}

impl Default for BossScalingRules {
    fn default() -> Self {
        Self {
            enabled: false,
            hp_multiplier: default_boss_hp_multiplier(),
            stat_multiplier: default_boss_stat_multiplier(),
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

impl Default for DifficultyRewardsRules {
    fn default() -> Self {
        Self {
            exp: false,
            currency: false,
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

pub const READINESS_SPEED_MIN: f32 = 0.5;
pub const READINESS_SPEED_MAX: f32 = 5.0;
pub const READINESS_SPEED_STEP: f32 = 0.5;
pub const DIFFICULTY_SCALE_DEFAULT: f32 = 1.0;
pub const DIFFICULTY_SCALE_MIN: f32 = 0.5;
pub const DIFFICULTY_SCALE_MAX: f32 = 2.0;
pub const DIFFICULTY_SCALE_STEP: f32 = 0.1;

fn default_setting_visible() -> bool {
    true
}

fn default_setting_editable() -> bool {
    true
}

fn default_readiness_speed_min() -> f32 {
    READINESS_SPEED_MIN
}

fn default_readiness_speed_max() -> f32 {
    READINESS_SPEED_MAX
}

fn default_readiness_speed_step() -> f32 {
    READINESS_SPEED_STEP
}

fn default_death_marker_glyph() -> String {
    "✞".to_string()
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

fn default_battle_exp_for_fallen() -> bool {
    false
}

fn default_battle_commands() -> Vec<BattleCommandDefinition> {
    vec![
        BattleCommandDefinition {
            id: "attack".to_string(),
            label: "Attack".to_string(),
            kind: "attack".to_string(),
            sort_order: 10,
            ability_group: None,
            ability_id: None,
        },
        BattleCommandDefinition {
            id: "magic".to_string(),
            label: "Magic".to_string(),
            kind: "magic".to_string(),
            sort_order: 30,
            ability_group: None,
            ability_id: None,
        },
        BattleCommandDefinition {
            id: "abilities".to_string(),
            label: "Abilities".to_string(),
            kind: "abilities".to_string(),
            sort_order: 40,
            ability_group: None,
            ability_id: None,
        },
        BattleCommandDefinition {
            id: "items".to_string(),
            label: "Items".to_string(),
            kind: "items".to_string(),
            sort_order: 50,
            ability_group: None,
            ability_id: None,
        },
        BattleCommandDefinition {
            id: "defend".to_string(),
            label: "Defend".to_string(),
            kind: "defend".to_string(),
            sort_order: 60,
            ability_group: None,
            ability_id: None,
        },
        BattleCommandDefinition {
            id: "run".to_string(),
            label: "Run".to_string(),
            kind: "run".to_string(),
            sort_order: 70,
            ability_group: None,
            ability_id: None,
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

fn default_battle_crit_multiplier() -> f32 {
    1.5
}

fn default_enemy_ai_reroll() -> f32 {
    0.5
}

fn default_enemy_ai_cooldown() -> u32 {
    0
}

fn default_boss_hp_multiplier() -> f32 {
    1.2
}

fn default_boss_stat_multiplier() -> f32 {
    1.1
}

fn default_battle_command_sort_order() -> i32 {
    100
}

fn default_progression_mode() -> ProgressionMode {
    ProgressionMode::Character
}

fn default_secondary_job_enabled() -> bool {
    false
}

impl Default for JobSystemRules {
    fn default() -> Self {
        Self {
            secondary_jobs: default_secondary_job_enabled(),
            jp_mode: JpMode::Earn,
            job_exp_curve: ExpCurveRules::default(),
        }
    }
}

fn default_activity_damage_scale() -> f32 {
    0.25
}

fn default_activity_hit_bonus() -> f32 {
    0.15
}

fn default_activity_unarmed_category() -> String {
    "unarmed".to_string()
}

fn default_activity_ranks() -> Vec<ActivityRank> {
    vec![
        ActivityRank {
            min: 0.0,
            label: "Novice".to_string(),
        },
        ActivityRank {
            min: 0.2,
            label: "Skilled".to_string(),
        },
        ActivityRank {
            min: 0.5,
            label: "Veteran".to_string(),
        },
        ActivityRank {
            min: 0.8,
            label: "Master".to_string(),
        },
    ]
}

fn default_activity_growth_base_rate() -> f32 {
    0.35
}

fn default_activity_growth_min_gain_threshold() -> f32 {
    0.25
}

fn default_activity_growth_min_battle_turns() -> u32 {
    1
}

fn default_activity_growth_danger_min() -> f32 {
    0.25
}

fn default_activity_growth_danger_max() -> f32 {
    2.0
}

fn default_activity_growth_floor_exponent() -> f32 {
    0.0
}

fn default_activity_growth_status_weight() -> f32 {
    1.0
}

fn default_activity_growth_initiative_weight() -> f32 {
    0.0
}

fn default_activity_growth_combo_weight() -> f32 {
    0.0
}

fn default_activity_growth_survival_bonus() -> f32 {
    1.2
}
