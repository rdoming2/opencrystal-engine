pub mod battle;
pub mod entities;
pub mod events;
pub mod rules;
pub mod save;
pub mod world;

pub struct Engine {
    pub rules: rules::Ruleset,
    pub world: world::WorldState,
}

impl Engine {
    pub fn new(rules: rules::Ruleset, world: world::WorldState) -> Self {
        Self { rules, world }
    }
}
