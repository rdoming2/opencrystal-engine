use std::collections::HashMap;
use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::dialog::{prompt_choice, prompt_text};
use crate::input::{is_actionable_key, InputBindings};
use crate::session::TuiSession;
use crate::utils::{palette_style, truncate_line};

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
}

#[derive(Clone, Debug)]
pub enum MapEditorOutcome {
    Saved(MapData),
    Cancelled,
}

#[derive(Clone, Debug)]
struct YankBuffer {
    tiles: Vec<Vec<char>>,
    width: i32,
    height: i32,
}

#[derive(Clone, Debug)]
struct Selection {
    anchor: (i32, i32),
}

#[derive(Clone, Debug)]
struct EditorState {
    map: MapData,
    cursor: (i32, i32),
    selection: Option<Selection>,
    yank: Option<YankBuffer>,
    active_tile_index: usize,
    moving: Option<MovingObject>,
    undo_stack: Vec<MapData>,
    redo_stack: Vec<MapData>,
    dirty: bool,
    status: String,
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
    map_ids.sort();
    event_ids.sort();
    vehicle_ids.sort();
    npc_ids.sort();

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
                key,
            )? {
                EditorAction::Continue => {}
                EditorAction::Exit(outcome) => return Ok(outcome),
            }
        }
    }
}

impl EditorState {
    fn new(mut map: MapData) -> Self {
        normalize_tiles(&mut map, None);
        let active_tile_index = if map.legend.is_empty() { 0 } else { 0 };
        let active_tile_index = active_tile_index.min(map.legend.len().saturating_sub(1));
        let cursor = (0, 0);
        Self {
            map,
            cursor,
            selection: None,
            yank: None,
            active_tile_index,
            moving: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
            status: "Ready".to_string(),
        }
    }

    fn active_glyph(&self) -> char {
        self.map
            .legend
            .get(self.active_tile_index)
            .map(|entry| entry.glyph)
            .unwrap_or('.')
    }
}

#[derive(Clone, Debug)]
enum ExitAction {
    Save,
    Discard,
    Cancel,
}

enum EditorAction {
    Continue,
    Exit(MapEditorOutcome),
}

#[derive(Clone, Debug)]
struct MovingObject {
    target: CursorObject,
    pos: [i32; 2],
    undo_pushed: bool,
}

fn confirm_exit(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &EditorState,
) -> io::Result<ExitAction> {
    if !state.dirty {
        return Ok(ExitAction::Discard);
    }
    let options = vec![
        "Save and quit".to_string(),
        "Quit without saving".to_string(),
        "Cancel".to_string(),
    ];
    let selection = prompt_choice(
        session,
        bindings,
        "Unsaved Changes",
        "Select an action:",
        &options,
        0,
    )?;
    Ok(match selection {
        Some(0) => ExitAction::Save,
        Some(1) => ExitAction::Discard,
        _ => ExitAction::Cancel,
    })
}

fn handle_key(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    map_ids: &[String],
    event_ids: &[String],
    vehicle_ids: &[String],
    npc_ids: &[String],
    key: KeyEvent,
) -> io::Result<EditorAction> {
    match key.code {
        KeyCode::Char('u') => {
            undo(state);
            return Ok(EditorAction::Continue);
        }
        KeyCode::Char('U') => {
            redo(state);
            return Ok(EditorAction::Continue);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            move_cursor(state, 0, -1);
            return Ok(EditorAction::Continue);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            move_cursor(state, 0, 1);
            return Ok(EditorAction::Continue);
        }
        KeyCode::Left | KeyCode::Char('h') => {
            move_cursor(state, -1, 0);
            return Ok(EditorAction::Continue);
        }
        KeyCode::Right | KeyCode::Char('l') => {
            move_cursor(state, 1, 0);
            return Ok(EditorAction::Continue);
        }
        KeyCode::Char('v') | KeyCode::Char('V') => {
            toggle_visual(state);
            return Ok(EditorAction::Continue);
        }
        KeyCode::Char('y') => {
            yank_selection(state);
            return Ok(EditorAction::Continue);
        }
        KeyCode::Char('p') => {
            paste_selection(state);
            return Ok(EditorAction::Continue);
        }
        KeyCode::Char('R') | KeyCode::Char('r') => {
            paint_active_tile(state);
            return Ok(EditorAction::Continue);
        }
        KeyCode::Char('[') => {
            cycle_active_tile(state, -1);
            return Ok(EditorAction::Continue);
        }
        KeyCode::Char(']') => {
            cycle_active_tile(state, 1);
            return Ok(EditorAction::Continue);
        }
        KeyCode::Char('t') => {
            choose_active_tile(session, bindings, state)?;
            return Ok(EditorAction::Continue);
        }
        KeyCode::Char('L') => {
            edit_legend(session, bindings, state)?;
            return Ok(EditorAction::Continue);
        }
        KeyCode::Char('o') => {
            edit_objects(
                session,
                bindings,
                state,
                map_ids,
                event_ids,
                vehicle_ids,
                npc_ids,
            )?;
            return Ok(EditorAction::Continue);
        }
        KeyCode::Char('m') => {
            toggle_move_object(session, bindings, state)?;
            return Ok(EditorAction::Continue);
        }
        KeyCode::Char('e') => {
            edit_object_at_cursor(session, bindings, state, map_ids, event_ids, vehicle_ids)?;
            return Ok(EditorAction::Continue);
        }
        KeyCode::Char('x') => {
            delete_object_at_cursor(state);
            return Ok(EditorAction::Continue);
        }
        KeyCode::Char('s') => {
            state.dirty = false;
            return Ok(EditorAction::Exit(MapEditorOutcome::Saved(
                state.map.clone(),
            )));
        }
        KeyCode::Char('=') => {
            resize_map(session, bindings, state)?;
            return Ok(EditorAction::Continue);
        }
        KeyCode::Char('q') => {
            let action = confirm_exit(session, bindings, state)?;
            return Ok(match action {
                ExitAction::Cancel => EditorAction::Continue,
                ExitAction::Discard => EditorAction::Exit(MapEditorOutcome::Cancelled),
                ExitAction::Save => {
                    state.dirty = false;
                    EditorAction::Exit(MapEditorOutcome::Saved(state.map.clone()))
                }
            });
        }
        _ => {}
    }
    Ok(EditorAction::Continue)
}

fn move_cursor(state: &mut EditorState, dx: i32, dy: i32) {
    let width = state.map.width as i32;
    let height = state.map.height as i32;
    if width <= 0 || height <= 0 {
        return;
    }
    let next_x = (state.cursor.0 + dx).clamp(0, width - 1);
    let next_y = (state.cursor.1 + dy).clamp(0, height - 1);
    state.cursor = (next_x, next_y);
    if let Some(mut moving) = state.moving.take() {
        let new_pos = [next_x, next_y];
        if moving.pos != new_pos {
            if !moving.undo_pushed {
                push_undo(state);
                moving.undo_pushed = true;
            }
            apply_moving_position(state, &mut moving, new_pos);
            state.dirty = true;
        }
        state.moving = Some(moving);
    }
}

fn push_undo(state: &mut EditorState) {
    if state.undo_stack.len() >= 50 {
        state.undo_stack.remove(0);
    }
    state.undo_stack.push(state.map.clone());
    state.redo_stack.clear();
}

fn undo(state: &mut EditorState) {
    if let Some(previous) = state.undo_stack.pop() {
        state.redo_stack.push(state.map.clone());
        state.map = previous;
        state.status = "Undo".to_string();
        state.dirty = true;
    } else {
        state.status = "Nothing to undo".to_string();
    }
}

fn redo(state: &mut EditorState) {
    if let Some(next) = state.redo_stack.pop() {
        state.undo_stack.push(state.map.clone());
        state.map = next;
        state.status = "Redo".to_string();
        state.dirty = true;
    } else {
        state.status = "Nothing to redo".to_string();
    }
}

fn toggle_visual(state: &mut EditorState) {
    if state.selection.is_some() {
        state.selection = None;
        state.status = "Visual mode off".to_string();
    } else {
        state.selection = Some(Selection {
            anchor: state.cursor,
        });
        state.status = "Visual mode".to_string();
    }
}

fn selection_rect(state: &EditorState) -> Option<(i32, i32, i32, i32)> {
    let selection = state.selection.as_ref()?;
    let (x0, y0) = selection.anchor;
    let (x1, y1) = state.cursor;
    let min_x = x0.min(x1);
    let max_x = x0.max(x1);
    let min_y = y0.min(y1);
    let max_y = y0.max(y1);
    Some((min_x, min_y, max_x, max_y))
}

fn yank_selection(state: &mut EditorState) {
    let (min_x, min_y, max_x, max_y) = selection_rect(state).unwrap_or((
        state.cursor.0,
        state.cursor.1,
        state.cursor.0,
        state.cursor.1,
    ));
    let width = max_x - min_x + 1;
    let height = max_y - min_y + 1;
    let mut tiles = Vec::new();
    for y in 0..height {
        let mut row = Vec::new();
        for x in 0..width {
            let tile = tile_at(&state.map, min_x + x, min_y + y);
            row.push(tile);
        }
        tiles.push(row);
    }
    state.yank = Some(YankBuffer {
        tiles,
        width,
        height,
    });
    state.status = format!("Yanked {}x{}", width, height);
}

