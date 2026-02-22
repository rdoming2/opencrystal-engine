use std::collections::{HashMap, HashSet};

use engine::menu::MenuFocus;
use engine::runtime::{GameRuntime, GameState};
use rand::Rng;
use tui::dialog::ChoiceView;
use tui::input::{Action, InputBindings};
use tui::menu::{MenuPanelView, PanelSpanStyle};
use tui::overworld::{
    draw_overworld, draw_overworld_with_tooltip, show_centered_dialog_on_map, MapView, NpcView,
    TileRender, TransitionView, VehicleView,
};
use tui::session::TuiSession;
use tui::ui::{BattleUiFile, DialogUiFile, MenuUiFile, ProgressUiFile};

use crate::battle::{try_start_random_battle, BattleOutcome, BattleSource, LastBattleContext};
use crate::dialog::run_dialog_on_map;
use crate::events::{run_event_loop, EventLoopOutcome};
use crate::menu::inventory::{panel_line_spans, panel_span};
use crate::menu::run_menu_loop;
use crate::shop::lookup_item_name;
use crate::utils::read_action;

#[derive(Clone, Debug)]
pub enum OverworldOutcome {
    Continue,
    Defeat(LastBattleContext),
    Quit,
    ReturnTitle,
}

pub fn run_overworld_loop(
    session: &mut TuiSession,
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    battle_ui: &BattleUiFile,
    menu_ui: &MenuUiFile,
    progress_ui: &ProgressUiFile,
    bindings: &InputBindings,
    map_id: &str,
    start_pos: (i32, i32),
    save_dir: &std::path::Path,
) -> std::io::Result<OverworldOutcome> {
    let mut current_map_id = map_id.to_string();
    let mut player_pos = start_pos;
    let mut return_positions: HashMap<String, (String, (i32, i32))> = HashMap::new();
    let mut last_map_id = String::new();
    let mut area_name_active = false;
    let mut rng = rand::thread_rng();
    let mut encounter_meter: f32 = 0.0;

    let mut running = true;
    while running {
        let map = match build_map_view(runtime, &current_map_id) {
            Some(map) => map,
            None => {
                println!("Map not found: {}", current_map_id);
                running = false;
                continue;
            }
        };

        mark_map_visited(runtime, &current_map_id);

        if current_map_id != last_map_id {
            area_name_active = !map.hide_name && !map.name.is_empty();
            last_map_id = current_map_id.clone();
            encounter_meter = 0.0;
        }

        if area_name_active {
            draw_overworld_with_tooltip(session, &map, player_pos, dialog_ui, &map.name)?;
        } else {
            draw_overworld(session, &map, player_pos)?;
        }

        let previous_pos = player_pos;
        let mut moved = false;
        let mut transitioned = false;
        if let Some(action) = read_action(bindings) {
            match action {
                Action::MoveUp | Action::MoveDown | Action::MoveLeft | Action::MoveRight => {
                    let (dx, dy) = match action {
                        Action::MoveUp => (0, -1),
                        Action::MoveDown => (0, 1),
                        Action::MoveLeft => (-1, 0),
                        Action::MoveRight => (1, 0),
                        _ => (0, 0),
                    };
                    let speed = movement_speed(runtime);
                    for _ in 0..speed {
                        let raw_next_pos = (player_pos.0 + dx, player_pos.1 + dy);
                        let Some(next_pos) =
                            normalize_map_pos(runtime, &current_map_id, raw_next_pos)
                        else {
                            break;
                        };
                        if runtime.active_vehicle.is_none() {
                            if let Some(door) = door_at(runtime, &current_map_id, next_pos) {
                                if door_locked(runtime, &door) {
                                    break;
                                }
                                if let Some(target_map) = door.target_map.as_ref() {
                                    let target_pos = door
                                        .target_pos
                                        .map(|pos| (pos[0], pos[1]))
                                        .unwrap_or(player_pos);
                                    let (next_map, next_pos) = if door.return_to_last {
                                        return_positions
                                            .get(&current_map_id)
                                            .cloned()
                                            .unwrap_or((target_map.clone(), target_pos))
                                    } else {
                                        (target_map.clone(), target_pos)
                                    };

                                    if runtime.is_overworld_map(&current_map_id)
                                        && !runtime.is_overworld_map(&next_map)
                                    {
                                        runtime.record_last_overworld(&current_map_id, player_pos);
                                    }

                                    if !door.return_to_last
                                        && !is_returning_from_child(
                                            &return_positions,
                                            &current_map_id,
                                            &next_map,
                                        )
                                    {
                                        return_positions.insert(
                                            next_map.clone(),
                                            (current_map_id.clone(), player_pos),
                                        );
                                    }
                                    current_map_id = next_map;
                                    player_pos = next_pos;
                                    runtime.world.map_id = current_map_id.clone();
                                    runtime.world.position = player_pos;
                                    transitioned = true;
                                    break;
                                }
                            }
                        }
                        if !can_move_to(runtime, &current_map_id, next_pos) {
                            break;
                        }
                        player_pos = next_pos;
                        moved = true;
                        area_name_active = false;
                        if let Some(active_vehicle) = runtime.active_vehicle.clone() {
                            update_vehicle_position(
                                runtime,
                                &active_vehicle,
                                &current_map_id,
                                next_pos,
                            );
                        }
                        if runtime.active_vehicle.is_none() {
                            if let Some(transition) =
                                find_transition(runtime, &current_map_id, player_pos)
                            {
                                let (next_map, next_pos) = if transition.return_to_last {
                                    return_positions
                                        .get(&current_map_id)
                                        .cloned()
                                        .map(|(return_map, return_pos)| (return_map, return_pos))
                                        .unwrap_or_else(|| {
                                            (
                                                transition.target_map.clone(),
                                                (
                                                    transition.target_pos[0],
                                                    transition.target_pos[1],
                                                ),
                                            )
                                        })
                                } else {
                                    (
                                        transition.target_map.clone(),
                                        (transition.target_pos[0], transition.target_pos[1]),
                                    )
                                };

                                if runtime.is_overworld_map(&current_map_id)
                                    && !runtime.is_overworld_map(&next_map)
                                {
                                    runtime.record_last_overworld(&current_map_id, player_pos);
                                }

                                if !transition.return_to_last
                                    && !is_returning_from_child(
                                        &return_positions,
                                        &current_map_id,
                                        &next_map,
                                    )
                                {
                                    return_positions.insert(
                                        next_map.clone(),
                                        (current_map_id.clone(), player_pos),
                                    );
                                }
                                current_map_id = next_map;
                                player_pos = next_pos;
                                runtime.world.map_id = current_map_id.clone();
                                runtime.world.position = player_pos;
                                transitioned = true;
                                break;
                            }
                        }
                    }
                }
                Action::Confirm => {
                    if runtime.active_vehicle.is_some() {
                        if let Some(new_pos) =
                            find_disembark_pos(runtime, &current_map_id, player_pos)
                        {
                            runtime.active_vehicle = None;
                            runtime.vehicle_slow_mode = false;
                            player_pos = new_pos;
                            runtime.world.position = player_pos;
                            moved = true;
                            area_name_active = false;
                        }
                    } else if is_on_save_point(runtime, &current_map_id, player_pos) {
                        runtime.open_menu();
                        runtime.menu_state.active_submenu = Some("save".to_string());
                        runtime.menu_state.focus = MenuFocus::Detail;
                        runtime.menu_state.detail_page = 0;
                        runtime.menu_state.detail_selection = 0;
                        match run_menu_loop(
                            session,
                            runtime,
                            menu_ui,
                            progress_ui,
                            dialog_ui,
                            bindings,
                            &current_map_id,
                            player_pos,
                            save_dir,
                        ) {
                            Ok(crate::menu::MenuOutcome::ReturnTitle) => {
                                return Ok(OverworldOutcome::ReturnTitle)
                            }
                            Ok(crate::menu::MenuOutcome::Continue) => {}
                            Err(err) => {
                                if err.kind() == std::io::ErrorKind::Interrupted {
                                    return Err(err);
                                }
                            }
                        }
                    } else if let Some(chest) = find_chest(runtime, &current_map_id, player_pos) {
                        let chest_text = open_chest(runtime, &chest);
                        show_centered_dialog_on_map(
                            session,
                            &map,
                            player_pos,
                            dialog_ui,
                            bindings,
                            &chest_text,
                        )?;
                    } else if let Some(text) = find_sign_text(runtime, &current_map_id, player_pos)
                    {
                        show_centered_dialog_on_map(
                            session, &map, player_pos, dialog_ui, bindings, &text,
                        )?;
                    } else if let Some(door) =
                        find_adjacent_door(runtime, &current_map_id, player_pos)
                    {
                        if door_locked(runtime, &door) {
                            if let Some(event_id) = door.locked_event.as_ref() {
                                runtime.queue_event(event_id);
                                if let OverworldOutcome::Defeat(context) = run_pending_events(
                                    runtime,
                                    dialog_ui,
                                    battle_ui,
                                    bindings,
                                    session,
                                    Some(map.clone()),
                                )? {
                                    return Ok(OverworldOutcome::Defeat(context));
                                }
                            } else {
                                let text =
                                    door.locked_text.as_deref().unwrap_or("The door is locked.");
                                show_centered_dialog_on_map(
                                    session, &map, player_pos, dialog_ui, bindings, text,
                                )?;
                            }
                        } else if let Some(target_map) = door.target_map.as_ref() {
                            let target_pos = door
                                .target_pos
                                .map(|pos| (pos[0], pos[1]))
                                .unwrap_or(player_pos);
                            let (next_map, next_pos) = if door.return_to_last {
                                return_positions
                                    .get(&current_map_id)
                                    .cloned()
                                    .unwrap_or((target_map.clone(), target_pos))
                            } else {
                                (target_map.clone(), target_pos)
                            };

                            if runtime.is_overworld_map(&current_map_id)
                                && !runtime.is_overworld_map(&next_map)
                            {
                                runtime.record_last_overworld(&current_map_id, player_pos);
                            }

                            if !door.return_to_last
                                && !is_returning_from_child(
                                    &return_positions,
                                    &current_map_id,
                                    &next_map,
                                )
                            {
                                return_positions
                                    .insert(next_map.clone(), (current_map_id.clone(), player_pos));
                            }
                            current_map_id = next_map;
                            player_pos = next_pos;
                            runtime.world.map_id = current_map_id.clone();
                            runtime.world.position = player_pos;
                            transitioned = true;
                            moved = true;
                            area_name_active = false;
                        }
                    } else if let Some(puzzle) =
                        find_adjacent_puzzle(runtime, &current_map_id, player_pos)
                    {
                        if let Some(flag) = puzzle.set_flag.as_ref() {
                            runtime.set_flag(flag);
                        }
                        if let Some(event_id) = puzzle.event.as_ref() {
                            runtime.queue_event(event_id);
                            if let OverworldOutcome::Defeat(context) = run_pending_events(
                                runtime,
                                dialog_ui,
                                battle_ui,
                                bindings,
                                session,
                                Some(map.clone()),
                            )? {
                                return Ok(OverworldOutcome::Defeat(context));
                            }
                        } else if let Some(text) = puzzle.text.as_ref() {
                            show_centered_dialog_on_map(
                                session, &map, player_pos, dialog_ui, bindings, text,
                            )?;
                        }
                    } else if let Some(campfire) =
                        find_adjacent_campfire(runtime, &current_map_id, player_pos)
                    {
                        open_campfire(
                            runtime, dialog_ui, bindings, session, &map, player_pos, &campfire,
                        )?;
                    } else if let Some(dialog_id) =
                        find_npc_dialog(runtime, &current_map_id, player_pos)
                    {
                        run_dialog_on_map(
                            runtime, dialog_ui, bindings, session, &dialog_id, &map, player_pos,
                        )?;
                        if let OverworldOutcome::Defeat(context) = run_pending_events(
                            runtime,
                            dialog_ui,
                            battle_ui,
                            bindings,
                            session,
                            Some(map.clone()),
                        )? {
                            return Ok(OverworldOutcome::Defeat(context));
                        }
                    } else if let Some((vehicle_id, vehicle_pos)) =
                        find_adjacent_vehicle(runtime, &current_map_id, player_pos)
                    {
                        runtime.active_vehicle = Some(vehicle_id.clone());
                        runtime.vehicle_slow_mode = false;
                        player_pos = vehicle_pos;
                        runtime.world.position = player_pos;
                        update_vehicle_position(runtime, &vehicle_id, &current_map_id, player_pos);
                        area_name_active = false;
                    }
                }
                Action::Menu => {
                    runtime.open_menu();
                    match run_menu_loop(
                        session,
                        runtime,
                        menu_ui,
                        progress_ui,
                        dialog_ui,
                        bindings,
                        &current_map_id,
                        player_pos,
                        save_dir,
                    ) {
                        Ok(crate::menu::MenuOutcome::ReturnTitle) => {
                            return Ok(OverworldOutcome::ReturnTitle)
                        }
                        Ok(crate::menu::MenuOutcome::Continue) => {}
                        Err(err) => {
                            if err.kind() == std::io::ErrorKind::Interrupted {
                                return Err(err);
                            }
                        }
                    }
                }
                Action::Cancel => {
                    if runtime.active_vehicle.is_some() {
                        runtime.vehicle_slow_mode = !runtime.vehicle_slow_mode;
                    }
                }
                Action::Quit => {
                    if tui::dialog::confirm_quit(session, |frame| {
                        tui::overworld::draw_overworld_frame(frame, &map, player_pos);
                    })? {
                        return Ok(OverworldOutcome::Quit);
                    }
                }
                _ => {}
            }
        }

        if !moved
            && !transitioned
            && (runtime.world.map_id != current_map_id || runtime.world.position != player_pos)
        {
            current_map_id = runtime.world.map_id.clone();
            player_pos = runtime.world.position;
            transitioned = true;
            moved = true;
            area_name_active = false;
        }

        if transitioned {
            if runtime.effective_autosave_enabled() {
                if let Err(err) = write_autosave(runtime, save_dir) {
                    eprintln!("Failed to autosave: {}", err);
                }
            }
            let on_enter_events = runtime.get_on_enter_events_for_map(&current_map_id);
            for event_id in on_enter_events {
                runtime.queue_event(&event_id);
            }
            if !runtime.event_queue.is_empty() {
                runtime.state = GameState::Event;
                runtime.start_next_event();
                let outcome = run_event_loop(
                    runtime,
                    dialog_ui,
                    battle_ui,
                    bindings,
                    session,
                    Some(map.clone()),
                )?;
                if let EventLoopOutcome::Defeat(context) = outcome {
                    return Ok(OverworldOutcome::Defeat(context));
                }
            }
        }

        if moved && !transitioned {
            runtime.world.map_id = current_map_id.clone();
            runtime.world.position = player_pos;
            let step_events_pos =
                runtime.get_on_step_events_for_position(&current_map_id, player_pos);
            let step_events_zone =
                runtime.get_on_step_events_for_zone(&current_map_id, player_pos, previous_pos);
            for event_id in step_events_pos.into_iter().chain(step_events_zone) {
                runtime.queue_event(&event_id);
            }
            if !runtime.event_queue.is_empty() {
                runtime.state = GameState::Event;
                runtime.start_next_event();
                let outcome = run_event_loop(
                    runtime,
                    dialog_ui,
                    battle_ui,
                    bindings,
                    session,
                    Some(map.clone()),
                )?;
                if let EventLoopOutcome::Defeat(context) = outcome {
                    return Ok(OverworldOutcome::Defeat(context));
                }
            }
            apply_overworld_poison(runtime);
            update_roaming_npcs(runtime, &current_map_id, player_pos, &mut rng);
            if let Some(report) = try_start_random_battle(
                runtime,
                battle_ui,
                bindings,
                session,
                &current_map_id,
                player_pos,
                &mut encounter_meter,
                &mut rng,
            )? {
                if matches!(report.outcome, BattleOutcome::Defeat) {
                    let context = LastBattleContext::new(
                        report.formation,
                        report.snapshot,
                        BattleSource::Random,
                        current_map_id.clone(),
                        player_pos,
                    );
                    return Ok(OverworldOutcome::Defeat(context));
                }
            }
        }
    }
    Ok(OverworldOutcome::Quit)
}

