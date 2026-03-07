use crate::content::Content;
use crate::events::{EventExecutionResult, EventStep};
use crate::inventory::InventoryState;
use crate::maps::MapState;
use crate::menu::{MenuFocus, MenuState};
use crate::party::{reset_magic_tier_charges, PartyState};
use crate::rules::Ruleset;
use crate::world::WorldState;
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
    pub shop_states: HashMap<String, ShopState>,
    pub world: WorldState,
    pub last_overworld: Option<LastOverworld>,
    pub active_vehicle: Option<String>,
    pub vehicle_positions: HashMap<String, VehiclePosition>,
    pub vehicle_slow_mode: bool,
    pub settings: SettingsState,
    pub stats: HashMap<String, i32>,
    pub playtime: u64,
    pub start_time: Instant,
    pub last_manual_save_slot: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct VehiclePosition {
    pub map_id: String,
    pub pos: (i32, i32),
}

#[derive(Clone, Debug)]
pub struct LastOverworld {
    pub world_id: String,
    pub map_id: String,
    pub pos: (i32, i32),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ShopState {
    pub currency: i32,
    pub stock: HashMap<String, i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SettingsState {
    pub autosave_enabled: bool,
    pub readiness_speed: f32,
    #[serde(default = "default_difficulty_scale")]
    pub difficulty_scale: f32,
    pub battle_mode: crate::rules::BattleMode,
    #[serde(default = "default_death_markers_visible")]
    pub death_markers_visible: bool,
}

impl SettingsState {
    pub fn from_rules(rules: &crate::rules::RulesFile) -> Self {
        let autosave = rules
            .settings
            .autosave_enabled
            .as_ref()
            .map(|setting| setting.value)
            .unwrap_or(false);
        let readiness = rules
            .settings
            .readiness_speed
            .as_ref()
            .map(|setting| setting.value)
            .unwrap_or(rules.game.readiness_speed);
        let battle_mode = rules
            .settings
            .battle_mode
            .as_ref()
            .map(|setting| setting.value.clone())
            .unwrap_or(rules.game.battle_mode.clone());
        let difficulty_scale = rules
            .settings
            .difficulty_scale
            .as_ref()
            .map(|setting| setting.value)
            .unwrap_or(crate::rules::DIFFICULTY_SCALE_DEFAULT);
        let death_markers_visible = rules
            .settings
            .death_markers_visible
            .as_ref()
            .map(|setting| setting.value)
            .unwrap_or(true);
        Self {
            autosave_enabled: autosave,
            readiness_speed: readiness,
            difficulty_scale,
            battle_mode,
            death_markers_visible,
        }
    }
}

fn default_death_markers_visible() -> bool {
    true
}

fn default_difficulty_scale() -> f32 {
    crate::rules::DIFFICULTY_SCALE_DEFAULT
}

impl GameRuntime {
    pub fn new(content: Content) -> Self {
        let start_location = content.rules.game.start_location.clone();
        let vehicle_positions = initial_vehicle_positions(&content);
        let settings = SettingsState::from_rules(&content.rules);
        let stats = initialize_stats(&content.rules.stats.track);
        let shop_states = initial_shop_states(&content);
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
            shop_states,
            world: WorldState::new(
                &start_location.world,
                &start_location.map,
                (start_location.x, start_location.y),
            ),
            last_overworld: None,
            active_vehicle: None,
            vehicle_positions,
            vehicle_slow_mode: false,
            settings,
            stats,
            playtime: 0,
            start_time: Instant::now(),
            last_manual_save_slot: None,
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

    pub fn stat_value(&self, stat_id: &str) -> i32 {
        if stat_id == "time_played" {
            let current_session = self.start_time.elapsed().as_secs();
            return (self.playtime + current_session) as i32;
        }
        self.stats.get(stat_id).copied().unwrap_or(0)
    }

    pub fn set_stat(&mut self, stat_id: &str, value: i32) {
        if stat_id == "time_played" {
            self.playtime = value.max(0) as u64;
            self.start_time = Instant::now();
            return;
        }
        self.stats.insert(stat_id.to_string(), value);
    }

    pub fn add_stat(&mut self, stat_id: &str, delta: i32) {
        if delta == 0 {
            return;
        }
        let value = self.stat_value(stat_id).saturating_add(delta);
        self.set_stat(stat_id, value);
    }

    pub fn track_max_stat(&mut self, stat_id: &str, value: i32) {
        if value <= 0 {
            return;
        }
        let current = self.stat_value(stat_id);
        if value > current {
            self.set_stat(stat_id, value);
        }
    }

    pub fn stats_for_save(&self) -> HashMap<String, i32> {
        let mut stats = self.stats.clone();
        stats.insert("time_played".to_string(), self.stat_value("time_played"));
        stats
    }

    pub fn ensure_tracked_stats(&mut self) {
        for stat in &self.content.rules.stats.track {
            if stat == "time_played" {
                continue;
            }
            self.stats.entry(stat.clone()).or_insert(0);
        }
    }

    pub fn effective_autosave_enabled(&self) -> bool {
        let setting = self.autosave_setting();
        if !setting.editable {
            setting.value
        } else {
            self.settings.autosave_enabled
        }
    }

    pub fn effective_readiness_speed(&self) -> f32 {
        let setting = self.readiness_setting();
        let mut value = if setting.editable {
            self.settings.readiness_speed
        } else {
            setting.value
        };
        value = value.clamp(setting.min, setting.max);
        if setting.step > 0.0 {
            value = (value / setting.step).round() * setting.step;
        }
        value
    }

    pub fn effective_difficulty_scale(&self) -> f32 {
        let setting = self.difficulty_scale_setting();
        let mut value = if setting.editable {
            self.settings.difficulty_scale
        } else {
            setting.value
        };
        value = value.clamp(setting.min, setting.max);
        if setting.step > 0.0 {
            value = (value / setting.step).round() * setting.step;
        }
        value
    }

    pub fn effective_battle_mode(&self) -> crate::rules::BattleMode {
        let setting = self.battle_mode_setting();
        if !setting.editable || setting.options.len() <= 1 {
            return setting.value;
        }
        if setting.options.is_empty() || setting.options.contains(&self.settings.battle_mode) {
            self.settings.battle_mode.clone()
        } else {
            setting.value
        }
    }

    pub fn effective_death_markers_visible(&self) -> bool {
        if !self.content.rules.render.death_markers.show_on_map {
            return false;
        }
        let setting = self.death_markers_setting();
        if !setting.editable {
            setting.value
        } else {
            self.settings.death_markers_visible
        }
    }

    pub fn autosave_setting(&self) -> crate::rules::ToggleSetting {
        self.content
            .rules
            .settings
            .autosave_enabled
            .clone()
            .unwrap_or(crate::rules::ToggleSetting {
                value: false,
                visible: true,
                editable: true,
            })
    }

    pub fn readiness_setting(&self) -> crate::rules::RangeSetting {
        self.content
            .rules
            .settings
            .readiness_speed
            .clone()
            .unwrap_or(crate::rules::RangeSetting {
                value: self.content.rules.game.readiness_speed,
                min: crate::rules::READINESS_SPEED_MIN,
                max: crate::rules::READINESS_SPEED_MAX,
                step: crate::rules::READINESS_SPEED_STEP,
                visible: true,
                editable: true,
            })
    }

    pub fn difficulty_scale_setting(&self) -> crate::rules::RangeSetting {
        self.content
            .rules
            .settings
            .difficulty_scale
            .clone()
            .unwrap_or(crate::rules::RangeSetting {
                value: crate::rules::DIFFICULTY_SCALE_DEFAULT,
                min: crate::rules::DIFFICULTY_SCALE_MIN,
                max: crate::rules::DIFFICULTY_SCALE_MAX,
                step: crate::rules::DIFFICULTY_SCALE_STEP,
                visible: true,
                editable: true,
            })
    }

    pub fn battle_mode_setting(&self) -> crate::rules::ChoiceSetting<crate::rules::BattleMode> {
        self.content
            .rules
            .settings
            .battle_mode
            .clone()
            .unwrap_or(crate::rules::ChoiceSetting {
                value: self.content.rules.game.battle_mode.clone(),
                options: vec![
                    crate::rules::BattleMode::Turn,
                    crate::rules::BattleMode::Dynamic,
                    crate::rules::BattleMode::DynamicWait,
                ],
                visible: true,
                editable: true,
            })
    }

    pub fn death_markers_setting(&self) -> crate::rules::ToggleSetting {
        self.content
            .rules
            .settings
            .death_markers_visible
            .clone()
            .unwrap_or(crate::rules::ToggleSetting {
                value: true,
                visible: true,
                editable: true,
            })
    }

    pub fn set_flag(&mut self, flag: &str) {
        self.flags.insert(flag.to_string());
    }

    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    pub fn overworld_map_id(&self, world_id: &str) -> Option<&str> {
        self.content
            .worlds
            .worlds
            .iter()
            .find(|world| world.id == world_id)
            .map(|world| world.overworld_map_id.as_str())
    }

    pub fn is_overworld_map(&self, map_id: &str) -> bool {
        self.overworld_map_id(&self.world.world_id)
            .map(|overworld_map_id| overworld_map_id == map_id)
            .unwrap_or(false)
    }

    pub fn record_last_overworld(&mut self, map_id: &str, pos: (i32, i32)) {
        if !self.is_overworld_map(map_id) {
            return;
        }
        self.last_overworld = Some(LastOverworld {
            world_id: self.world.world_id.clone(),
            map_id: map_id.to_string(),
            pos,
        });
    }

    pub fn record_last_overworld_on_exit(&mut self, next_map_id: &str) {
        let is_overworld = self.is_overworld_map(&self.world.map_id);
        let is_next_overworld = self.is_overworld_map(next_map_id);
        if is_overworld && !is_next_overworld {
            let map_id = self.world.map_id.clone();
            let pos = self.world.position;
            self.record_last_overworld(&map_id, pos);
        }
    }

    pub fn warp_to_last_overworld(&mut self) -> bool {
        let Some(last_overworld) = self.last_overworld.clone() else {
            return false;
        };
        self.world.world_id = last_overworld.world_id;
        self.world.map_id = last_overworld.map_id;
        self.world.position = last_overworld.pos;
        self.active_vehicle = None;
        self.vehicle_slow_mode = false;
        true
    }

    pub fn warp_to_map(&mut self, map_id: &str, pos: (i32, i32)) {
        self.record_last_overworld_on_exit(map_id);
        self.world.map_id = map_id.to_string();
        self.world.position = pos;
        self.active_vehicle = None;
        self.vehicle_slow_mode = false;
    }

    pub fn start_new_game(&mut self, rules: &Ruleset) {
        if matches!(
            rules.party_mode,
            crate::rules::PartyMode::Preset | crate::rules::PartyMode::PresetRename
        ) || self.party.roster.is_empty()
        {
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
        self.stats = initialize_stats(&self.content.rules.stats.track);
        self.playtime = 0;
        self.start_time = Instant::now();
        self.world.world_id = rules.start_location.world.clone();
        self.world.map_id = rules.start_location.map.clone();
        self.world.position = (rules.start_location.x, rules.start_location.y);
        if self.is_overworld_map(&self.world.map_id) {
            self.last_overworld = Some(LastOverworld {
                world_id: self.world.world_id.clone(),
                map_id: self.world.map_id.clone(),
                pos: self.world.position,
            });
        } else {
            self.last_overworld = None;
        }
        self.active_vehicle = None;
        self.vehicle_positions = initial_vehicle_positions(&self.content);
        self.vehicle_slow_mode = false;
        if let Some(event_id) = &rules.start_event {
            self.queue_event(event_id);
            self.state = GameState::Event;
            self.start_next_event();
        } else {
            self.state = GameState::Overworld;
        }
        self.last_manual_save_slot = None;
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
        if !self.event_queue.is_empty() {
            self.event_queue.remove(0);
        }
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

fn initialize_stats(stats: &[String]) -> HashMap<String, i32> {
    let mut map = HashMap::new();
    for stat in stats {
        if stat == "time_played" {
            continue;
        }
        map.entry(stat.clone()).or_insert(0);
    }
    map
}

pub fn initial_shop_states(content: &Content) -> HashMap<String, ShopState> {
    let mut states = HashMap::new();
    for shop in &content.shops.shops {
        let mut stock = HashMap::new();
        for entry in &shop.inventory {
            if let Some(count) = entry.stock {
                stock.insert(entry.item.clone(), count.max(0));
            }
        }
        let currency = if shop.currency_pool == "tracked" {
            shop.currency_amount.unwrap_or(0).max(0)
        } else {
            0
        };
        if !stock.is_empty() || shop.currency_pool == "tracked" {
            states.insert(shop.id.clone(), ShopState { currency, stock });
        }
    }
    states
}

fn initial_vehicle_positions(content: &Content) -> HashMap<String, VehiclePosition> {
    let mut positions = HashMap::new();
    for map in &content.maps {
        for vehicle in &map.vehicles {
            positions.insert(
                vehicle.vehicle_id.clone(),
                VehiclePosition {
                    map_id: map.id.clone(),
                    pos: (vehicle.pos[0], vehicle.pos[1]),
                },
            );
        }
    }
    positions
}

#[cfg(test)]
mod tests {
    use super::{initialize_stats, GameRuntime};
    use crate::content::Content;
    use crate::rules::{BattleMode, ChoiceSetting, RangeSetting, ToggleSetting};
    use std::path::PathBuf;

    fn load_runtime() -> GameRuntime {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../content/opencrystal-peak");
        let content = Content::load(dir).expect("content should load for tests");
        GameRuntime::new(content)
    }

    #[test]
    fn initialize_stats_skips_time_played_and_deduplicates() {
        let stats = initialize_stats(&[
            "wins".to_string(),
            "time_played".to_string(),
            "wins".to_string(),
        ]);
        assert_eq!(stats.len(), 1);
        assert_eq!(stats.get("wins"), Some(&0));
        assert!(!stats.contains_key("time_played"));
    }

    #[test]
    #[ignore = "depends on local content bundle"]
    fn effective_readiness_speed_clamps_and_rounds_step() {
        let mut runtime = load_runtime();
        runtime.content.rules.settings.readiness_speed = Some(RangeSetting {
            value: 2.0,
            min: 1.0,
            max: 3.0,
            step: 0.5,
            visible: true,
            editable: true,
        });

        runtime.settings.readiness_speed = 3.4;
        assert_eq!(runtime.effective_readiness_speed(), 3.0);

        runtime.settings.readiness_speed = 2.24;
        assert_eq!(runtime.effective_readiness_speed(), 2.0);
    }

    #[test]
    #[ignore = "depends on local content bundle"]
    fn effective_difficulty_scale_respects_locked_setting() {
        let mut runtime = load_runtime();
        runtime.content.rules.settings.difficulty_scale = Some(RangeSetting {
            value: 1.7,
            min: 0.5,
            max: 2.0,
            step: 0.1,
            visible: true,
            editable: false,
        });
        runtime.settings.difficulty_scale = 0.5;

        assert_eq!(runtime.effective_difficulty_scale(), 1.7);
    }

    #[test]
    #[ignore = "depends on local content bundle"]
    fn effective_battle_mode_falls_back_when_selected_option_invalid() {
        let mut runtime = load_runtime();
        runtime.content.rules.settings.battle_mode = Some(ChoiceSetting {
            value: BattleMode::Turn,
            options: vec![BattleMode::Turn, BattleMode::Dynamic],
            visible: true,
            editable: true,
        });
        runtime.settings.battle_mode = BattleMode::DynamicWait;
        assert_eq!(runtime.effective_battle_mode(), BattleMode::Turn);

        runtime.settings.battle_mode = BattleMode::Dynamic;
        assert_eq!(runtime.effective_battle_mode(), BattleMode::Dynamic);
    }

    #[test]
    #[ignore = "depends on local content bundle"]
    fn effective_death_markers_visible_obeys_render_and_setting() {
        let mut runtime = load_runtime();
        runtime.content.rules.settings.death_markers_visible = Some(ToggleSetting {
            value: false,
            visible: true,
            editable: false,
        });
        runtime.settings.death_markers_visible = true;

        runtime.content.rules.render.death_markers.show_on_map = true;
        assert!(!runtime.effective_death_markers_visible());

        runtime.content.rules.render.death_markers.show_on_map = false;
        assert!(!runtime.effective_death_markers_visible());
    }

    #[test]
    #[ignore = "depends on local content bundle"]
    fn record_and_warp_last_overworld_state() {
        let mut runtime = load_runtime();
        let overworld_map = runtime
            .overworld_map_id(&runtime.world.world_id)
            .expect("world should have overworld map")
            .to_string();
        let non_overworld_map = runtime
            .content
            .maps
            .iter()
            .find(|map| map.id != overworld_map)
            .expect("need non-overworld map")
            .id
            .clone();

        runtime.world.map_id = overworld_map.clone();
        runtime.world.position = (3, 4);
        runtime.record_last_overworld_on_exit(&non_overworld_map);

        runtime.world.map_id = non_overworld_map;
        runtime.world.position = (99, 99);
        assert!(runtime.warp_to_last_overworld());
        assert_eq!(runtime.world.map_id, overworld_map);
        assert_eq!(runtime.world.position, (3, 4));
    }
}
