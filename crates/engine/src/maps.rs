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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TileLegend {
    pub tile: String,
    pub passable: bool,
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
    pub name: String,
    pub pos: [i32; 2],
    pub sprite: String,
    pub script: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MapShop {
    pub id: String,
    pub pos: [i32; 2],
}

impl MapFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}