fn paste_selection(state: &mut EditorState) {
    let Some(buffer) = state.yank.clone() else {
        state.status = "Nothing to paste".to_string();
        return;
    };
    push_undo(state);
    let width = state.map.width as i32;
    let height = state.map.height as i32;
    for y in 0..buffer.height {
        for x in 0..buffer.width {
            let target_x = state.cursor.0 + x;
            let target_y = state.cursor.1 + y;
            if target_x < 0 || target_y < 0 || target_x >= width || target_y >= height {
                continue;
            }
            set_tile(
                &mut state.map,
                target_x,
                target_y,
                buffer.tiles[y as usize][x as usize],
            );
        }
    }
    state.dirty = true;
    state.status = format!("Pasted {}x{}", buffer.width, buffer.height);
}

fn paint_active_tile(state: &mut EditorState) {
    let glyph = state.active_glyph();
    push_undo(state);
    if let Some((min_x, min_y, max_x, max_y)) = selection_rect(state) {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                set_tile(&mut state.map, x, y, glyph);
            }
        }
        state.status = format!("Painted {}x{}", max_x - min_x + 1, max_y - min_y + 1);
    } else {
        set_tile(&mut state.map, state.cursor.0, state.cursor.1, glyph);
        state.status = "Painted tile".to_string();
    }
    state.dirty = true;
}

fn cycle_active_tile(state: &mut EditorState, delta: i32) {
    if state.map.legend.is_empty() {
        state.status = "No legend entries".to_string();
        return;
    }
    let len = state.map.legend.len() as i32;
    let next = (state.active_tile_index as i32 + delta).rem_euclid(len) as usize;
    state.active_tile_index = next;
    let glyph = state.active_glyph();
    state.status = format!("Active tile: {}", glyph);
}

fn choose_active_tile(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
) -> io::Result<()> {
    if state.map.legend.is_empty() {
        state.status = "No legend entries".to_string();
        return Ok(());
    }
    let options = state
        .map
        .legend
        .iter()
        .map(|entry| format!("{}  {}", entry.glyph, entry.tile))
        .collect::<Vec<_>>();
    if let Some(choice) = prompt_choice(
        session,
        bindings,
        "Tile Legend",
        "Select active tile:",
        &options,
        state.active_tile_index,
    )? {
        state.active_tile_index = choice;
        state.status = format!("Active tile: {}", state.active_glyph());
    }
    Ok(())
}

fn edit_legend(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
) -> io::Result<()> {
    let options = vec![
        "Add tile".to_string(),
        "Edit tile".to_string(),
        "Remove tile".to_string(),
    ];
    let Some(choice) = prompt_choice(session, bindings, "Legend", "Select action:", &options, 0)?
    else {
        return Ok(());
    };
    match choice {
        0 => add_legend_entry(session, state)?,
        1 => edit_legend_entry(session, bindings, state)?,
        2 => remove_legend_entry(session, bindings, state)?,
        _ => {}
    }
    Ok(())
}

fn add_legend_entry(session: &mut TuiSession, state: &mut EditorState) -> io::Result<()> {
    let glyph = prompt_glyph(session, "Legend", "Glyph:", ".")?;
    let Some(glyph) = glyph else {
        return Ok(());
    };
    let tile = prompt_text(session, "Legend", "Tile id:", "floor", 32)?;
    let Some(tile) = tile else {
        return Ok(());
    };
    let passable = prompt_yes_no(session, "Legend", "Passable?", true)?;
    let palette = prompt_text(session, "Legend", "Palette (optional):", "", 24)?;
    let palette = palette.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    push_undo(state);
    state.map.legend.push(LegendEntry {
        glyph,
        tile,
        passable,
        palette,
    });
    state.active_tile_index = state.map.legend.len().saturating_sub(1);
    state.dirty = true;
    state.status = "Legend entry added".to_string();
    Ok(())
}

fn edit_legend_entry(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
) -> io::Result<()> {
    if state.map.legend.is_empty() {
        state.status = "No legend entries".to_string();
        return Ok(());
    }
    let options = state
        .map
        .legend
        .iter()
        .map(|entry| format!("{}  {}", entry.glyph, entry.tile))
        .collect::<Vec<_>>();
    let Some(choice) = prompt_choice(
        session,
        bindings,
        "Legend",
        "Select tile:",
        &options,
        state.active_tile_index,
    )?
    else {
        return Ok(());
    };
    let entry = state.map.legend[choice].clone();
    let glyph_default = entry.glyph.to_string();
    let glyph = prompt_glyph(session, "Legend", "Glyph:", &glyph_default)?;
    let Some(glyph) = glyph else {
        return Ok(());
    };
    let tile = prompt_text(session, "Legend", "Tile id:", &entry.tile, 32)?;
    let Some(tile) = tile else {
        return Ok(());
    };
    let passable = prompt_yes_no(session, "Legend", "Passable?", entry.passable)?;
    let palette_default = entry.palette.clone().unwrap_or_default();
    let palette = prompt_text(
        session,
        "Legend",
        "Palette (optional):",
        &palette_default,
        24,
    )?;
    let palette = palette.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    push_undo(state);
    push_undo(state);
    if glyph != entry.glyph {
        replace_tile_glyph(&mut state.map, entry.glyph, glyph);
    }
    state.map.legend[choice] = LegendEntry {
        glyph,
        tile,
        passable,
        palette,
    };
    state.active_tile_index = choice;
    state.dirty = true;
    state.status = "Legend entry updated".to_string();
    Ok(())
}

fn remove_legend_entry(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
) -> io::Result<()> {
    if state.map.legend.is_empty() {
        state.status = "No legend entries".to_string();
        return Ok(());
    }
    let options = state
        .map
        .legend
        .iter()
        .map(|entry| format!("{}  {}", entry.glyph, entry.tile))
        .collect::<Vec<_>>();
    let Some(choice) = prompt_choice(
        session,
        bindings,
        "Legend",
        "Remove tile:",
        &options,
        state.active_tile_index,
    )?
    else {
        return Ok(());
    };
    push_undo(state);
    let removed = state.map.legend.remove(choice);
    if state.active_tile_index >= state.map.legend.len() {
        state.active_tile_index = state.map.legend.len().saturating_sub(1);
    }
    state.status = format!("Removed legend {}", removed.glyph);
    state.dirty = true;
    Ok(())
}

fn edit_objects(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    map_ids: &[String],
    event_ids: &[String],
    vehicle_ids: &[String],
    _npc_ids: &[String],
) -> io::Result<()> {
    let options = vec![
        "Add transition".to_string(),
        "Add door".to_string(),
        "Add puzzle".to_string(),
        "Add sign".to_string(),
        "Add chest".to_string(),
        "Add vehicle".to_string(),
        "Add campfire".to_string(),
        "Add save point".to_string(),
        "Add event".to_string(),
    ];
    let Some(choice) = prompt_choice(
        session,
        bindings,
        "Map Objects",
        "Select action:",
        &options,
        0,
    )?
    else {
        return Ok(());
    };
    match choice {
        0 => add_transition(session, bindings, state, map_ids)?,
        1 => add_door(session, bindings, state, map_ids)?,
        2 => add_puzzle(session, state)?,
        3 => add_sign(session, state)?,
        4 => add_chest(session, state)?,
        5 => add_vehicle(session, bindings, state, vehicle_ids)?,
        6 => add_campfire(session, state)?,
        7 => add_save_point(state),
        8 => add_event(session, bindings, state, event_ids)?,
        _ => {}
    }
    Ok(())
}

fn edit_object_at_cursor(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    map_ids: &[String],
    event_ids: &[String],
    vehicle_ids: &[String],
) -> io::Result<()> {
    let pos = [state.cursor.0, state.cursor.1];
    let (choices, refs) = cursor_objects(state, pos);
    if choices.is_empty() {
        state.status = "No objects at cursor".to_string();
        return Ok(());
    }

    let Some(choice) = prompt_choice(
        session,
        bindings,
        "Edit Object",
        "Select object:",
        &choices,
        0,
    )?
    else {
        return Ok(());
    };

    match refs[choice] {
        CursorObject::Transition(index) => {
            edit_transition(session, bindings, state, map_ids, index)?;
        }
        CursorObject::Door(index) => {
            edit_door(session, bindings, state, map_ids, index)?;
        }
        CursorObject::Puzzle(index) => {
            edit_puzzle(session, state, index)?;
        }
        CursorObject::Sign(index) => {
            edit_sign(session, state, index)?;
        }
        CursorObject::Chest(index) => {
            edit_chest(session, state, index)?;
        }
        CursorObject::Vehicle(index) => {
            edit_vehicle(session, bindings, state, vehicle_ids, index)?;
        }
        CursorObject::Campfire(index) => {
            edit_campfire(session, state, index)?;
        }
        CursorObject::Event(index) => {
            edit_event(session, bindings, state, event_ids, index)?;
        }
        CursorObject::Npc(index) => {
            edit_npc(session, state, index)?;
        }
        CursorObject::SavePoint => {
            push_undo(state);
            state.map.save_points.retain(|entry| *entry != pos);
            state.dirty = true;
            state.status = "Save point removed".to_string();
        }
    }
    Ok(())
}

