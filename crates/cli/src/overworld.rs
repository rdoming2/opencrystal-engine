use std::collections::HashMap;

use engine::runtime::{GameRuntime, GameState};
use tui::input::{Action, InputBindings};
use tui::overworld::{
    draw_overworld, draw_overworld_with_tooltip, show_centered_dialog_on_map, MapView, NpcView,
    TileRender, TransitionView,
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
        if let Some(action) = read_action(bindings) {
            match action {
                Action::MoveUp => {
                    player_pos.1 -= 1;
                    area_name_active = false;
                }
                Action::MoveDown => {
                    player_pos.1 += 1;
                    area_name_active = false;
                }
                Action::MoveLeft => {
                    player_pos.0 -= 1;
                    area_name_active = false;
                }
                Action::MoveRight => {
                    player_pos.0 += 1;
                    area_name_active = false;
                }
                Action::Confirm => {
                    if let Some(chest) = find_chest(runtime, &current_map_id, player_pos) {
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
                    ) {
                        if err.kind() == std::io::ErrorKind::Interrupted {
                            return Err(err);
                        }
                    }
                }
                Action::Cancel => {}
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

        let mut transitioned = false;
        if let Some(transition) = find_transition(runtime, &current_map_id, player_pos) {
            let (next_map, next_pos) = if transition.return_to_last {
                return_positions
                    .get(&current_map_id)
                    .cloned()
                    .map(|(return_map, return_pos)| (return_map, return_pos))
                    .unwrap_or_else(|| {
                        (
                            transition.target_map.clone(),
                            (transition.target_pos[0], transition.target_pos[1]),
                        )
                    })
            } else {
                (
                    transition.target_map.clone(),
                    (transition.target_pos[0], transition.target_pos[1]),
                )
            };

            if !transition.return_to_last
                && !is_returning_from_child(&return_positions, &current_map_id, &next_map)
            {
                return_positions.insert(next_map.clone(), (current_map_id.clone(), player_pos));
            }
            current_map_id = next_map;
            player_pos = next_pos;
            transitioned = true;
        }

        if !is_passable(runtime, &current_map_id, player_pos)
            || npc_at(runtime, &current_map_id, player_pos)
            || sign_at(runtime, &current_map_id, player_pos)
            || chest_at(runtime, &current_map_id, player_pos)
        {
            player_pos = previous_pos;
            transitioned = false;
        }

        if transitioned {
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

        let moved = player_pos != previous_pos;
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
    let npcs = map
        .npcs
        .iter()
        .map(|npc| NpcView {
            id: npc.id.clone(),
            pos: (npc.pos[0], npc.pos[1]),
            glyph: npc_glyph(runtime, &npc.id),
            palette: runtime
                .content
                .npcs
                .npcs
                .iter()
                .find(|entry| entry.id == npc.id)
                .and_then(|entry| entry.palette.clone()),
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
    map.npcs.iter().any(|npc| (npc.pos[0], npc.pos[1]) == pos)
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
    let target = map.npcs.iter().find(|npc| {
        let npc_def = runtime
            .content
            .npcs
            .npcs
            .iter()
            .find(|def| def.id == npc.id);
        if let Some(npc_def) = npc_def {
            let range = npc_def.interaction_range.unwrap_or(1);
            let dx = (npc.pos[0] - pos.0).abs();
            let dy = (npc.pos[1] - pos.1).abs();
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
