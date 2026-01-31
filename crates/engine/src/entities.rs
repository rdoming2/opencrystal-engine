use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

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
    pub method: String,
    #[serde(default)]
    pub level: Option<u32>,
    #[serde(default)]
    pub tier: Option<u32>,
    #[serde(default)]
    pub item: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JobAbility {
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub level: Option<u32>,
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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AbilityDefinition {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub default_target: String,
    pub allowed_targets: Vec<String>,
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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AbilityEffect {
    pub r#type: String,
    pub power: i32,
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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ItemEffect {
    pub r#type: String,
    pub power: Option<i32>,
    pub target: Option<String>,
    pub destination: Option<ItemDestination>,
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
    pub currency: i32,
    pub loot: Vec<EnemyLoot>,
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
    pub persist: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShopDefinition {
    pub id: String,
    pub name: String,
    pub currency: String,
    pub inventory: Vec<ShopEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ShopEntry {
    pub item: String,
    pub price: i32,
}

impl JobsFile {
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