pub fn build_map_view(runtime: &GameRuntime, map_id: &str) -> Option<MapView> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let map_state = runtime.map_states.get(map_id);
    let npcs = map
        .npcs
        .iter()
        .filter_map(|npc| {
            if !requires_flags_met(runtime, &npc.requires_flags) {
                return None;
            }
            let mut pos = (npc.pos[0], npc.pos[1]);
            let mut visible = true;
            if let Some(state) = map_state.and_then(|state| state.entities.get(&npc.id)) {
                if let Some(state_pos) = state.pos {
                    pos = state_pos;
                }
                if let Some(state_visible) = state.visible {
                    visible = state_visible;
                }
            }
            if !visible {
                return None;
            }
            Some(NpcView {
                id: npc.id.clone(),
                pos,
                glyph: npc_glyph(runtime, &npc.id),
                palette: runtime
                    .content
                    .npcs
                    .npcs
                    .iter()
                    .find(|entry| entry.id == npc.id)
                    .and_then(|entry| entry.palette.clone()),
            })
        })
        .collect();
    let signs = map
        .signs
        .iter()
        .map(|sign| tui::overworld::SignView {
            id: sign.id.clone(),
            pos: (sign.pos[0], sign.pos[1]),
            glyph: sign
                .glyph
                .as_ref()
                .and_then(|glyph| glyph.chars().next())
                .unwrap_or('⚑'),
            palette: sign.palette.clone(),
            text: sign.text.clone(),
        })
        .collect();
    let chests = map
        .chests
        .iter()
        .map(|chest| tui::overworld::ChestView {
            id: chest.id.clone(),
            pos: (chest.pos[0], chest.pos[1]),
            glyph_closed: chest
                .glyph_closed
                .as_ref()
                .and_then(|glyph| glyph.chars().next())
                .unwrap_or('▣'),
            glyph_open: chest
                .glyph_open
                .as_ref()
                .and_then(|glyph| glyph.chars().next())
                .unwrap_or('▢'),
            palette: chest.palette.clone(),
            opened: runtime.has_flag(&chest.opened_flag),
        })
        .collect();
    let doors = map
        .doors
        .iter()
        .map(|door| tui::overworld::DoorView {
            id: door.id.clone(),
            pos: (door.pos[0], door.pos[1]),
            glyph: door
                .glyph
                .as_ref()
                .and_then(|glyph| glyph.chars().next())
                .unwrap_or('+'),
            palette: door.palette.clone(),
            locked: door
                .requires_flag
                .as_ref()
                .map(|flag| !runtime.has_flag(flag))
                .unwrap_or(false),
        })
        .collect();
    let puzzles = map
        .puzzles
        .iter()
        .filter(|puzzle| puzzle_visible(runtime, puzzle))
        .map(|puzzle| tui::overworld::PuzzleView {
            id: puzzle.id.clone(),
            pos: (puzzle.pos[0], puzzle.pos[1]),
            glyph: puzzle
                .glyph
                .as_ref()
                .and_then(|glyph| glyph.chars().next())
                .unwrap_or('?'),
            palette: puzzle.palette.clone(),
        })
        .collect();
    let campfires = map
        .campfires
        .iter()
        .filter(|campfire| requires_flags_met(runtime, &campfire.requires_flags))
        .map(|campfire| tui::overworld::CampfireView {
            id: campfire.id.clone(),
            pos: (campfire.pos[0], campfire.pos[1]),
            glyph: campfire
                .glyph
                .as_ref()
                .and_then(|glyph| glyph.chars().next())
                .unwrap_or('C'),
            palette: campfire.palette.clone(),
        })
        .collect();
    let save_points = map.save_points.iter().map(|pos| (pos[0], pos[1])).collect();
    let legend = map
        .legend
        .iter()
        .filter_map(|(glyph, entry)| {
            let key = glyph.chars().next()?;
            Some((
                key,
                TileRender {
                    palette: entry.palette.clone(),
                },
            ))
        })
        .collect();
    let transitions = map
        .transitions
        .iter()
        .map(|transition| TransitionView {
            pos: (transition.pos[0], transition.pos[1]),
            glyph: transition
                .glyph
                .as_ref()
                .and_then(|glyph| glyph.chars().next()),
            palette: transition.palette.clone(),
        })
        .collect();
    let vehicles = map
        .vehicles
        .iter()
        .filter_map(|vehicle| {
            let vehicle_def = runtime
                .content
                .vehicles
                .vehicles
                .iter()
                .find(|entry| entry.id == vehicle.vehicle_id)?;
            if !is_vehicle_unlocked(runtime, vehicle_def)
                || !requires_flags_met(runtime, &vehicle.requires_flags)
            {
                return None;
            }
            let position = runtime
                .vehicle_positions
                .get(&vehicle.vehicle_id)
                .map(|entry| (entry.map_id.clone(), entry.pos))
                .unwrap_or_else(|| (map.id.clone(), (vehicle.pos[0], vehicle.pos[1])));
            if position.0 != map.id {
                return None;
            }
            if runtime
                .active_vehicle
                .as_deref()
                .is_some_and(|id| id == vehicle.vehicle_id)
            {
                return None;
            }
            Some(VehicleView {
                id: vehicle.vehicle_id.clone(),
                pos: position.1,
                glyph: vehicle_def
                    .glyph
                    .as_ref()
                    .and_then(|glyph| glyph.chars().next())
                    .unwrap_or('V'),
                palette: vehicle_def.palette.clone(),
            })
        })
        .collect();
    let active_vehicle = runtime
        .active_vehicle
        .as_ref()
        .and_then(|vehicle_id| {
            runtime
                .content
                .vehicles
                .vehicles
                .iter()
                .find(|vehicle| vehicle.id == *vehicle_id)
        })
        .map(|vehicle| tui::overworld::ActiveVehicleView {
            glyph: vehicle
                .glyph
                .as_ref()
                .and_then(|glyph| glyph.chars().next())
                .unwrap_or('V'),
            palette: vehicle.palette.clone(),
        });
    let use_color = runtime
        .content
        .rules
        .render
        .palette
        .eq_ignore_ascii_case("terminal");
    let death_marker_rules = &runtime.content.rules.render.death_markers;
    let death_marker_glyph = death_marker_rules.glyph.chars().next().unwrap_or('✞');
    let death_markers = if runtime.effective_death_markers_visible() {
        map_state
            .map(|state| {
                state
                    .death_markers
                    .iter()
                    .map(|marker| tui::overworld::DeathMarkerView {
                        pos: (marker.pos[0], marker.pos[1]),
                        count: marker.count,
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    Some(MapView {
        name: map.name.clone(),
        hide_name: map.hide_name,
        width: map.width as u16,
        height: map.height as u16,
        loop_x: map.loop_config.x,
        loop_y: map.loop_config.y,
        tiles: map.tiles.clone(),
        legend,
        transitions,
        vehicles,
        active_vehicle,
        npcs,
        signs,
        chests,
        doors,
        puzzles,
        campfires,
        save_points,
        death_markers,
        death_marker_glyph,
        death_marker_palette: None,
        use_color,
    })
}

fn mark_map_visited(runtime: &mut GameRuntime, map_id: &str) {
    let state = runtime.map_states.entry(map_id.to_string()).or_default();
    state.flags.insert("visited".to_string());
}

pub fn record_death_marker(runtime: &mut GameRuntime, map_id: &str, pos: (i32, i32)) {
    if !runtime.content.rules.render.death_markers.show_on_map {
        return;
    }
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return,
    };
    let width = map.width.max(1) as i32;
    let height = map.height.max(1) as i32;
    let start_pos = match normalize_map_pos(runtime, map_id, pos) {
        Some(pos) => pos,
        None => return,
    };
    let total = width.saturating_mul(height).max(1);
    let start_index = (start_pos.1 * width + start_pos.0).rem_euclid(total);
    let existing_markers: HashSet<(i32, i32)> = runtime
        .map_states
        .get(map_id)
        .map(|state| {
            state
                .death_markers
                .iter()
                .map(|marker| (marker.pos[0], marker.pos[1]))
                .collect()
        })
        .unwrap_or_default();
    let mut target = None;
    for offset in 0..total {
        let index = (start_index + offset).rem_euclid(total);
        let candidate = (index % width, index / width);
        if !death_marker_passable(runtime, map_id, candidate) {
            continue;
        }
        if existing_markers.contains(&candidate) {
            continue;
        }
        target = Some(candidate);
        break;
    }

    let state = runtime.map_states.entry(map_id.to_string()).or_default();
    add_death_marker(state, target.unwrap_or(start_pos));
}

fn add_death_marker(state: &mut engine::maps::MapState, pos: (i32, i32)) {
    if let Some(marker) = state
        .death_markers
        .iter_mut()
        .find(|marker| marker.pos == [pos.0, pos.1])
    {
        marker.count = marker.count.saturating_add(1);
        return;
    }
    state.death_markers.push(engine::maps::DeathMarkerState {
        pos: [pos.0, pos.1],
        count: 1,
    });
}

fn death_marker_passable(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
    if !is_passable(runtime, map_id, pos) {
        return false;
    }
    if npc_at(runtime, map_id, pos)
        || sign_at(runtime, map_id, pos)
        || chest_at(runtime, map_id, pos)
        || door_at(runtime, map_id, pos).is_some()
        || puzzle_at(runtime, map_id, pos)
        || campfire_at(runtime, map_id, pos)
        || vehicle_at(runtime, map_id, pos, None)
    {
        return false;
    }
    true
}

pub fn is_passable(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
    let pos = match normalize_map_pos(runtime, map_id, pos) {
        Some(pos) => pos,
        None => return false,
    };
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    let tile = map
        .tiles
        .get(pos.1 as usize)
        .and_then(|row| row.chars().nth(pos.0 as usize))
        .unwrap_or(' ');
    let key = tile.to_string();
    map.legend
        .get(&key)
        .map(|entry| entry.passable)
        .unwrap_or(false)
}

fn tile_id_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> Option<String> {
    let pos = normalize_map_pos(runtime, map_id, pos)?;
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let tile = map
        .tiles
        .get(pos.1 as usize)
        .and_then(|row| row.chars().nth(pos.0 as usize))?;
    map.legend
        .get(&tile.to_string())
        .map(|entry| entry.tile.clone())
}

fn normalize_map_pos(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> Option<(i32, i32)> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let width = map.width as i32;
    let height = map.height as i32;
    if width <= 0 || height <= 0 {
        return None;
    }
    let mut x = pos.0;
    let mut y = pos.1;
    if map.loop_config.x {
        x = x.rem_euclid(width);
    } else if x < 0 || x >= width {
        return None;
    }
    if map.loop_config.y {
        y = y.rem_euclid(height);
    } else if y < 0 || y >= height {
        return None;
    }
    Some((x, y))
}

fn is_vehicle_passable(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
    vehicle_id: &str,
) -> bool {
    let vehicle = match runtime
        .content
        .vehicles
        .vehicles
        .iter()
        .find(|vehicle| vehicle.id == vehicle_id)
    {
        Some(vehicle) => vehicle,
        None => return false,
    };
    let tile_id = match tile_id_at(runtime, map_id, pos) {
        Some(tile_id) => tile_id,
        None => return false,
    };
    vehicle.allowed_tiles.iter().any(|tile| tile == &tile_id)
}

fn is_vehicle_unlocked(
    runtime: &GameRuntime,
    vehicle: &engine::entities::VehicleDefinition,
) -> bool {
    if vehicle.unlock_flag.trim().is_empty() {
        return true;
    }
    runtime.has_flag(&vehicle.unlock_flag)
}

fn requires_flags_met(runtime: &GameRuntime, flags: &Option<Vec<String>>) -> bool {
    flags.as_ref().map_or(true, |flags| {
        flags.iter().all(|flag| runtime.has_flag(flag))
    })
}

fn vehicle_at(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
    active_vehicle: Option<&str>,
) -> bool {
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    map.vehicles.iter().any(|vehicle| {
        if active_vehicle.is_some_and(|id| id == vehicle.vehicle_id) {
            return false;
        }
        let vehicle_def = runtime
            .content
            .vehicles
            .vehicles
            .iter()
            .find(|entry| entry.id == vehicle.vehicle_id);
        let Some(vehicle_def) = vehicle_def else {
            return false;
        };
        if !is_vehicle_unlocked(runtime, vehicle_def)
            || !requires_flags_met(runtime, &vehicle.requires_flags)
        {
            return false;
        }
        let position = runtime
            .vehicle_positions
            .get(&vehicle.vehicle_id)
            .map(|entry| (entry.map_id.clone(), entry.pos))
            .unwrap_or_else(|| (map.id.clone(), (vehicle.pos[0], vehicle.pos[1])));
        position.0 == map.id && position.1 == pos
    })
}

fn can_move_to(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
    let Some(pos) = normalize_map_pos(runtime, map_id, pos) else {
        return false;
    };
    let passable = if let Some(vehicle_id) = runtime.active_vehicle.as_deref() {
        is_vehicle_passable(runtime, map_id, pos, vehicle_id)
    } else {
        is_passable(runtime, map_id, pos)
    };
    if !passable {
        return false;
    }
    if npc_at(runtime, map_id, pos)
        || sign_at(runtime, map_id, pos)
        || chest_at(runtime, map_id, pos)
        || vehicle_at(runtime, map_id, pos, runtime.active_vehicle.as_deref())
    {
        return false;
    }
    true
}

fn movement_speed(runtime: &GameRuntime) -> i32 {
    if let Some(vehicle_id) = runtime.active_vehicle.as_deref() {
        if runtime.vehicle_slow_mode {
            return 1;
        }
        if let Some(vehicle) = runtime
            .content
            .vehicles
            .vehicles
            .iter()
            .find(|vehicle| vehicle.id == vehicle_id)
        {
            return vehicle.speed.max(1);
        }
    }
    1
}

fn apply_overworld_poison(runtime: &mut GameRuntime) {
    let active_ids = runtime.party.active_ids();
    for actor_id in active_ids {
        if let Some(actor) = runtime.party.roster.get_mut(&actor_id) {
            engine::battle::apply_overworld_poison_tick(&runtime.content, actor);
        }
    }
}

fn update_vehicle_position(
    runtime: &mut GameRuntime,
    vehicle_id: &str,
    map_id: &str,
    pos: (i32, i32),
) {
    runtime.vehicle_positions.insert(
        vehicle_id.to_string(),
        engine::runtime::VehiclePosition {
            map_id: map_id.to_string(),
            pos,
        },
    );
}

fn find_adjacent_vehicle(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<(String, (i32, i32))> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    for vehicle in &map.vehicles {
        if runtime
            .active_vehicle
            .as_deref()
            .is_some_and(|id| id == vehicle.vehicle_id)
        {
            continue;
        }
        let Some(vehicle_def) = runtime
            .content
            .vehicles
            .vehicles
            .iter()
            .find(|entry| entry.id == vehicle.vehicle_id)
        else {
            continue;
        };
        if !is_vehicle_unlocked(runtime, vehicle_def)
            || !requires_flags_met(runtime, &vehicle.requires_flags)
        {
            continue;
        }
        let position = runtime
            .vehicle_positions
            .get(&vehicle.vehicle_id)
            .map(|entry| (entry.map_id.clone(), entry.pos))
            .unwrap_or_else(|| (map.id.clone(), (vehicle.pos[0], vehicle.pos[1])));
        if position.0 != map.id {
            continue;
        }
        let dx = (position.1 .0 - pos.0).abs();
        let dy = (position.1 .1 - pos.1).abs();
        if dx + dy == 1 {
            return Some((vehicle.vehicle_id.clone(), position.1));
        }
    }
    None
}

fn find_disembark_pos(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> Option<(i32, i32)> {
    let candidates = [(0, -1), (0, 1), (-1, 0), (1, 0)];
    for (dx, dy) in candidates {
        let raw_target = (pos.0 + dx, pos.1 + dy);
        let Some(target) = normalize_map_pos(runtime, map_id, raw_target) else {
            continue;
        };
        if is_passable(runtime, map_id, target)
            && !npc_at(runtime, map_id, target)
            && !sign_at(runtime, map_id, target)
            && !chest_at(runtime, map_id, target)
            && !vehicle_at(runtime, map_id, target, runtime.active_vehicle.as_deref())
        {
            return Some(target);
        }
    }
    None
}

pub fn find_spawn(runtime: &GameRuntime, map_id: &str, fallback: (i32, i32)) -> (i32, i32) {
    if is_passable(runtime, map_id, fallback) {
        return fallback;
    }

    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return fallback,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return fallback,
    };

    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            if is_passable(runtime, map_id, (x, y)) {
                return (x, y);
            }
        }
    }

    fallback
}

fn find_transition(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<engine::maps::MapTransition> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let pos = normalize_map_pos(runtime, map_id, pos)?;
    map.transitions
        .iter()
        .find(|transition| transition.pos == [pos.0, pos.1])
        .cloned()
}

fn npc_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
    let pos = match normalize_map_pos(runtime, map_id, pos) {
        Some(pos) => pos,
        None => return false,
    };
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    let map_state = runtime.map_states.get(map_id);
    map.npcs.iter().any(|npc| {
        if !requires_flags_met(runtime, &npc.requires_flags) {
            return false;
        }
        let mut npc_pos = (npc.pos[0], npc.pos[1]);
        let mut visible = true;
        if let Some(state) = map_state.and_then(|state| state.entities.get(&npc.id)) {
            if let Some(state_pos) = state.pos {
                npc_pos = state_pos;
            }
            if let Some(state_visible) = state.visible {
                visible = state_visible;
            }
        }
        visible && npc_pos == pos
    })
}

