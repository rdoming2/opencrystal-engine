use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DialogFile {
    pub version: u32,
    pub id: String,
    pub nodes: Vec<DialogNode>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DialogNode {
    pub id: String,
    pub speaker: Option<String>,
    pub text: String,
    pub actions: Option<Vec<DialogAction>>,
    pub choices: Option<Vec<DialogChoice>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DialogAction {
    pub r#type: String,
    pub shop: Option<String>,
    pub flag: Option<String>,
    pub event: Option<String>,
    pub item: Option<String>,
    pub qty: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DialogChoice {
    pub label: String,
    pub next: String,
}

impl DialogFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}