fn toggle_move_object(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
) -> io::Result<()> {
    if let Some(moving) = state.moving.take() {
        state.status = format!("Placed {}", moving_label(&moving.target));
        return Ok(());
    }

    let pos = [state.cursor.0, state.cursor.1];
    let (choices, refs) = cursor_objects(state, pos);
    if choices.is_empty() {
        state.status = "No objects at cursor".to_string();
        return Ok(());
    }
    let selection = if choices.len() == 1 {
        Some(0)
    } else {
        prompt_choice(
            session,
            bindings,
            "Move Object",
            "Select object:",
            &choices,
            0,
        )?
    };
    let Some(choice) = selection else {
        return Ok(());
    };
    state.moving = Some(MovingObject {
        target: refs[choice],
        pos,
        undo_pushed: false,
    });
    state.status = format!("Moving {} (press m to place)", moving_label(&refs[choice]));
    Ok(())
}

fn apply_moving_position(state: &mut EditorState, moving: &mut MovingObject, new_pos: [i32; 2]) {
    match moving.target {
        CursorObject::Transition(index) => {
            if let Some(item) = state.map.transitions.get_mut(index) {
                item.pos = new_pos;
            }
        }
        CursorObject::Door(index) => {
            if let Some(item) = state.map.doors.get_mut(index) {
                item.pos = new_pos;
            }
        }
        CursorObject::Puzzle(index) => {
            if let Some(item) = state.map.puzzles.get_mut(index) {
                item.pos = new_pos;
            }
        }
        CursorObject::Sign(index) => {
            if let Some(item) = state.map.signs.get_mut(index) {
                item.pos = new_pos;
            }
        }
        CursorObject::Chest(index) => {
            if let Some(item) = state.map.chests.get_mut(index) {
                item.pos = new_pos;
            }
        }
        CursorObject::Vehicle(index) => {
            if let Some(item) = state.map.vehicles.get_mut(index) {
                item.pos = new_pos;
            }
        }
        CursorObject::Campfire(index) => {
            if let Some(item) = state.map.campfires.get_mut(index) {
                item.pos = new_pos;
            }
        }
        CursorObject::Event(index) => {
            if let Some(item) = state.map.events.get_mut(index) {
                item.pos = Some(new_pos);
            }
        }
        CursorObject::Npc(index) => {
            if let Some(item) = state.map.npcs.get_mut(index) {
                item.pos = new_pos;
            }
        }
        CursorObject::SavePoint => {
            let old_pos = moving.pos;
            if let Some(index) = state
                .map
                .save_points
                .iter()
                .position(|entry| *entry == old_pos)
            {
                state.map.save_points.remove(index);
            }
            if !state.map.save_points.iter().any(|entry| *entry == new_pos) {
                state.map.save_points.push(new_pos);
            }
        }
    }
    moving.pos = new_pos;
}

fn cursor_objects(state: &EditorState, pos: [i32; 2]) -> (Vec<String>, Vec<CursorObject>) {
    let mut choices = Vec::new();
    let mut refs = Vec::new();
    for (index, item) in state.map.transitions.iter().enumerate() {
        if item.pos == pos {
            choices.push(format!("transition:{}", item.id));
            refs.push(CursorObject::Transition(index));
        }
    }
    for (index, item) in state.map.doors.iter().enumerate() {
        if item.pos == pos {
            choices.push(format!("door:{}", item.id));
            refs.push(CursorObject::Door(index));
        }
    }
    for (index, item) in state.map.puzzles.iter().enumerate() {
        if item.pos == pos {
            choices.push(format!("puzzle:{}", item.id));
            refs.push(CursorObject::Puzzle(index));
        }
    }
    for (index, item) in state.map.signs.iter().enumerate() {
        if item.pos == pos {
            choices.push(format!("sign:{}", item.id));
            refs.push(CursorObject::Sign(index));
        }
    }
    for (index, item) in state.map.chests.iter().enumerate() {
        if item.pos == pos {
            choices.push(format!("chest:{}", item.id));
            refs.push(CursorObject::Chest(index));
        }
    }
    for (index, item) in state.map.vehicles.iter().enumerate() {
        if item.pos == pos {
            choices.push(format!("vehicle:{}", item.vehicle_id));
            refs.push(CursorObject::Vehicle(index));
        }
    }
    for (index, item) in state.map.campfires.iter().enumerate() {
        if item.pos == pos {
            choices.push(format!("campfire:{}", item.id));
            refs.push(CursorObject::Campfire(index));
        }
    }
    for (index, item) in state.map.events.iter().enumerate() {
        if item.pos == Some(pos) {
            choices.push(format!("event:{}", item.id));
            refs.push(CursorObject::Event(index));
        }
    }
    for (index, item) in state.map.npcs.iter().enumerate() {
        if item.pos == pos {
            choices.push(format!("npc:{}", item.id));
            refs.push(CursorObject::Npc(index));
        }
    }
    if state.map.save_points.iter().any(|entry| *entry == pos) {
        choices.push("save_point".to_string());
        refs.push(CursorObject::SavePoint);
    }
    (choices, refs)
}

#[derive(Clone, Copy, Debug)]
enum CursorObject {
    Transition(usize),
    Door(usize),
    Puzzle(usize),
    Sign(usize),
    Chest(usize),
    Vehicle(usize),
    Campfire(usize),
    Event(usize),
    Npc(usize),
    SavePoint,
}

fn moving_label(target: &CursorObject) -> String {
    match target {
        CursorObject::Transition(_) => "transition".to_string(),
        CursorObject::Door(_) => "door".to_string(),
        CursorObject::Puzzle(_) => "puzzle".to_string(),
        CursorObject::Sign(_) => "sign".to_string(),
        CursorObject::Chest(_) => "chest".to_string(),
        CursorObject::Vehicle(_) => "vehicle".to_string(),
        CursorObject::Campfire(_) => "campfire".to_string(),
        CursorObject::Event(_) => "event".to_string(),
        CursorObject::Npc(_) => "npc".to_string(),
        CursorObject::SavePoint => "save_point".to_string(),
    }
}

fn add_transition(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    map_ids: &[String],
) -> io::Result<()> {
    let id = prompt_text(session, "Transition", "Id:", "to_location", 32)?;
    let Some(id) = id else {
        return Ok(());
    };
    let target_map =
        choose_from_list_or_custom(session, bindings, "Transition", "Target map:", map_ids, "")?;
    let Some(target_map) = target_map else {
        return Ok(());
    };
    let target_pos = prompt_pos(session, "Transition", "Target pos (x,y):", "0,0")?;
    let Some(target_pos) = target_pos else {
        return Ok(());
    };
    let label =
        prompt_text(session, "Transition", "Label (optional):", "", 32)?.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
    let requires_flag = prompt_text(session, "Transition", "Requires flag (optional):", "", 48)?
        .and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
    let return_to_last = prompt_yes_no(session, "Transition", "Return to last?", false)?;
    push_undo(state);
    state.map.transitions.push(MapTransition {
        id,
        pos: [state.cursor.0, state.cursor.1],
        target_map,
        target_pos,
        label,
        requires_flag,
        cost: None,
        return_to_last,
        glyph: None,
        palette: None,
    });
    state.dirty = true;
    state.status = "Transition added".to_string();
    Ok(())
}

fn add_door(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    map_ids: &[String],
) -> io::Result<()> {
    let id = prompt_text(session, "Door", "Id:", "door", 32)?;
    let Some(id) = id else {
        return Ok(());
    };
    let target_map = choose_optional_from_list_or_custom(
        session,
        bindings,
        "Door",
        "Target map (optional):",
        map_ids,
        "",
    )?;
    let target_pos = if target_map.is_some() {
        prompt_pos(session, "Door", "Target pos (x,y):", "0,0")?
    } else {
        None
    };
    let requires_flag = prompt_text(session, "Door", "Requires flag (optional):", "", 48)?
        .and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
    let locked_text =
        prompt_text(session, "Door", "Locked text (optional):", "", 64)?.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
    let locked_event =
        prompt_text(session, "Door", "Locked event (optional):", "", 32)?.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
    let return_to_last = prompt_yes_no(session, "Door", "Return to last?", false)?;
    push_undo(state);
    state.map.doors.push(MapDoor {
        id,
        pos: [state.cursor.0, state.cursor.1],
        requires_flag,
        locked_text,
        locked_event,
        target_map,
        target_pos,
        return_to_last,
        glyph: None,
        palette: None,
    });
    state.dirty = true;
    state.status = "Door added".to_string();
    Ok(())
}

