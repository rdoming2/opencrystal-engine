use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::inventory::InventoryStack;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MapFile {
    pub version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub hide_name: bool,
    pub world: String,
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<String>,
    pub legend: HashMap<String, TileLegend>,
    pub encounters: Vec<EncounterZone>,
    #[serde(default)]
    pub encounter_rate: f32,
    pub events: Vec<MapEvent>,
    pub npcs: Vec<MapNpc>,
    #[serde(default)]
    pub signs: Vec<MapSign>,
    #[serde(default)]
    pub chests: Vec<MapChest>,
    pub shops: Vec<MapShop>,
    #[serde(default = "default_allow_save")]
    pub allow_save: bool,
    #[serde(default)]
    pub save_points: Vec<[i32; 2]>,
    #[serde(default)]
    pub transitions: Vec<MapTransition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MapSign {
    pub id: String,
    pub pos: [i32; 2],
    #[serde(default)]
    pub glyph: Option<String>,
    #[serde(default)]
    pub palette: Option<String>,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MapChest {
    pub id: String,
    pub pos: [i32; 2],
    #[serde(default)]
    pub glyph_closed: Option<String>,
    #[serde(default)]
    pub glyph_open: Option<String>,
    #[serde(default)]
    pub palette: Option<String>,
    pub opened_flag: String,
    #[serde(default)]
    pub loot: MapChestLoot,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct MapChestLoot {
    #[serde(default)]
    pub items: Vec<InventoryStack>,
    #[serde(default)]
    pub equipment: Vec<InventoryStack>,
    #[serde(default)]
    pub currency: Vec<MapCurrencyStack>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MapCurrencyStack {
    pub id: String,
    pub amount: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TileLegend {
    pub tile: String,
    pub passable: bool,
    #[serde(default)]
    pub palette: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EncounterZone {
    pub zone_id: String,
    pub rect: [i32; 4],
    pub table: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MapEvent {
    pub id: String,
    pub trigger: String,
    pub script: String,
    #[serde(default)]
    pub zone: Option<String>,
    #[serde(default)]
    pub pos: Option<[i32; 2]>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MapNpc {
    pub id: String,
    pub pos: [i32; 2],
    #[serde(default)]
    pub script: Option<String>,
    pub requires_flags: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MapShop {
    pub id: String,
    pub pos: [i32; 2],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MapTransition {
    pub id: String,
    pub pos: [i32; 2],
    pub target_map: String,
    pub target_pos: [i32; 2],
    #[serde(default)]
    pub return_to_last: bool,
    #[serde(default)]
    pub glyph: Option<String>,
    #[serde(default)]
    pub palette: Option<String>,
}

impl MapFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

fn default_allow_save() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityState {
    pub pos: Option<(i32, i32)>,
    pub state: Option<String>,
    pub visible: Option<bool>,
    pub sprite: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MapState {
    pub flags: HashSet<String>,
    pub entities: HashMap<String, EntityState>,
}
