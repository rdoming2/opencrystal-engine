pub mod input;
pub mod layout;
pub mod renderer;

#[derive(Clone, Debug)]
pub struct TuiApp {
    pub should_quit: bool,
}

impl TuiApp {
    pub fn new() -> Self {
        Self { should_quit: false }
    }
}
