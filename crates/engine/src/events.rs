use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct EventQueue {
    pub pending: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventFile {
    pub version: u32,
    pub id: String,
    pub steps: Vec<EventStep>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventStep {
    pub r#type: String,
    pub speaker: Option<String>,
    pub text: Option<String>,
    pub flag: Option<String>,
    pub flags: Option<Vec<String>>,
    pub requires: Option<Vec<String>>,
    pub item: Option<String>,
    pub qty: Option<i32>,
    pub shop: Option<String>,
    pub target: Option<EventTarget>,
    pub encounter: Option<String>,
    pub formation: Option<Vec<FormationMember>>,
    pub npc: Option<String>,
    pub pos: Option<[i32; 2]>,
    pub sprite: Option<String>,
    pub dialog: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventTarget {
    pub map: String,
    pub pos: [i32; 2],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FormationMember {
    pub enemy: String,
    pub pos: [i32; 2],
}

impl EventFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}
