use crate::content::Content;
use crate::rules::Ruleset;

#[derive(Clone, Debug, PartialEq)]
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
    pub event_queue: Vec<String>,
    pub active_event: Option<String>,
    pub event_step: usize,
}

impl GameRuntime {
    pub fn new(content: Content) -> Self {
        Self {
            content,
            state: GameState::Title,
            event_queue: Vec::new(),
            active_event: None,
            event_step: 0,
        }
    }

    pub fn transition_to(&mut self, state: GameState) {
        self.state = state;
    }

    pub fn start_new_game(&mut self, rules: &Ruleset) {
        if let Some(event_id) = &rules.start_event {
            self.event_queue.push(event_id.clone());
            self.state = GameState::Event;
            self.start_next_event();
        } else {
            self.state = GameState::Overworld;
        }
    }

    pub fn next_event_step(&mut self) -> Option<crate::events::EventStep> {
        if self.state != GameState::Event {
            return None;
        }

        let event_id = match self.active_event.clone() {
            Some(event_id) => event_id,
            None => return None,
        };

        let event_index = match self.content.event_index.get(&event_id) {
            Some(index) => *index,
            None => {
                self.advance_event_queue();
                return None;
            }
        };

        let event = &self.content.events[event_index];
        if self.event_step >= event.steps.len() {
            self.advance_event_queue();
            return None;
        }

        let step = event.steps[self.event_step].clone();
        self.event_step += 1;
        Some(step)
    }

    pub fn is_event_complete(&self) -> bool {
        if self.state != GameState::Event {
            return true;
        }

        let event_id = match &self.active_event {
            Some(event_id) => event_id,
            None => return true,
        };

        let event_index = match self.content.event_index.get(event_id) {
            Some(index) => *index,
            None => return true,
        };
        let event = &self.content.events[event_index];
        self.event_step >= event.steps.len()
    }

    fn start_next_event(&mut self) {
        while let Some(event_id) = self.event_queue.first().cloned() {
            if self.content.event_index.contains_key(&event_id) {
                self.active_event = Some(event_id);
                self.event_step = 0;
                return;
            }
            self.event_queue.remove(0);
        }

        self.active_event = None;
        self.state = GameState::Overworld;
    }

    fn advance_event_queue(&mut self) {
        if !self.event_queue.is_empty() {
            self.event_queue.remove(0);
        }
        self.start_next_event();
    }
}
