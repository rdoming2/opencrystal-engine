use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::maps::MapCurrencyStack;
use crate::rules::{AbilityAcquisition, MagicAcquisition};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EffectsFile {
    pub version: u32,
    #[serde(default)]
    pub elements: Vec<ElementDefinition>,
    #[serde(default)]
    pub effects: Vec<EffectDefinition>,
    #[serde(default)]
    pub statuses: Vec<StatusDefinition>,
    #[serde(default)]
    pub traits: Vec<TraitDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StringsFile {
    pub version: u32,
    #[serde(default)]
    pub strings: HashMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ElementDefinition {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EffectDefinition {
    pub id: String,
    pub label: String,
    pub kind: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub element: Option<String>,
    #[serde(default)]
    pub damage_kind: Option<String>,
    #[serde(default)]
    pub power: Option<i32>,
    #[serde(default)]
    pub percent: Option<f32>,
    #[serde(default)]
    pub chance: Option<f32>,
    #[serde(default)]
    pub multiplier: Option<f32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StatusDefinition {
    pub id: String,
    pub label: String,
    pub short: String,
    #[serde(default)]
    pub default_duration: i32,
    #[serde(default)]
    pub reapply: String,
    #[serde(default)]
    pub tick: String,
    #[serde(default)]
    pub clear_on_battle_end: bool,
    #[serde(default)]
    pub effects: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TraitDefinition {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub effects: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct PartyMember {
    pub id: String,
    pub name: String,
    pub job_id: String,
}

#[derive(Clone, Debug)]
pub struct EnemyInstance {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JobsFile {
    pub version: u32,
    pub jobs: Vec<JobDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JobDefinition {
    pub id: String,
    pub name: String,
    pub stats: HashMap<String, i32>,
    pub growth: GrowthConfig,
    pub equipment: JobEquipment,
    #[serde(default)]
    pub equipment_slots: Vec<String>,
    #[serde(default)]
    pub accessory_slots: u8,
    #[serde(default)]
    pub can_dual_wield: bool,
    #[serde(default)]
    pub stat_modifiers: HashMap<String, StatModifier>,
    #[serde(default)]
    pub spells: Vec<JobSpell>,
    #[serde(default)]
    pub abilities: Vec<JobAbility>,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub starting_equipment: HashMap<String, String>,
    #[serde(default)]
    pub sprite: JobSprite,
    #[serde(default)]
    pub art: Option<JobArt>,
    #[serde(default)]
    pub unlock_flag: Option<String>,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub sort_order: Option<i32>,
    #[serde(default)]
    pub magic_slots: Option<HashMap<u32, Vec<i32>>>,
    #[serde(default)]
    pub magic_equip_progression: Option<MagicEquipProgression>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub magic_schools: Vec<String>,
    #[serde(default)]
    pub acquisition: Option<JobAcquisitionOverride>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MagicEquipProgression {
    #[serde(default)]
    pub slots: HashMap<u32, i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GrowthConfig {
    pub mode: String,
    #[serde(default)]
    pub per_level: HashMap<String, String>,
    #[serde(default)]
    pub tables: HashMap<String, Vec<i32>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JobEquipment {
    pub weapons: Vec<String>,
    pub armor: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JobSpell {
    pub id: String,
    #[serde(default)]
    pub level: Option<u32>,
    #[serde(default)]
    pub tier: Option<u32>,
    #[serde(default)]
    pub item: Option<String>,
    #[serde(default)]
    pub unlock_level: Option<u32>,
    #[serde(default)]
    pub jp_cost: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JobAbility {
    pub id: String,
    #[serde(default)]
    pub level: Option<u32>,
    #[serde(default)]
    pub unlock_level: Option<u32>,
    #[serde(default)]
    pub jp_cost: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum MagicAcquisitionOverride {
    Mode(MagicAcquisition),
    BySchool(HashMap<String, MagicAcquisition>),
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct JobAcquisitionOverride {
    #[serde(default)]
    pub magic: Option<MagicAcquisitionOverride>,
    #[serde(default)]
    pub abilities: Option<AbilityAcquisition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JobSprite {
    #[serde(default = "default_job_glyph")]
    pub glyph: String,
    #[serde(default = "default_job_palette")]
    pub palette: String,
}

impl Default for JobSprite {
    fn default() -> Self {
        Self {
            glyph: default_job_glyph(),
            palette: default_job_palette(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JobArt {
    pub lines: Vec<String>,
    pub palette: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StatModifier {
    #[serde(default)]
    pub add: Option<i32>,
    #[serde(default)]
    pub mult: Option<f32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpellsFile {
    pub version: u32,
    pub schools: Vec<MagicSchool>,
    pub spells: Vec<SpellDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AbilitiesFile {
    pub version: u32,
    pub abilities: Vec<AbilityDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MagicSchool {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpellDefinition {
    pub id: String,
    pub name: String,
    pub school: String,
    pub tier: u32,
    pub cost: SpellCost,
    pub default_target: String,
    pub allowed_targets: Vec<String>,
    #[serde(default = "default_target_mode")]
    pub target_mode: String,
    #[serde(default)]
    pub multi_attenuation: Option<f32>,
    pub effect: SpellEffect,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpellCost {
    pub r#type: String,
    pub value: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpellEffect {
    pub r#type: String,
    pub power: i32,
    pub element: Option<String>,
    #[serde(default)]
    pub effects: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AbilityDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub command_group: Option<String>,
    pub default_target: String,
    pub allowed_targets: Vec<String>,
    #[serde(default = "default_target_mode")]
    pub target_mode: String,
    #[serde(default)]
    pub multi_attenuation: Option<f32>,
    pub effect: AbilityEffect,
    #[serde(default)]
    pub cost: Option<AbilityCost>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AbilityCost {
    pub r#type: String,
    pub value: i32,
    #[serde(default)]
    pub item_id: Option<String>,
    #[serde(default)]
    pub currency_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AbilityEffect {
    pub r#type: String,
    pub power: i32,
    #[serde(default)]
    pub windup_turns: u32,
    #[serde(default)]
    pub vanish_during_windup: bool,
    #[serde(default)]
    pub effects: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ItemsFile {
    pub version: u32,
    pub items: Vec<ItemDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ItemDefinition {
    pub id: String,
    pub name: String,
    pub r#type: String,
    #[serde(default)]
    pub description: Option<String>,
    pub usage: ItemUsage,
    pub effect: ItemEffect,
    #[serde(default)]
    pub price: Option<HashMap<String, i32>>,
    #[serde(default)]
    pub sellable: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ItemEffect {
    pub r#type: String,
    pub power: Option<i32>,
    pub target: Option<String>,
    pub destination: Option<ItemDestination>,
    #[serde(default)]
    pub effects: Vec<String>,
    #[serde(default)]
    pub statuses: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ItemUsage {
    pub context: String,
    pub target: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ItemDestination {
    pub map: String,
    pub pos: [i32; 2],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EquipmentFile {
    pub version: u32,
    pub equipment: Vec<EquipmentDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EquipmentDefinition {
    pub id: String,
    pub name: String,
    pub category: String,
    pub slot: String,
    pub allowed_jobs: Option<Vec<String>>,
    pub stats: HashMap<String, i32>,
    #[serde(default)]
    pub spells: Vec<String>,
    #[serde(default)]
    pub abilities: Vec<String>,
    #[serde(default)]
    pub traits: Vec<String>,
    #[serde(default)]
    pub price: Option<HashMap<String, i32>>,
    #[serde(default)]
    pub sellable: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnemiesFile {
    pub version: u32,
    pub enemies: Vec<EnemyDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnemyDefinition {
    pub id: String,
    pub name: String,
    pub stats: HashMap<String, i32>,
    pub traits: Vec<String>,
    pub sprite: EnemySprite,
    pub art: Option<EnemyArt>,
    #[serde(default)]
    pub exp: i32,
    #[serde(default)]
    pub currency: Vec<MapCurrencyStack>,
    #[serde(default)]
    pub jp: i32,
    pub loot: Vec<EnemyLoot>,
    #[serde(default)]
    pub spells: Vec<String>,
    #[serde(default)]
    pub abilities: Vec<String>,
    #[serde(default = "default_enemy_mp_pool")]
    pub mp_pool: String,
    #[serde(default)]
    pub ai: EnemyAiConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnemyAiConfig {
    #[serde(default = "default_enemy_ai_mode")]
    pub mode: String,
    #[serde(default)]
    pub weights: EnemyAiWeights,
    #[serde(default = "default_enemy_ai_heal_threshold")]
    pub heal_below_hp: f32,
    #[serde(default = "default_enemy_ai_prefer_revive")]
    pub prefer_revive: bool,
}

impl Default for EnemyAiConfig {
    fn default() -> Self {
        Self {
            mode: default_enemy_ai_mode(),
            weights: EnemyAiWeights::default(),
            heal_below_hp: default_enemy_ai_heal_threshold(),
            prefer_revive: default_enemy_ai_prefer_revive(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnemyAiWeights {
    #[serde(default = "default_enemy_ai_attack_weight")]
    pub attack: i32,
    #[serde(default = "default_enemy_ai_spell_weight")]
    pub spells: i32,
    #[serde(default = "default_enemy_ai_ability_weight")]
    pub abilities: i32,
}

impl Default for EnemyAiWeights {
    fn default() -> Self {
        Self {
            attack: default_enemy_ai_attack_weight(),
            spells: default_enemy_ai_spell_weight(),
            abilities: default_enemy_ai_ability_weight(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnemySprite {
    pub glyph: String,
    pub palette: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnemyArt {
    pub lines: Vec<String>,
    pub palette: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnemyLoot {
    pub item: String,
    pub chance: f32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VehiclesFile {
    pub version: u32,
    pub vehicles: Vec<VehicleDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VehicleDefinition {
    pub id: String,
    pub name: String,
    pub speed: i32,
    pub allowed_tiles: Vec<String>,
    pub unlock_flag: String,
    #[serde(default)]
    pub glyph: Option<String>,
    #[serde(default)]
    pub palette: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShopsFile {
    pub version: u32,
    pub shops: Vec<ShopDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NpcsFile {
    pub version: u32,
    pub npcs: Vec<NpcDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NpcDefinition {
    pub id: String,
    pub name: String,
    pub sprite: String,
    #[serde(default)]
    pub palette: Option<String>,
    pub dialog: String,
    pub behavior: NpcBehavior,
    #[serde(default)]
    pub interaction_range: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NpcBehavior {
    pub r#type: String,
    pub radius: Option<i32>,
    pub path: Option<Vec<[i32; 2]>>,
    #[serde(default)]
    pub idle_chance: f32,
    pub persist: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShopDefinition {
    pub id: String,
    pub name: String,
    pub currency: String,
    pub inventory: Vec<ShopEntry>,
    #[serde(default = "default_buy_price_multiplier")]
    pub buy_price_multiplier: f32,
    #[serde(default = "default_sell_price_multiplier")]
    pub sell_price_multiplier: f32,
    #[serde(default = "default_sell_behavior")]
    pub sell_behavior: String,
    #[serde(default = "default_currency_pool")]
    pub currency_pool: String,
    #[serde(default)]
    pub currency_amount: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShopEntry {
    pub item: String,
    pub price: i32,
    #[serde(default)]
    pub stock: Option<i32>,
    #[serde(default)]
    pub sell_price: Option<i32>,
    #[serde(default)]
    pub category: Option<String>,
}

fn default_buy_price_multiplier() -> f32 {
    1.0
}

fn default_sell_price_multiplier() -> f32 {
    0.5
}

fn default_sell_behavior() -> String {
    "disappear".to_string()
}

fn default_currency_pool() -> String {
    "infinite".to_string()
}

impl JobsFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

impl EffectsFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

impl StringsFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

impl SpellsFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

impl AbilitiesFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

fn default_job_glyph() -> String {
    "@".to_string()
}

fn default_job_palette() -> String {
    "bright_cyan".to_string()
}

fn default_target_mode() -> String {
    "single".to_string()
}

fn default_enemy_mp_pool() -> String {
    "limited".to_string()
}

fn default_enemy_ai_mode() -> String {
    "basic".to_string()
}

fn default_enemy_ai_attack_weight() -> i32 {
    10
}

fn default_enemy_ai_spell_weight() -> i32 {
    8
}

fn default_enemy_ai_ability_weight() -> i32 {
    8
}

fn default_enemy_ai_heal_threshold() -> f32 {
    0.35
}

fn default_enemy_ai_prefer_revive() -> bool {
    true
}

impl ItemsFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

impl EquipmentFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

impl EnemiesFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

impl VehiclesFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

impl ShopsFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

impl NpcsFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}
