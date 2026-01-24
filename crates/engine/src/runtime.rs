use crate::content::Content;

#[derive(Clone, Debug)]
pub enum GameState {
    Title,
    Overworld,
    Dialog,
    Menu,
    Battle,
    Event,
}

pub struct GameRuntime {
    pub content: Content,
    pub state: GameState,
}

impl GameRuntime {
    pub fn new(content: Content) -> Self {
        Self {
            content,
            state: GameState::Title,
        }
    }

    pub fn transition_to(&mut self, state: GameState) {
        self.state = state;
    }
}
