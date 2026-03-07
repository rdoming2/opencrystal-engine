use std::io;

use crate::dialog::prompt_choice;
use crate::input::InputBindings;
use crate::session::TuiSession;

use super::super::state::{push_undo, CursorObject, EditorState, MovingObject};
use super::catalog::cursor_objects;
use super::create_edit::{
    add_campfire, add_chest, add_door, add_encounter_zone, add_event, add_npc, add_puzzle,
    add_save_point, add_sign, add_transition, add_vehicle, edit_campfire, edit_chest, edit_door,
    edit_encounter_zone, edit_event, edit_npc, edit_puzzle, edit_sign, edit_transition,
    edit_vehicle,
};
use super::{mark_dirty, moving_label};

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
            mark_dirty(state, "Save point removed");
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
