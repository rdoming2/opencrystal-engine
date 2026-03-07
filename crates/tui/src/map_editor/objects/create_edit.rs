use std::io;

use crate::dialog::{prompt_choice, prompt_text};
use crate::input::InputBindings;
use crate::session::TuiSession;

use super::super::prompts::{
    choose_from_list_or_custom, choose_optional_from_list_or_custom, flags_to_string, prompt_flags,
    prompt_optional_glyph_string, prompt_optional_text, prompt_pos, prompt_yes_no,
};
use super::super::state::{push_undo, EditorState};
use super::super::{
    EncounterZone, MapCampfire, MapChest, MapChestLoot, MapCurrencyStack, MapDoor, MapEvent,
    MapNpc, MapPuzzle, MapSign, MapTransition, MapVehicle,
};
use super::geometry::{normalize_zone_rect, prompt_rect, rect_from_selection};
use super::loot::edit_chest_loot;
use super::mark_dirty;

pub(super) fn add_transition(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    map_ids: &[String],
    currency_ids: &[String],
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
    let cost = prompt_cost(session, bindings, "Transition", currency_ids, None)?;
    let Some(cost) = cost else {
        return Ok(());
    };
    let glyph = prompt_optional_glyph_string(session, "Transition", "Glyph (optional):", "")?;
    let palette = prompt_optional_text(session, "Transition", "Palette (optional):", "", 24)?;
    push_undo(state);
    state.map.transitions.push(MapTransition {
        id,
        pos: [state.cursor.0, state.cursor.1],
        target_map,
        target_pos,
        label,
        requires_flag,
        cost,
        return_to_last,
        glyph,
        palette,
    });
    mark_dirty(state, "Transition added");
    Ok(())
}

pub(super) fn add_door(
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
    let glyph = prompt_optional_glyph_string(session, "Door", "Glyph (optional):", "")?;
    let palette = prompt_optional_text(session, "Door", "Palette (optional):", "", 24)?;
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
        glyph,
        palette,
    });
    mark_dirty(state, "Door added");
    Ok(())
}

pub(super) fn add_puzzle(session: &mut TuiSession, state: &mut EditorState) -> io::Result<()> {
    let id = prompt_text(session, "Puzzle", "Id:", "puzzle", 32)?;
    let Some(id) = id else {
        return Ok(());
    };
    let requires_flags = prompt_flags(session, "Puzzle", "Requires flags (comma):", "")?;
    let text = prompt_optional_text(session, "Puzzle", "Text (optional):", "", 72)?;
    let event = prompt_optional_text(session, "Puzzle", "Event (optional):", "", 32)?;
    let set_flag = prompt_optional_text(session, "Puzzle", "Set flag (optional):", "", 48)?;
    let glyph = prompt_optional_glyph_string(session, "Puzzle", "Glyph (optional):", "")?;
    let palette = prompt_optional_text(session, "Puzzle", "Palette (optional):", "", 24)?;
    push_undo(state);
    state.map.puzzles.push(MapPuzzle {
        id,
        pos: [state.cursor.0, state.cursor.1],
        requires_flags,
        text,
        event,
        set_flag,
        glyph,
        palette,
    });
    mark_dirty(state, "Puzzle added");
    Ok(())
}

pub(super) fn add_sign(session: &mut TuiSession, state: &mut EditorState) -> io::Result<()> {
    let id = prompt_text(session, "Sign", "Id:", "sign", 32)?;
    let Some(id) = id else {
        return Ok(());
    };
    let text = prompt_text(session, "Sign", "Text:", "", 72)?;
    let Some(text) = text else {
        return Ok(());
    };
    let glyph = prompt_optional_glyph_string(session, "Sign", "Glyph (optional):", "")?;
    let palette = prompt_optional_text(session, "Sign", "Palette (optional):", "", 24)?;
    push_undo(state);
    state.map.signs.push(MapSign {
        id,
        pos: [state.cursor.0, state.cursor.1],
        glyph,
        palette,
        text,
    });
    mark_dirty(state, "Sign added");
    Ok(())
}

