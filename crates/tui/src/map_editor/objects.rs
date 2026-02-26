use std::io;

use crate::dialog::{prompt_choice, prompt_text};
use crate::input::InputBindings;
use crate::session::TuiSession;

use super::prompts::{
    choose_from_list_or_custom, choose_optional_from_list_or_custom, flags_to_string, prompt_flags,
    prompt_optional_glyph_string, prompt_optional_text, prompt_pos, prompt_yes_no,
};
use super::state::{
    push_undo, selection_rect, CursorObject, EditorState, MovingObject, ObjectGlyphMode,
};
use super::{
    EncounterZone, InventoryStack, MapCampfire, MapChest, MapChestLoot, MapCurrencyStack, MapDoor,
    MapEvent, MapNpc, MapPuzzle, MapSign, MapTransition, MapVehicle,
};

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
    let options = vec![
        "Add transition".to_string(),
        "Add door".to_string(),
        "Add puzzle".to_string(),
        "Add sign".to_string(),
        "Add chest".to_string(),
        "Add vehicle".to_string(),
        "Add campfire".to_string(),
        "Add save point".to_string(),
        "Add encounter zone".to_string(),
        "Add event".to_string(),
        "Add npc".to_string(),
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
        0 => add_transition(session, bindings, state, map_ids, currency_ids)?,
        1 => add_door(session, bindings, state, map_ids)?,
        2 => add_puzzle(session, state)?,
        3 => add_sign(session, state)?,
        4 => add_chest(
            session,
            bindings,
            state,
            item_ids,
            equipment_ids,
            currency_ids,
        )?,
        5 => add_vehicle(session, bindings, state, vehicle_ids)?,
        6 => add_campfire(session, bindings, state, campfire_ids)?,
        7 => add_save_point(state),
        8 => add_encounter_zone(session, bindings, state, encounter_table_ids)?,
        9 => add_event(session, bindings, state, event_ids, encounter_zone_ids)?,
        10 => add_npc(session, bindings, state, npc_ids)?,
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
    npc_ids: &[String],
    item_ids: &[String],
    equipment_ids: &[String],
    currency_ids: &[String],
    campfire_ids: &[String],
    encounter_zone_ids: &[String],
    encounter_table_ids: &[String],
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
            edit_transition(session, bindings, state, map_ids, currency_ids, index)?;
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
            edit_chest(
                session,
                bindings,
                state,
                item_ids,
                equipment_ids,
                currency_ids,
                index,
            )?;
        }
        CursorObject::Vehicle(index) => {
            edit_vehicle(session, bindings, state, vehicle_ids, index)?;
        }
        CursorObject::Campfire(index) => {
            edit_campfire(session, bindings, state, campfire_ids, index)?;
        }
        CursorObject::Event(index) => {
            edit_event(
                session,
                bindings,
                state,
                event_ids,
                encounter_zone_ids,
                index,
            )?;
        }
        CursorObject::Npc(index) => {
            edit_npc(session, bindings, state, npc_ids, index)?;
        }
        CursorObject::EncounterZone(index) => {
            edit_encounter_zone(session, bindings, state, encounter_table_ids, index)?;
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
    marker_glyph: char,
    configured_glyph: Option<char>,
    palette: Option<String>,
}

pub(super) struct ObjectGlyph {
    pub(super) glyph: char,
    pub(super) palette: Option<String>,
}

