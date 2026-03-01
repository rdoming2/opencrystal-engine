use engine::runtime::GameRuntime;

use super::vehicles::{is_vehicle_passable, vehicle_at};
use super::{puzzle_visible, requires_flags_met};

pub(crate) fn is_passable(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
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

pub(crate) fn tile_id_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> Option<String> {
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

pub(crate) fn normalize_map_pos(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<(i32, i32)> {
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

pub(crate) fn can_move_to(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
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

pub(crate) fn find_transition(
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

pub(crate) fn npc_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
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

pub(crate) fn chest_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
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

pub(crate) fn sign_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
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

pub(crate) fn door_at(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<engine::maps::MapDoor> {
    let pos = normalize_map_pos(runtime, map_id, pos)?;
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    map.doors
        .iter()
        .find(|door| (door.pos[0], door.pos[1]) == pos)
        .cloned()
}

pub(crate) fn puzzle_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
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

pub(crate) fn campfire_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
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