fn chest_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
    let pos = match normalize_map_pos(runtime, map_id, pos) {
        Some(pos) => pos,
        None => return false,
    };
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    map.chests
        .iter()
        .any(|chest| (chest.pos[0], chest.pos[1]) == pos)
}

fn sign_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
    let pos = match normalize_map_pos(runtime, map_id, pos) {
        Some(pos) => pos,
        None => return false,
    };
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    map.signs
        .iter()
        .any(|sign| (sign.pos[0], sign.pos[1]) == pos)
}

fn door_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> Option<engine::maps::MapDoor> {
    let pos = normalize_map_pos(runtime, map_id, pos)?;
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    map.doors
        .iter()
        .find(|door| (door.pos[0], door.pos[1]) == pos)
        .cloned()
}

fn door_locked(runtime: &GameRuntime, door: &engine::maps::MapDoor) -> bool {
    door.requires_flag
        .as_ref()
        .map(|flag| !runtime.has_flag(flag))
        .unwrap_or(false)
}

fn puzzle_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
    let pos = match normalize_map_pos(runtime, map_id, pos) {
        Some(pos) => pos,
        None => return false,
    };
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    map.puzzles
        .iter()
        .any(|puzzle| puzzle_visible(runtime, puzzle) && (puzzle.pos[0], puzzle.pos[1]) == pos)
}

