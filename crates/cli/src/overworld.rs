use std::collections::HashMap;

use engine::menu::MenuFocus;
use engine::runtime::{GameRuntime, GameState};
use tui::input::{Action, InputBindings};
use tui::overworld::{
    draw_overworld, draw_overworld_with_tooltip, show_centered_dialog_on_map, MapView, NpcView,
    TileRender, TransitionView, VehicleView,
};
use tui::session::TuiSession;
use tui::ui::{BattleUiFile, DialogUiFile, MenuUiFile};

use crate::battle::{try_start_random_battle, BattleOutcome};
use crate::dialog::run_dialog_on_map;
use crate::events::run_event_loop;
use crate::menu::run_menu_loop;
use crate::shop::lookup_item_name;
use crate::utils::read_action;

pub fn run_overworld_loop(
    session: &mut TuiSession,
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    battle_ui: &BattleUiFile,
    menu_ui: &MenuUiFile,
    bindings: &InputBindings,
    map_id: &str,
    start_pos: (i32, i32),
    save_dir: &std::path::Path,
) -> std::io::Result<()> {
    let mut current_map_id = map_id.to_string();
    let mut player_pos = start_pos;
    let mut return_positions: HashMap<String, (String, (i32, i32))> = HashMap::new();
    let mut last_map_id = String::new();
    let mut area_name_active = false;
    let mut rng = rand::thread_rng();

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

        if current_map_id != last_map_id {
            area_name_active = !map.hide_name && !map.name.is_empty();
            last_map_id = current_map_id.clone();
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
                        let next_pos = (player_pos.0 + dx, player_pos.1 + dy);
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
                        if let Err(err) = run_menu_loop(
                            session,
                            runtime,
                            menu_ui,
                            bindings,
                            &current_map_id,
                            player_pos,
                            save_dir,
                        ) {
                            if err.kind() == std::io::ErrorKind::Interrupted {
                                return Err(err);
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
                    } else if let Some(dialog_id) =
                        find_npc_dialog(runtime, &current_map_id, player_pos)
                    {
                        run_dialog_on_map(
                            runtime, dialog_ui, bindings, session, &dialog_id, &map, player_pos,
                        )?;
                        if !runtime.event_queue.is_empty() {
                            runtime.state = GameState::Event;
                            runtime.start_next_event();
                            if let Err(err) = run_event_loop(
                                runtime,
                                dialog_ui,
                                battle_ui,
                                bindings,
                                session,
                                Some(map.clone()),
                            ) {
                                if err.kind() == std::io::ErrorKind::Interrupted {
                                    return Err(err);
                                }
                            }
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
                    if let Err(err) = run_menu_loop(
                        session,
                        runtime,
                        menu_ui,
                        bindings,
                        &current_map_id,
                        player_pos,
                        save_dir,
                    ) {
                        if err.kind() == std::io::ErrorKind::Interrupted {
                            return Err(err);
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
                        return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "quit"));
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
            if runtime.content.rules.save.autosave_enabled {
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
                if let Err(err) = run_event_loop(
                    runtime,
                    dialog_ui,
                    battle_ui,
                    bindings,
                    session,
                    Some(map.clone()),
                ) {
                    if err.kind() == std::io::ErrorKind::Interrupted {
                        return Err(err);
                    }
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
                if let Err(err) = run_event_loop(
                    runtime,
                    dialog_ui,
                    battle_ui,
                    bindings,
                    session,
                    Some(map.clone()),
                ) {
                    if err.kind() == std::io::ErrorKind::Interrupted {
                        return Err(err);
                    }
                }
            }
            apply_overworld_poison(runtime);
            if let Some(outcome) = try_start_random_battle(
                runtime,
                battle_ui,
                bindings,
                session,
                &current_map_id,
                player_pos,
                &mut rng,
            )? {
                if matches!(outcome, BattleOutcome::Defeat) {
                    tui::dialog::show_dialog(
                        session,
                        dialog_ui,
                        bindings,
                        "",
                        "The party was defeated.",
                    )?;
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "defeat",
                    ));
                }
            }
        }
    }
    Ok(())
}

pub fn build_map_view(runtime: &GameRuntime, map_id: &str) -> Option<MapView> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let map_state = runtime.map_states.get(map_id);
    let npcs = map
        .npcs
        .iter()
        .filter_map(|npc| {
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

    Some(MapView {
        name: map.name.clone(),
        hide_name: map.hide_name,
        width: map.width as u16,
        height: map.height as u16,
        tiles: map.tiles.clone(),
        legend,
        transitions,
        vehicles,
        active_vehicle,
        npcs,
        signs,
        chests,
        save_points,
        use_color,
    })
}

pub fn is_passable(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    if pos.0 < 0 || pos.1 < 0 || pos.0 >= map.width as i32 || pos.1 >= map.height as i32 {
        return false;
    }
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
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    if pos.0 < 0 || pos.1 < 0 || pos.0 >= map.width as i32 || pos.1 >= map.height as i32 {
        return None;
    }
    let tile = map
        .tiles
        .get(pos.1 as usize)
        .and_then(|row| row.chars().nth(pos.0 as usize))?;
    map.legend
        .get(&tile.to_string())
        .map(|entry| entry.tile.clone())
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
    let active_ids = runtime.party.active.clone();
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
        let target = (pos.0 + dx, pos.1 + dy);
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
    map.transitions
        .iter()
        .find(|transition| transition.pos == [pos.0, pos.1])
        .cloned()
}

fn npc_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
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

fn find_npc_dialog(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> Option<String> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let map_state = runtime.map_states.get(map_id);
    let target = map.npcs.iter().find(|npc| {
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
    if currency.id == rules.game.currency.id {
        format!("{}{}", rules.game.currency.symbol, currency.amount)
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
