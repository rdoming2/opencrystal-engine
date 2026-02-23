use std::collections::HashMap;

use super::{LegendEntry, MapData};

#[derive(Clone, Debug)]
pub(super) struct YankBuffer {
    tiles: Vec<Vec<char>>,
    pub(super) width: i32,
    pub(super) height: i32,
}

#[derive(Clone, Debug)]
pub(super) struct Selection {
    anchor: (i32, i32),
}

#[derive(Clone, Debug)]
pub(super) struct EditorState {
    pub(super) map: MapData,
    pub(super) cursor: (i32, i32),
    pub(super) selection: Option<Selection>,
    pub(super) yank: Option<YankBuffer>,
    pub(super) active_tile_index: usize,
    pub(super) moving: Option<MovingObject>,
    pub(super) undo_stack: Vec<MapData>,
    pub(super) redo_stack: Vec<MapData>,
    pub(super) dirty: bool,
    pub(super) status: String,
}

#[derive(Clone, Debug)]
pub(super) struct MovingObject {
    pub(super) target: CursorObject,
    pub(super) pos: [i32; 2],
    pub(super) undo_pushed: bool,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum CursorObject {
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

impl EditorState {
    pub(super) fn new(mut map: MapData) -> Self {
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

    pub(super) fn active_glyph(&self) -> char {
        self.map
            .legend
            .get(self.active_tile_index)
            .map(|entry| entry.glyph)
            .unwrap_or('.')
    }
}

pub(super) fn move_cursor(state: &mut EditorState, dx: i32, dy: i32) {
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

pub(super) fn push_undo(state: &mut EditorState) {
    if state.undo_stack.len() >= 50 {
        state.undo_stack.remove(0);
    }
    state.undo_stack.push(state.map.clone());
    state.redo_stack.clear();
}

pub(super) fn undo(state: &mut EditorState) {
    if let Some(previous) = state.undo_stack.pop() {
        state.redo_stack.push(state.map.clone());
        state.map = previous;
        state.status = "Undo".to_string();
        state.dirty = true;
    } else {
        state.status = "Nothing to undo".to_string();
    }
}

pub(super) fn redo(state: &mut EditorState) {
    if let Some(next) = state.redo_stack.pop() {
        state.undo_stack.push(state.map.clone());
        state.map = next;
        state.status = "Redo".to_string();
        state.dirty = true;
    } else {
        state.status = "Nothing to redo".to_string();
    }
}

pub(super) fn toggle_visual(state: &mut EditorState) {
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

pub(super) fn selection_rect(state: &EditorState) -> Option<(i32, i32, i32, i32)> {
    let selection = state.selection.as_ref()?;
    let (x0, y0) = selection.anchor;
    let (x1, y1) = state.cursor;
    let min_x = x0.min(x1);
    let max_x = x0.max(x1);
    let min_y = y0.min(y1);
    let max_y = y0.max(y1);
    Some((min_x, min_y, max_x, max_y))
}

pub(super) fn yank_selection(state: &mut EditorState) {
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

pub(super) fn paste_selection(state: &mut EditorState) {
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

pub(super) fn paint_active_tile(state: &mut EditorState) {
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

pub(super) fn cycle_active_tile(state: &mut EditorState, delta: i32) {
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

pub(super) fn replace_tile_glyph(map: &mut MapData, from: char, to: char) {
    for row in &mut map.tiles {
        for glyph in row {
            if *glyph == from {
                *glyph = to;
            }
        }
    }
}

pub(super) fn tile_at(map: &MapData, x: i32, y: i32) -> char {
    if x < 0 || y < 0 {
        return ' ';
    }
    let Some(row) = map.tiles.get(y as usize) else {
        return ' ';
    };
    *row.get(x as usize).unwrap_or(&' ')
}

pub(super) fn set_tile(map: &mut MapData, x: i32, y: i32, glyph: char) {
    if x < 0 || y < 0 {
        return;
    }
    if let Some(row) = map.tiles.get_mut(y as usize) {
        if let Some(cell) = row.get_mut(x as usize) {
            *cell = glyph;
        }
    }
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