fn campfire_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
    let pos = match normalize_map_pos(runtime, map_id, pos) {
        Some(pos) => pos,
        None => return false,
    };
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    map.campfires.iter().any(|campfire| {
        requires_flags_met(runtime, &campfire.requires_flags)
            && (campfire.pos[0], campfire.pos[1]) == pos
    })
}

fn find_npc_dialog(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> Option<String> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let map_state = runtime.map_states.get(map_id);
    let target = map.npcs.iter().find(|npc| {
        if !requires_flags_met(runtime, &npc.requires_flags) {
            return false;
        }
        let mut npc_pos = (npc.pos[0], npc.pos[1]);
        let mut visible = true;
        if let Some(state) = map_state.and_then(|state| state.entities.get(&npc.id)) {
            if let Some(state_pos) = state.pos {
                npc_pos = state_pos;
            }
            if let Some(state_visible) = state.visible {
                visible = state_visible;
            }
        }
        if !visible {
            return false;
        }
        let npc_def = runtime
            .content
            .npcs
            .npcs
            .iter()
            .find(|def| def.id == npc.id);
        if let Some(npc_def) = npc_def {
            let range = npc_def.interaction_range.unwrap_or(1);
            let dx = (npc_pos.0 - pos.0).abs();
            let dy = (npc_pos.1 - pos.1).abs();
            let distance = dx + dy;
            distance > 0 && distance <= range
        } else {
            false
        }
    })?;

    runtime
        .content
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.id == target.id)
        .map(|npc| npc.dialog.clone())
}

