mod input;
mod objects;
mod prompts;
mod render;
mod resize;
mod state;

use std::io;

use crossterm::event::{self, Event};

use crate::input::{is_actionable_key, InputBindings};
use crate::session::TuiSession;

use input::{handle_key, EditorAction};
use render::draw_editor_frame;
use state::EditorState;

#[derive(Clone, Debug)]
pub struct MapData {
    pub version: u32,
    pub id: String,
    pub name: String,
    pub hide_name: bool,
    pub world: String,
    pub width: u32,
    pub height: u32,
    pub loop_x: bool,
    pub loop_y: bool,
    pub tiles: Vec<Vec<char>>,
    pub legend: Vec<LegendEntry>,
    pub encounters: Vec<EncounterZone>,
    pub encounter_rate: f32,
    pub events: Vec<MapEvent>,
    pub npcs: Vec<MapNpc>,
    pub signs: Vec<MapSign>,
    pub chests: Vec<MapChest>,
    pub doors: Vec<MapDoor>,
    pub puzzles: Vec<MapPuzzle>,
    pub campfires: Vec<MapCampfire>,
    pub allow_save: bool,
    pub save_points: Vec<[i32; 2]>,
    pub transitions: Vec<MapTransition>,
    pub vehicles: Vec<MapVehicle>,
}

#[derive(Clone, Debug)]
pub struct LegendEntry {
    pub glyph: char,
    pub tile: String,
    pub passable: bool,
    pub palette: Option<String>,
}

#[derive(Clone, Debug)]
pub struct EncounterZone {
    pub zone_id: String,
    pub rect: [i32; 4],
    pub table: String,
}

#[derive(Clone, Debug)]
pub struct MapEvent {
    pub id: String,
    pub trigger: String,
    pub script: String,
    pub zone: Option<String>,
    pub pos: Option<[i32; 2]>,
}

#[derive(Clone, Debug)]
pub struct MapNpc {
    pub id: String,
    pub pos: [i32; 2],
    pub script: Option<String>,
    pub requires_flags: Option<Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct MapSign {
    pub id: String,
    pub pos: [i32; 2],
    pub glyph: Option<String>,
    pub palette: Option<String>,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct MapChest {
    pub id: String,
    pub pos: [i32; 2],
    pub glyph_closed: Option<String>,
    pub glyph_open: Option<String>,
    pub palette: Option<String>,
    pub opened_flag: String,
    pub loot: MapChestLoot,
}

#[derive(Clone, Debug)]
pub struct MapChestLoot {
    pub items: Vec<InventoryStack>,
    pub equipment: Vec<InventoryStack>,
    pub currency: Vec<MapCurrencyStack>,
}

#[derive(Clone, Debug)]
pub struct InventoryStack {
    pub id: String,
    pub qty: i32,
}

#[derive(Clone, Debug)]
pub struct MapCurrencyStack {
    pub id: String,
    pub amount: i32,
}

#[derive(Clone, Debug)]
pub struct MapTransition {
    pub id: String,
    pub pos: [i32; 2],
    pub target_map: String,
    pub target_pos: [i32; 2],
    pub label: Option<String>,
    pub requires_flag: Option<String>,
    pub cost: Option<MapCurrencyStack>,
    pub return_to_last: bool,
    pub glyph: Option<String>,
    pub palette: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MapVehicle {
    pub vehicle_id: String,
    pub pos: [i32; 2],
    pub requires_flags: Option<Vec<String>>,
}

#[derive(Clone, Debug)]
pub struct MapDoor {
    pub id: String,
    pub pos: [i32; 2],
    pub requires_flag: Option<String>,
    pub locked_text: Option<String>,
    pub locked_event: Option<String>,
    pub target_map: Option<String>,
    pub target_pos: Option<[i32; 2]>,
    pub return_to_last: bool,
    pub glyph: Option<String>,
    pub palette: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MapPuzzle {
    pub id: String,
    pub pos: [i32; 2],
    pub requires_flags: Option<Vec<String>>,
    pub text: Option<String>,
    pub event: Option<String>,
    pub set_flag: Option<String>,
    pub glyph: Option<String>,
    pub palette: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MapCampfire {
    pub id: String,
    pub pos: [i32; 2],
    pub campfire_id: String,
    pub requires_flags: Option<Vec<String>>,
    pub glyph: Option<String>,
    pub palette: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MapEditorConfig {
    pub map: MapData,
    pub map_ids: Vec<String>,
    pub event_ids: Vec<String>,
    pub vehicle_ids: Vec<String>,
    pub npc_ids: Vec<String>,
    pub item_ids: Vec<String>,
    pub equipment_ids: Vec<String>,
    pub currency_ids: Vec<String>,
    pub campfire_ids: Vec<String>,
    pub encounter_zone_ids: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum MapEditorOutcome {
    Saved(MapData),
    Cancelled,
}

pub fn run_map_editor(
    session: &mut TuiSession,
    config: MapEditorConfig,
) -> io::Result<MapEditorOutcome> {
    let mut state = EditorState::new(config.map);
    let bindings = InputBindings::default_bindings();
    let mut map_ids = config.map_ids;
    let mut event_ids = config.event_ids;
    let mut vehicle_ids = config.vehicle_ids;
    let mut npc_ids = config.npc_ids;
    let mut item_ids = config.item_ids;
    let mut equipment_ids = config.equipment_ids;
    let mut currency_ids = config.currency_ids;
    let mut campfire_ids = config.campfire_ids;
    let mut encounter_zone_ids = config.encounter_zone_ids;
    map_ids.sort();
    event_ids.sort();
    vehicle_ids.sort();
    npc_ids.sort();
    item_ids.sort();
    equipment_ids.sort();
    currency_ids.sort();
    campfire_ids.sort();
    encounter_zone_ids.sort();

    loop {
        session.terminal_mut().draw(|frame| {
            draw_editor_frame(frame, &state, &map_ids, &event_ids, &vehicle_ids, &npc_ids);
        })?;

        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            match handle_key(
                session,
                &bindings,
                &mut state,
                &map_ids,
                &event_ids,
                &vehicle_ids,
                &npc_ids,
                &item_ids,
                &equipment_ids,
                &currency_ids,
                &campfire_ids,
                &encounter_zone_ids,
                key,
            )? {
                EditorAction::Continue => {}
                EditorAction::Exit(outcome) => return Ok(outcome),
            }
        }
    }
}
