mod catalog;
mod create_edit;
mod geometry;
mod loot;
mod menu;

use std::io;

use crate::input::InputBindings;
use crate::session::TuiSession;

use super::state::{push_undo, CursorObject, EditorState};

pub(super) struct ObjectGlyph {
    pub(super) glyph: char,
    pub(super) palette: Option<String>,
}

pub(super) fn edit_objects(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    map_ids: &[String],
    event_ids: &[String],
    vehicle_ids: &[String],
    npc_ids: &[String],
    item_ids: &[String],
    equipment_ids: &[String],
    currency_ids: &[String],
    campfire_ids: &[String],
    encounter_zone_ids: &[String],
    encounter_table_ids: &[String],
) -> io::Result<()> {
    menu::edit_objects(
        session,
        bindings,
        state,
        map_ids,
        event_ids,
        vehicle_ids,
        npc_ids,
        item_ids,
        equipment_ids,
        currency_ids,
        campfire_ids,
        encounter_zone_ids,
        encounter_table_ids,
    )
}

pub(super) fn edit_object_at_cursor(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    map_ids: &[String],
    event_ids: &[String],
    vehicle_ids: &[String],
    npc_ids: &[String],
    item_ids: &[String],
    equipment_ids: &[String],
    currency_ids: &[String],
    campfire_ids: &[String],
    encounter_zone_ids: &[String],
    encounter_table_ids: &[String],
) -> io::Result<()> {
    menu::edit_object_at_cursor(
        session,
        bindings,
        state,
        map_ids,
        event_ids,
        vehicle_ids,
        npc_ids,
        item_ids,
        equipment_ids,
        currency_ids,
        campfire_ids,
        encounter_zone_ids,
        encounter_table_ids,
    )
}

pub(super) fn toggle_move_object(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
) -> io::Result<()> {
    menu::toggle_move_object(session, bindings, state)
}

pub(super) fn object_glyph_at(state: &EditorState, x: i32, y: i32) -> Option<ObjectGlyph> {
    catalog::object_glyph_at(state, x, y)
}

pub(super) fn objects_at_cursor(state: &EditorState) -> Vec<String> {
    catalog::objects_at_cursor(state)
}

pub(super) fn moving_label(target: &CursorObject) -> String {
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
        CursorObject::EncounterZone(_) => "encounter_zone".to_string(),
        CursorObject::SavePoint => "save_point".to_string(),
    }
}

pub(super) fn delete_object_at_cursor(state: &mut EditorState) {
    let pos = [state.cursor.0, state.cursor.1];
    if !has_object_at_cursor(state, pos) {
        state.status = "No objects at cursor".to_string();
        return;
    }
    push_undo(state);
    if remove_objects_at_pos(state, pos) {
        mark_dirty(state, "Removed object(s)");
    }
}

pub(super) fn mark_dirty(state: &mut EditorState, status: &str) {
    state.dirty = true;
    state.status = status.to_string();
}

fn has_object_at_cursor(state: &EditorState, pos: [i32; 2]) -> bool {
    !catalog::object_entries_at_pos(state, pos).is_empty()
}

fn remove_objects_at_pos(state: &mut EditorState, pos: [i32; 2]) -> bool {
    let mut removed = false;
    removed |= retain_by_pos(&mut state.map.transitions, pos, |item, pos| item.pos == pos);
    removed |= retain_by_pos(&mut state.map.doors, pos, |item, pos| item.pos == pos);
    removed |= retain_by_pos(&mut state.map.puzzles, pos, |item, pos| item.pos == pos);
    removed |= retain_by_pos(&mut state.map.signs, pos, |item, pos| item.pos == pos);
    removed |= retain_by_pos(&mut state.map.chests, pos, |item, pos| item.pos == pos);
    removed |= retain_by_pos(&mut state.map.vehicles, pos, |item, pos| item.pos == pos);
    removed |= retain_by_pos(&mut state.map.campfires, pos, |item, pos| item.pos == pos);
    removed |= retain_by_pos(&mut state.map.npcs, pos, |item, pos| item.pos == pos);
    removed |= retain_by_pos(&mut state.map.save_points, pos, |item, pos| *item == pos);
    removed |= retain_by_pos(&mut state.map.events, pos, |item, pos| {
        item.pos == Some(pos)
    });
    removed |= retain_by_pos(&mut state.map.encounters, pos, |item, pos| {
        geometry::pos_in_rect(pos, item.rect)
    });
    removed
}

fn retain_by_pos<T>(
    items: &mut Vec<T>,
    pos: [i32; 2],
    mut matches: impl FnMut(&T, [i32; 2]) -> bool,
) -> bool {
    let before = items.len();
    items.retain(|item| !matches(item, pos));
    before != items.len()
}
