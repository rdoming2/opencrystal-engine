pub mod input;
pub mod layout;
pub mod renderer;
pub mod ui;
pub mod app;

#[derive(Clone, Debug)]
pub struct TuiApp {
    pub should_quit: bool,
}

impl TuiApp {
    pub fn new() -> Self {
        Self { should_quit: false }
    }
}
