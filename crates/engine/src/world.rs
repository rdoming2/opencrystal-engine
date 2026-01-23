use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorldsFile {
    pub version: u32,
    pub worlds: Vec<WorldDefinition>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorldDefinition {
    pub id: String,
    pub name: String,
    pub starting_map: String,
    pub zoom_levels: Vec<String>,
    pub overview: OverviewConfig,
    pub vehicles: Vec<String>,
    pub fast_travel: FastTravelConfig,
    pub links: Vec<WorldLink>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OverviewConfig {
    pub enabled: bool,
    pub map_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FastTravelConfig {
    pub enabled: bool,
    pub requires_flag: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorldLink {
    pub to_world: String,
    pub requires_flag: String,
}

impl WorldsFile {
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

#[derive(Clone, Debug)]
pub struct WorldState {
    pub world_id: String,
    pub map_id: String,
    pub position: (i32, i32),
}

impl WorldState {
    pub fn new(
        world_id: impl Into<String>,
        map_id: impl Into<String>,
        position: (i32, i32),
    ) -> Self {
        Self {
            world_id: world_id.into(),
            map_id: map_id.into(),
            position,
        }
    }
}
