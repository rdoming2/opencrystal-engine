use engine::runtime::GameRuntime;
use tui::overworld::{MapView, NpcView, TileRender, TransitionView, VehicleView};

use super::npc::npc_glyph;
use super::vehicles::is_vehicle_unlocked;
use super::{puzzle_visible, requires_flags_met};

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

pub(crate) fn mark_map_visited(runtime: &mut GameRuntime, map_id: &str) {
    let state = runtime.map_states.entry(map_id.to_string()).or_default();
    state.flags.insert("visited".to_string());
}
