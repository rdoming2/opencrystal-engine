use crate::content::Content;
use crate::events::{EventExecutionResult, EventStep};
use crate::inventory::InventoryState;
use crate::maps::MapState;
use crate::menu::{MenuFocus, MenuState};
use crate::party::{reset_magic_tier_charges, PartyState};
use crate::rules::Ruleset;
use crate::world::WorldState;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

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
    pub flags: HashSet<String>,
    pub map_states: HashMap<String, MapState>,
    pub menu_state: MenuState,
    pub party: PartyState,
    pub inventory: InventoryState,
    pub world: WorldState,
    pub playtime: u64,
    pub start_time: Instant,
}

impl GameRuntime {
    pub fn new(content: Content) -> Self {
        let start_location = content.rules.game.start_location.clone();
        Self {
            content,
            state: GameState::Title,
            event_queue: Vec::new(),
            active_event: None,
            event_step: 0,
            flags: HashSet::new(),
            map_states: HashMap::new(),
            menu_state: MenuState::default(),
            party: PartyState::empty(),
            inventory: InventoryState::default(),
            world: WorldState::new(
                &start_location.world,
                &start_location.map,
                (start_location.x, start_location.y),
            ),
            playtime: 0,
            start_time: Instant::now(),
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
        self.menu_state.detail_page = 0;
        self.menu_state.detail_selection = 0;
        self.menu_state.detail_filter = 0;
        self.menu_state.detail_sort = 0;
        self.menu_state.detail_actor = 0;
        self.menu_state.detail_slot = 0;
        self.menu_state.detail_target = 0;
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
        reset_magic_tier_charges(&mut self.party, &self.content, rules);
        if self.inventory.is_empty() {
            for item in &rules.inventory.items {
                self.inventory
                    .add_item(&item.id, item.qty, rules.inventory.max_stack);
            }
            for item in &rules.inventory.equipment {
                self.inventory
                    .add_equipment(&item.id, item.qty, rules.inventory.max_stack);
            }
        }
        self.playtime = 0;
        self.start_time = Instant::now();
        self.world.world_id = rules.start_location.world.clone();
        self.world.map_id = rules.start_location.map.clone();
        self.world.position = (rules.start_location.x, rules.start_location.y);
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

    pub fn start_next_event(&mut self) {
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

    pub fn abort_event(&mut self) {
        self.active_event = None;
        self.event_step = 0;
        self.state = GameState::Overworld;
    }

    pub fn apply_event_step(&mut self, step: &EventStep) -> EventExecutionResult {
        crate::events::apply_event_step(self, step)
    }

    pub fn get_on_enter_events_for_map(&self, map_id: &str) -> Vec<String> {
        self.content.get_map_on_enter_events(map_id)
    }

    pub fn get_on_step_events_for_position(&self, map_id: &str, pos: (i32, i32)) -> Vec<String> {
        self.content.get_map_on_step_events(map_id, pos)
    }

    pub fn get_on_step_events_for_zone(
        &self,
        map_id: &str,
        pos: (i32, i32),
        previous_pos: (i32, i32),
    ) -> Vec<String> {
        self.content
            .get_zone_on_step_events(map_id, pos, previous_pos)
    }

    pub fn apply_dialog_action(
        &mut self,
        action: &crate::dialog::DialogAction,
    ) -> EventExecutionResult {
        crate::dialog::apply_dialog_action(self, action)
    }
}