fn add_puzzle(session: &mut TuiSession, state: &mut EditorState) -> io::Result<()> {
    let id = prompt_text(session, "Puzzle", "Id:", "puzzle", 32)?;
    let Some(id) = id else {
        return Ok(());
    };
    let requires_flags = prompt_flags(session, "Puzzle", "Requires flags (comma):", "")?;
    let text = prompt_text(session, "Puzzle", "Text (optional):", "", 72)?.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    let event = prompt_text(session, "Puzzle", "Event (optional):", "", 32)?.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });
    let set_flag =
        prompt_text(session, "Puzzle", "Set flag (optional):", "", 48)?.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
    push_undo(state);
    state.map.puzzles.push(MapPuzzle {
        id,
        pos: [state.cursor.0, state.cursor.1],
        requires_flags,
        text,
        event,
        set_flag,
        glyph: None,
        palette: None,
    });
    state.dirty = true;
    state.status = "Puzzle added".to_string();
    Ok(())
}

fn add_sign(session: &mut TuiSession, state: &mut EditorState) -> io::Result<()> {
    let id = prompt_text(session, "Sign", "Id:", "sign", 32)?;
    let Some(id) = id else {
        return Ok(());
    };
    let text = prompt_text(session, "Sign", "Text:", "", 72)?;
    let Some(text) = text else {
        return Ok(());
    };
    push_undo(state);
    state.map.signs.push(MapSign {
        id,
        pos: [state.cursor.0, state.cursor.1],
        glyph: None,
        palette: None,
        text,
    });
    state.dirty = true;
    state.status = "Sign added".to_string();
    Ok(())
}

fn add_chest(session: &mut TuiSession, state: &mut EditorState) -> io::Result<()> {
    let id = prompt_text(session, "Chest", "Id:", "chest", 32)?;
    let Some(id) = id else {
        return Ok(());
    };
    let opened_flag = prompt_text(session, "Chest", "Opened flag:", "chest.opened", 64)?;
    let Some(opened_flag) = opened_flag else {
        return Ok(());
    };
    push_undo(state);
    state.map.chests.push(MapChest {
        id,
        pos: [state.cursor.0, state.cursor.1],
        glyph_closed: None,
        glyph_open: None,
        palette: None,
        opened_flag,
        loot: MapChestLoot {
            items: Vec::new(),
            equipment: Vec::new(),
            currency: Vec::new(),
        },
    });
    state.dirty = true;
    state.status = "Chest added".to_string();
    Ok(())
}

fn add_vehicle(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    vehicle_ids: &[String],
) -> io::Result<()> {
    let vehicle_id = choose_from_list_or_custom(
        session,
        bindings,
        "Vehicle",
        "Vehicle id:",
        vehicle_ids,
        "ship",
    )?;
    let Some(vehicle_id) = vehicle_id else {
        return Ok(());
    };
    let requires_flags = prompt_flags(session, "Vehicle", "Requires flags (comma):", "")?;
    push_undo(state);
    state.map.vehicles.push(MapVehicle {
        vehicle_id,
        pos: [state.cursor.0, state.cursor.1],
        requires_flags,
    });
    state.dirty = true;
    state.status = "Vehicle added".to_string();
    Ok(())
}

fn add_campfire(session: &mut TuiSession, state: &mut EditorState) -> io::Result<()> {
    let id = prompt_text(session, "Campfire", "Id:", "campfire", 32)?;
    let Some(id) = id else {
        return Ok(());
    };
    let campfire_id = prompt_text(session, "Campfire", "Campfire set id:", "campfire", 32)?;
    let Some(campfire_id) = campfire_id else {
        return Ok(());
    };
    let requires_flags = prompt_flags(session, "Campfire", "Requires flags (comma):", "")?;
    push_undo(state);
    state.map.campfires.push(MapCampfire {
        id,
        pos: [state.cursor.0, state.cursor.1],
        campfire_id,
        requires_flags,
        glyph: None,
        palette: None,
    });
    state.dirty = true;
    state.status = "Campfire added".to_string();
    Ok(())
}

fn add_save_point(state: &mut EditorState) {
    let pos = [state.cursor.0, state.cursor.1];
    if !state.map.save_points.iter().any(|entry| *entry == pos) {
        push_undo(state);
        state.map.save_points.push(pos);
        state.dirty = true;
        state.status = "Save point added".to_string();
    } else {
        state.status = "Save point already present".to_string();
    }
}

fn add_event(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    event_ids: &[String],
) -> io::Result<()> {
    let id = prompt_text(session, "Event", "Id:", "event", 32)?;
    let Some(id) = id else {
        return Ok(());
    };
    let trigger = prompt_text(session, "Event", "Trigger:", "on_step", 16)?;
    let Some(trigger) = trigger else {
        return Ok(());
    };
    let script =
        choose_from_list_or_custom(session, bindings, "Event", "Script id:", event_ids, "")?;
    let Some(script) = script else {
        return Ok(());
    };
    push_undo(state);
    state.map.events.push(MapEvent {
        id,
        trigger,
        script,
        zone: None,
        pos: Some([state.cursor.0, state.cursor.1]),
    });
    state.dirty = true;
    state.status = "Event added".to_string();
    Ok(())
}

fn edit_transition(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    map_ids: &[String],
    index: usize,
) -> io::Result<()> {
    let entry = state.map.transitions.get(index).cloned();
    let Some(entry) = entry else {
        return Ok(());
    };
    let target_map = choose_from_list_or_custom(
        session,
        bindings,
        "Transition",
        "Target map:",
        map_ids,
        &entry.target_map,
    )?;
    let Some(target_map) = target_map else {
        return Ok(());
    };
    let target_pos_default = format!("{},{}", entry.target_pos[0], entry.target_pos[1]);
    let target_pos = prompt_pos(
        session,
        "Transition",
        "Target pos (x,y):",
        &target_pos_default,
    )?;
    let Some(target_pos) = target_pos else {
        return Ok(());
    };
    let label = prompt_optional_text(
        session,
        "Transition",
        "Label (optional):",
        entry.label.as_deref().unwrap_or(""),
        32,
    )?;
    let requires_flag = prompt_optional_text(
        session,
        "Transition",
        "Requires flag (optional):",
        entry.requires_flag.as_deref().unwrap_or(""),
        48,
    )?;
    let return_to_last = prompt_yes_no(
        session,
        "Transition",
        "Return to last?",
        entry.return_to_last,
    )?;
    let glyph = prompt_optional_glyph_string(
        session,
        "Transition",
        "Glyph (optional):",
        entry.glyph.as_deref().unwrap_or(""),
    )?;
    let palette = prompt_optional_text(
        session,
        "Transition",
        "Palette (optional):",
        entry.palette.as_deref().unwrap_or(""),
        24,
    )?;
    push_undo(state);
    if let Some(slot) = state.map.transitions.get_mut(index) {
        slot.target_map = target_map;
        slot.target_pos = target_pos;
        slot.label = label;
        slot.requires_flag = requires_flag;
        slot.return_to_last = return_to_last;
        slot.glyph = glyph;
        slot.palette = palette;
    }
    state.dirty = true;
    state.status = "Transition updated".to_string();
    Ok(())
}

fn edit_door(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    map_ids: &[String],
    index: usize,
) -> io::Result<()> {
    let entry = state.map.doors.get(index).cloned();
    let Some(entry) = entry else {
        return Ok(());
    };
    let target_map = choose_optional_from_list_or_custom(
        session,
        bindings,
        "Door",
        "Target map (optional):",
        map_ids,
        entry.target_map.as_deref().unwrap_or(""),
    )?;
    let target_pos = if target_map.is_some() {
        let default = entry
            .target_pos
            .map(|pos| format!("{},{}", pos[0], pos[1]))
            .unwrap_or_else(|| "0,0".to_string());
        prompt_pos(session, "Door", "Target pos (x,y):", &default)?
    } else {
        None
    };
    let requires_flag = prompt_optional_text(
        session,
        "Door",
        "Requires flag (optional):",
        entry.requires_flag.as_deref().unwrap_or(""),
        48,
    )?;
    let locked_text = prompt_optional_text(
        session,
        "Door",
        "Locked text (optional):",
        entry.locked_text.as_deref().unwrap_or(""),
        64,
    )?;
    let locked_event = prompt_optional_text(
        session,
        "Door",
        "Locked event (optional):",
        entry.locked_event.as_deref().unwrap_or(""),
        32,
    )?;
    let return_to_last = prompt_yes_no(session, "Door", "Return to last?", entry.return_to_last)?;
    let glyph = prompt_optional_glyph_string(
        session,
        "Door",
        "Glyph (optional):",
        entry.glyph.as_deref().unwrap_or(""),
    )?;
    let palette = prompt_optional_text(
        session,
        "Door",
        "Palette (optional):",
        entry.palette.as_deref().unwrap_or(""),
        24,
    )?;
    push_undo(state);
    if let Some(slot) = state.map.doors.get_mut(index) {
        slot.target_map = target_map;
        slot.target_pos = target_pos;
        slot.requires_flag = requires_flag;
        slot.locked_text = locked_text;
        slot.locked_event = locked_event;
        slot.return_to_last = return_to_last;
        slot.glyph = glyph;
        slot.palette = palette;
    }
    state.dirty = true;
    state.status = "Door updated".to_string();
    Ok(())
}

