use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StatsFile {
    pub version: u32,
    pub stats: StatsDefinition,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StatsDefinition {
    pub base: Vec<StatEntry>,
    pub derived: Vec<StatEntry>,
    pub formulas: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub growth_formulas: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StatEntry {
    pub id: String,
    pub name: String,
    pub min: Option<i32>,
}

impl StatsFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}
