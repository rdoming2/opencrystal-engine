use std::io;

use crossterm::event::{KeyCode, KeyEvent};

use crate::dialog::{prompt_choice, prompt_text};
use crate::input::InputBindings;
use crate::session::TuiSession;

use super::objects::{
    delete_object_at_cursor, edit_object_at_cursor, edit_objects, toggle_move_object,
};
use super::prompts::{prompt_glyph, prompt_yes_no};
use super::resize::resize_map;
use super::state::{
    cycle_active_tile, move_cursor, paint_active_tile, paste_selection, push_undo, redo,
    replace_tile_glyph, toggle_visual, undo, yank_selection, EditorState,
};
use super::MapEditorOutcome;

#[derive(Clone, Debug)]
enum ExitAction {
    Save,
    Discard,
    Cancel,
}

pub(super) enum EditorAction {
    Continue,
    Exit(MapEditorOutcome),
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

pub(super) fn handle_key(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    map_ids: &[String],
    event_ids: &[String],
    vehicle_ids: &[String],
    npc_ids: &[String],
    key: KeyEvent,
) -> io::Result<EditorAction> {
    if let Some(action) = handle_immediate_key(state, key.code) {
        return Ok(action);
    }
    if let Some(action) = handle_prompted_key(
        session,
        bindings,
        state,
        map_ids,
        event_ids,
        vehicle_ids,
        npc_ids,
        key.code,
    )? {
        return Ok(action);
    }
    Ok(EditorAction::Continue)
}

fn handle_immediate_key(state: &mut EditorState, code: KeyCode) -> Option<EditorAction> {
    match code {
        KeyCode::Char('u') => {
            undo(state);
            Some(EditorAction::Continue)
        }
        KeyCode::Char('U') => {
            redo(state);
            Some(EditorAction::Continue)
        }
        KeyCode::Up | KeyCode::Char('k') => {
            move_cursor(state, 0, -1);
            Some(EditorAction::Continue)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            move_cursor(state, 0, 1);
            Some(EditorAction::Continue)
        }
        KeyCode::Left | KeyCode::Char('h') => {
            move_cursor(state, -1, 0);
            Some(EditorAction::Continue)
        }
        KeyCode::Right | KeyCode::Char('l') => {
            move_cursor(state, 1, 0);
            Some(EditorAction::Continue)
        }
        KeyCode::Char('v') | KeyCode::Char('V') => {
            toggle_visual(state);
            Some(EditorAction::Continue)
        }
        KeyCode::Char('y') => {
            yank_selection(state);
            Some(EditorAction::Continue)
        }
        KeyCode::Char('p') => {
            paste_selection(state);
            Some(EditorAction::Continue)
        }
        KeyCode::Char('R') | KeyCode::Char('r') => {
            paint_active_tile(state);
            Some(EditorAction::Continue)
        }
        KeyCode::Char('[') => {
            cycle_active_tile(state, -1);
            Some(EditorAction::Continue)
        }
        KeyCode::Char(']') => {
            cycle_active_tile(state, 1);
            Some(EditorAction::Continue)
        }
        KeyCode::Char('x') => {
            delete_object_at_cursor(state);
            Some(EditorAction::Continue)
        }
        KeyCode::Char('s') => {
            state.dirty = false;
            Some(EditorAction::Exit(MapEditorOutcome::Saved(
                state.map.clone(),
            )))
        }
        _ => None,
    }
}

fn handle_prompted_key(
    session: &mut TuiSession,
    bindings: &InputBindings,
    state: &mut EditorState,
    map_ids: &[String],
    event_ids: &[String],
    vehicle_ids: &[String],
    npc_ids: &[String],
    code: KeyCode,
) -> io::Result<Option<EditorAction>> {
    match code {
        KeyCode::Char('t') => {
            choose_active_tile(session, bindings, state)?;
            Ok(Some(EditorAction::Continue))
        }
        KeyCode::Char('L') => {
            edit_legend(session, bindings, state)?;
            Ok(Some(EditorAction::Continue))
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
            Ok(Some(EditorAction::Continue))
        }
        KeyCode::Char('m') => {
            toggle_move_object(session, bindings, state)?;
            Ok(Some(EditorAction::Continue))
        }
        KeyCode::Char('e') => {
            edit_object_at_cursor(session, bindings, state, map_ids, event_ids, vehicle_ids)?;
            Ok(Some(EditorAction::Continue))
        }
        KeyCode::Char('=') => {
            resize_map(session, bindings, state)?;
            Ok(Some(EditorAction::Continue))
        }
        KeyCode::Char('q') => {
            let action = confirm_exit(session, bindings, state)?;
            Ok(Some(match action {
                ExitAction::Cancel => EditorAction::Continue,
                ExitAction::Discard => EditorAction::Exit(MapEditorOutcome::Cancelled),
                ExitAction::Save => {
                    state.dirty = false;
                    EditorAction::Exit(MapEditorOutcome::Saved(state.map.clone()))
                }
            }))
        }
        _ => Ok(None),
    }
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
    state.map.legend.push(super::LegendEntry {
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
    if glyph != entry.glyph {
        replace_tile_glyph(&mut state.map, entry.glyph, glyph);
    }
    state.map.legend[choice] = super::LegendEntry {
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