fn edit_puzzle(session: &mut TuiSession, state: &mut EditorState, index: usize) -> io::Result<()> {
    let entry = state.map.puzzles.get(index).cloned();
    let Some(entry) = entry else {
        return Ok(());
    };
    let requires_flags = prompt_flags(session, "Puzzle", "Requires flags (comma):", "")?;
    let text = prompt_optional_text(
        session,
        "Puzzle",
        "Text (optional):",
        entry.text.as_deref().unwrap_or(""),
        72,
    )?;
    let event = prompt_optional_text(
        session,
        "Puzzle",
        "Event (optional):",
        entry.event.as_deref().unwrap_or(""),
        32,
    )?;
    let set_flag = prompt_optional_text(
        session,
        "Puzzle",
        "Set flag (optional):",
        entry.set_flag.as_deref().unwrap_or(""),
        48,
    )?;
    let glyph = prompt_optional_glyph_string(
        session,
        "Puzzle",
        "Glyph (optional):",
        entry.glyph.as_deref().unwrap_or(""),
    )?;
    let palette = prompt_optional_text(
        session,
        "Puzzle",
        "Palette (optional):",
        entry.palette.as_deref().unwrap_or(""),
        24,
    )?;
    push_undo(state);
    if let Some(slot) = state.map.puzzles.get_mut(index) {
        slot.requires_flags = requires_flags;
        slot.text = text;
        slot.event = event;
        slot.set_flag = set_flag;
        slot.glyph = glyph;
        slot.palette = palette;
    }
    state.dirty = true;
    state.status = "Puzzle updated".to_string();
    Ok(())
}

fn edit_sign(session: &mut TuiSession, state: &mut EditorState, index: usize) -> io::Result<()> {
    let entry = state.map.signs.get(index).cloned();
    let Some(entry) = entry else {
        return Ok(());
    };
    let text = prompt_text(session, "Sign", "Text:", &entry.text, 72)?;
    let Some(text) = text else {
        return Ok(());
    };
    let glyph = prompt_optional_glyph_string(
        session,
        "Sign",
        "Glyph (optional):",
        entry.glyph.as_deref().unwrap_or(""),
    )?;
    let palette = prompt_optional_text(
        session,
        "Sign",
        "Palette (optional):",
        entry.palette.as_deref().unwrap_or(""),
        24,
    )?;
    push_undo(state);
    if let Some(slot) = state.map.signs.get_mut(index) {
        slot.text = text;
        slot.glyph = glyph;
        slot.palette = palette;
    }
    state.dirty = true;
    state.status = "Sign updated".to_string();
    Ok(())
}

fn edit_chest(session: &mut TuiSession, state: &mut EditorState, index: usize) -> io::Result<()> {
    let entry = state.map.chests.get(index).cloned();
    let Some(entry) = entry else {
        return Ok(());
    };
    let opened_flag = prompt_text(session, "Chest", "Opened flag:", &entry.opened_flag, 64)?;
    let Some(opened_flag) = opened_flag else {
        return Ok(());
    };
    let glyph_closed = prompt_optional_glyph_string(
        session,
        "Chest",
        "Closed glyph (optional):",
        entry.glyph_closed.as_deref().unwrap_or(""),
    )?;
    let glyph_open = prompt_optional_glyph_string(
        session,
        "Chest",
        "Open glyph (optional):",
        entry.glyph_open.as_deref().unwrap_or(""),
    )?;
    let palette = prompt_optional_text(
        session,
        "Chest",
        "Palette (optional):",
        entry.palette.as_deref().unwrap_or(""),
        24,
    )?;
    push_undo(state);
    if let Some(slot) = state.map.chests.get_mut(index) {
        slot.opened_flag = opened_flag;
        slot.glyph_closed = glyph_closed;
        slot.glyph_open = glyph_open;
        slot.palette = palette;
    }
    state.dirty = true;
    state.status = "Chest updated".to_string();
    Ok(())
}

fn edit_vehicle(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    vehicle_ids: &[String],
    index: usize,
) -> io::Result<()> {
    let entry = state.map.vehicles.get(index).cloned();
    let Some(entry) = entry else {
        return Ok(());
    };
    let vehicle_id = choose_from_list_or_custom(
        session,
        bindings,
        "Vehicle",
        "Vehicle id:",
        vehicle_ids,
        &entry.vehicle_id,
    )?;
    let Some(vehicle_id) = vehicle_id else {
        return Ok(());
    };
    let requires_flags = prompt_flags(session, "Vehicle", "Requires flags (comma):", "")?;
    push_undo(state);
    if let Some(slot) = state.map.vehicles.get_mut(index) {
        slot.vehicle_id = vehicle_id;
        slot.requires_flags = requires_flags;
    }
    state.dirty = true;
    state.status = "Vehicle updated".to_string();
    Ok(())
}

fn edit_campfire(
    session: &mut TuiSession,
    state: &mut EditorState,
    index: usize,
) -> io::Result<()> {
    let entry = state.map.campfires.get(index).cloned();
    let Some(entry) = entry else {
        return Ok(());
    };
    let campfire_id = prompt_text(
        session,
        "Campfire",
        "Campfire set id:",
        &entry.campfire_id,
        32,
    )?;
    let Some(campfire_id) = campfire_id else {
        return Ok(());
    };
    let requires_flags = prompt_flags(session, "Campfire", "Requires flags (comma):", "")?;
    let glyph = prompt_optional_glyph_string(
        session,
        "Campfire",
        "Glyph (optional):",
        entry.glyph.as_deref().unwrap_or(""),
    )?;
    let palette = prompt_optional_text(
        session,
        "Campfire",
        "Palette (optional):",
        entry.palette.as_deref().unwrap_or(""),
        24,
    )?;
    push_undo(state);
    if let Some(slot) = state.map.campfires.get_mut(index) {
        slot.campfire_id = campfire_id;
        slot.requires_flags = requires_flags;
        slot.glyph = glyph;
        slot.palette = palette;
    }
    state.dirty = true;
    state.status = "Campfire updated".to_string();
    Ok(())
}

fn edit_event(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    event_ids: &[String],
    index: usize,
) -> io::Result<()> {
    let entry = state.map.events.get(index).cloned();
    let Some(entry) = entry else {
        return Ok(());
    };
    let trigger = prompt_text(session, "Event", "Trigger:", &entry.trigger, 16)?;
    let Some(trigger) = trigger else {
        return Ok(());
    };
    let script = choose_from_list_or_custom(
        session,
        bindings,
        "Event",
        "Script id:",
        event_ids,
        &entry.script,
    )?;
    let Some(script) = script else {
        return Ok(());
    };
    let zone = prompt_optional_text(
        session,
        "Event",
        "Zone (optional):",
        entry.zone.as_deref().unwrap_or(""),
        32,
    )?;
    push_undo(state);
    if let Some(slot) = state.map.events.get_mut(index) {
        slot.trigger = trigger;
        slot.script = script;
        slot.zone = zone;
        slot.pos = Some([state.cursor.0, state.cursor.1]);
    }
    state.dirty = true;
    state.status = "Event updated".to_string();
    Ok(())
}

fn edit_npc(session: &mut TuiSession, state: &mut EditorState, index: usize) -> io::Result<()> {
    let entry = state.map.npcs.get(index).cloned();
    let Some(entry) = entry else {
        return Ok(());
    };
    let script = prompt_optional_text(
        session,
        "NPC",
        "Script (optional):",
        entry.script.as_deref().unwrap_or(""),
        32,
    )?;
    let requires_default = flags_to_string(entry.requires_flags.as_ref());
    let requires_flags =
        prompt_flags(session, "NPC", "Requires flags (comma):", &requires_default)?;
    push_undo(state);
    if let Some(slot) = state.map.npcs.get_mut(index) {
        slot.script = script;
        slot.requires_flags = requires_flags;
    }
    state.dirty = true;
    state.status = "NPC updated".to_string();
    Ok(())
}

fn delete_object_at_cursor(state: &mut EditorState) {
    let pos = [state.cursor.0, state.cursor.1];
    if !has_object_at_cursor(state, pos) {
        state.status = "No objects at cursor".to_string();
        return;
    }
    push_undo(state);
    let mut removed = false;
    let before = state.map.transitions.len();
    state.map.transitions.retain(|item| item.pos != pos);
    removed |= before != state.map.transitions.len();
    let before = state.map.doors.len();
    state.map.doors.retain(|item| item.pos != pos);
    removed |= before != state.map.doors.len();
    let before = state.map.puzzles.len();
    state.map.puzzles.retain(|item| item.pos != pos);
    removed |= before != state.map.puzzles.len();
    let before = state.map.signs.len();
    state.map.signs.retain(|item| item.pos != pos);
    removed |= before != state.map.signs.len();
    let before = state.map.chests.len();
    state.map.chests.retain(|item| item.pos != pos);
    removed |= before != state.map.chests.len();
    let before = state.map.vehicles.len();
    state.map.vehicles.retain(|item| item.pos != pos);
    removed |= before != state.map.vehicles.len();
    let before = state.map.campfires.len();
    state.map.campfires.retain(|item| item.pos != pos);
    removed |= before != state.map.campfires.len();
    let before = state.map.save_points.len();
    state.map.save_points.retain(|item| *item != pos);
    removed |= before != state.map.save_points.len();
    let before = state.map.events.len();
    state.map.events.retain(|item| item.pos != Some(pos));
    removed |= before != state.map.events.len();
    if removed {
        state.dirty = true;
        state.status = "Removed object(s)".to_string();
    }
}