pub(super) fn add_chest(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    item_ids: &[String],
    equipment_ids: &[String],
    currency_ids: &[String],
) -> io::Result<()> {
    let id = prompt_text(session, "Chest", "Id:", "chest", 32)?;
    let Some(id) = id else {
        return Ok(());
    };
    let opened_flag = prompt_text(session, "Chest", "Opened flag:", "chest.opened", 64)?;
    let Some(opened_flag) = opened_flag else {
        return Ok(());
    };
    let glyph_closed =
        prompt_optional_glyph_string(session, "Chest", "Closed glyph (optional):", "")?;
    let glyph_open = prompt_optional_glyph_string(session, "Chest", "Open glyph (optional):", "")?;
    let palette = prompt_optional_text(session, "Chest", "Palette (optional):", "", 24)?;
    let mut loot = MapChestLoot {
        items: Vec::new(),
        equipment: Vec::new(),
        currency: Vec::new(),
    };
    edit_chest_loot(
        session,
        bindings,
        &mut loot,
        item_ids,
        equipment_ids,
        currency_ids,
    )?;
    push_undo(state);
    state.map.chests.push(MapChest {
        id,
        pos: [state.cursor.0, state.cursor.1],
        glyph_closed,
        glyph_open,
        palette,
        opened_flag,
        loot,
    });
    mark_dirty(state, "Chest added");
    Ok(())
}

pub(super) fn add_vehicle(
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

pub(super) fn add_campfire(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    campfire_ids: &[String],
) -> io::Result<()> {
    let id = prompt_text(session, "Campfire", "Id:", "campfire", 32)?;
    let Some(id) = id else {
        return Ok(());
    };
    let campfire_id = choose_from_list_or_custom(
        session,
        bindings,
        "Campfire",
        "Campfire set id:",
        campfire_ids,
        "campfire",
    )?;
    let Some(campfire_id) = campfire_id else {
        return Ok(());
    };
    let requires_flags = prompt_flags(session, "Campfire", "Requires flags (comma):", "")?;
    let glyph = prompt_optional_glyph_string(session, "Campfire", "Glyph (optional):", "")?;
    let palette = prompt_optional_text(session, "Campfire", "Palette (optional):", "", 24)?;
    push_undo(state);
    state.map.campfires.push(MapCampfire {
        id,
        pos: [state.cursor.0, state.cursor.1],
        campfire_id,
        requires_flags,
        glyph,
        palette,
    });
    mark_dirty(state, "Campfire added");
    Ok(())
}

pub(super) fn add_save_point(state: &mut EditorState) {
    let pos = [state.cursor.0, state.cursor.1];
    if !state.map.save_points.iter().any(|entry| *entry == pos) {
        push_undo(state);
        state.map.save_points.push(pos);
        mark_dirty(state, "Save point added");
    } else {
        state.status = "Save point already present".to_string();
    }
}

pub(super) fn add_encounter_zone(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    encounter_table_ids: &[String],
) -> io::Result<()> {
    let id = prompt_text(session, "Encounter Zone", "Id:", "zone", 32)?;
    let Some(id) = id else {
        return Ok(());
    };
    let table = choose_from_list_or_custom(
        session,
        bindings,
        "Encounter Zone",
        "Table id:",
        encounter_table_ids,
        "",
    )?;
    let Some(table) = table else {
        return Ok(());
    };
    let rect_default = rect_from_selection(state).unwrap_or([state.cursor.0, state.cursor.1, 1, 1]);
    let rect = prompt_rect(session, "Encounter Zone", rect_default)?;
    let Some(rect) = rect else {
        return Ok(());
    };
    push_undo(state);
    state.map.encounters.push(EncounterZone {
        zone_id: id,
        rect: normalize_zone_rect(&state.map, rect),
        table,
    });
    mark_dirty(state, "Encounter zone added");
    Ok(())
}

pub(super) fn add_event(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    event_ids: &[String],
    encounter_zone_ids: &[String],
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
    let location = prompt_event_location(
        session,
        bindings,
        "Event",
        encounter_zone_ids,
        None,
        Some([state.cursor.0, state.cursor.1]),
        [state.cursor.0, state.cursor.1],
    )?;
    let Some((zone, pos)) = location else {
        return Ok(());
    };
    push_undo(state);
    state.map.events.push(MapEvent {
        id,
        trigger,
        script,
        zone,
        pos,
    });
    mark_dirty(state, "Event added");
    Ok(())
}

pub(super) fn add_npc(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    npc_ids: &[String],
) -> io::Result<()> {
    let npc_id = choose_from_list_or_custom(session, bindings, "NPC", "NPC id:", npc_ids, "npc")?;
    let Some(npc_id) = npc_id else {
        return Ok(());
    };
    let script = prompt_optional_text(session, "NPC", "Script (optional):", "", 32)?;
    let requires_flags = prompt_flags(session, "NPC", "Requires flags (comma):", "")?;
    push_undo(state);
    state.map.npcs.push(MapNpc {
        id: npc_id,
        pos: [state.cursor.0, state.cursor.1],
        script,
        requires_flags,
    });
    mark_dirty(state, "NPC added");
    Ok(())
}

pub(super) fn edit_transition(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    map_ids: &[String],
    currency_ids: &[String],
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
    let cost = prompt_cost(
        session,
        bindings,
        "Transition",
        currency_ids,
        entry.cost.as_ref(),
    )?;
    let Some(cost) = cost else {
        return Ok(());
    };
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
        slot.cost = cost;
        slot.glyph = glyph;
        slot.palette = palette;
    }
    mark_dirty(state, "Transition updated");
    Ok(())
}