fn object_entries_at_pos(state: &EditorState, pos: [i32; 2]) -> Vec<ObjectEntry> {
    let mut entries = Vec::new();
    for (index, item) in state.map.transitions.iter().enumerate() {
        if item.pos == pos {
            let configured_glyph = glyph_from_option(&item.glyph);
            entries.push(ObjectEntry {
                label: format!(
                    "transition:{} -> {}@{},{}",
                    item.id, item.target_map, item.target_pos[0], item.target_pos[1]
                ),
                cursor: CursorObject::Transition(index),
                marker_glyph: 'T',
                configured_glyph,
                palette: configured_glyph.and(item.palette.clone()),
            });
        }
    }
    for (index, item) in state.map.doors.iter().enumerate() {
        if item.pos == pos {
            let configured_glyph = glyph_from_option(&item.glyph);
            let label = if let Some(target_map) = item.target_map.as_deref() {
                if let Some(target_pos) = item.target_pos {
                    format!(
                        "door:{} -> {}@{},{}",
                        item.id, target_map, target_pos[0], target_pos[1]
                    )
                } else {
                    format!("door:{} -> {}", item.id, target_map)
                }
            } else {
                format!("door:{}", item.id)
            };
            entries.push(ObjectEntry {
                label,
                cursor: CursorObject::Door(index),
                marker_glyph: '+',
                configured_glyph,
                palette: configured_glyph.and(item.palette.clone()),
            });
        }
    }
    for (index, item) in state.map.puzzles.iter().enumerate() {
        if item.pos == pos {
            let configured_glyph = glyph_from_option(&item.glyph);
            let label = if let Some(event) = item.event.as_deref() {
                format!("puzzle:{} event:{}", item.id, event)
            } else if let Some(set_flag) = item.set_flag.as_deref() {
                format!("puzzle:{} set:{}", item.id, set_flag)
            } else {
                format!("puzzle:{}", item.id)
            };
            entries.push(ObjectEntry {
                label,
                cursor: CursorObject::Puzzle(index),
                marker_glyph: '?',
                configured_glyph,
                palette: configured_glyph.and(item.palette.clone()),
            });
        }
    }
    for (index, item) in state.map.signs.iter().enumerate() {
        if item.pos == pos {
            let configured_glyph = glyph_from_option(&item.glyph);
            let preview = truncate_label(&item.text, 24);
            entries.push(ObjectEntry {
                label: format!("sign:{} \"{}\"", item.id, preview),
                cursor: CursorObject::Sign(index),
                marker_glyph: '!',
                configured_glyph,
                palette: configured_glyph.and(item.palette.clone()),
            });
        }
    }
    for (index, item) in state.map.chests.iter().enumerate() {
        if item.pos == pos {
            let configured_glyph = glyph_from_option(&item.glyph_closed)
                .or_else(|| glyph_from_option(&item.glyph_open));
            entries.push(ObjectEntry {
                label: format!("chest:{} flag:{}", item.id, item.opened_flag),
                cursor: CursorObject::Chest(index),
                marker_glyph: 'C',
                configured_glyph,
                palette: configured_glyph.and(item.palette.clone()),
            });
        }
    }
    for (index, item) in state.map.vehicles.iter().enumerate() {
        if item.pos == pos {
            entries.push(ObjectEntry {
                label: format!("vehicle:{}", item.vehicle_id),
                cursor: CursorObject::Vehicle(index),
                marker_glyph: 'V',
                configured_glyph: None,
                palette: None,
            });
        }
    }
    for (index, item) in state.map.campfires.iter().enumerate() {
        if item.pos == pos {
            let configured_glyph = glyph_from_option(&item.glyph);
            entries.push(ObjectEntry {
                label: format!("campfire:{} set:{}", item.id, item.campfire_id),
                cursor: CursorObject::Campfire(index),
                marker_glyph: 'F',
                configured_glyph,
                palette: configured_glyph.and(item.palette.clone()),
            });
        }
    }
    for (index, item) in state.map.events.iter().enumerate() {
        if item.pos == Some(pos) {
            let mut label = format!("event:{} {} script:{}", item.id, item.trigger, item.script);
            if let Some(zone) = item.zone.as_deref() {
                label.push_str(&format!(" zone:{}", zone));
            }
            entries.push(ObjectEntry {
                label,
                cursor: CursorObject::Event(index),
                marker_glyph: 'E',
                configured_glyph: None,
                palette: None,
            });
        }
    }
    for (index, item) in state.map.npcs.iter().enumerate() {
        if item.pos == pos {
            let label = if let Some(script) = item.script.as_deref() {
                format!("npc:{} script:{}", item.id, script)
            } else {
                format!("npc:{}", item.id)
            };
            entries.push(ObjectEntry {
                label,
                cursor: CursorObject::Npc(index),
                marker_glyph: 'N',
                configured_glyph: None,
                palette: None,
            });
        }
    }
    for (index, item) in state.map.encounters.iter().enumerate() {
        if pos_in_rect(pos, item.rect) {
            entries.push(ObjectEntry {
                label: format!(
                    "encounter_zone:{} table:{} rect:{},{},{}x{}",
                    item.zone_id,
                    item.table,
                    item.rect[0],
                    item.rect[1],
                    item.rect[2],
                    item.rect[3]
                ),
                cursor: CursorObject::EncounterZone(index),
                marker_glyph: 'Z',
                configured_glyph: None,
                palette: None,
            });
        }
    }
    if state.map.save_points.iter().any(|entry| *entry == pos) {
        entries.push(ObjectEntry {
            label: "save_point".to_string(),
            cursor: CursorObject::SavePoint,
            marker_glyph: 'S',
            configured_glyph: None,
            palette: None,
        });
    }
    entries
}