fn has_object_at_cursor(state: &EditorState, pos: [i32; 2]) -> bool {
    state.map.transitions.iter().any(|item| item.pos == pos)
        || state.map.doors.iter().any(|item| item.pos == pos)
        || state.map.puzzles.iter().any(|item| item.pos == pos)
        || state.map.signs.iter().any(|item| item.pos == pos)
        || state.map.chests.iter().any(|item| item.pos == pos)
        || state.map.vehicles.iter().any(|item| item.pos == pos)
        || state.map.campfires.iter().any(|item| item.pos == pos)
        || state.map.save_points.iter().any(|item| *item == pos)
        || state.map.events.iter().any(|item| item.pos == Some(pos))
        || state.map.npcs.iter().any(|item| item.pos == pos)
}

fn resize_map(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
) -> io::Result<()> {
    let width_default = state.map.width.to_string();
    let height_default = state.map.height.to_string();
    let width_input = prompt_text(session, "Resize", "New width:", &width_default, 6)?;
    let Some(width_input) = width_input else {
        return Ok(());
    };
    let height_input = prompt_text(session, "Resize", "New height:", &height_default, 6)?;
    let Some(height_input) = height_input else {
        return Ok(());
    };
    let new_width: i32 = width_input.trim().parse().unwrap_or(state.map.width as i32);
    let new_height: i32 = height_input
        .trim()
        .parse()
        .unwrap_or(state.map.height as i32);
    if new_width <= 0 || new_height <= 0 {
        state.status = "Invalid size".to_string();
        return Ok(());
    }
    let anchors = vec![
        "center".to_string(),
        "top_left".to_string(),
        "top".to_string(),
        "top_right".to_string(),
        "left".to_string(),
        "right".to_string(),
        "bottom_left".to_string(),
        "bottom".to_string(),
        "bottom_right".to_string(),
    ];
    let anchor_choice = prompt_choice(session, bindings, "Resize", "Anchor:", &anchors, 0)?;
    let Some(anchor_choice) = anchor_choice else {
        return Ok(());
    };
    let anchor = anchors[anchor_choice].as_str();
    let old_width = state.map.width as i32;
    let old_height = state.map.height as i32;
    let (offset_x, offset_y) =
        resize_anchor_offsets(anchor, old_width, old_height, new_width, new_height);
    let warnings = resize_warnings(&state.map, offset_x, offset_y, new_width, new_height);
    if !warnings.is_empty() {
        let proceed = confirm_resize_warnings(session, &warnings, |frame| {
            draw_editor_frame(frame, state, &[], &[], &[], &[]);
        })?;
        if !proceed {
            state.status = "Resize cancelled".to_string();
            return Ok(());
        }
    }
    push_undo(state);
    apply_resize(state, offset_x, offset_y, new_width, new_height);
    state.status = format!("Resized to {}x{}", new_width, new_height);
    state.dirty = true;
    Ok(())
}

fn resize_anchor_offsets(
    anchor: &str,
    old_width: i32,
    old_height: i32,
    new_width: i32,
    new_height: i32,
) -> (i32, i32) {
    let dx = new_width - old_width;
    let dy = new_height - old_height;
    match anchor {
        "top_left" => (0, 0),
        "top" => (dx / 2, 0),
        "top_right" => (dx, 0),
        "left" => (0, dy / 2),
        "right" => (dx, dy / 2),
        "bottom_left" => (0, dy),
        "bottom" => (dx / 2, dy),
        "bottom_right" => (dx, dy),
        _ => (dx / 2, dy / 2),
    }
}

fn resize_warnings(
    map: &MapData,
    offset_x: i32,
    offset_y: i32,
    width: i32,
    height: i32,
) -> Vec<String> {
    let mut warnings = Vec::new();
    for transition in &map.transitions {
        if !pos_in_bounds(transition.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("transition:{}", transition.id));
        }
    }
    for door in &map.doors {
        if !pos_in_bounds(door.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("door:{}", door.id));
        }
    }
    for puzzle in &map.puzzles {
        if !pos_in_bounds(puzzle.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("puzzle:{}", puzzle.id));
        }
    }
    for sign in &map.signs {
        if !pos_in_bounds(sign.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("sign:{}", sign.id));
        }
    }
    for chest in &map.chests {
        if !pos_in_bounds(chest.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("chest:{}", chest.id));
        }
    }
    for vehicle in &map.vehicles {
        if !pos_in_bounds(vehicle.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("vehicle:{}", vehicle.vehicle_id));
        }
    }
    for campfire in &map.campfires {
        if !pos_in_bounds(campfire.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("campfire:{}", campfire.id));
        }
    }
    for save in &map.save_points {
        if !pos_in_bounds(*save, offset_x, offset_y, width, height) {
            warnings.push(format!("save_point:[{},{}]", save[0], save[1]));
        }
    }
    for event in &map.events {
        if let Some(pos) = event.pos {
            if !pos_in_bounds(pos, offset_x, offset_y, width, height) {
                warnings.push(format!("event:{}", event.id));
            }
        }
    }
    for npc in &map.npcs {
        if !pos_in_bounds(npc.pos, offset_x, offset_y, width, height) {
            warnings.push(format!("npc:{}", npc.id));
        }
    }
    for zone in &map.encounters {
        let outcome = truncate_rect(zone.rect, offset_x, offset_y, width, height);
        if outcome.truncated || outcome.removed {
            if outcome.removed {
                warnings.push(format!("encounter:{} (removed)", zone.zone_id));
            } else {
                warnings.push(format!("encounter:{} (truncated)", zone.zone_id));
            }
        }
    }
    warnings
}

fn pos_in_bounds(pos: [i32; 2], offset_x: i32, offset_y: i32, width: i32, height: i32) -> bool {
    let x = pos[0] + offset_x;
    let y = pos[1] + offset_y;
    x >= 0 && y >= 0 && x < width && y < height
}

struct RectOutcome {
    rect: Option<[i32; 4]>,
    truncated: bool,
    removed: bool,
}

fn truncate_rect(
    rect: [i32; 4],
    offset_x: i32,
    offset_y: i32,
    width: i32,
    height: i32,
) -> RectOutcome {
    let x = rect[0] + offset_x;
    let y = rect[1] + offset_y;
    let w = rect[2];
    let h = rect[3];
    if w <= 0 || h <= 0 {
        return RectOutcome {
            rect: None,
            truncated: true,
            removed: true,
        };
    }
    let x2 = x + w;
    let y2 = y + h;
    let new_x = x.max(0);
    let new_y = y.max(0);
    let new_x2 = x2.min(width);
    let new_y2 = y2.min(height);
    if new_x2 <= new_x || new_y2 <= new_y {
        return RectOutcome {
            rect: None,
            truncated: true,
            removed: true,
        };
    }
    let new_w = new_x2 - new_x;
    let new_h = new_y2 - new_y;
    let truncated = new_x != x || new_y != y || new_w != w || new_h != h;
    RectOutcome {
        rect: Some([new_x, new_y, new_w, new_h]),
        truncated,
        removed: false,
    }
}

fn apply_resize(state: &mut EditorState, offset_x: i32, offset_y: i32, width: i32, height: i32) {
    let fill = state.active_glyph();
    let mut tiles = vec![vec![fill; width as usize]; height as usize];
    for y in 0..state.map.height as i32 {
        for x in 0..state.map.width as i32 {
            let new_x = x + offset_x;
            let new_y = y + offset_y;
            if new_x < 0 || new_y < 0 || new_x >= width || new_y >= height {
                continue;
            }
            tiles[new_y as usize][new_x as usize] = tile_at(&state.map, x, y);
        }
    }
    state.map.tiles = tiles;
    state.map.width = width as u32;
    state.map.height = height as u32;

    shift_objects(state, offset_x, offset_y, width, height);
}

