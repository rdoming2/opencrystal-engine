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
