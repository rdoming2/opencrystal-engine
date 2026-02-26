use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EncountersFile {
    pub version: u32,
    pub tables: Vec<EncounterTable>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EncounterTable {
    pub id: String,
    pub entries: Vec<EncounterEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EncounterEntry {
    pub weight: i32,
    #[serde(default)]
    pub tile: Option<String>,
    pub formation: Vec<EncounterMember>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EncounterMember {
    pub enemy: String,
    pub pos: [i32; 2],
}

impl EncountersFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}