fn shift_objects(state: &mut EditorState, offset_x: i32, offset_y: i32, width: i32, height: i32) {
    let within = |pos: [i32; 2]| pos_in_bounds(pos, offset_x, offset_y, width, height);
    for transition in &mut state.map.transitions {
        transition.pos[0] += offset_x;
        transition.pos[1] += offset_y;
    }
    state.map.transitions.retain(|item| within(item.pos));
    for door in &mut state.map.doors {
        door.pos[0] += offset_x;
        door.pos[1] += offset_y;
    }
    state.map.doors.retain(|item| within(item.pos));
    for puzzle in &mut state.map.puzzles {
        puzzle.pos[0] += offset_x;
        puzzle.pos[1] += offset_y;
    }
    state.map.puzzles.retain(|item| within(item.pos));
    for sign in &mut state.map.signs {
        sign.pos[0] += offset_x;
        sign.pos[1] += offset_y;
    }
    state.map.signs.retain(|item| within(item.pos));
    for chest in &mut state.map.chests {
        chest.pos[0] += offset_x;
        chest.pos[1] += offset_y;
    }
    state.map.chests.retain(|item| within(item.pos));
    for vehicle in &mut state.map.vehicles {
        vehicle.pos[0] += offset_x;
        vehicle.pos[1] += offset_y;
    }
    state.map.vehicles.retain(|item| within(item.pos));
    for campfire in &mut state.map.campfires {
        campfire.pos[0] += offset_x;
        campfire.pos[1] += offset_y;
    }
    state.map.campfires.retain(|item| within(item.pos));
    for save in &mut state.map.save_points {
        save[0] += offset_x;
        save[1] += offset_y;
    }
    state.map.save_points.retain(|item| within(*item));
    let mut new_events = Vec::new();
    for mut event in state.map.events.drain(..) {
        if let Some(pos) = event.pos {
            let updated = [pos[0] + offset_x, pos[1] + offset_y];
            if within(updated) {
                event.pos = Some(updated);
                new_events.push(event);
            }
        } else {
            new_events.push(event);
        }
    }
    state.map.events = new_events;
    for npc in &mut state.map.npcs {
        npc.pos[0] += offset_x;
        npc.pos[1] += offset_y;
    }
    state.map.npcs.retain(|item| within(item.pos));

    let mut new_zones = Vec::new();
    for zone in &state.map.encounters {
        let outcome = truncate_rect(zone.rect, offset_x, offset_y, width, height);
        if let Some(rect) = outcome.rect {
            let mut updated = zone.clone();
            updated.rect = rect;
            new_zones.push(updated);
        }
    }
    state.map.encounters = new_zones;
}

fn confirm_resize_warnings<F>(
    session: &mut TuiSession,
    warnings: &[String],
    draw_background: F,
) -> io::Result<bool>
where
    F: Fn(&mut Frame),
{
    let mut offset = 0usize;
    loop {
        session.terminal_mut().draw(|frame| {
            draw_background(frame);
            let area = centered_rect(frame.size(), 60, 18);
            frame.render_widget(ratatui::widgets::Clear, area);
            let title = "Resize Warnings";
            let mut lines = Vec::new();
            lines.push(Line::from(Span::raw("Objects truncated or removed:")));
            let view_height = area.height.saturating_sub(5) as usize;
            for warning in warnings.iter().skip(offset).take(view_height) {
                lines.push(Line::from(Span::raw(warning)));
            }
            lines.push(Line::from(Span::raw("")));
            lines.push(Line::from(Span::raw(
                "Enter=Proceed  Esc=Cancel  Up/Down=Scroll",
            )));
            let paragraph = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title(title))
                .alignment(ratatui::layout::Alignment::Left)
                .wrap(Wrap { trim: false });
            frame.render_widget(paragraph, area);
        })?;

        if let Event::Key(key) = event::read()? {
            if !is_actionable_key(&key) {
                continue;
            }
            match key.code {
                KeyCode::Enter => return Ok(true),
                KeyCode::Esc => return Ok(false),
                KeyCode::Up => {
                    if offset > 0 {
                        offset -= 1;
                    }
                }
                KeyCode::Down => {
                    if offset + 1 < warnings.len() {
                        offset += 1;
                    }
                }
                _ => {}
            }
        }
    }
}

fn prompt_glyph(
    session: &mut TuiSession,
    title: &str,
    prompt: &str,
    default: &str,
) -> io::Result<Option<char>> {
    let value = prompt_text(session, title, prompt, default, 2)?;
    let Some(value) = value else {
        return Ok(None);
    };
    let mut chars = value.trim().chars();
    let glyph = chars.next().unwrap_or('.');
    Ok(Some(glyph))
}

fn prompt_optional_text(
    session: &mut TuiSession,
    title: &str,
    prompt: &str,
    default: &str,
    max_len: usize,
) -> io::Result<Option<String>> {
    let value = prompt_text(session, title, prompt, default, max_len)?;
    Ok(value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }))
}

fn prompt_optional_glyph_string(
    session: &mut TuiSession,
    title: &str,
    prompt: &str,
    default: &str,
) -> io::Result<Option<String>> {
    let value = prompt_text(session, title, prompt, default, 2)?;
    Ok(value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            trimmed.chars().next().map(|ch| ch.to_string())
        }
    }))
}

fn prompt_yes_no(
    session: &mut TuiSession,
    title: &str,
    prompt: &str,
    default: bool,
) -> io::Result<bool> {
    let options = vec!["No".to_string(), "Yes".to_string()];
    let default_index = if default { 1 } else { 0 };
    let selection = prompt_choice(
        session,
        &InputBindings::default_bindings(),
        title,
        prompt,
        &options,
        default_index,
    )?;
    Ok(matches!(selection, Some(1)))
}

fn prompt_pos(
    session: &mut TuiSession,
    title: &str,
    prompt: &str,
    default: &str,
) -> io::Result<Option<[i32; 2]>> {
    let value = prompt_text(session, title, prompt, default, 16)?;
    let Some(value) = value else {
        return Ok(None);
    };
    let parts = value.split(',').collect::<Vec<_>>();
    if parts.len() != 2 {
        return Ok(None);
    }
    let x: i32 = parts[0].trim().parse().unwrap_or(0);
    let y: i32 = parts[1].trim().parse().unwrap_or(0);
    Ok(Some([x, y]))
}

fn prompt_flags(
    session: &mut TuiSession,
    title: &str,
    prompt: &str,
    default: &str,
) -> io::Result<Option<Vec<String>>> {
    let value = prompt_text(session, title, prompt, default, 128)?;
    let Some(value) = value else {
        return Ok(None);
    };
    let flags = value
        .split(',')
        .map(|flag| flag.trim())
        .filter(|flag| !flag.is_empty())
        .map(|flag| flag.to_string())
        .collect::<Vec<_>>();
    if flags.is_empty() {
        Ok(None)
    } else {
        Ok(Some(flags))
    }
}

fn flags_to_string(flags: Option<&Vec<String>>) -> String {
    flags.map(|items| items.join(", ")).unwrap_or_default()
}

fn choose_from_list_or_custom(
    session: &mut TuiSession,
    bindings: &InputBindings,
    title: &str,
    prompt: &str,
    options: &[String],
    default: &str,
) -> io::Result<Option<String>> {
    if options.is_empty() {
        return prompt_text(session, title, prompt, default, 48);
    }
    let mut choices = vec!["<custom>".to_string()];
    choices.extend(options.iter().cloned());
    let selected = prompt_choice(session, bindings, title, prompt, &choices, 1)?;
    match selected {
        Some(0) => prompt_text(session, title, prompt, default, 48),
        Some(index) => Ok(choices.get(index).cloned()),
        None => Ok(None),
    }
}

fn choose_optional_from_list_or_custom(
    session: &mut TuiSession,
    bindings: &InputBindings,
    title: &str,
    prompt: &str,
    options: &[String],
    default: &str,
) -> io::Result<Option<String>> {
    if options.is_empty() {
        let value = prompt_text(session, title, prompt, default, 48)?;
        return Ok(value.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }));
    }
    let mut choices = vec!["<none>".to_string(), "<custom>".to_string()];
    choices.extend(options.iter().cloned());
    let selected = prompt_choice(session, bindings, title, prompt, &choices, 0)?;
    match selected {
        Some(0) | None => Ok(None),
        Some(1) => {
            let value = prompt_text(session, title, prompt, default, 48)?;
            Ok(value.and_then(|value| {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }))
        }
        Some(index) => Ok(choices.get(index).cloned()),
    }
}

fn normalize_tiles(map: &mut MapData, default_glyph: Option<char>) {
    let width = map.width as usize;
    let height = map.height as usize;
    let fill =
        default_glyph.unwrap_or_else(|| map.legend.first().map(|entry| entry.glyph).unwrap_or('.'));
    while map.tiles.len() < height {
        map.tiles.push(vec![fill; width]);
    }
    if map.tiles.len() > height {
        map.tiles.truncate(height);
    }
    for row in &mut map.tiles {
        if row.len() < width {
            row.extend(std::iter::repeat(fill).take(width - row.len()));
        } else if row.len() > width {
            row.truncate(width);
        }
    }
    let mut existing = HashMap::new();
    for entry in &map.legend {
        existing.insert(entry.glyph, entry.tile.clone());
    }
    for row in &map.tiles {
        for glyph in row {
            if !existing.contains_key(glyph) {
                map.legend.push(LegendEntry {
                    glyph: *glyph,
                    tile: "unknown".to_string(),
                    passable: true,
                    palette: None,
                });
                existing.insert(*glyph, "unknown".to_string());
            }
        }
    }
}

