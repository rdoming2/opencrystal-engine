use std::collections::HashSet;

use engine::runtime::GameRuntime;

use super::movement::{
    campfire_at, chest_at, door_at, is_passable, normalize_map_pos, npc_at, puzzle_at, sign_at,
};
use super::vehicles::vehicle_at;

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
