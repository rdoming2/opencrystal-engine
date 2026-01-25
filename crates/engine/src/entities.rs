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
    pub spells: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GrowthConfig {
    pub r#type: String,
    pub per_level: HashMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JobEquipment {
    pub weapons: Vec<String>,
    pub armor: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpellsFile {
    pub version: u32,
    pub schools: Vec<MagicSchool>,
    pub spells: Vec<SpellDefinition>,
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
pub struct ItemsFile {
    pub version: u32,
    pub items: Vec<ItemDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ItemDefinition {
    pub id: String,
    pub name: String,
    pub r#type: String,
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