fn glyph_from_option(value: &Option<String>) -> Option<char> {
    value.as_ref().and_then(|glyph| glyph.chars().next())
}

fn truncate_label(value: &str, max: usize) -> String {
    let mut chars = value.chars();
    let collected: String = chars.by_ref().take(max).collect();
    if chars.next().is_some() {
        format!("{}...", collected)
    } else {
        collected
    }
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

fn add_transition(
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

fn add_puzzle(session: &mut TuiSession, state: &mut EditorState) -> io::Result<()> {
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

fn add_sign(session: &mut TuiSession, state: &mut EditorState) -> io::Result<()> {
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

fn add_chest(
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

fn add_campfire(
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

fn add_encounter_zone(
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

fn add_event(
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

fn add_npc(
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

fn edit_transition(
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

fn edit_chest(
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

fn edit_event(
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

fn edit_npc(
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

fn edit_encounter_zone(
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

fn edit_chest_loot(
    session: &mut TuiSession,
    bindings: &InputBindings,
    loot: &mut MapChestLoot,
    item_ids: &[String],
    equipment_ids: &[String],
    currency_ids: &[String],
) -> io::Result<()> {
    loop {
        let options = vec![
            "Items".to_string(),
            "Equipment".to_string(),
            "Currency".to_string(),
            "Done".to_string(),
        ];
        let selection = prompt_choice(
            session,
            bindings,
            "Chest Loot",
            "Select section:",
            &options,
            0,
        )?;
        let Some(selection) = selection else {
            break;
        };
        match selection {
            0 => edit_loot_items(session, bindings, &mut loot.items, item_ids)?,
            1 => edit_loot_equipment(session, bindings, &mut loot.equipment, equipment_ids)?,
            2 => edit_loot_currency(session, bindings, &mut loot.currency, currency_ids)?,
            _ => break,
        }
    }
    Ok(())
}

fn edit_loot_items(
    session: &mut TuiSession,
    bindings: &InputBindings,
    items: &mut Vec<InventoryStack>,
    item_ids: &[String],
) -> io::Result<()> {
    loop {
        let options = vec![
            "Add or update".to_string(),
            "Remove".to_string(),
            "Back".to_string(),
        ];
        let selection = prompt_choice(
            session,
            bindings,
            "Loot Items",
            "Select action:",
            &options,
            0,
        )?;
        let Some(selection) = selection else {
            break;
        };
        match selection {
            0 => {
                let item_id = choose_from_list_or_custom(
                    session,
                    bindings,
                    "Loot Items",
                    "Item id:",
                    item_ids,
                    "",
                )?;
                let Some(item_id) = item_id else {
                    continue;
                };
                let default_qty = items
                    .iter()
                    .find(|item| item.id == item_id)
                    .map(|item| item.qty.to_string())
                    .unwrap_or_else(|| "1".to_string());
                let qty_text = prompt_text(session, "Loot Items", "Qty:", &default_qty, 8)?;
                let Some(qty_text) = qty_text else {
                    continue;
                };
                let qty: i32 = qty_text.trim().parse().unwrap_or(0);
                if qty <= 0 {
                    continue;
                }
                upsert_inventory_stack(items, item_id, qty);
            }
            1 => {
                if items.is_empty() {
                    continue;
                }
                let options = items
                    .iter()
                    .map(|item| format!("{} x{}", item.id, item.qty))
                    .collect::<Vec<_>>();
                let selection = prompt_choice(
                    session,
                    bindings,
                    "Loot Items",
                    "Remove which?",
                    &options,
                    0,
                )?;
                if let Some(index) = selection {
                    items.remove(index);
                }
            }
            _ => break,
        }
    }
    Ok(())
}

fn edit_loot_equipment(
    session: &mut TuiSession,
    bindings: &InputBindings,
    equipment: &mut Vec<InventoryStack>,
    equipment_ids: &[String],
) -> io::Result<()> {
    loop {
        let options = vec![
            "Add or update".to_string(),
            "Remove".to_string(),
            "Back".to_string(),
        ];
        let selection = prompt_choice(
            session,
            bindings,
            "Loot Equipment",
            "Select action:",
            &options,
            0,
        )?;
        let Some(selection) = selection else {
            break;
        };
        match selection {
            0 => {
                let equipment_id = choose_from_list_or_custom(
                    session,
                    bindings,
                    "Loot Equipment",
                    "Equipment id:",
                    equipment_ids,
                    "",
                )?;
                let Some(equipment_id) = equipment_id else {
                    continue;
                };
                let default_qty = equipment
                    .iter()
                    .find(|item| item.id == equipment_id)
                    .map(|item| item.qty.to_string())
                    .unwrap_or_else(|| "1".to_string());
                let qty_text = prompt_text(session, "Loot Equipment", "Qty:", &default_qty, 8)?;
                let Some(qty_text) = qty_text else {
                    continue;
                };
                let qty: i32 = qty_text.trim().parse().unwrap_or(0);
                if qty <= 0 {
                    continue;
                }
                upsert_inventory_stack(equipment, equipment_id, qty);
            }
            1 => {
                if equipment.is_empty() {
                    continue;
                }
                let options = equipment
                    .iter()
                    .map(|item| format!("{} x{}", item.id, item.qty))
                    .collect::<Vec<_>>();
                let selection = prompt_choice(
                    session,
                    bindings,
                    "Loot Equipment",
                    "Remove which?",
                    &options,
                    0,
                )?;
                if let Some(index) = selection {
                    equipment.remove(index);
                }
            }
            _ => break,
        }
    }
    Ok(())
}

fn edit_loot_currency(
    session: &mut TuiSession,
    bindings: &InputBindings,
    currency: &mut Vec<MapCurrencyStack>,
    currency_ids: &[String],
) -> io::Result<()> {
    loop {
        let options = vec![
            "Add or update".to_string(),
            "Remove".to_string(),
            "Back".to_string(),
        ];
        let selection = prompt_choice(
            session,
            bindings,
            "Loot Currency",
            "Select action:",
            &options,
            0,
        )?;
        let Some(selection) = selection else {
            break;
        };
        match selection {
            0 => {
                let currency_id = choose_from_list_or_custom(
                    session,
                    bindings,
                    "Loot Currency",
                    "Currency id:",
                    currency_ids,
                    "",
                )?;
                let Some(currency_id) = currency_id else {
                    continue;
                };
                let default_amount = currency
                    .iter()
                    .find(|item| item.id == currency_id)
                    .map(|item| item.amount.to_string())
                    .unwrap_or_else(|| "1".to_string());
                let amount_text =
                    prompt_text(session, "Loot Currency", "Amount:", &default_amount, 8)?;
                let Some(amount_text) = amount_text else {
                    continue;
                };
                let amount: i32 = amount_text.trim().parse().unwrap_or(0);
                if amount <= 0 {
                    continue;
                }
                upsert_currency_stack(currency, currency_id, amount);
            }
            1 => {
                if currency.is_empty() {
                    continue;
                }
                let options = currency
                    .iter()
                    .map(|item| format!("{} x{}", item.id, item.amount))
                    .collect::<Vec<_>>();
                let selection = prompt_choice(
                    session,
                    bindings,
                    "Loot Currency",
                    "Remove which?",
                    &options,
                    0,
                )?;
                if let Some(index) = selection {
                    currency.remove(index);
                }
            }
            _ => break,
        }
    }
    Ok(())
}

fn upsert_inventory_stack(items: &mut Vec<InventoryStack>, id: String, qty: i32) {
    if let Some(item) = items.iter_mut().find(|item| item.id == id) {
        item.qty = qty;
    } else {
        items.push(InventoryStack { id, qty });
    }
}

fn upsert_currency_stack(currency: &mut Vec<MapCurrencyStack>, id: String, amount: i32) {
    if let Some(item) = currency.iter_mut().find(|item| item.id == id) {
        item.amount = amount;
    } else {
        currency.push(MapCurrencyStack { id, amount });
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

fn has_object_at_cursor(state: &EditorState, pos: [i32; 2]) -> bool {
    !object_entries_at_pos(state, pos).is_empty()
}

pub(super) fn object_glyph_at(state: &EditorState, x: i32, y: i32) -> Option<ObjectGlyph> {
    let pos = [x, y];
    let use_configured = matches!(state.object_glyphs, ObjectGlyphMode::Configured);
    let entry = object_entries_at_pos(state, pos)
        .into_iter()
        .find(|entry| !matches!(entry.cursor, CursorObject::EncounterZone(_)))?;
    Some({
        let (glyph, palette) = if use_configured {
            let glyph = entry.configured_glyph.unwrap_or(entry.marker_glyph);
            let palette = entry.configured_glyph.and_then(|_| entry.palette.clone());
            (glyph, palette)
        } else {
            (entry.marker_glyph, None)
        };
        ObjectGlyph { glyph, palette }
    })
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
    removed |= retain_by_pos(&mut state.map.npcs, pos, |item, pos| item.pos == pos);
    removed |= retain_by_pos(&mut state.map.save_points, pos, |item, pos| *item == pos);
    removed |= retain_by_pos(&mut state.map.events, pos, |item, pos| {
        item.pos == Some(pos)
    });
    removed |= retain_by_pos(&mut state.map.encounters, pos, |item, pos| {
        pos_in_rect(pos, item.rect)
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

fn rect_from_selection(state: &EditorState) -> Option<[i32; 4]> {
    let (min_x, min_y, max_x, max_y) = selection_rect(state)?;
    Some([min_x, min_y, max_x - min_x + 1, max_y - min_y + 1])
}

fn prompt_rect(
    session: &mut TuiSession,
    title: &str,
    default_rect: [i32; 4],
) -> io::Result<Option<[i32; 4]>> {
    let default = format!(
        "{},{},{},{}",
        default_rect[0], default_rect[1], default_rect[2], default_rect[3]
    );
    let value = prompt_text(session, title, "Rect (x,y,w,h):", &default, 24)?;
    let Some(value) = value else {
        return Ok(None);
    };
    parse_rect(&value)
}

fn parse_rect(value: &str) -> io::Result<Option<[i32; 4]>> {
    let parts = value.split(',').collect::<Vec<_>>();
    if parts.len() != 4 {
        return Ok(None);
    }
    let x: i32 = parts[0].trim().parse().unwrap_or(0);
    let y: i32 = parts[1].trim().parse().unwrap_or(0);
    let w: i32 = parts[2].trim().parse().unwrap_or(1).max(1);
    let h: i32 = parts[3].trim().parse().unwrap_or(1).max(1);
    Ok(Some([x, y, w, h]))
}

fn normalize_zone_rect(map: &super::MapData, rect: [i32; 4]) -> [i32; 4] {
    let max_w = map.width as i32;
    let max_h = map.height as i32;
    if max_w <= 0 || max_h <= 0 {
        return rect;
    }
    let x = rect[0].max(0).min(max_w - 1);
    let y = rect[1].max(0).min(max_h - 1);
    let mut w = rect[2].max(1);
    let mut h = rect[3].max(1);
    w = w.min(max_w - x).max(1);
    h = h.min(max_h - y).max(1);
    [x, y, w, h]
}

fn pos_in_rect(pos: [i32; 2], rect: [i32; 4]) -> bool {
    let x = pos[0];
    let y = pos[1];
    x >= rect[0] && y >= rect[1] && x < rect[0] + rect[2] && y < rect[1] + rect[3]
}

fn mark_dirty(state: &mut EditorState, status: &str) {
    state.dirty = true;
    state.status = status.to_string();
}
