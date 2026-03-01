use engine::runtime::GameRuntime;

use super::movement::{chest_at, is_passable, normalize_map_pos, npc_at, sign_at, tile_id_at};
use super::requires_flags_met;

pub(crate) fn is_vehicle_passable(
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

pub(crate) fn is_vehicle_unlocked(
    runtime: &GameRuntime,
    vehicle: &engine::entities::VehicleDefinition,
) -> bool {
    if vehicle.unlock_flag.trim().is_empty() {
        return true;
    }
    runtime.has_flag(&vehicle.unlock_flag)
}

pub(crate) fn vehicle_at(
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

pub(crate) fn movement_speed(runtime: &GameRuntime) -> i32 {
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

pub(crate) fn update_vehicle_position(
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

pub(crate) fn find_adjacent_vehicle(
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

pub(crate) fn find_disembark_pos(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<(i32, i32)> {
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
