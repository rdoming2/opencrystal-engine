use std::path::Path;

use engine::menu::MenuFocus;
use engine::rules::ProgressionMode;
use engine::runtime::GameRuntime;
use tui::input::{Action, InputBindings};
use tui::menu::MenuPane;
use tui::session::TuiSession;
use tui::ui::{DialogUiFile, MenuUiFile, ProgressUiFile};

use crate::utils::read_action;

use super::abilities::{build_ability_entries, selected_ability_targets};
use super::common::{filter_from_index, sort_from_index};
use super::confirm::handle_confirm;
use super::equipment::{equipment_entries_for_menu, equipment_slots_for_menu};
use super::inventory::{
    apply_inventory_sort_action, build_inventory_entries, item_targets_for_entry,
};
use super::jobs::{job_menu_options, learnable_count};
use super::magic::{build_spell_entries, selected_spell_targets};
use super::magic_equip::{magic_equip_entries_for_menu, magic_equip_slots_for_menu};
use super::overworld::{
    move_overworld_selection, overworld_destinations_for_runtime, overworld_travel_allowed,
};
use super::panels::{
    build_menu_stats_view, build_progress_panel, menu_default_panel, menu_detail_panel,
    menu_footer_text, menu_panel_size,
};
use super::party::{party_actions, party_list_entries, PartyList};
use super::save::{build_save_slots, default_save_selection, move_save_selection, SaveMessage};
use super::settings::settings_entry_count;
use super::status::build_status_screen_view;
use super::{
    build_menu_entries, journal_entry_count, map_save_allowed, wrap_index, MenuOutcome, PanelSize,
};

