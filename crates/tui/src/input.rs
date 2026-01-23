use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Confirm,
    Cancel,
    Menu,
    Pause,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InputFile {
    pub version: u32,
    pub bindings: HashMap<String, Vec<String>>,
}

impl InputFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|err| format!("{}: {}", path.display(), err))?;
        serde_json::from_reader(file).map_err(|err| format!("{}: {}", path.display(), err))
    }
}
