use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MapFile {
    pub version: u32,
    pub id: String,
    pub name: String,
    pub world: String,
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<String>,
    pub legend: HashMap<String, TileLegend>,
    pub encounters: Vec<EncounterZone>,
    pub events: Vec<MapEvent>,
    pub npcs: Vec<MapNpc>,
    pub shops: Vec<MapShop>,
    #[serde(default = "default_allow_save")]
    pub allow_save: bool,
    #[serde(default)]
    pub save_points: Vec<[i32; 2]>,
    #[serde(default)]
    pub transitions: Vec<MapTransition>,
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