pub(super) fn edit_door(
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

pub(super) fn edit_puzzle(
    session: &mut TuiSession,
    state: &mut EditorState,
    index: usize,
) -> io::Result<()> {
    let entry = state.map.puzzles.get(index).cloned();
    let Some(entry) = entry else {
        return Ok(());
    };
    let requires_default = flags_to_string(entry.requires_flags.as_ref());
    let requires_flags = prompt_flags(
        session,
        "Puzzle",
        "Requires flags (comma):",
        &requires_default,
    )?;
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

pub(super) fn edit_sign(
    session: &mut TuiSession,
    state: &mut EditorState,
    index: usize,
) -> io::Result<()> {
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

pub(super) fn edit_chest(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    item_ids: &[String],
    equipment_ids: &[String],
    currency_ids: &[String],
    index: usize,
) -> io::Result<()> {
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
    let mut loot = entry.loot.clone();
    edit_chest_loot(
        session,
        bindings,
        &mut loot,
        item_ids,
        equipment_ids,
        currency_ids,
    )?;
    push_undo(state);
    if let Some(slot) = state.map.chests.get_mut(index) {
        slot.opened_flag = opened_flag;
        slot.glyph_closed = glyph_closed;
        slot.glyph_open = glyph_open;
        slot.palette = palette;
        slot.loot = loot;
    }
    mark_dirty(state, "Chest updated");
    Ok(())
}

pub(super) fn edit_vehicle(
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

pub(super) fn edit_campfire(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    campfire_ids: &[String],
    index: usize,
) -> io::Result<()> {
    let entry = state.map.campfires.get(index).cloned();
    let Some(entry) = entry else {
        return Ok(());
    };
    let campfire_id = choose_from_list_or_custom(
        session,
        bindings,
        "Campfire",
        "Campfire set id:",
        campfire_ids,
        &entry.campfire_id,
    )?;
    let Some(campfire_id) = campfire_id else {
        return Ok(());
    };
    let requires_default = flags_to_string(entry.requires_flags.as_ref());
    let requires_flags = prompt_flags(
        session,
        "Campfire",
        "Requires flags (comma):",
        &requires_default,
    )?;
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

pub(super) fn edit_event(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    event_ids: &[String],
    encounter_zone_ids: &[String],
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
    let location = prompt_event_location(
        session,
        bindings,
        "Event",
        encounter_zone_ids,
        entry.zone.clone(),
        entry.pos,
        [state.cursor.0, state.cursor.1],
    )?;
    let Some((zone, pos)) = location else {
        return Ok(());
    };
    push_undo(state);
    if let Some(slot) = state.map.events.get_mut(index) {
        slot.trigger = trigger;
        slot.script = script;
        slot.zone = zone;
        slot.pos = pos;
    }
    mark_dirty(state, "Event updated");
    Ok(())
}

pub(super) fn edit_npc(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    npc_ids: &[String],
    index: usize,
) -> io::Result<()> {
    let entry = state.map.npcs.get(index).cloned();
    let Some(entry) = entry else {
        return Ok(());
    };
    let npc_id =
        choose_from_list_or_custom(session, bindings, "NPC", "NPC id:", npc_ids, &entry.id)?;
    let Some(npc_id) = npc_id else {
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
        slot.id = npc_id;
        slot.script = script;
        slot.requires_flags = requires_flags;
    }
    mark_dirty(state, "NPC updated");
    Ok(())
}

pub(super) fn edit_encounter_zone(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    encounter_table_ids: &[String],
    index: usize,
) -> io::Result<()> {
    let entry = state.map.encounters.get(index).cloned();
    let Some(entry) = entry else {
        return Ok(());
    };
    let zone_id = prompt_text(session, "Encounter Zone", "Id:", &entry.zone_id, 32)?;
    let Some(zone_id) = zone_id else {
        return Ok(());
    };
    let table = choose_from_list_or_custom(
        session,
        bindings,
        "Encounter Zone",
        "Table id:",
        encounter_table_ids,
        &entry.table,
    )?;
    let Some(table) = table else {
        return Ok(());
    };
    let rect_default = rect_from_selection(state).unwrap_or(entry.rect);
    let rect = prompt_rect(session, "Encounter Zone", rect_default)?;
    let Some(rect) = rect else {
        return Ok(());
    };
    let rect = normalize_zone_rect(&state.map, rect);
    push_undo(state);
    if let Some(slot) = state.map.encounters.get_mut(index) {
        slot.zone_id = zone_id;
        slot.table = table;
        slot.rect = rect;
    }
    mark_dirty(state, "Encounter zone updated");
    Ok(())
}

fn prompt_cost(
    session: &mut TuiSession,
    bindings: &InputBindings,
    title: &str,
    currency_ids: &[String],
    default_cost: Option<&MapCurrencyStack>,
) -> io::Result<Option<Option<MapCurrencyStack>>> {
    let has_cost = prompt_yes_no(session, title, "Has cost?", default_cost.is_some())?;
    if !has_cost {
        return Ok(Some(None));
    }
    let default_id = default_cost.map(|cost| cost.id.as_str()).unwrap_or("");
    let currency_id = choose_from_list_or_custom(
        session,
        bindings,
        title,
        "Currency id:",
        currency_ids,
        default_id,
    )?;
    let Some(currency_id) = currency_id else {
        return Ok(None);
    };
    let default_amount = default_cost
        .map(|cost| cost.amount.to_string())
        .unwrap_or_else(|| "1".to_string());
    let amount_text = prompt_text(session, title, "Amount:", &default_amount, 8)?;
    let Some(amount_text) = amount_text else {
        return Ok(None);
    };
    let amount: i32 = amount_text.trim().parse().unwrap_or(0);
    if amount <= 0 {
        return Ok(Some(None));
    }
    Ok(Some(Some(MapCurrencyStack {
        id: currency_id,
        amount,
    })))
}

fn prompt_event_location(
    session: &mut TuiSession,
    bindings: &InputBindings,
    title: &str,
    encounter_zone_ids: &[String],
    default_zone: Option<String>,
    default_pos: Option<[i32; 2]>,
    cursor: [i32; 2],
) -> io::Result<Option<(Option<String>, Option<[i32; 2]>)>> {
    let options = vec![
        "Position (cursor)".to_string(),
        "Zone".to_string(),
        "Clear".to_string(),
    ];
    let default_index = if default_zone.is_some() {
        1
    } else if default_pos.is_some() {
        0
    } else {
        2
    };
    let selection = prompt_choice(
        session,
        bindings,
        title,
        "Location:",
        &options,
        default_index,
    )?;
    let Some(selection) = selection else {
        return Ok(None);
    };
    match selection {
        0 => Ok(Some((None, Some(cursor)))),
        1 => {
            let zone_default = default_zone.as_deref().unwrap_or("");
            let zone = choose_from_list_or_custom(
                session,
                bindings,
                title,
                "Zone id:",
                encounter_zone_ids,
                zone_default,
            )?;
            let Some(zone) = zone else {
                return Ok(None);
            };
            Ok(Some((Some(zone), None)))
        }
        _ => Ok(Some((None, None))),
    }
}