fn replace_tile_glyph(map: &mut MapData, from: char, to: char) {
    for row in &mut map.tiles {
        for glyph in row {
            if *glyph == from {
                *glyph = to;
            }
        }
    }
}

fn tile_at(map: &MapData, x: i32, y: i32) -> char {
    if x < 0 || y < 0 {
        return ' ';
    }
    let Some(row) = map.tiles.get(y as usize) else {
        return ' ';
    };
    *row.get(x as usize).unwrap_or(&' ')
}

fn set_tile(map: &mut MapData, x: i32, y: i32, glyph: char) {
    if x < 0 || y < 0 {
        return;
    }
    if let Some(row) = map.tiles.get_mut(y as usize) {
        if let Some(cell) = row.get_mut(x as usize) {
            *cell = glyph;
        }
    }
}

fn draw_editor_frame(
    frame: &mut Frame,
    state: &EditorState,
    map_ids: &[String],
    event_ids: &[String],
    vehicle_ids: &[String],
    npc_ids: &[String],
) {
    let area = frame.size();
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);
    let map_area = chunks[0];
    let info_area = chunks[1];

    draw_map_panel(frame, map_area, state);
    draw_info_panel(
        frame,
        info_area,
        state,
        map_ids,
        event_ids,
        vehicle_ids,
        npc_ids,
    );
}

fn draw_map_panel(frame: &mut Frame, area: Rect, state: &EditorState) {
    let inner_width = area.width.saturating_sub(2);
    let inner_height = area.height.saturating_sub(2);
    let view_width = inner_width as i32;
    let view_height = inner_height as i32;
    let (start_x, start_y) = viewport_origin(state, view_width, view_height);
    let legend_map = legend_map(&state.map);

    let mut lines = Vec::new();
    for y in 0..view_height {
        let mut row = Vec::new();
        for x in 0..view_width {
            let map_x = start_x + x;
            let map_y = start_y + y;
            let mut glyph = if map_x >= 0
                && map_y >= 0
                && map_x < state.map.width as i32
                && map_y < state.map.height as i32
            {
                tile_at(&state.map, map_x, map_y)
            } else {
                ' '
            };
            let mut style = legend_map
                .get(&glyph)
                .and_then(|entry| entry.palette.as_deref())
                .map(|palette| palette_style(true, Some(palette)))
                .unwrap_or_default();
            if let Some(object_glyph) = object_glyph_at(state, map_x, map_y) {
                glyph = object_glyph;
                style = Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD);
            }
            if let Some((min_x, min_y, max_x, max_y)) = selection_rect(state) {
                if map_x >= min_x && map_x <= max_x && map_y >= min_y && map_y <= max_y {
                    style = style.bg(Color::Yellow).fg(Color::Black);
                }
            }
            if (map_x, map_y) == state.cursor {
                style = style
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD);
            }
            row.push(Span::styled(glyph.to_string(), style));
        }
        lines.push(Line::from(row));
    }

    let title = format!("Map: {}", state.map.id);
    let block = Block::default().borders(Borders::ALL).title(title);
    let paragraph = Paragraph::new(lines)
        .block(block)
        .alignment(ratatui::layout::Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_info_panel(
    frame: &mut Frame,
    area: Rect,
    state: &EditorState,
    map_ids: &[String],
    event_ids: &[String],
    vehicle_ids: &[String],
    npc_ids: &[String],
) {
    let mut lines = Vec::new();
    lines.push(format!("Cursor: {},{}", state.cursor.0, state.cursor.1));
    let mode = if state.selection.is_some() {
        "VISUAL"
    } else {
        "NORMAL"
    };
    lines.push(format!("Mode: {}", mode));
    lines.push(format!("Size: {}x{}", state.map.width, state.map.height));
    lines.push(format!("Active tile: {}", state.active_glyph()));
    if let Some(buffer) = &state.yank {
        lines.push(format!("Yank: {}x{}", buffer.width, buffer.height));
    }
    lines.push(String::new());
    lines.push("Keys:".to_string());
    lines.push("Arrows/HJKL move".to_string());
    lines.push("V visual, y yank, p paste".to_string());
    lines.push("R paint, t tile, L legend".to_string());
    lines.push("o add, e edit, m move".to_string());
    lines.push("x delete, u undo, U redo".to_string());
    lines.push("= resize".to_string());
    lines.push("s save, q quit".to_string());
    lines.push(String::new());
    lines.push("Objects at cursor:".to_string());
    let objects = objects_at_cursor(state);
    if objects.is_empty() {
        lines.push("(none)".to_string());
    } else {
        lines.extend(objects);
    }
    lines.push(String::new());
    lines.push("Lookups:".to_string());
    lines.push(format!(
        "maps: {}  events: {}",
        map_ids.len(),
        event_ids.len()
    ));
    lines.push(format!(
        "vehicles: {}  npcs: {}",
        vehicle_ids.len(),
        npc_ids.len()
    ));
    lines.push(String::new());
    if let Some(moving) = &state.moving {
        lines.push(format!("Moving: {}", moving_label(&moving.target)));
    }
    lines.push(format!("Status: {}", state.status));

    let width = area.width.saturating_sub(2) as usize;
    let content = lines
        .into_iter()
        .map(|line| truncate_line(&line, width))
        .map(|line| Line::from(Span::raw(line)))
        .collect::<Vec<_>>();
    let paragraph = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title("Details"))
        .alignment(ratatui::layout::Alignment::Left)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn object_glyph_at(state: &EditorState, x: i32, y: i32) -> Option<char> {
    let pos = [x, y];
    if state.map.transitions.iter().any(|item| item.pos == pos) {
        return Some('T');
    }
    if state.map.doors.iter().any(|item| item.pos == pos) {
        return Some('+');
    }
    if state.map.puzzles.iter().any(|item| item.pos == pos) {
        return Some('?');
    }
    if state.map.signs.iter().any(|item| item.pos == pos) {
        return Some('!');
    }
    if state.map.chests.iter().any(|item| item.pos == pos) {
        return Some('C');
    }
    if state.map.vehicles.iter().any(|item| item.pos == pos) {
        return Some('V');
    }
    if state.map.campfires.iter().any(|item| item.pos == pos) {
        return Some('F');
    }
    if state.map.save_points.iter().any(|item| *item == pos) {
        return Some('S');
    }
    if state.map.events.iter().any(|item| item.pos == Some(pos)) {
        return Some('E');
    }
    if state.map.npcs.iter().any(|item| item.pos == pos) {
        return Some('N');
    }
    None
}

fn objects_at_cursor(state: &EditorState) -> Vec<String> {
    let pos = [state.cursor.0, state.cursor.1];
    let mut out = Vec::new();
    for transition in &state.map.transitions {
        if transition.pos == pos {
            out.push(format!("transition:{}", transition.id));
        }
    }
    for door in &state.map.doors {
        if door.pos == pos {
            out.push(format!("door:{}", door.id));
        }
    }
    for puzzle in &state.map.puzzles {
        if puzzle.pos == pos {
            out.push(format!("puzzle:{}", puzzle.id));
        }
    }
    for sign in &state.map.signs {
        if sign.pos == pos {
            out.push(format!("sign:{}", sign.id));
        }
    }
    for chest in &state.map.chests {
        if chest.pos == pos {
            out.push(format!("chest:{}", chest.id));
        }
    }
    for vehicle in &state.map.vehicles {
        if vehicle.pos == pos {
            out.push(format!("vehicle:{}", vehicle.vehicle_id));
        }
    }
    for campfire in &state.map.campfires {
        if campfire.pos == pos {
            out.push(format!("campfire:{}", campfire.id));
        }
    }
    for save in &state.map.save_points {
        if *save == pos {
            out.push("save_point".to_string());
        }
    }
    for event in &state.map.events {
        if event.pos == Some(pos) {
            out.push(format!("event:{}", event.id));
        }
    }
    for npc in &state.map.npcs {
        if npc.pos == pos {
            out.push(format!("npc:{}", npc.id));
        }
    }
    out
}

fn viewport_origin(state: &EditorState, view_width: i32, view_height: i32) -> (i32, i32) {
    let map_width = state.map.width as i32;
    let map_height = state.map.height as i32;
    let start_x = if map_width <= view_width {
        -((view_width - map_width) / 2)
    } else {
        let half_width = view_width / 2;
        let max_x = map_width - view_width;
        (state.cursor.0 - half_width).clamp(0, max_x.max(0))
    };
    let start_y = if map_height <= view_height {
        -((view_height - map_height) / 2)
    } else {
        let half_height = view_height / 2;
        let max_y = map_height - view_height;
        (state.cursor.1 - half_height).clamp(0, max_y.max(0))
    };
    (start_x, start_y)
}

fn legend_map(map: &MapData) -> HashMap<char, LegendEntry> {
    map.legend
        .iter()
        .cloned()
        .map(|entry| (entry.glyph, entry))
        .collect::<HashMap<_, _>>()
}

fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    crate::utils::centered_rect(area, width, height)
}