pub fn run_menu_loop(
    session: &mut TuiSession,
    runtime: &mut GameRuntime,
    menu_ui: &MenuUiFile,
    progress_ui: &ProgressUiFile,
    dialog_ui: &DialogUiFile,
    bindings: &InputBindings,
    map_id: &str,
    player_pos: (i32, i32),
    save_dir: &Path,
) -> std::io::Result<MenuOutcome> {
    let mut save_message: Option<SaveMessage> = None;
    let entries = build_menu_entries(runtime, menu_ui, map_id, player_pos);
    if entries.is_empty() {
        runtime.close_menu();
        return Ok(MenuOutcome::Continue);
    }
    let entry_views = entries
        .iter()
        .map(|entry| entry.view.clone())
        .collect::<Vec<_>>();

    if runtime.menu_state.selected >= entry_views.len() {
        runtime.menu_state.selected = 0;
    }

    if runtime.menu_state.active_submenu.as_deref() == Some("save") {
        if let Some(index) = entries.iter().position(|entry| entry.action == "save") {
            runtime.menu_state.selected = index;
            runtime.menu_state.focus = MenuFocus::Detail;
        }
        let slots = build_save_slots(runtime, save_dir);
        if let Some(index) = default_save_selection(runtime, &slots) {
            runtime.menu_state.detail_selection = index;
        }
    }

    loop {
        let focus = match runtime.menu_state.focus {
            MenuFocus::List => MenuPane::List,
            MenuFocus::Detail => MenuPane::Detail,
        };
        let selected = entries.get(runtime.menu_state.selected);
        let label = selected
            .map(|entry| entry.view.label.as_str())
            .unwrap_or("Menu");
        let submenu_action = runtime
            .menu_state
            .active_submenu
            .as_deref()
            .or_else(|| selected.map(|entry| entry.action.as_str()))
            .unwrap_or("menu")
            .to_string();
        let stats_view = build_menu_stats_view(runtime);
        let panel_size = menu_panel_size(session, menu_ui, Some(&stats_view));
        let show_status = matches!(focus, MenuPane::Detail) && submenu_action == "status";
        let right_panel = if matches!(focus, MenuPane::Detail) && !show_status {
            menu_detail_panel(
                label,
                submenu_action.as_str(),
                runtime,
                progress_ui,
                runtime.menu_state.detail_page,
                save_dir,
                save_message.as_ref(),
                panel_size,
            )
        } else {
            menu_default_panel(menu_ui, progress_ui, runtime)
        };

        let allow_overworld_travel = overworld_travel_allowed(runtime);
        let footer_text = menu_footer_text(
            focus,
            submenu_action.as_str(),
            runtime.menu_state.detail_page,
            allow_overworld_travel,
        );
        if show_status {
            let status_view = build_status_screen_view(runtime, runtime.menu_state.detail_actor);
            tui::menu::draw_menu_status(
                session,
                menu_ui,
                &entry_views,
                runtime.menu_state.selected,
                focus,
                &status_view,
                footer_text,
            )?;
        } else {
            tui::menu::draw_menu(
                session,
                menu_ui,
                &entry_views,
                runtime.menu_state.selected,
                focus,
                &right_panel,
                Some(&stats_view),
                footer_text,
            )?;
        }

        if let Some(action) = read_action(bindings) {
            match action {
                Action::MoveUp => {
                    handle_move_up(
                        runtime,
                        focus,
                        submenu_action.as_str(),
                        panel_size,
                        label,
                        progress_ui,
                        save_dir,
                        map_id,
                        player_pos,
                        entry_views.len(),
                    );
                }
                Action::MoveDown => {
                    handle_move_down(
                        runtime,
                        focus,
                        submenu_action.as_str(),
                        panel_size,
                        label,
                        progress_ui,
                        save_dir,
                        map_id,
                        player_pos,
                        entry_views.len(),
                    );
                }
                Action::Confirm => {
                    if let Some(outcome) = handle_confirm(
                        session,
                        runtime,
                        menu_ui,
                        dialog_ui,
                        bindings,
                        &entries,
                        &entry_views,
                        focus,
                        &right_panel,
                        footer_text,
                        save_dir,
                        map_id,
                        player_pos,
                        submenu_action.as_str(),
                        &mut save_message,
                    )? {
                        return Ok(outcome);
                    }
                }
                Action::Cancel | Action::Menu => {
                    if matches!(focus, MenuPane::Detail) {
                        if submenu_action == "equipment" && runtime.menu_state.detail_page == 1 {
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = runtime.menu_state.detail_slot;
                        } else if submenu_action == "magic_equip"
                            && runtime.menu_state.detail_page == 1
                        {
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = runtime.menu_state.detail_slot;
                        } else if submenu_action == "items" && runtime.menu_state.detail_page == 1 {
                            runtime.menu_state.detail_page = 0;
                            if runtime.menu_state.detail_slot != usize::MAX {
                                runtime.menu_state.detail_selection =
                                    runtime.menu_state.detail_slot;
                            }
                            runtime.menu_state.detail_slot = usize::MAX;
                            runtime.menu_state.detail_target = 0;
                        } else if submenu_action == "items"
                            && (runtime.menu_state.detail_page == 2
                                || runtime.menu_state.detail_page == 3)
                        {
                            runtime.menu_state.detail_page = 1;
                            runtime.menu_state.detail_target = 0;
                        } else if submenu_action == "party" && runtime.menu_state.detail_page == 2 {
                            runtime.menu_state.detail_page = 1;
                            runtime.menu_state.detail_selection = 0;
                        } else if submenu_action == "party" && runtime.menu_state.detail_page == 1 {
                            runtime.menu_state.detail_page = 0;
                            if runtime.menu_state.detail_slot != usize::MAX {
                                runtime.menu_state.detail_selection =
                                    runtime.menu_state.detail_slot;
                            }
                            runtime.menu_state.detail_slot = usize::MAX;
                        } else if submenu_action == "magic" && runtime.menu_state.detail_page == 1 {
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_target = 0;
                        } else if submenu_action == "abilities"
                            && runtime.menu_state.detail_page == 1
                        {
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_target = 0;
                        } else if submenu_action == "journal" && runtime.menu_state.detail_page > 0
                        {
                            runtime.menu_state.detail_page = 0;
                        } else if submenu_action == "jobs" && runtime.menu_state.detail_page > 0 {
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_target = 0;
                            runtime.menu_state.detail_selection = 0;
                        } else {
                            runtime.menu_state.focus = MenuFocus::List;
                            runtime.menu_state.active_submenu = None;
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_scroll = 0;
                            runtime.menu_state.detail_selection = 0;
                            if submenu_action == "save" {
                                save_message = None;
                            }
                        }
                    } else {
                        runtime.close_menu();
                        return Ok(MenuOutcome::Continue);
                    }
                }
                Action::MoveLeft | Action::MoveRight => {
                    if matches!(focus, MenuPane::Detail) && submenu_action == "status" {
                        let actor_count = runtime.party.active_ids().len();
                        if actor_count > 0 {
                            runtime.menu_state.detail_actor = if matches!(action, Action::MoveRight)
                            {
                                (runtime.menu_state.detail_actor + 1) % actor_count
                            } else if runtime.menu_state.detail_actor == 0 {
                                actor_count - 1
                            } else {
                                runtime.menu_state.detail_actor - 1
                            };
                        }
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "settings" {
                        let direction = if matches!(action, Action::MoveRight) {
                            1
                        } else {
                            -1
                        };
                        super::settings::adjust_settings(
                            runtime,
                            runtime.menu_state.detail_selection,
                            direction,
                        );
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "items" {
                        if runtime.menu_state.detail_page == 0 {
                            runtime.menu_state.detail_filter =
                                if matches!(action, Action::MoveRight) {
                                    super::common::next_filter_index(
                                        runtime.menu_state.detail_filter,
                                    )
                                } else {
                                    super::common::prev_filter_index(
                                        runtime.menu_state.detail_filter,
                                    )
                                };
                            runtime.menu_state.detail_selection = 0;
                        }
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "equipment" {
                        let actor_count = runtime.party.active_ids().len();
                        if actor_count > 0 {
                            runtime.menu_state.detail_actor = if matches!(action, Action::MoveRight)
                            {
                                (runtime.menu_state.detail_actor + 1) % actor_count
                            } else if runtime.menu_state.detail_actor == 0 {
                                actor_count - 1
                            } else {
                                runtime.menu_state.detail_actor - 1
                            };
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = 0;
                        }
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "party" {
                        if runtime.menu_state.detail_page == 0 {
                            runtime.menu_state.detail_target =
                                if matches!(action, Action::MoveRight) {
                                    1
                                } else {
                                    0
                                };
                            let list = if runtime.menu_state.detail_target == 0 {
                                PartyList::Active
                            } else {
                                PartyList::Reserve
                            };
                            let entries = party_list_entries(runtime, list);
                            runtime.menu_state.detail_selection =
                                if entries.is_empty() { 0 } else { 0 };
                        }
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "magic_equip" {
                        let actor_count = runtime.party.active_ids().len();
                        if actor_count > 0 {
                            runtime.menu_state.detail_actor = if matches!(action, Action::MoveRight)
                            {
                                (runtime.menu_state.detail_actor + 1) % actor_count
                            } else if runtime.menu_state.detail_actor == 0 {
                                actor_count - 1
                            } else {
                                runtime.menu_state.detail_actor - 1
                            };
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = 0;
                        }
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "magic" {
                        let actor_count = runtime.party.active_ids().len();
                        if actor_count > 0 {
                            runtime.menu_state.detail_actor = if matches!(action, Action::MoveRight)
                            {
                                (runtime.menu_state.detail_actor + 1) % actor_count
                            } else if runtime.menu_state.detail_actor == 0 {
                                actor_count - 1
                            } else {
                                runtime.menu_state.detail_actor - 1
                            };
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = 0;
                        }
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "abilities" {
                        let actor_count = runtime.party.active_ids().len();
                        if actor_count > 0 {
                            runtime.menu_state.detail_actor = if matches!(action, Action::MoveRight)
                            {
                                (runtime.menu_state.detail_actor + 1) % actor_count
                            } else if runtime.menu_state.detail_actor == 0 {
                                actor_count - 1
                            } else {
                                runtime.menu_state.detail_actor - 1
                            };
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = 0;
                            runtime.menu_state.detail_target = 0;
                        }
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "jobs" {
                        if runtime.menu_state.detail_page == 0 {
                            let actor_count = runtime.party.active_ids().len();
                            if actor_count > 0 {
                                runtime.menu_state.detail_actor =
                                    if matches!(action, Action::MoveRight) {
                                        (runtime.menu_state.detail_actor + 1) % actor_count
                                    } else if runtime.menu_state.detail_actor == 0 {
                                        actor_count - 1
                                    } else {
                                        runtime.menu_state.detail_actor - 1
                                    };
                                runtime.menu_state.detail_selection = 0;
                                runtime.menu_state.detail_target = 0;
                            }
                        } else if runtime.menu_state.detail_page == 1 {
                            let jobs = runtime.content.jobs.jobs.len();
                            if jobs > 0 {
                                runtime.menu_state.detail_selection =
                                    if matches!(action, Action::MoveRight) {
                                        (runtime.menu_state.detail_selection + 1) % jobs
                                    } else if runtime.menu_state.detail_selection == 0 {
                                        jobs - 1
                                    } else {
                                        runtime.menu_state.detail_selection - 1
                                    };
                                runtime.menu_state.detail_target = 0;
                            }
                        }
                    }
                }
                Action::Pause => {
                    if matches!(focus, MenuPane::Detail)
                        && submenu_action == "items"
                        && runtime.menu_state.detail_page == 0
                    {
                        let current_entries = build_inventory_entries(
                            runtime,
                            &filter_from_index(runtime.menu_state.detail_filter),
                        );
                        let selected_id = current_entries
                            .get(runtime.menu_state.detail_selection)
                            .map(|entry| entry.id.clone());
                        runtime.menu_state.detail_sort =
                            super::common::toggle_sort_index(runtime.menu_state.detail_sort);
                        let sort = sort_from_index(runtime.menu_state.detail_sort);
                        apply_inventory_sort_action(runtime, sort);
                        let updated_entries = build_inventory_entries(
                            runtime,
                            &filter_from_index(runtime.menu_state.detail_filter),
                        );
                        if let Some(selected_id) = selected_id {
                            if let Some(index) = updated_entries
                                .iter()
                                .position(|entry| entry.id == selected_id)
                            {
                                runtime.menu_state.detail_selection = index;
                            } else {
                                runtime.menu_state.detail_selection = 0;
                            }
                        } else {
                            runtime.menu_state.detail_selection = 0;
                        }
                    }
                }
                Action::Learn => {
                    if matches!(focus, MenuPane::Detail)
                        && submenu_action == "jobs"
                        && runtime.menu_state.detail_page == 0
                        && runtime.content.rules.progression_mode == ProgressionMode::JobPoints
                    {
                        runtime.menu_state.detail_page = 2;
                        runtime.menu_state.detail_target = 0;
                    }
                }
                Action::Quit => {
                    let confirm_stats = build_menu_stats_view(runtime);
                    if tui::dialog::confirm_quit(session, |frame| {
                        if show_status {
                            let status_view =
                                build_status_screen_view(runtime, runtime.menu_state.detail_actor);
                            tui::menu::draw_menu_status_frame(
                                frame,
                                menu_ui,
                                &entry_views,
                                runtime.menu_state.selected,
                                focus,
                                &status_view,
                                footer_text,
                            );
                        } else {
                            tui::menu::draw_menu_frame(
                                frame,
                                menu_ui,
                                &entry_views,
                                runtime.menu_state.selected,
                                focus,
                                &right_panel,
                                Some(&confirm_stats),
                                footer_text,
                            );
                        }
                    })? {
                        return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "quit"));
                    }
                }
            }
        }
    }
}

fn handle_move_up(
    runtime: &mut GameRuntime,
    focus: MenuPane,
    submenu_action: &str,
    panel_size: PanelSize,
    label: &str,
    progress_ui: &ProgressUiFile,
    save_dir: &Path,
    map_id: &str,
    player_pos: (i32, i32),
    entry_views_len: usize,
) {
    if matches!(focus, MenuPane::Detail) && submenu_action == "gameplay_stats" {
        let lines_len = build_progress_panel(label.to_string(), progress_ui, runtime)
            .lines
            .len();
        let page_size = panel_size.height as usize;
        if page_size > 0 && lines_len > page_size {
            let total_pages = (lines_len + page_size - 1) / page_size;
            runtime.menu_state.detail_scroll =
                wrap_index(runtime.menu_state.detail_scroll, total_pages, -1);
        }
        return;
    }
    if matches!(focus, MenuPane::List) {
        runtime.menu_state.selected = wrap_index(runtime.menu_state.selected, entry_views_len, -1);
    } else if submenu_action == "save" {
        let slots = build_save_slots(runtime, save_dir);
        runtime.menu_state.detail_selection =
            move_save_selection(runtime.menu_state.detail_selection, &slots, -1);
    } else if submenu_action == "items" {
        let entries = build_inventory_entries(
            runtime,
            &filter_from_index(runtime.menu_state.detail_filter),
        );
        match runtime.menu_state.detail_page {
            0 => {
                runtime.menu_state.detail_selection =
                    wrap_index(runtime.menu_state.detail_selection, entries.len(), -1);
            }
            1 => {
                let list_selection = runtime
                    .menu_state
                    .detail_slot
                    .min(entries.len().saturating_sub(1));
                let action_len = super::inventory::item_actions_len(entries.get(list_selection));
                runtime.menu_state.detail_selection =
                    wrap_index(runtime.menu_state.detail_selection, action_len, -1);
            }
            2 => {
                let list_selection = runtime
                    .menu_state
                    .detail_slot
                    .min(entries.len().saturating_sub(1));
                let target_len = entries
                    .get(list_selection)
                    .map(|entry| item_targets_for_entry(runtime, entry).len())
                    .unwrap_or(0);
                runtime.menu_state.detail_target =
                    wrap_index(runtime.menu_state.detail_target, target_len, -1);
            }
            3 => {
                runtime.menu_state.detail_target =
                    wrap_index(runtime.menu_state.detail_target, entries.len(), -1);
            }
            _ => {}
        }
    } else if submenu_action == "magic" {
        if runtime.menu_state.detail_page == 0 {
            let entries = build_spell_entries(runtime);
            runtime.menu_state.detail_selection =
                wrap_index(runtime.menu_state.detail_selection, entries.len(), -1);
        } else {
            let targets = selected_spell_targets(runtime);
            runtime.menu_state.detail_target =
                wrap_index(runtime.menu_state.detail_target, targets.len(), -1);
        }
    } else if submenu_action == "abilities" {
        if runtime.menu_state.detail_page == 0 {
            let entries = build_ability_entries(runtime);
            runtime.menu_state.detail_selection =
                wrap_index(runtime.menu_state.detail_selection, entries.len(), -1);
        } else {
            let targets = selected_ability_targets(runtime);
            runtime.menu_state.detail_target =
                wrap_index(runtime.menu_state.detail_target, targets.len(), -1);
        }
    } else if submenu_action == "journal" {
        let count = journal_entry_count(runtime);
        runtime.menu_state.detail_selection =
            wrap_index(runtime.menu_state.detail_selection, count, -1);
    } else if submenu_action == "party" {
        if runtime.menu_state.detail_page == 0 {
            let list = if runtime.menu_state.detail_target == 0 {
                PartyList::Active
            } else {
                PartyList::Reserve
            };
            let entries = party_list_entries(runtime, list);
            runtime.menu_state.detail_selection =
                wrap_index(runtime.menu_state.detail_selection, entries.len(), -1);
        } else if runtime.menu_state.detail_page == 1 {
            let list = if runtime.menu_state.detail_target == 0 {
                PartyList::Active
            } else {
                PartyList::Reserve
            };
            let entries = party_list_entries(runtime, list);
            let member_index = runtime.menu_state.detail_slot;
            if member_index == usize::MAX {
                return;
            }
            let member_index = member_index.min(entries.len().saturating_sub(1));
            let swap_allowed = map_save_allowed(runtime, map_id, player_pos);
            let actions = party_actions(runtime, list, member_index, swap_allowed);
            runtime.menu_state.detail_selection =
                wrap_index(runtime.menu_state.detail_selection, actions.len(), -1);
        } else {
            let list = if runtime.menu_state.detail_target == 0 {
                PartyList::Active
            } else {
                PartyList::Reserve
            };
            let target_list = list.toggle();
            let entries = party_list_entries(runtime, target_list);
            runtime.menu_state.detail_selection =
                wrap_index(runtime.menu_state.detail_selection, entries.len(), -1);
        }
    } else if submenu_action == "overworld_map" {
        let destinations = overworld_destinations_for_runtime(runtime);
        runtime.menu_state.detail_selection =
            move_overworld_selection(runtime.menu_state.detail_selection, destinations.len(), -1);
    } else if submenu_action == "magic_equip" {
        if runtime.menu_state.detail_page == 0 {
            let slots = magic_equip_slots_for_menu(runtime);
            runtime.menu_state.detail_selection =
                wrap_index(runtime.menu_state.detail_selection, slots.len(), -1);
        } else {
            let entries = magic_equip_entries_for_menu(runtime);
            runtime.menu_state.detail_selection =
                wrap_index(runtime.menu_state.detail_selection, entries.len(), -1);
        }
    } else if submenu_action == "equipment" {
        if runtime.menu_state.detail_page == 0 {
            let slots = equipment_slots_for_menu(runtime);
            runtime.menu_state.detail_selection =
                wrap_index(runtime.menu_state.detail_selection, slots.len(), -1);
        } else {
            let entries = equipment_entries_for_menu(runtime);
            runtime.menu_state.detail_selection =
                wrap_index(runtime.menu_state.detail_selection, entries.len(), -1);
        }
    } else if submenu_action == "jobs" {
        if runtime.menu_state.detail_page == 0 {
            let options = job_menu_options(runtime);
            runtime.menu_state.detail_slot =
                wrap_index(runtime.menu_state.detail_slot, options.len(), -1);
        } else if runtime.menu_state.detail_page == 1 {
            runtime.menu_state.detail_selection = wrap_index(
                runtime.menu_state.detail_selection,
                runtime.content.jobs.jobs.len(),
                -1,
            );
        } else if runtime.menu_state.detail_page == 2 {
            let entries = learnable_count(runtime);
            runtime.menu_state.detail_target =
                wrap_index(runtime.menu_state.detail_target, entries, -1);
        }
    } else if submenu_action == "settings" {
        let limit = settings_entry_count(runtime);
        runtime.menu_state.detail_selection =
            wrap_index(runtime.menu_state.detail_selection, limit, -1);
    }
}

fn handle_move_down(
    runtime: &mut GameRuntime,
    focus: MenuPane,
    submenu_action: &str,
    panel_size: PanelSize,
    label: &str,
    progress_ui: &ProgressUiFile,
    save_dir: &Path,
    map_id: &str,
    player_pos: (i32, i32),
    entry_views_len: usize,
) {
    if matches!(focus, MenuPane::Detail) && submenu_action == "gameplay_stats" {
        let lines_len = build_progress_panel(label.to_string(), progress_ui, runtime)
            .lines
            .len();
        let page_size = panel_size.height as usize;
        if page_size > 0 && lines_len > page_size {
            let total_pages = (lines_len + page_size - 1) / page_size;
            runtime.menu_state.detail_scroll =
                wrap_index(runtime.menu_state.detail_scroll, total_pages, 1);
        }
        return;
    }
    if matches!(focus, MenuPane::List) {
        runtime.menu_state.selected = wrap_index(runtime.menu_state.selected, entry_views_len, 1);
    } else if submenu_action == "save" {
        let slots = build_save_slots(runtime, save_dir);
        runtime.menu_state.detail_selection =
            move_save_selection(runtime.menu_state.detail_selection, &slots, 1);
    } else if submenu_action == "items" {
        let entries = build_inventory_entries(
            runtime,
            &filter_from_index(runtime.menu_state.detail_filter),
        );
        match runtime.menu_state.detail_page {
            0 => {
                runtime.menu_state.detail_selection =
                    wrap_index(runtime.menu_state.detail_selection, entries.len(), 1);
            }
            1 => {
                let list_selection = runtime
                    .menu_state
                    .detail_slot
                    .min(entries.len().saturating_sub(1));
                let action_len = super::inventory::item_actions_len(entries.get(list_selection));
                runtime.menu_state.detail_selection =
                    wrap_index(runtime.menu_state.detail_selection, action_len, 1);
            }
            2 => {
                let list_selection = runtime
                    .menu_state
                    .detail_slot
                    .min(entries.len().saturating_sub(1));
                let targets = entries
                    .get(list_selection)
                    .map(|entry| item_targets_for_entry(runtime, entry))
                    .unwrap_or_default();
                runtime.menu_state.detail_target =
                    wrap_index(runtime.menu_state.detail_target, targets.len(), 1);
            }
            3 => {
                runtime.menu_state.detail_target =
                    wrap_index(runtime.menu_state.detail_target, entries.len(), 1);
            }
            _ => {}
        }
    } else if submenu_action == "magic" {
        if runtime.menu_state.detail_page == 0 {
            let entries = build_spell_entries(runtime);
            runtime.menu_state.detail_selection =
                wrap_index(runtime.menu_state.detail_selection, entries.len(), 1);
        } else {
            let targets = selected_spell_targets(runtime);
            runtime.menu_state.detail_target =
                wrap_index(runtime.menu_state.detail_target, targets.len(), 1);
        }
    } else if submenu_action == "abilities" {
        if runtime.menu_state.detail_page == 0 {
            let entries = build_ability_entries(runtime);
            runtime.menu_state.detail_selection =
                wrap_index(runtime.menu_state.detail_selection, entries.len(), 1);
        } else {
            let targets = selected_ability_targets(runtime);
            runtime.menu_state.detail_target =
                wrap_index(runtime.menu_state.detail_target, targets.len(), 1);
        }
    } else if submenu_action == "equipment" {
        let limit = if runtime.menu_state.detail_page == 0 {
            equipment_slots_for_menu(runtime).len()
        } else {
            equipment_entries_for_menu(runtime).len()
        };
        runtime.menu_state.detail_selection =
            wrap_index(runtime.menu_state.detail_selection, limit, 1);
    } else if submenu_action == "journal" {
        let count = journal_entry_count(runtime);
        runtime.menu_state.detail_selection =
            wrap_index(runtime.menu_state.detail_selection, count, 1);
    } else if submenu_action == "party" {
        if runtime.menu_state.detail_page == 0 {
            let list = if runtime.menu_state.detail_target == 0 {
                PartyList::Active
            } else {
                PartyList::Reserve
            };
            let entries = party_list_entries(runtime, list);
            runtime.menu_state.detail_selection =
                wrap_index(runtime.menu_state.detail_selection, entries.len(), 1);
        } else if runtime.menu_state.detail_page == 1 {
            let list = if runtime.menu_state.detail_target == 0 {
                PartyList::Active
            } else {
                PartyList::Reserve
            };
            let member_index = runtime.menu_state.detail_slot;
            if member_index == usize::MAX {
                return;
            }
            let swap_allowed = map_save_allowed(runtime, map_id, player_pos);
            let actions = party_actions(runtime, list, member_index, swap_allowed);
            runtime.menu_state.detail_selection =
                wrap_index(runtime.menu_state.detail_selection, actions.len(), 1);
        } else if runtime.menu_state.detail_page == 2 {
            let list = if runtime.menu_state.detail_target == 0 {
                PartyList::Active
            } else {
                PartyList::Reserve
            };
            let target_entries = party_list_entries(runtime, list.toggle());
            runtime.menu_state.detail_selection =
                wrap_index(runtime.menu_state.detail_selection, target_entries.len(), 1);
        }
    } else if submenu_action == "overworld_map" {
        let destinations = overworld_destinations_for_runtime(runtime);
        runtime.menu_state.detail_selection =
            move_overworld_selection(runtime.menu_state.detail_selection, destinations.len(), 1);
    } else if submenu_action == "magic_equip" {
        let limit = if runtime.menu_state.detail_page == 0 {
            magic_equip_slots_for_menu(runtime).len()
        } else {
            magic_equip_entries_for_menu(runtime).len()
        };
        runtime.menu_state.detail_selection =
            wrap_index(runtime.menu_state.detail_selection, limit, 1);
    } else if submenu_action == "jobs" {
        if runtime.menu_state.detail_page == 0 {
            let options = job_menu_options(runtime);
            runtime.menu_state.detail_slot =
                wrap_index(runtime.menu_state.detail_slot, options.len(), 1);
        } else if runtime.menu_state.detail_page == 1 {
            runtime.menu_state.detail_selection = wrap_index(
                runtime.menu_state.detail_selection,
                runtime.content.jobs.jobs.len(),
                1,
            );
        } else if runtime.menu_state.detail_page == 2 {
            let entries = learnable_count(runtime);
            runtime.menu_state.detail_target =
                wrap_index(runtime.menu_state.detail_target, entries, 1);
        }
    } else if submenu_action == "settings" {
        let limit = settings_entry_count(runtime);
        runtime.menu_state.detail_selection =
            wrap_index(runtime.menu_state.detail_selection, limit, 1);
    }
}