fn find_chest(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<engine::maps::MapChest> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let chest = map.chests.iter().find(|chest| {
        let dx = (chest.pos[0] - pos.0).abs();
        let dy = (chest.pos[1] - pos.1).abs();
        (dx == 1 && dy == 0) || (dx == 0 && dy == 1)
    })?;
    Some(chest.clone())
}

fn find_sign_text(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> Option<String> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let sign = map.signs.iter().find(|sign| {
        let dx = (sign.pos[0] - pos.0).abs();
        let dy = (sign.pos[1] - pos.1).abs();
        (dx == 1 && dy == 0) || (dx == 0 && dy == 1)
    })?;
    Some(sign.text.clone())
}

fn find_adjacent_door(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<engine::maps::MapDoor> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let door = map.doors.iter().find(|door| {
        let dx = (door.pos[0] - pos.0).abs();
        let dy = (door.pos[1] - pos.1).abs();
        (dx == 1 && dy == 0) || (dx == 0 && dy == 1)
    })?;
    Some(door.clone())
}

fn find_adjacent_puzzle(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<engine::maps::MapPuzzle> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let puzzle = map.puzzles.iter().find(|puzzle| {
        if !puzzle_visible(runtime, puzzle) {
            return false;
        }
        let dx = (puzzle.pos[0] - pos.0).abs();
        let dy = (puzzle.pos[1] - pos.1).abs();
        dx + dy <= 1
    })?;
    Some(puzzle.clone())
}

