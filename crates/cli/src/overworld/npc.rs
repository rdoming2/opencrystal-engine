use std::collections::HashSet;

use engine::runtime::GameRuntime;
use rand::{Rng, RngExt};

use super::movement::{
    campfire_at, chest_at, find_transition, is_passable, normalize_map_pos, puzzle_at, sign_at,
};
use super::requires_flags_met;
use super::vehicles::vehicle_at;

pub(crate) fn find_npc_dialog(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<String> {
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

pub(crate) fn npc_glyph(runtime: &GameRuntime, npc_id: &str) -> char {
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

pub(crate) fn update_roaming_npcs(
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
                if idle_chance > 0.0 && rng.random::<f32>() < idle_chance {
                    next_state = Some("roam".to_string());
                } else {
                    let mut directions = vec![(0, -1), (0, 1), (-1, 0), (1, 0)];
                    for _ in 0..directions.len() {
                        let index = rng.random_range(0..directions.len());
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
    if super::movement::door_at(runtime, map_id, pos).is_some() {
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
