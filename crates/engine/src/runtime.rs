use crate::content::Content;
use crate::party::PartyState;
use crate::rules::Ruleset;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub enum GameState {
    Title,
    Overworld,
    Dialog,
    Menu,
    Battle,
    Event,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MenuFocus {
    List,
    Detail,
}

#[derive(Clone, Debug)]
pub struct MenuState {
    pub focus: MenuFocus,
    pub selected: usize,
    pub active_submenu: Option<String>,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            focus: MenuFocus::List,
            selected: 0,
            active_submenu: None,
        }
    }
}

pub struct GameRuntime {
    pub content: Content,
    pub state: GameState,
    pub event_queue: Vec<String>,
    pub active_event: Option<String>,
    pub event_step: usize,
    pub flags: HashSet<String>,
    pub menu_state: MenuState,
    pub party: PartyState,
}

impl GameRuntime {
    pub fn new(content: Content) -> Self {
        Self {
            content,
            state: GameState::Title,
            event_queue: Vec::new(),
            active_event: None,
            event_step: 0,
            flags: HashSet::new(),
            menu_state: MenuState::default(),
            party: PartyState::empty(),
        }
    }

    pub fn transition_to(&mut self, state: GameState) {
        self.state = state;
    }

    pub fn open_menu(&mut self) {
        self.state = GameState::Menu;
        self.menu_state = MenuState::default();
    }

    pub fn close_menu(&mut self) {
        self.state = GameState::Overworld;
        self.menu_state.active_submenu = None;
        self.menu_state.focus = MenuFocus::List;
    }

    pub fn set_flag(&mut self, flag: &str) {
        self.flags.insert(flag.to_string());
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    pub fn start_new_game(&mut self, rules: &Ruleset) {
        if rules.party_mode == crate::rules::PartyMode::Predefined || self.party.roster.is_empty() {
            self.party = PartyState::from_content(&self.content, rules);
        }
        if let Some(event_id) = &rules.start_event {
            self.queue_event(event_id);
            self.state = GameState::Event;
            self.start_next_event();
        } else {
            self.state = GameState::Overworld;
        }
    }

    pub fn queue_event(&mut self, event_id: &str) {
        self.event_queue.push(event_id.to_string());
    }

    pub fn get_dialog(&self, dialog_id: &str) -> Option<&crate::dialog::DialogFile> {
        let index = self.content.dialog_index.get(dialog_id)?;
        self.content.dialogs.get(*index)
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
