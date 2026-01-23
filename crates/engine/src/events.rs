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
}

impl EventFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}
