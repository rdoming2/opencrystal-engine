pub mod abilities;
pub mod common;
mod confirm;
pub mod equipment;
mod input;
pub mod inventory;
pub mod jobs;
pub mod journal;
pub mod magic;
pub mod magic_equip;
mod overworld;
mod panels;
pub mod party;
mod save;
pub mod settings;
pub mod status;

use engine::rules::ProgressionMode;
use engine::runtime::GameRuntime;
use tui::menu::MenuEntryView;
use tui::ui::MenuUiFile;

use self::common::MenuEntryState;
use self::overworld::overworld_map_available;

pub enum MenuOutcome {
    Continue,
    ReturnTitle,
}

pub use input::run_menu_loop;
pub use panels::PanelSize;

fn build_menu_entries(
    runtime: &GameRuntime,
    menu_ui: &MenuUiFile,
    map_id: &str,
    player_pos: (i32, i32),
) -> Vec<MenuEntryState> {
    let save_allowed = map_save_allowed(runtime, map_id, player_pos);
    let overview_available = overworld_map_available(runtime);
    menu_ui
        .menu
        .iter()
        .filter_map(|entry| {
            let system_enabled = system_enabled(runtime, entry.system.as_deref());
            if !system_enabled {
                return None;
            }
            let unlock_enabled = unlock_flag_enabled(runtime, entry.unlock_flag.as_deref());
            let mut selectable = entry.enabled && unlock_enabled;
            if entry.action == "save" && !save_allowed {
                selectable = false;
            }
            if entry.action == "overworld_map" && !overview_available {
                selectable = false;
            }
            let show = selectable
                || (!entry.enabled && entry.locked_behavior.as_deref() == Some("disable"))
                || (!unlock_enabled && entry.locked_behavior.as_deref() == Some("disable"))
                || (entry.action == "save"
                    && !save_allowed
                    && entry.locked_behavior.as_deref() == Some("disable"));
            if !show {
                return None;
            }
            Some(MenuEntryState {
                view: MenuEntryView {
                    id: entry.id.clone(),
                    label: entry.label.clone(),
                    enabled: selectable,
                },
                action: entry.action.clone(),
                selectable,
            })
        })
        .collect()
}

fn journal_entry_count(runtime: &GameRuntime) -> usize {
    let journal_enabled = runtime
        .content
        .rules
        .systems
        .get("journal")
        .copied()
        .unwrap_or(false);
    if !journal_enabled {
        return 0;
    }
    if runtime.content.quests.is_empty() {
        return 0;
    }
    let mut all_quest_states = Vec::new();
    for quest_file in &runtime.content.quests {
        let quest_states = quest_file.resolve_quests(&runtime.flags);
        all_quest_states.extend(quest_states);
    }
    all_quest_states.len()
}

fn map_save_allowed(runtime: &GameRuntime, map_id: &str, player_pos: (i32, i32)) -> bool {
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    map.allow_save
        || map
            .save_points
            .iter()
            .any(|pos| (pos[0], pos[1]) == player_pos)
}

pub fn system_enabled(runtime: &GameRuntime, system: Option<&str>) -> bool {
    match system {
        Some(key) if !key.trim().is_empty() => {
            if key == "jobs" && runtime.content.rules.progression_mode == ProgressionMode::Activity
            {
                return false;
            }
            runtime
                .content
                .rules
                .systems
                .get(key)
                .copied()
                .unwrap_or(false)
        }
        _ => true,
    }
}

pub fn unlock_flag_enabled(runtime: &GameRuntime, unlock_flag: Option<&str>) -> bool {
    match unlock_flag {
        Some(flag) if !flag.trim().is_empty() => runtime.has_flag(flag),
        _ => true,
    }
}

fn wrap_index(current: usize, len: usize, direction: i32) -> usize {
    if len == 0 {
        return 0;
    }
    if direction < 0 {
        if current == 0 {
            len.saturating_sub(1)
        } else {
            current - 1
        }
    } else if current + 1 >= len {
        0
    } else {
        current + 1
    }
}
