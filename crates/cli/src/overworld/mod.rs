use std::collections::HashMap;

use engine::runtime::GameRuntime;

mod death_markers;
mod interactions;
mod map_view;
mod movement;
mod npc;
mod r#loop;
mod vehicles;

pub use death_markers::record_death_marker;
pub use map_view::build_map_view;
pub use movement::find_spawn;
pub use r#loop::run_overworld_loop;

#[derive(Clone, Debug)]
pub enum OverworldOutcome {
    Continue,
    Defeat(crate::battle::LastBattleContext),
    Quit,
    ReturnTitle,
}

pub(crate) fn requires_flags_met(runtime: &GameRuntime, flags: &Option<Vec<String>>) -> bool {
    flags.as_ref().map_or(true, |flags| {
        flags.iter().all(|flag| runtime.has_flag(flag))
    })
}

pub(crate) fn puzzle_visible(runtime: &GameRuntime, puzzle: &engine::maps::MapPuzzle) -> bool {
    if !requires_flags_met(runtime, &puzzle.requires_flags) {
        return false;
    }
    if let Some(flag) = puzzle.set_flag.as_ref() {
        return !runtime.has_flag(flag);
    }
    true
}

pub(crate) fn is_returning_from_child(
    return_positions: &HashMap<String, (String, (i32, i32))>,
    current_map_id: &str,
    target_map_id: &str,
) -> bool {
    return_positions
        .get(current_map_id)
        .map(|(return_map, _)| return_map == target_map_id)
        .unwrap_or(false)
}
