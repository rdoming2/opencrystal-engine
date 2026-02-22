pub mod battle;
pub mod dialog;
pub mod input;
pub mod layout;
pub mod menu;
pub mod map_editor;
pub mod overworld;
pub mod renderer;
pub mod session;
pub mod shop;
pub mod title;
pub mod ui;
pub mod utils;

#[derive(Clone, Debug)]
pub struct TuiApp {
    pub should_quit: bool,
}

impl TuiApp {
    pub fn new() -> Self {
        Self { should_quit: false }
    }
}
