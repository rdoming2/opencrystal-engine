use std::io;

use crate::dialog::{prompt_choice, prompt_text};
use crate::input::InputBindings;
use crate::session::TuiSession;

use super::prompts::{
    choose_from_list_or_custom, choose_optional_from_list_or_custom, flags_to_string, prompt_flags,
    prompt_optional_glyph_string, prompt_optional_text, prompt_pos, prompt_yes_no,
};
use super::state::{push_undo, CursorObject, EditorState, MovingObject};
use super::{
    MapCampfire, MapChest, MapChestLoot, MapDoor, MapEvent, MapPuzzle, MapSign, MapTransition,
    MapVehicle,
};

pub(super) fn edit_objects(
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

pub(super) fn edit_object_at_cursor(
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

pub(super) fn toggle_move_object(
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

fn cursor_objects(state: &EditorState, pos: [i32; 2]) -> (Vec<String>, Vec<CursorObject>) {
    let entries = object_entries_at_pos(state, pos);
    let choices = entries.iter().map(|entry| entry.label.clone()).collect();
    let refs = entries.iter().map(|entry| entry.cursor).collect();
    (choices, refs)
}

struct ObjectEntry {
    label: String,
    cursor: CursorObject,
    glyph: Option<char>,
}

fn object_entries_at_pos(state: &EditorState, pos: [i32; 2]) -> Vec<ObjectEntry> {
    let mut entries = Vec::new();
    for (index, item) in state.map.transitions.iter().enumerate() {
        if item.pos == pos {
            entries.push(ObjectEntry {
                label: format!("transition:{}", item.id),
                cursor: CursorObject::Transition(index),
                glyph: Some('T'),
            });
        }
    }
    for (index, item) in state.map.doors.iter().enumerate() {
        if item.pos == pos {
            entries.push(ObjectEntry {
                label: format!("door:{}", item.id),
                cursor: CursorObject::Door(index),
                glyph: Some('+'),
            });
        }
    }
    for (index, item) in state.map.puzzles.iter().enumerate() {
        if item.pos == pos {
            entries.push(ObjectEntry {
                label: format!("puzzle:{}", item.id),
                cursor: CursorObject::Puzzle(index),
                glyph: Some('?'),
            });
        }
    }
    for (index, item) in state.map.signs.iter().enumerate() {
        if item.pos == pos {
            entries.push(ObjectEntry {
                label: format!("sign:{}", item.id),
                cursor: CursorObject::Sign(index),
                glyph: Some('!'),
            });
        }
    }
    for (index, item) in state.map.chests.iter().enumerate() {
        if item.pos == pos {
            entries.push(ObjectEntry {
                label: format!("chest:{}", item.id),
                cursor: CursorObject::Chest(index),
                glyph: Some('C'),
            });
        }
    }
    for (index, item) in state.map.vehicles.iter().enumerate() {
        if item.pos == pos {
            entries.push(ObjectEntry {
                label: format!("vehicle:{}", item.vehicle_id),
                cursor: CursorObject::Vehicle(index),
                glyph: Some('V'),
            });
        }
    }
    for (index, item) in state.map.campfires.iter().enumerate() {
        if item.pos == pos {
            entries.push(ObjectEntry {
                label: format!("campfire:{}", item.id),
                cursor: CursorObject::Campfire(index),
                glyph: Some('F'),
            });
        }
    }
    for (index, item) in state.map.events.iter().enumerate() {
        if item.pos == Some(pos) {
            entries.push(ObjectEntry {
                label: format!("event:{}", item.id),
                cursor: CursorObject::Event(index),
                glyph: Some('E'),
            });
        }
    }
    for (index, item) in state.map.npcs.iter().enumerate() {
        if item.pos == pos {
            entries.push(ObjectEntry {
                label: format!("npc:{}", item.id),
                cursor: CursorObject::Npc(index),
                glyph: Some('N'),
            });
        }
    }
    if state.map.save_points.iter().any(|entry| *entry == pos) {
        entries.push(ObjectEntry {
            label: "save_point".to_string(),
            cursor: CursorObject::SavePoint,
            glyph: Some('S'),
        });
    }
    entries
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
    let label = prompt_optional_text(session, "Transition", "Label (optional):", "", 32)?;
    let requires_flag =
        prompt_optional_text(session, "Transition", "Requires flag (optional):", "", 48)?;
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
    mark_dirty(state, "Transition added");
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
    let requires_flag = prompt_optional_text(session, "Door", "Requires flag (optional):", "", 48)?;
    let locked_text = prompt_optional_text(session, "Door", "Locked text (optional):", "", 64)?;
    let locked_event = prompt_optional_text(session, "Door", "Locked event (optional):", "", 32)?;
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
    mark_dirty(state, "Door added");
    Ok(())
}

fn add_puzzle(session: &mut TuiSession, state: &mut EditorState) -> io::Result<()> {
    let id = prompt_text(session, "Puzzle", "Id:", "puzzle", 32)?;
    let Some(id) = id else {
        return Ok(());
    };
    let requires_flags = prompt_flags(session, "Puzzle", "Requires flags (comma):", "")?;
    let text = prompt_optional_text(session, "Puzzle", "Text (optional):", "", 72)?;
    let event = prompt_optional_text(session, "Puzzle", "Event (optional):", "", 32)?;
    let set_flag = prompt_optional_text(session, "Puzzle", "Set flag (optional):", "", 48)?;
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
    mark_dirty(state, "Puzzle added");
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
    mark_dirty(state, "Sign added");
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
    mark_dirty(state, "Chest added");
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
    mark_dirty(state, "Vehicle added");
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
    mark_dirty(state, "Campfire added");
    Ok(())
}

fn add_save_point(state: &mut EditorState) {
    let pos = [state.cursor.0, state.cursor.1];
    if !state.map.save_points.iter().any(|entry| *entry == pos) {
        push_undo(state);
        state.map.save_points.push(pos);
        mark_dirty(state, "Save point added");
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
    mark_dirty(state, "Event added");
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
    mark_dirty(state, "Transition updated");
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
    mark_dirty(state, "Door updated");
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
    mark_dirty(state, "Puzzle updated");
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
    mark_dirty(state, "Sign updated");
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
    mark_dirty(state, "Chest updated");
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
    mark_dirty(state, "Vehicle updated");
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
    mark_dirty(state, "Campfire updated");
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
    mark_dirty(state, "Event updated");
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
    mark_dirty(state, "NPC updated");
    Ok(())
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

fn has_object_at_cursor(state: &EditorState, pos: [i32; 2]) -> bool {
    !object_entries_at_pos(state, pos).is_empty()
}

pub(super) fn object_glyph_at(state: &EditorState, x: i32, y: i32) -> Option<char> {
    let pos = [x, y];
    object_entries_at_pos(state, pos)
        .into_iter()
        .find_map(|entry| entry.glyph)
}

pub(super) fn objects_at_cursor(state: &EditorState) -> Vec<String> {
    let pos = [state.cursor.0, state.cursor.1];
    object_entries_at_pos(state, pos)
        .into_iter()
        .map(|entry| entry.label)
        .collect()
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
    removed |= retain_by_pos(&mut state.map.save_points, pos, |item, pos| *item == pos);
    removed |= retain_by_pos(&mut state.map.events, pos, |item, pos| {
        item.pos == Some(pos)
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

fn mark_dirty(state: &mut EditorState, status: &str) {
    state.dirty = true;
    state.status = status.to_string();
}
