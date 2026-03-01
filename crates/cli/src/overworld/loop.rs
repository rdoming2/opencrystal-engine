use std::collections::HashMap;

use engine::menu::MenuFocus;
use engine::runtime::{GameRuntime, GameState};
use tui::input::{Action, InputBindings};
use tui::overworld::{draw_overworld, draw_overworld_with_tooltip, show_centered_dialog_on_map};
use tui::session::TuiSession;
use tui::ui::{BattleUiFile, DialogUiFile, MenuUiFile, ProgressUiFile};

use crate::battle::{try_start_random_battle, BattleOutcome, BattleSource, LastBattleContext};
use crate::dialog::run_dialog_on_map;
use crate::events::{run_event_loop, EventLoopOutcome};
use crate::menu::run_menu_loop;
use crate::utils::read_action;

use super::interactions::{
    door_locked, find_adjacent_campfire, find_adjacent_door, find_adjacent_puzzle, find_chest,
    find_sign_text, is_on_save_point, open_campfire, open_chest, run_pending_events,
    write_autosave,
};
use super::map_view::{build_map_view, mark_map_visited};
use super::movement::{can_move_to, door_at, find_transition, normalize_map_pos};
use super::npc::{find_npc_dialog, update_roaming_npcs};
use super::vehicles::{
    find_adjacent_vehicle, find_disembark_pos, movement_speed, update_vehicle_position,
};
use super::{is_returning_from_child, OverworldOutcome};

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
    let mut rng = rand::rng();
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
                                return Ok(OverworldOutcome::ReturnTitle);
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
                            return Ok(OverworldOutcome::ReturnTitle);
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

fn apply_overworld_poison(runtime: &mut GameRuntime) {
    let active_ids = runtime.party.active_ids();
    for actor_id in active_ids {
        if let Some(actor) = runtime.party.roster.get_mut(&actor_id) {
            engine::battle::apply_overworld_poison_tick(&runtime.content, actor);
        }
    }
}