fn puzzle_visible(runtime: &GameRuntime, puzzle: &engine::maps::MapPuzzle) -> bool {
    if !requires_flags_met(runtime, &puzzle.requires_flags) {
        return false;
    }
    if let Some(flag) = puzzle.set_flag.as_ref() {
        return !runtime.has_flag(flag);
    }
    true
}

fn find_adjacent_campfire(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<engine::maps::MapCampfire> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let campfire = map.campfires.iter().find(|campfire| {
        if !requires_flags_met(runtime, &campfire.requires_flags) {
            return false;
        }
        let dx = (campfire.pos[0] - pos.0).abs();
        let dy = (campfire.pos[1] - pos.1).abs();
        (dx == 1 && dy == 0) || (dx == 0 && dy == 1)
    })?;
    Some(campfire.clone())
}

fn open_campfire(
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    bindings: &InputBindings,
    session: &mut TuiSession,
    map: &tui::overworld::MapView,
    player_pos: (i32, i32),
    campfire: &engine::maps::MapCampfire,
) -> std::io::Result<()> {
    let cooking_enabled = runtime
        .content
        .rules
        .systems
        .get("cooking")
        .copied()
        .unwrap_or(false);
    if !cooking_enabled {
        show_centered_dialog_on_map(
            session,
            map,
            player_pos,
            dialog_ui,
            bindings,
            "Cooking is unavailable.",
        )?;
        return Ok(());
    }

    let recipe = {
        let Some(cooking) = runtime.content.cooking.as_ref() else {
            show_centered_dialog_on_map(
                session,
                map,
                player_pos,
                dialog_ui,
                bindings,
                "No recipes are available.",
            )?;
            return Ok(());
        };

        let Some(campfire_def) = cooking
            .campfires
            .iter()
            .find(|entry| entry.id == campfire.campfire_id)
        else {
            show_centered_dialog_on_map(
                session,
                map,
                player_pos,
                dialog_ui,
                bindings,
                "No recipes are available.",
            )?;
            return Ok(());
        };

        let recipes = campfire_def
            .recipes
            .iter()
            .filter_map(|recipe_id| {
                cooking
                    .recipes
                    .iter()
                    .find(|recipe| recipe.id == *recipe_id)
            })
            .filter(|recipe| recipe_unlocked(runtime, recipe))
            .collect::<Vec<_>>();

        if recipes.is_empty() {
            show_centered_dialog_on_map(
                session,
                map,
                player_pos,
                dialog_ui,
                bindings,
                "No recipes are available.",
            )?;
            return Ok(());
        }

        let choices = recipes
            .iter()
            .map(|recipe| ChoiceView {
                label: recipe.name.clone(),
                show_next: false,
            })
            .collect::<Vec<_>>();

        let details = recipes
            .iter()
            .map(|recipe| build_recipe_detail_panel(runtime, recipe))
            .collect::<Vec<_>>();

        let selection = tui::overworld::show_dialog_with_choices_and_details_on_map(
            session,
            map,
            player_pos,
            dialog_ui,
            bindings,
            "Campfire",
            &format!("{} Recipes", campfire_def.label),
            &choices,
            &details,
        )?;

        let Some(selection) = selection else {
            return Ok(());
        };

        match recipes.get(selection) {
            Some(recipe) => (*recipe).clone(),
            None => return Ok(()),
        }
    };

    if !can_cook_recipe(runtime, &recipe) {
        show_centered_dialog_on_map(
            session,
            map,
            player_pos,
            dialog_ui,
            bindings,
            "You lack the ingredients.",
        )?;
        return Ok(());
    }

    apply_cooking_recipe(runtime, &recipe);
    let result_text = format_cooking_results(runtime, &recipe);
    show_centered_dialog_on_map(session, map, player_pos, dialog_ui, bindings, &result_text)?;
    Ok(())
}

