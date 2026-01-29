use crate::content::Content;
use crate::events::{EventExecutionResult, EventStep};
use crate::inventory::InventoryState;
use crate::party::{reset_magic_tier_charges, PartyState};
use crate::rules::Ruleset;
use serde::{Deserialize, Serialize};
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
    pub detail_page: usize,
    pub detail_selection: usize,
    pub detail_filter: usize,
    pub detail_sort: usize,
    pub detail_actor: usize,
    pub detail_slot: usize,
    pub detail_target: usize,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            focus: MenuFocus::List,
            selected: 0,
            active_submenu: None,
            detail_page: 0,
            detail_selection: 0,
            detail_filter: 0,
            detail_sort: 0,
            detail_actor: 0,
            detail_slot: 0,
            detail_target: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntityState {
    pub pos: Option<(i32, i32)>,
    pub state: Option<String>,
    pub visible: Option<bool>,
    pub sprite: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MapState {
    pub flags: HashSet<String>,
    pub entities: HashMap<String, EntityState>,
}

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
            world: WorldState::new("gaia", "overworld_gaia", (0, 0)),
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
        reset_magic_tier_charges(&mut self.party, rules);
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
        match step.r#type.as_str() {
            "dialog" => {
                let speaker = step.speaker.as_deref().unwrap_or("Narrator");
                let text = step.text.as_deref().unwrap_or("");
                EventExecutionResult::Dialog {
                    speaker: speaker.to_string(),
                    text: text.to_string(),
                }
            }
            "narration" => {
                let text = step.text.as_deref().unwrap_or("");
                EventExecutionResult::Narration {
                    text: text.to_string(),
                }
            }
            "start_dialog" => {
                if let Some(dialog) = &step.dialog {
                    EventExecutionResult::StartDialog {
                        dialog_id: dialog.clone(),
                    }
                } else {
                    EventExecutionResult::Continue
                }
            }
            "set_flag" => {
                if let Some(flag) = &step.flag {
                    self.set_flag(flag);
                }
                EventExecutionResult::Continue
            }
            "require_flags" => {
                if let Some(flags) = &step.flags {
                    let missing = flags
                        .iter()
                        .filter(|flag| !self.has_flag(flag))
                        .cloned()
                        .collect::<Vec<_>>();
                    if !missing.is_empty() {
                        self.abort_event();
                        return EventExecutionResult::Abort;
                    }
                }
                EventExecutionResult::Continue
            }
            "give_item" => {
                if let Some(item) = &step.item {
                    let qty = step.qty.unwrap_or(1);
                    let max_stack = self.content.rules.inventory.max_stack;
                    self.inventory.add_item(item, qty, max_stack);
                }
                EventExecutionResult::Continue
            }
            "give_equipment" => {
                if let Some(item) = &step.item {
                    let qty = step.qty.unwrap_or(1);
                    let max_stack = self.content.rules.inventory.max_stack;
                    self.inventory.add_equipment(item, qty, max_stack);
                }
                EventExecutionResult::Continue
            }
            "warp" => {
                if let Some(target) = &step.target {
                    self.world.map_id = target.map.clone();
                    self.world.position = (target.pos[0], target.pos[1]);
                }
                EventExecutionResult::Continue
            }
            "start_battle" => {
                let encounter = step.encounter.clone().unwrap_or_default();
                let formation = step.formation.clone().unwrap_or_default();
                EventExecutionResult::StartBattle {
                    encounter,
                    formation,
                }
            }
            "open_shop" => {
                if let Some(shop) = &step.shop {
                    EventExecutionResult::OpenShop {
                        shop_id: shop.clone(),
                    }
                } else {
                    EventExecutionResult::Continue
                }
            }
            "npc_show" | "npc_hide" | "npc_move" | "npc_set_sprite" => {
                if let Some(npc_id) = &step.npc {
                    let map_id = self.world.map_id.clone();
                    let map_state = self.map_states.entry(map_id).or_default();
                    let entity_state =
                        map_state
                            .entities
                            .entry(npc_id.clone())
                            .or_insert(EntityState {
                                pos: None,
                                state: None,
                                visible: None,
                                sprite: None,
                            });

                    match step.r#type.as_str() {
                        "npc_show" => entity_state.visible = Some(true),
                        "npc_hide" => entity_state.visible = Some(false),
                        "npc_move" => {
                            if let Some(pos) = step.pos {
                                entity_state.pos = Some((pos[0], pos[1]));
                            }
                        }
                        "npc_set_sprite" => {
                            if let Some(sprite) = &step.sprite {
                                entity_state.sprite = Some(sprite.clone());
                            }
                        }
                        _ => {}
                    }
                }
                EventExecutionResult::Continue
            }
            _ => EventExecutionResult::Continue,
        }
    }

    pub fn get_on_enter_events_for_map(&self, map_id: &str) -> Vec<String> {
        let map_index = match self.content.map_index.get(map_id) {
            Some(index) => *index,
            None => return Vec::new(),
        };
        let map = &self.content.maps[map_index];
        let mut events = Vec::new();
        for map_event in &map.events {
            if map_event.trigger == "on_enter" {
                events.push(map_event.script.clone());
            }
        }
        events
    }

    pub fn get_on_step_events_for_position(&self, map_id: &str, pos: (i32, i32)) -> Vec<String> {
        let map_index = match self.content.map_index.get(map_id) {
            Some(index) => *index,
            None => return Vec::new(),
        };
        let map = &self.content.maps[map_index];
        let mut events = Vec::new();
        for map_event in &map.events {
            if map_event.trigger != "on_step" {
                continue;
            }
            if let Some(event_pos) = map_event.pos {
                if event_pos[0] == pos.0 && event_pos[1] == pos.1 {
                    events.push(map_event.script.clone());
                }
            }
        }
        events
    }

    pub fn get_on_step_events_for_zone(
        &self,
        map_id: &str,
        pos: (i32, i32),
        previous_pos: (i32, i32),
    ) -> Vec<String> {
        let map_index = match self.content.map_index.get(map_id) {
            Some(index) => *index,
            None => return Vec::new(),
        };
        let map = &self.content.maps[map_index];
        let mut events = Vec::new();
        for map_event in &map.events {
            if map_event.trigger != "on_step" {
                continue;
            }
            let Some(zone_id) = &map_event.zone else {
                continue;
            };
            let zone = map.encounters.iter().find(|z| &z.zone_id == zone_id);
            let Some(zone_rect) = zone.map(|z| z.rect) else {
                continue;
            };
            let in_zone_current = pos.0 >= zone_rect[0]
                && pos.0 < zone_rect[2]
                && pos.1 >= zone_rect[1]
                && pos.1 < zone_rect[3];
            let in_zone_previous = previous_pos.0 >= zone_rect[0]
                && previous_pos.0 < zone_rect[2]
                && previous_pos.1 >= zone_rect[1]
                && previous_pos.1 < zone_rect[3];
            if in_zone_current && !in_zone_previous {
                events.push(map_event.script.clone());
            }
        }
        events
    }

    pub fn apply_dialog_action(
        &mut self,
        action: &crate::dialog::DialogAction,
    ) -> EventExecutionResult {
        match action.r#type.as_str() {
            "start_event" => {
                if let Some(event_id) = &action.event {
                    self.queue_event(event_id);
                }
                EventExecutionResult::Continue
            }
            "open_shop" => {
                if let Some(shop_id) = &action.shop {
                    EventExecutionResult::OpenShop {
                        shop_id: shop_id.clone(),
                    }
                } else {
                    EventExecutionResult::Continue
                }
            }
            "set_flag" => {
                if let Some(flag) = &action.flag {
                    self.set_flag(flag);
                }
                EventExecutionResult::Continue
            }
            "give_item" => {
                if let Some(item) = &action.item {
                    let qty = action.qty.unwrap_or(1);
                    let max_stack = self.content.rules.inventory.max_stack;
                    self.inventory.add_item(item, qty, max_stack);
                }
                EventExecutionResult::Continue
            }
            _ => EventExecutionResult::Continue,
        }
    }
}
