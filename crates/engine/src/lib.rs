pub mod battle;
pub mod content;
pub mod dialog;
pub mod encounters;
pub mod entities;
pub mod events;
pub mod expr;
pub mod inventory;

pub mod io;
pub mod maps;
pub mod party;
pub mod rules;
pub mod runtime;
pub mod save;
pub mod stats;
pub mod validate;
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