fn can_cook_recipe(runtime: &GameRuntime, recipe: &engine::content::CookingRecipe) -> bool {
    recipe
        .ingredients
        .iter()
        .all(|ingredient| runtime.inventory.item_qty(&ingredient.id) >= ingredient.qty)
}

fn apply_cooking_recipe(runtime: &mut GameRuntime, recipe: &engine::content::CookingRecipe) {
    for ingredient in &recipe.ingredients {
        runtime
            .inventory
            .remove_item(&ingredient.id, ingredient.qty);
    }

    let max_stack = runtime.content.rules.inventory.max_stack;
    for item in &recipe.results.items {
        runtime.inventory.add_item(&item.id, item.qty, max_stack);
    }
    for item in &recipe.results.equipment {
        runtime
            .inventory
            .add_equipment(&item.id, item.qty, max_stack);
    }
    for currency in &recipe.results.currency {
        runtime
            .inventory
            .add_currency(&currency.id, currency.amount);
    }
}

fn format_cooking_results(
    runtime: &GameRuntime,
    recipe: &engine::content::CookingRecipe,
) -> String {
    let cost = recipe
        .ingredients
        .iter()
        .filter(|ingredient| ingredient.qty > 0)
        .map(|ingredient| {
            format!(
                "{} x{}",
                lookup_item_name(runtime, &ingredient.id),
                ingredient.qty
            )
        })
        .collect::<Vec<_>>();
    let mut found = Vec::new();
    for item in &recipe.results.items {
        if item.qty <= 0 {
            continue;
        }
        found.push(format!(
            "{} x{}",
            lookup_item_name(runtime, &item.id),
            item.qty
        ));
    }
    for item in &recipe.results.equipment {
        if item.qty <= 0 {
            continue;
        }
        found.push(format!(
            "{} x{}",
            lookup_item_name(runtime, &item.id),
            item.qty
        ));
    }
    for currency in &recipe.results.currency {
        if currency.amount <= 0 {
            continue;
        }
        found.push(format_currency_stack(&runtime.content.rules, currency));
    }

    let cost_text = if cost.is_empty() {
        "Cost: None.".to_string()
    } else {
        format!("Cost: {}.", cost.join(", "))
    };
    let result_text = if found.is_empty() {
        format!("Cooked: {}.", recipe.name)
    } else {
        format!("Cooked: {}.", found.join(", "))
    };
    format!("{}\n{}", cost_text, result_text)
}

fn build_recipe_detail_panel(
    runtime: &GameRuntime,
    recipe: &engine::content::CookingRecipe,
) -> MenuPanelView {
    let mut lines = Vec::new();
    lines.push(panel_line_spans(vec![panel_span(
        "Ingredients",
        PanelSpanStyle::Accent,
    )]));

    let mut ready = true;
    for ingredient in &recipe.ingredients {
        let have = runtime.inventory.item_qty(&ingredient.id);
        let need = ingredient.qty;
        if have < need {
            ready = false;
        }
        let count_style = if have >= need {
            PanelSpanStyle::Accent
        } else {
            PanelSpanStyle::Muted
        };
        lines.push(panel_line_spans(vec![
            panel_span(
                format!("{}: ", lookup_item_name(runtime, &ingredient.id)),
                PanelSpanStyle::Normal,
            ),
            panel_span(format!("{}/{}", have, need), count_style),
        ]));
    }

    let status_text = if ready {
        "Ready"
    } else {
        "Missing ingredients"
    };
    let status_style = if ready {
        PanelSpanStyle::Accent
    } else {
        PanelSpanStyle::Muted
    };
    lines.push(panel_line_spans(vec![
        panel_span("Status: ", PanelSpanStyle::Normal),
        panel_span(status_text, status_style),
    ]));

    MenuPanelView {
        title: recipe.name.clone(),
        lines,
    }
}

fn recipe_unlocked(runtime: &GameRuntime, recipe: &engine::content::CookingRecipe) -> bool {
    recipe
        .unlock_flag
        .as_ref()
        .map(|flag| runtime.has_flag(flag))
        .unwrap_or(true)
}

fn run_pending_events(
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    battle_ui: &BattleUiFile,
    bindings: &InputBindings,
    session: &mut TuiSession,
    map_view: Option<MapView>,
) -> std::io::Result<OverworldOutcome> {
    if runtime.event_queue.is_empty() {
        return Ok(OverworldOutcome::Continue);
    }
    runtime.state = GameState::Event;
    runtime.start_next_event();
    let outcome = run_event_loop(runtime, dialog_ui, battle_ui, bindings, session, map_view)?;
    if let EventLoopOutcome::Defeat(context) = outcome {
        return Ok(OverworldOutcome::Defeat(context));
    }
    Ok(OverworldOutcome::Continue)
}

fn is_on_save_point(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    map.save_points
        .iter()
        .any(|save_pos| (save_pos[0], save_pos[1]) == pos)
}

fn write_autosave(runtime: &GameRuntime, save_dir: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(save_dir).map_err(|err| format!("{}: {}", save_dir.display(), err))?;
    let save = engine::save::SaveFile::from_runtime(runtime, 0);
    let path = save_dir.join("slot_0.json");
    save.write(path)
}

fn open_chest(runtime: &mut GameRuntime, chest: &engine::maps::MapChest) -> String {
    if runtime.has_flag(&chest.opened_flag) {
        return "The chest is empty.".to_string();
    }

    let max_stack = runtime.content.rules.inventory.max_stack;
    let mut found = Vec::new();

    for item in &chest.loot.items {
        if item.qty <= 0 {
            continue;
        }
        runtime.inventory.add_item(&item.id, item.qty, max_stack);
        found.push(format!(
            "{} x{}",
            lookup_item_name(runtime, &item.id),
            item.qty
        ));
    }

    for item in &chest.loot.equipment {
        if item.qty <= 0 {
            continue;
        }
        runtime
            .inventory
            .add_equipment(&item.id, item.qty, max_stack);
        found.push(format!(
            "{} x{}",
            lookup_item_name(runtime, &item.id),
            item.qty
        ));
    }

    for currency in &chest.loot.currency {
        if currency.amount <= 0 {
            continue;
        }
        runtime
            .inventory
            .add_currency(&currency.id, currency.amount);
        found.push(format_currency_stack(&runtime.content.rules, currency));
    }

    runtime.set_flag(&chest.opened_flag);

    if found.is_empty() {
        "The chest is empty.".to_string()
    } else {
        format!("Found: {}.", found.join(", "))
    }
}

fn format_currency_stack(
    rules: &engine::rules::RulesFile,
    currency: &engine::maps::MapCurrencyStack,
) -> String {
    if let Some(definition) = rules.game.currency(&currency.id) {
        if definition.symbol.trim().is_empty() {
            format!("{} {}", currency.amount, definition.name)
        } else {
            format!("{}{}", definition.symbol, currency.amount)
        }
    } else {
        format!("{} {}", currency.amount, currency.id)
    }
}

fn npc_glyph(runtime: &GameRuntime, npc_id: &str) -> char {
    runtime
        .content
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.id == npc_id)
        .and_then(|npc| npc.name.chars().next())
        .unwrap_or('N')
        .to_ascii_uppercase()
}

fn update_roaming_npcs(
    runtime: &mut GameRuntime,
    map_id: &str,
    player_pos: (i32, i32),
    rng: &mut impl Rng,
) {
    let mut map_states = std::mem::take(&mut runtime.map_states);
    let map_index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => {
            runtime.map_states = map_states;
            return;
        }
    };
    let map = match runtime.content.maps.get(map_index) {
        Some(map) => map,
        None => {
            runtime.map_states = map_states;
            return;
        }
    };
    let map_state = map_states.entry(map_id.to_string()).or_default();
    let mut occupied: HashSet<(i32, i32)> = HashSet::new();

    for npc in &map.npcs {
        if !requires_flags_met(runtime, &npc.requires_flags) {
            continue;
        }
        let (pos, visible) = npc_position_and_visibility(map_state, npc);
        if visible {
            occupied.insert(pos);
        }
    }

    for npc in &map.npcs {
        if !requires_flags_met(runtime, &npc.requires_flags) {
            continue;
        }
        let Some(definition) = runtime
            .content
            .npcs
            .npcs
            .iter()
            .find(|entry| entry.id == npc.id)
        else {
            continue;
        };
        let behavior_type = definition.behavior.r#type.as_str();
        if behavior_type == "static" {
            continue;
        }
        let (pos, visible) = npc_position_and_visibility(map_state, npc);
        if !visible {
            continue;
        }

        let state = map_state
            .entities
            .entry(npc.id.clone())
            .or_insert(engine::maps::EntityState {
                pos: None,
                state: None,
                visible: None,
                sprite: None,
            });

        let mut next_pos = pos;
        let mut next_state = None;

        match behavior_type {
            "roam" => {
                let radius = definition.behavior.radius.unwrap_or(2).max(1);
                let origin = (npc.pos[0], npc.pos[1]);
                let idle_chance = definition.behavior.idle_chance.clamp(0.0, 1.0);
                if idle_chance > 0.0 && rng.r#gen::<f32>() < idle_chance {
                    next_state = Some("roam".to_string());
                } else {
                    let mut directions = vec![(0, -1), (0, 1), (-1, 0), (1, 0)];
                    for _ in 0..directions.len() {
                        let index = rng.gen_range(0..directions.len());
                        let (dx, dy) = directions.swap_remove(index);
                        let candidate = (pos.0 + dx, pos.1 + dy);
                        let manhattan =
                            (candidate.0 - origin.0).abs() + (candidate.1 - origin.1).abs();
                        if manhattan > radius {
                            continue;
                        }
                        if npc_can_move_to(runtime, map_id, candidate, &occupied, player_pos) {
                            next_pos = candidate;
                            next_state = Some("roam".to_string());
                            break;
                        }
                    }
                }
            }
            "patrol" => {
                if let Some(path) = definition.behavior.path.as_ref() {
                    if !path.is_empty() {
                        let mut index = patrol_index(state.state.as_deref());
                        if index >= path.len() {
                            index = 0;
                        }
                        let mut target = (path[index][0], path[index][1]);
                        if pos == target {
                            index = (index + 1) % path.len();
                            target = (path[index][0], path[index][1]);
                        }
                        let dx = (target.0 - pos.0).signum();
                        let dy = (target.1 - pos.1).signum();
                        let step = if dx != 0 {
                            (pos.0 + dx, pos.1)
                        } else {
                            (pos.0, pos.1 + dy)
                        };
                        if npc_can_move_to(runtime, map_id, step, &occupied, player_pos) {
                            next_pos = step;
                        }
                        next_state = Some(format!("patrol:{}", index));
                    }
                }
            }
            _ => {}
        }

        occupied.remove(&pos);
        occupied.insert(next_pos);

        if next_pos != pos {
            state.pos = Some(next_pos);
        }
        if let Some(state_value) = next_state {
            state.state = Some(state_value);
        }
    }

    runtime.map_states = map_states;
}

fn npc_position_and_visibility(
    map_state: &engine::maps::MapState,
    npc: &engine::maps::MapNpc,
) -> ((i32, i32), bool) {
    let mut pos = (npc.pos[0], npc.pos[1]);
    let mut visible = true;
    if let Some(state) = map_state.entities.get(&npc.id) {
        if let Some(state_pos) = state.pos {
            pos = state_pos;
        }
        if let Some(state_visible) = state.visible {
            visible = state_visible;
        }
    }
    (pos, visible)
}

fn npc_can_move_to(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
    occupied: &HashSet<(i32, i32)>,
    player_pos: (i32, i32),
) -> bool {
    let Some(pos) = normalize_map_pos(runtime, map_id, pos) else {
        return false;
    };
    if pos == player_pos {
        return false;
    }
    if occupied.contains(&pos) {
        return false;
    }
    if !is_passable(runtime, map_id, pos) {
        return false;
    }
    if sign_at(runtime, map_id, pos)
        || chest_at(runtime, map_id, pos)
        || puzzle_at(runtime, map_id, pos)
        || campfire_at(runtime, map_id, pos)
    {
        return false;
    }
    if door_at(runtime, map_id, pos).is_some() {
        return false;
    }
    if find_transition(runtime, map_id, pos).is_some() {
        return false;
    }
    if vehicle_at(runtime, map_id, pos, runtime.active_vehicle.as_deref()) {
        return false;
    }
    true
}

fn patrol_index(state: Option<&str>) -> usize {
    let Some(state) = state else {
        return 0;
    };
    let Some(value) = state.strip_prefix("patrol:") else {
        return 0;
    };
    value.parse::<usize>().unwrap_or(0)
}

fn is_returning_from_child(
    return_positions: &HashMap<String, (String, (i32, i32))>,
    current_map_id: &str,
    target_map_id: &str,
) -> bool {
    return_positions
        .get(current_map_id)
        .map(|(return_map, _)| return_map == target_map_id)
        .unwrap_or(false)
}
