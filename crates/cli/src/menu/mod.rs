pub mod abilities;
pub mod common;
pub mod equipment;
pub mod inventory;
pub mod jobs;
pub mod journal;
pub mod magic;
pub mod magic_equip;
pub mod party;
pub mod settings;
pub mod status;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use engine::maps::MapFile;
use engine::menu::MenuFocus;
use engine::party::{actor_row_label, get_actor_max_charges, job_jp};
use engine::rules::JobProgressionMode;
use engine::runtime::GameRuntime;
use engine::save::SaveFile;
use tui::input::{Action, InputBindings};
use tui::menu::{MenuEntryView, MenuPane, MenuPanelLine, MenuPanelView, PanelSpanStyle};
use tui::session::TuiSession;
use tui::ui::{DialogUiFile, MenuUiFile, ProgressUiFile};

use crate::utils::read_action;

use self::abilities::{
    ability_targets_for_entry, build_abilities_panel, build_ability_entries,
    selected_ability_targets,
};
use self::common::{
    filter_from_index, next_filter_index, prev_filter_index, sort_from_index, toggle_sort_index,
    InventoryKind, MenuEntryState,
};
use self::equipment::{
    build_equipment_panel, detail_actor_id, equip_item, equipment_entries_for_menu,
    equipment_slot_for_menu, equipment_slots_for_menu,
};
use self::inventory::{
    apply_item_to_targets, build_inventory_entries, build_items_panel, item_targets_for_entry,
    panel_line, panel_line_spans, panel_span,
};
use self::jobs::{
    apply_learn_purchase, apply_primary_change, apply_secondary_change, build_job_picker,
    build_jobs_dashboard, build_learn_panel, job_menu_options, learnable_count, JobMenuOption,
};
use self::journal::{build_journal_detail_panel, build_journal_panel};
use self::magic::{
    apply_spell_to_targets, build_magic_panel, build_spell_entries, selected_spell_targets,
    spell_targets_for_entry,
};
use self::magic_equip::{
    build_magic_equip_panel, magic_equip_entries_for_menu, magic_equip_slot_for_menu,
    magic_equip_slots_for_menu,
};
use self::party::{
    build_party_panel, party_actions, party_list_entries, party_member_id, PartyActionId, PartyList,
};
use self::settings::{
    adjust_settings, apply_settings_confirm, build_settings_panel, settings_entry_count,
};
use self::status::build_status_panel;

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
) -> std::io::Result<()> {
    let mut save_message: Option<SaveMessage> = None;
    let entries = build_menu_entries(runtime, menu_ui, map_id, player_pos);
    if entries.is_empty() {
        runtime.close_menu();
        return Ok(());
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
        if let Some(index) = first_selectable_save_slot(&slots) {
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
        let right_panel = if matches!(focus, MenuPane::Detail) {
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

        let footer_text = menu_footer_text(
            focus,
            submenu_action.as_str(),
            runtime.menu_state.detail_page,
        );
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

        if let Some(action) = read_action(bindings) {
            match action {
                Action::MoveUp => {
                    if matches!(focus, MenuPane::List) {
                        if runtime.menu_state.selected > 0 {
                            runtime.menu_state.selected -= 1;
                        }
                    } else if submenu_action == "save" {
                        let slots = build_save_slots(runtime, save_dir);
                        runtime.menu_state.detail_selection =
                            move_save_selection(runtime.menu_state.detail_selection, &slots, -1);
                    } else if submenu_action == "items" {
                        if runtime.menu_state.detail_page == 0 {
                            if runtime.menu_state.detail_selection > 0 {
                                runtime.menu_state.detail_selection -= 1;
                            }
                        } else if runtime.menu_state.detail_target > 0 {
                            runtime.menu_state.detail_target -= 1;
                        }
                    } else if submenu_action == "magic" {
                        if runtime.menu_state.detail_page == 0 {
                            if runtime.menu_state.detail_selection > 0 {
                                runtime.menu_state.detail_selection -= 1;
                            }
                        } else if runtime.menu_state.detail_target > 0 {
                            runtime.menu_state.detail_target -= 1;
                        }
                    } else if submenu_action == "abilities" {
                        if runtime.menu_state.detail_page == 0 {
                            if runtime.menu_state.detail_selection > 0 {
                                runtime.menu_state.detail_selection -= 1;
                            }
                        } else if runtime.menu_state.detail_target > 0 {
                            runtime.menu_state.detail_target -= 1;
                        }
                    } else if submenu_action == "journal" {
                        if runtime.menu_state.detail_selection > 0 {
                            runtime.menu_state.detail_selection -= 1;
                        }
                    } else if submenu_action == "party" {
                        if runtime.menu_state.detail_page == 0 {
                            if runtime.menu_state.detail_selection > 0 {
                                runtime.menu_state.detail_selection -= 1;
                            }
                        } else if runtime.menu_state.detail_selection > 0 {
                            runtime.menu_state.detail_selection -= 1;
                        }
                    } else if submenu_action == "overworld_map" || submenu_action == "fast_travel" {
                        let destinations = overworld_destinations_for_runtime(runtime);
                        runtime.menu_state.detail_selection = move_overworld_selection(
                            runtime.menu_state.detail_selection,
                            destinations.len(),
                            -1,
                        );
                    } else if submenu_action == "magic_equip" {
                        if runtime.menu_state.detail_selection > 0 {
                            runtime.menu_state.detail_selection -= 1;
                        }
                    } else if submenu_action == "jobs" {
                        if runtime.menu_state.detail_page == 0 {
                            if runtime.menu_state.detail_slot > 0 {
                                runtime.menu_state.detail_slot -= 1;
                            }
                        } else if runtime.menu_state.detail_page == 1 {
                            if runtime.menu_state.detail_selection > 0 {
                                runtime.menu_state.detail_selection -= 1;
                            }
                        } else if runtime.menu_state.detail_page == 2 {
                            if runtime.menu_state.detail_target > 0 {
                                runtime.menu_state.detail_target -= 1;
                            }
                        }
                    } else if submenu_action == "settings" {
                        if runtime.menu_state.detail_selection > 0 {
                            runtime.menu_state.detail_selection -= 1;
                        }
                    }
                }
                Action::MoveDown => {
                    if matches!(focus, MenuPane::List) {
                        if runtime.menu_state.selected + 1 < entry_views.len() {
                            runtime.menu_state.selected += 1;
                        }
                    } else if submenu_action == "save" {
                        let slots = build_save_slots(runtime, save_dir);
                        runtime.menu_state.detail_selection =
                            move_save_selection(runtime.menu_state.detail_selection, &slots, 1);
                    } else if submenu_action == "items" {
                        if runtime.menu_state.detail_page == 0 {
                            let entries = build_inventory_entries(
                                runtime,
                                &filter_from_index(runtime.menu_state.detail_filter),
                                &sort_from_index(runtime.menu_state.detail_sort),
                            );
                            if runtime.menu_state.detail_selection + 1 < entries.len() {
                                runtime.menu_state.detail_selection += 1;
                            }
                        } else {
                            let entries = build_inventory_entries(
                                runtime,
                                &filter_from_index(runtime.menu_state.detail_filter),
                                &sort_from_index(runtime.menu_state.detail_sort),
                            );
                            let targets = entries
                                .get(runtime.menu_state.detail_selection)
                                .map(|entry| item_targets_for_entry(runtime, entry))
                                .unwrap_or_default();
                            if runtime.menu_state.detail_target + 1 < targets.len() {
                                runtime.menu_state.detail_target += 1;
                            }
                        }
                    } else if submenu_action == "magic" {
                        if runtime.menu_state.detail_page == 0 {
                            let entries = build_spell_entries(runtime);
                            if runtime.menu_state.detail_selection + 1 < entries.len() {
                                runtime.menu_state.detail_selection += 1;
                            }
                        } else {
                            let targets = selected_spell_targets(runtime);
                            if runtime.menu_state.detail_target + 1 < targets.len() {
                                runtime.menu_state.detail_target += 1;
                            }
                        }
                    } else if submenu_action == "abilities" {
                        if runtime.menu_state.detail_page == 0 {
                            let entries = build_ability_entries(runtime);
                            if runtime.menu_state.detail_selection + 1 < entries.len() {
                                runtime.menu_state.detail_selection += 1;
                            }
                        } else {
                            let targets = selected_ability_targets(runtime);
                            if runtime.menu_state.detail_target + 1 < targets.len() {
                                runtime.menu_state.detail_target += 1;
                            }
                        }
                    } else if submenu_action == "equipment" {
                        let limit = if runtime.menu_state.detail_page == 0 {
                            equipment_slots_for_menu(runtime).len()
                        } else {
                            equipment_entries_for_menu(runtime).len()
                        };
                        if runtime.menu_state.detail_selection + 1 < limit {
                            runtime.menu_state.detail_selection += 1;
                        }
                    } else if submenu_action == "journal" {
                        let mut all_quest_states = Vec::new();
                        for quest_file in &runtime.content.quests {
                            let quest_states = quest_file.resolve_quests(&runtime.flags);
                            all_quest_states.extend(quest_states);
                        }
                        if runtime.menu_state.detail_selection + 1 < all_quest_states.len() {
                            runtime.menu_state.detail_selection += 1;
                        }
                    } else if submenu_action == "party" {
                        if runtime.menu_state.detail_page == 0 {
                            let list = if runtime.menu_state.detail_target == 0 {
                                PartyList::Active
                            } else {
                                PartyList::Reserve
                            };
                            let entries = party_list_entries(runtime, list);
                            if runtime.menu_state.detail_selection + 1 < entries.len() {
                                runtime.menu_state.detail_selection += 1;
                            }
                        } else if runtime.menu_state.detail_page == 1 {
                            let list = if runtime.menu_state.detail_target == 0 {
                                PartyList::Active
                            } else {
                                PartyList::Reserve
                            };
                            let member_index = runtime.menu_state.detail_slot;
                            if member_index == usize::MAX {
                                continue;
                            }
                            let swap_allowed = map_save_allowed(runtime, map_id, player_pos);
                            let actions = party_actions(runtime, list, member_index, swap_allowed);
                            if runtime.menu_state.detail_selection + 1 < actions.len() {
                                runtime.menu_state.detail_selection += 1;
                            }
                        } else if runtime.menu_state.detail_page == 2 {
                            let list = if runtime.menu_state.detail_target == 0 {
                                PartyList::Active
                            } else {
                                PartyList::Reserve
                            };
                            let target_entries = party_list_entries(runtime, list.toggle());
                            if runtime.menu_state.detail_selection + 1 < target_entries.len() {
                                runtime.menu_state.detail_selection += 1;
                            }
                        }
                    } else if submenu_action == "overworld_map" || submenu_action == "fast_travel" {
                        let destinations = overworld_destinations_for_runtime(runtime);
                        runtime.menu_state.detail_selection = move_overworld_selection(
                            runtime.menu_state.detail_selection,
                            destinations.len(),
                            1,
                        );
                    } else if submenu_action == "magic_equip" {
                        let limit = if runtime.menu_state.detail_page == 0 {
                            magic_equip_slots_for_menu(runtime).len()
                        } else {
                            magic_equip_entries_for_menu(runtime).len()
                        };
                        if runtime.menu_state.detail_selection + 1 < limit {
                            runtime.menu_state.detail_selection += 1;
                        }
                    } else if submenu_action == "jobs" {
                        if runtime.menu_state.detail_page == 0 {
                            let options = job_menu_options(runtime);
                            let limit = options.len();
                            if runtime.menu_state.detail_slot + 1 < limit {
                                runtime.menu_state.detail_slot += 1;
                            }
                        } else if runtime.menu_state.detail_page == 1 {
                            if runtime.menu_state.detail_selection + 1
                                < runtime.content.jobs.jobs.len()
                            {
                                runtime.menu_state.detail_selection += 1;
                            }
                        } else if runtime.menu_state.detail_page == 2 {
                            let entries = learnable_count(runtime);
                            if runtime.menu_state.detail_target + 1 < entries {
                                runtime.menu_state.detail_target += 1;
                            }
                        }
                    } else if submenu_action == "settings" {
                        let limit = settings_entry_count(runtime);
                        if runtime.menu_state.detail_selection + 1 < limit {
                            runtime.menu_state.detail_selection += 1;
                        }
                    }
                }
                Action::Confirm => {
                    if matches!(focus, MenuPane::List) {
                        if let Some(entry) = entries.get(runtime.menu_state.selected) {
                            if entry.selectable {
                                match entry.action.as_str() {
                                    "items" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = 0;
                                        runtime.menu_state.detail_filter = 0;
                                        runtime.menu_state.detail_sort = 0;
                                        runtime.menu_state.detail_target = 0;
                                    }
                                    "magic" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = 0;
                                        runtime.menu_state.detail_actor = 0;
                                        runtime.menu_state.detail_target = 0;
                                    }
                                    "equipment" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = 0;
                                        runtime.menu_state.detail_actor = 0;
                                        runtime.menu_state.detail_slot = 0;
                                    }
                                    "magic_equip" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = 0;
                                        runtime.menu_state.detail_actor = 0;
                                        runtime.menu_state.detail_slot = 0;
                                    }
                                    "abilities" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = 0;
                                        runtime.menu_state.detail_actor = 0;
                                        runtime.menu_state.detail_target = 0;
                                    }
                                    "party" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = 0;
                                        runtime.menu_state.detail_slot = usize::MAX;
                                        runtime.menu_state.detail_target = 0;
                                    }
                                    "journal" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = 0;
                                    }
                                    "overworld_map" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = 0;
                                    }
                                    "fast_travel" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = 0;
                                    }
                                    "save" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        let slots = build_save_slots(runtime, save_dir);
                                        runtime.menu_state.detail_selection =
                                            first_selectable_save_slot(&slots).unwrap_or(0);
                                        save_message = None;
                                    }
                                    "settings" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = 0;
                                    }
                                    "jobs" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_slot = 0;
                                        runtime.menu_state.detail_selection = 0;
                                        runtime.menu_state.detail_target = 0;
                                    }
                                    _ => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                    }
                                }
                            } else if submenu_action == "jobs" {
                                apply_secondary_change(runtime);
                            }
                        }
                    } else if submenu_action == "settings" {
                        apply_settings_confirm(runtime, runtime.menu_state.detail_selection);
                    } else if submenu_action == "save" {
                        let slots = build_save_slots(runtime, save_dir);
                        if let Some(slot) = slots.get(runtime.menu_state.detail_selection) {
                            if slot.selectable {
                                match write_save_slot(runtime, save_dir, slot.slot) {
                                    Ok(()) => {
                                        save_message = Some(SaveMessage {
                                            text: "Saved.".to_string(),
                                            style: PanelSpanStyle::Accent,
                                        });
                                    }
                                    Err(err) => {
                                        eprintln!("Failed to save: {}", err);
                                        save_message = Some(SaveMessage {
                                            text: "Save failed.".to_string(),
                                            style: PanelSpanStyle::Muted,
                                        });
                                    }
                                }
                            }
                        }
                    } else if submenu_action == "fast_travel" {
                        let destinations = overworld_destinations_for_runtime(runtime);
                        if destinations.is_empty() {
                            continue;
                        }
                        let selection = runtime
                            .menu_state
                            .detail_selection
                            .min(destinations.len().saturating_sub(1));
                        let destination = match destinations.get(selection) {
                            Some(destination) => destination,
                            None => continue,
                        };
                        if !destination.enabled {
                            continue;
                        }
                        if let Some(cost) = &destination.cost {
                            runtime.inventory.add_currency(&cost.id, -cost.amount);
                        }
                        runtime.warp_to_map(&destination.map_id, destination.target_pos);
                        runtime.close_menu();
                        return Ok(());
                    } else if submenu_action == "items" {
                        let entries = build_inventory_entries(
                            runtime,
                            &filter_from_index(runtime.menu_state.detail_filter),
                            &sort_from_index(runtime.menu_state.detail_sort),
                        );
                        if let Some(entry) = entries.get(runtime.menu_state.detail_selection) {
                            if entry.kind == InventoryKind::Item && entry.usable {
                                if runtime.menu_state.detail_page == 0 {
                                    let targets = item_targets_for_entry(runtime, entry);
                                    if entry.usage_target == "party" {
                                        let result =
                                            apply_item_to_targets(runtime, entry, &targets);
                                        if result.consumed {
                                            runtime.inventory.remove_item(&entry.id, 1);
                                        }
                                        if result.consumed {
                                            if let Some(message) = result.warp_message {
                                                tui::dialog::show_dialog(
                                                    session, dialog_ui, bindings, "", &message,
                                                )?;
                                            }
                                        }
                                    } else if targets.is_empty() {
                                        runtime.menu_state.detail_page = 0;
                                    } else {
                                        runtime.menu_state.detail_page = 1;
                                        runtime.menu_state.detail_target = 0;
                                    }
                                } else {
                                    let targets = item_targets_for_entry(runtime, entry);
                                    if let Some(target_id) =
                                        targets.get(runtime.menu_state.detail_target)
                                    {
                                        let result = apply_item_to_targets(
                                            runtime,
                                            entry,
                                            &[target_id.clone()],
                                        );
                                        if result.consumed {
                                            runtime.inventory.remove_item(&entry.id, 1);
                                        }
                                        if result.consumed {
                                            if let Some(message) = result.warp_message {
                                                tui::dialog::show_dialog(
                                                    session, dialog_ui, bindings, "", &message,
                                                )?;
                                            }
                                        }
                                    }
                                    runtime.menu_state.detail_page = 0;
                                    runtime.menu_state.detail_target = 0;
                                }
                                runtime.menu_state.detail_selection = runtime
                                    .menu_state
                                    .detail_selection
                                    .min(entries.len().saturating_sub(1));
                            }
                        }
                    } else if submenu_action == "magic" {
                        let entries = build_spell_entries(runtime);
                        let selection = runtime
                            .menu_state
                            .detail_selection
                            .min(entries.len().saturating_sub(1));
                        let actor_id = match detail_actor_id(runtime) {
                            Some(actor_id) => actor_id,
                            None => continue,
                        };
                        if let Some(entry) = entries.get(selection) {
                            if entry.usable {
                                if runtime.menu_state.detail_page == 0 {
                                    let targets =
                                        spell_targets_for_entry(runtime, entry, &actor_id);
                                    if entry.default_target == "party"
                                        || entry.default_target == "self"
                                    {
                                        apply_spell_to_targets(runtime, entry, &actor_id, &targets);
                                    } else if targets.is_empty() {
                                        runtime.menu_state.detail_page = 0;
                                    } else {
                                        runtime.menu_state.detail_page = 1;
                                        runtime.menu_state.detail_target = 0;
                                    }
                                } else {
                                    let targets =
                                        spell_targets_for_entry(runtime, entry, &actor_id);
                                    if let Some(target_id) =
                                        targets.get(runtime.menu_state.detail_target)
                                    {
                                        apply_spell_to_targets(
                                            runtime,
                                            entry,
                                            &actor_id,
                                            &[target_id.clone()],
                                        );
                                    }
                                    runtime.menu_state.detail_page = 0;
                                    runtime.menu_state.detail_target = 0;
                                }
                            }
                            runtime.menu_state.detail_selection = selection;
                        }
                    } else if submenu_action == "abilities" {
                        let entries = build_ability_entries(runtime);
                        let selection = runtime
                            .menu_state
                            .detail_selection
                            .min(entries.len().saturating_sub(1));
                        let actor_id = match detail_actor_id(runtime) {
                            Some(actor_id) => actor_id,
                            None => continue,
                        };
                        if let Some(entry) = entries.get(selection) {
                            if runtime.menu_state.detail_page == 0 {
                                let targets = ability_targets_for_entry(runtime, entry, &actor_id);
                                if entry.default_target == "party" || entry.default_target == "self"
                                {
                                    runtime.menu_state.detail_page = 0;
                                } else if targets.is_empty() {
                                    runtime.menu_state.detail_page = 0;
                                } else {
                                    runtime.menu_state.detail_page = 1;
                                    runtime.menu_state.detail_target = 0;
                                }
                            } else {
                                runtime.menu_state.detail_page = 0;
                                runtime.menu_state.detail_target = 0;
                            }
                            runtime.menu_state.detail_selection = selection;
                        }
                    } else if submenu_action == "equipment" {
                        if runtime.menu_state.detail_page == 0 {
                            runtime.menu_state.detail_slot = runtime.menu_state.detail_selection;
                            runtime.menu_state.detail_page = 1;
                            runtime.menu_state.detail_selection = 0;
                        } else {
                            let entries = equipment_entries_for_menu(runtime);
                            if let Some(entry) = entries.get(runtime.menu_state.detail_selection) {
                                if entry.usable {
                                    let slot = equipment_slot_for_menu(runtime);
                                    if let Some(slot) = slot {
                                        let actor_id = detail_actor_id(runtime);
                                        if let Some(actor_id) = actor_id {
                                            equip_item(runtime, &actor_id, &slot, entry);
                                        }
                                    }
                                }
                            }
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = runtime.menu_state.detail_slot;
                        }
                    } else if submenu_action == "party" {
                        let swap_allowed = map_save_allowed(runtime, map_id, player_pos);
                        let list = if runtime.menu_state.detail_target == 0 {
                            PartyList::Active
                        } else {
                            PartyList::Reserve
                        };
                        if runtime.menu_state.detail_page == 0 {
                            let entries = party_list_entries(runtime, list);
                            if entries.is_empty() {
                                continue;
                            }
                            let selection = runtime
                                .menu_state
                                .detail_selection
                                .min(entries.len().saturating_sub(1));
                            runtime.menu_state.detail_slot = selection;
                            runtime.menu_state.detail_page = 1;
                            runtime.menu_state.detail_selection = 0;
                        } else if runtime.menu_state.detail_page == 1 {
                            let member_index = runtime.menu_state.detail_slot;
                            if member_index == usize::MAX {
                                continue;
                            }
                            let actions = party_actions(runtime, list, member_index, swap_allowed);
                            let Some(action) = actions.get(runtime.menu_state.detail_selection)
                            else {
                                continue;
                            };
                            if !action.enabled {
                                continue;
                            }
                            match action.id {
                                PartyActionId::MoveUp => {
                                    if list == PartyList::Active && member_index > 0 {
                                        runtime.party.active.swap(member_index, member_index - 1);
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = member_index - 1;
                                        runtime.menu_state.detail_slot = usize::MAX;
                                    }
                                }
                                PartyActionId::MoveDown => {
                                    if list == PartyList::Active
                                        && member_index + 1 < runtime.party.active.len()
                                    {
                                        runtime.party.active.swap(member_index, member_index + 1);
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = member_index + 1;
                                        runtime.menu_state.detail_slot = usize::MAX;
                                    }
                                }
                                PartyActionId::Swap => {
                                    if swap_allowed {
                                        runtime.menu_state.detail_page = 2;
                                        runtime.menu_state.detail_selection = 0;
                                    }
                                }
                                PartyActionId::ToggleRow => {
                                    if let Some(actor_id) =
                                        party_member_id(runtime, list, member_index)
                                    {
                                        if let Some(actor) = runtime.party.roster.get_mut(&actor_id)
                                        {
                                            engine::party::toggle_actor_row(actor);
                                        }
                                    }
                                    runtime.menu_state.detail_page = 0;
                                    runtime.menu_state.detail_selection = member_index;
                                    runtime.menu_state.detail_slot = usize::MAX;
                                }
                            }
                        } else if runtime.menu_state.detail_page == 2 {
                            if !swap_allowed {
                                continue;
                            }
                            let member_index = runtime.menu_state.detail_slot;
                            if member_index == usize::MAX {
                                continue;
                            }
                            let target_entries = party_list_entries(runtime, list.toggle());
                            if target_entries.is_empty() {
                                continue;
                            }
                            let target_index = runtime
                                .menu_state
                                .detail_selection
                                .min(target_entries.len().saturating_sub(1));
                            match list {
                                PartyList::Active => {
                                    if member_index < runtime.party.active.len()
                                        && target_index < runtime.party.reserve.len()
                                    {
                                        let reserve_id =
                                            runtime.party.reserve[target_index].clone();
                                        let active_id = runtime.party.active[member_index].clone();
                                        runtime.party.active[member_index] = reserve_id;
                                        runtime.party.reserve[target_index] = active_id;
                                    }
                                }
                                PartyList::Reserve => {
                                    if member_index < runtime.party.reserve.len()
                                        && target_index < runtime.party.active.len()
                                    {
                                        let active_id = runtime.party.active[target_index].clone();
                                        let reserve_id =
                                            runtime.party.reserve[member_index].clone();
                                        runtime.party.active[target_index] = reserve_id;
                                        runtime.party.reserve[member_index] = active_id;
                                    }
                                }
                            }
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = member_index
                                .min(party_list_entries(runtime, list).len().saturating_sub(1));
                            runtime.menu_state.detail_slot = usize::MAX;
                        }
                    } else if submenu_action == "magic_equip" {
                        if runtime.menu_state.detail_page == 0 {
                            runtime.menu_state.detail_slot = runtime.menu_state.detail_selection;
                            runtime.menu_state.detail_page = 1;
                            runtime.menu_state.detail_selection = 0;
                        } else {
                            let entries = magic_equip_entries_for_menu(runtime);
                            if let Some(entry) = entries.get(runtime.menu_state.detail_selection) {
                                if entry.usable {
                                    let slot = magic_equip_slot_for_menu(runtime);
                                    if let Some(slot) = slot {
                                        let actor_id = detail_actor_id(runtime);
                                        if let Some(actor_id) = actor_id {
                                            equip_item(runtime, &actor_id, &slot, entry);
                                        }
                                    }
                                }
                            }
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = runtime.menu_state.detail_slot;
                        }
                    } else if submenu_action == "journal" {
                        if runtime.menu_state.detail_page == 0 {
                            let mut all_quest_states = Vec::new();
                            for quest_file in &runtime.content.quests {
                                let quest_states = quest_file.resolve_quests(&runtime.flags);
                                all_quest_states.extend(quest_states);
                            }
                            if !all_quest_states.is_empty() {
                                runtime.menu_state.detail_page = 1;
                            }
                        }
                    } else if submenu_action == "jobs" {
                        if runtime.menu_state.detail_page == 0 {
                            let options = job_menu_options(runtime);
                            let selected = runtime
                                .menu_state
                                .detail_slot
                                .min(options.len().saturating_sub(1));
                            if let Some(option) = options.get(selected) {
                                match option {
                                    JobMenuOption::Primary | JobMenuOption::Secondary => {
                                        runtime.menu_state.detail_page = 1;
                                        runtime.menu_state.detail_selection = 0;
                                    }
                                    JobMenuOption::Learn => {
                                        runtime.menu_state.detail_page = 2;
                                        runtime.menu_state.detail_target = 0;
                                    }
                                }
                            }
                        } else if runtime.menu_state.detail_page == 1 {
                            let options = job_menu_options(runtime);
                            let selected = runtime
                                .menu_state
                                .detail_slot
                                .min(options.len().saturating_sub(1));
                            if let Some(option) = options.get(selected) {
                                match option {
                                    JobMenuOption::Primary => apply_primary_change(runtime),
                                    JobMenuOption::Secondary => apply_secondary_change(runtime),
                                    JobMenuOption::Learn => {}
                                }
                            }
                            runtime.menu_state.detail_page = 0;
                        } else if runtime.menu_state.detail_page == 2 {
                            apply_learn_purchase(runtime);
                        }
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
                            runtime.menu_state.detail_selection = 0;
                            if submenu_action == "save" {
                                save_message = None;
                            }
                        }
                    } else {
                        runtime.close_menu();
                        return Ok(());
                    }
                }
                Action::MoveLeft | Action::MoveRight => {
                    if matches!(focus, MenuPane::Detail) && submenu_action == "status" {
                        runtime.menu_state.detail_page = if runtime.menu_state.detail_page == 0 {
                            1
                        } else {
                            0
                        };
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "settings" {
                        let direction = if matches!(action, Action::MoveRight) {
                            1
                        } else {
                            -1
                        };
                        adjust_settings(runtime, runtime.menu_state.detail_selection, direction);
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "items" {
                        if runtime.menu_state.detail_page == 0 {
                            runtime.menu_state.detail_filter =
                                if matches!(action, Action::MoveRight) {
                                    next_filter_index(runtime.menu_state.detail_filter)
                                } else {
                                    prev_filter_index(runtime.menu_state.detail_filter)
                                };
                            runtime.menu_state.detail_selection = 0;
                        }
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "equipment" {
                        let actor_count = runtime.party.active.len();
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
                            if entries.is_empty() {
                                runtime.menu_state.detail_selection = 0;
                            } else if runtime.menu_state.detail_selection >= entries.len() {
                                runtime.menu_state.detail_selection = entries.len() - 1;
                            }
                        }
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "magic_equip" {
                        let actor_count = runtime.party.active.len();
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
                        let actor_count = runtime.party.active.len();
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
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "abilities" {
                        let actor_count = runtime.party.active.len();
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
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "journal" {
                        if runtime.menu_state.detail_page > 0 {
                            runtime.menu_state.detail_page = if runtime.menu_state.detail_page == 1
                            {
                                2
                            } else {
                                1
                            };
                        }
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "jobs" {
                        if runtime.menu_state.detail_page == 0 {
                            let actor_count = runtime.party.active.len();
                            if actor_count > 0 {
                                runtime.menu_state.detail_actor =
                                    if matches!(action, Action::MoveRight) {
                                        (runtime.menu_state.detail_actor + 1) % actor_count
                                    } else if runtime.menu_state.detail_actor == 0 {
                                        actor_count - 1
                                    } else {
                                        runtime.menu_state.detail_actor - 1
                                    };
                                runtime.menu_state.detail_page = 0;
                                runtime.menu_state.detail_selection = 0;
                            }
                        } else if runtime.menu_state.detail_page == 2 {
                            // Learn view navigation
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
                        runtime.menu_state.detail_sort =
                            toggle_sort_index(runtime.menu_state.detail_sort);
                    }
                }
                Action::Learn => {
                    if matches!(focus, MenuPane::Detail)
                        && submenu_action == "jobs"
                        && runtime.menu_state.detail_page == 0
                        && runtime.content.rules.job_system.progression_mode
                            == JobProgressionMode::JobPoints
                    {
                        runtime.menu_state.detail_page = 2;
                        runtime.menu_state.detail_target = 0;
                    }
                }
                Action::Quit => {
                    let confirm_stats = build_menu_stats_view(runtime);
                    if tui::dialog::confirm_quit(session, |frame| {
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
                    })? {
                        return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "quit"));
                    }
                }
            }
        }
    }
}

fn build_menu_entries(
    runtime: &GameRuntime,
    menu_ui: &MenuUiFile,
    map_id: &str,
    player_pos: (i32, i32),
) -> Vec<MenuEntryState> {
    let save_allowed = map_save_allowed(runtime, map_id, player_pos);
    let overview_available = overworld_map_available(runtime);
    let fast_travel_available = fast_travel_enabled(runtime);
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
            if entry.action == "fast_travel" && (!overview_available || !fast_travel_available) {
                selectable = false;
            }
            let show = selectable
                || (!entry.enabled && entry.locked_behavior.as_deref() == Some("disable"))
                || (!unlock_enabled && entry.locked_behavior.as_deref() == Some("disable"))
                || (entry.action == "fast_travel"
                    && !selectable
                    && entry.locked_behavior.as_deref() == Some("disable"))
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

fn menu_detail_panel(
    label: &str,
    action: &str,
    runtime: &GameRuntime,
    progress_ui: &ProgressUiFile,
    page: usize,
    save_dir: &Path,
    save_message: Option<&SaveMessage>,
    panel_size: PanelSize,
) -> MenuPanelView {
    if action == "status" {
        return MenuPanelView {
            title: "Status".to_string(),
            lines: build_status_panel(runtime, page),
        };
    }
    if action == "items" {
        return build_items_panel(runtime);
    }
    if action == "equipment" {
        return build_equipment_panel(runtime);
    }
    if action == "magic_equip" {
        return build_magic_equip_panel(runtime);
    }
    if action == "jobs" {
        if page == 2 {
            return build_learn_panel(runtime);
        }
        if page == 1 {
            return build_job_picker(runtime);
        }
        return build_jobs_dashboard(runtime);
    }
    if action == "magic" {
        return build_magic_panel(runtime);
    }
    if action == "abilities" {
        return build_abilities_panel(runtime);
    }
    if action == "party" {
        let list = if runtime.menu_state.detail_target == 0 {
            PartyList::Active
        } else {
            PartyList::Reserve
        };
        let selected_member = if runtime.menu_state.detail_slot == usize::MAX {
            None
        } else {
            Some(runtime.menu_state.detail_slot)
        };
        let swap_allowed = map_save_allowed(runtime, &runtime.world.map_id, runtime.world.position);
        return build_party_panel(
            runtime,
            list,
            runtime.menu_state.detail_page,
            runtime.menu_state.detail_selection,
            runtime.menu_state.detail_selection,
            selected_member,
            swap_allowed,
        );
    }
    if action == "journal" {
        let lines = if page == 0 {
            build_journal_panel(runtime, runtime.menu_state.detail_selection)
        } else {
            build_journal_detail_panel(runtime, runtime.menu_state.detail_selection, page - 1)
        };
        return MenuPanelView {
            title: "Journal".to_string(),
            lines,
        };
    }
    if action == "overworld_map" {
        return build_overworld_map_panel(runtime, panel_size, "Overworld Map", false);
    }
    if action == "fast_travel" {
        return build_overworld_map_panel(runtime, panel_size, "Fast Travel", true);
    }
    if action == "settings" {
        return build_settings_panel(runtime, runtime.menu_state.detail_selection);
    }
    if action == "save" {
        return build_save_panel(runtime, save_dir, save_message);
    }
    if action == "gameplay_stats" {
        return build_progress_panel(label.to_string(), progress_ui, runtime);
    }
    MenuPanelView {
        title: label.to_string(),
        lines: vec![
            panel_line(format!("{} menu not implemented.", label)),
            panel_line(format!("TODO: implement '{}' submenu.", action)),
        ],
    }
}

fn menu_default_panel(
    menu_ui: &MenuUiFile,
    progress_ui: &ProgressUiFile,
    runtime: &GameRuntime,
) -> MenuPanelView {
    let panel = menu_ui
        .panels
        .iter()
        .find(|panel| panel.id == menu_ui.default_panel);
    let (title, panel_type) = match panel {
        Some(panel) => (panel.title.clone(), panel.panel_type.as_str()),
        None => ("Status".to_string(), "unknown"),
    };
    match panel_type {
        "party_summary" => MenuPanelView {
            title,
            lines: build_party_summary(runtime),
        },
        "progress" => build_progress_panel(title, progress_ui, runtime),
        _ => MenuPanelView {
            title,
            lines: vec![panel_line("Menu panel not configured.")],
        },
    }
}

fn build_progress_panel(
    title: String,
    progress_ui: &ProgressUiFile,
    runtime: &GameRuntime,
) -> MenuPanelView {
    if progress_ui.panels.is_empty() {
        return MenuPanelView {
            title,
            lines: vec![panel_line("Progress panel not configured.")],
        };
    }

    let mut lines = Vec::new();
    let multiple_panels = progress_ui.panels.len() > 1;
    for (index, panel) in progress_ui.panels.iter().enumerate() {
        if multiple_panels {
            lines.push(panel_line_spans(vec![panel_span(
                panel.title.clone(),
                PanelSpanStyle::Accent,
            )]));
        }
        for item in &panel.items {
            let value = runtime.stat_value(item.value.as_str());
            let formatted_value = format_stat_value(item.value.as_str(), value);
            let text = if let Some(max) = item.max {
                format!("{}: {}/{}", item.label, formatted_value, max)
            } else {
                format!("{}: {}", item.label, formatted_value)
            };
            lines.push(panel_line(text));
        }
        if multiple_panels && index + 1 < progress_ui.panels.len() {
            lines.push(panel_line(""));
        }
    }

    MenuPanelView { title, lines }
}

fn format_stat_value(stat_id: &str, value: i32) -> String {
    if stat_id == "time_played" {
        let total_seconds = value.max(0) as u64;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        return format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
    }
    value.to_string()
}

fn menu_panel_size(
    session: &TuiSession,
    menu_ui: &MenuUiFile,
    stats_view: Option<&MenuPanelView>,
) -> PanelSize {
    let size = session.terminal().size().ok();
    let (width, height) = match size {
        Some(size) => tui::menu::right_panel_inner_size(
            menu_ui,
            size.width,
            size.height,
            stats_view.map(|view| view.lines.len()),
        ),
        None => (40, 18),
    };
    PanelSize { width, height }
}

fn menu_footer_text(focus: MenuPane, submenu: &str, page: usize) -> &'static str {
    match focus {
        MenuPane::List => "Confirm: open  Cancel: close",
        MenuPane::Detail => match submenu {
            "save" => "Confirm: save  Cancel: back",
            "status" => {
                if page == 0 {
                    "Left/Right: details  Cancel: back"
                } else {
                    "Left/Right: summary  Cancel: back"
                }
            }
            "overworld_map" => "Up/Down: select  Cancel: back",
            "fast_travel" => "Confirm: travel  Up/Down: select  Cancel: back",
            "settings" => "Confirm: toggle  Left/Right: adjust  Cancel: back",
            "items" => {
                if page == 0 {
                    "Confirm: use  Left/Right: filter  Pause: sort  Cancel: back"
                } else {
                    "Confirm: use  Cancel: back"
                }
            }
            "magic" => {
                if page == 0 {
                    "Confirm: cast  Left/Right: actor  Cancel: back"
                } else {
                    "Confirm: cast  Cancel: back"
                }
            }
            "abilities" => {
                if page == 0 {
                    "Confirm: preview  Left/Right: actor  Cancel: back"
                } else {
                    "Confirm: back  Cancel: back"
                }
            }
            "equipment" => {
                if page == 0 {
                    "Confirm: pick slot  Left/Right: actor  Cancel: back"
                } else {
                    "Confirm: equip  Left/Right: actor  Cancel: back"
                }
            }
            "magic_equip" => {
                if page == 0 {
                    "Confirm: pick slot  Left/Right: actor  Cancel: back"
                } else {
                    "Confirm: equip  Left/Right: actor  Cancel: back"
                }
            }
            "journal" => {
                if page == 0 {
                    "Confirm: details  Cancel: back"
                } else if page == 1 {
                    "Left/Right: history  Cancel: back"
                } else {
                    "Left/Right: steps  Cancel: back"
                }
            }
            "party" => {
                if page == 0 {
                    "Confirm: actions  Left/Right: list  Cancel: back"
                } else if page == 1 {
                    "Confirm: apply  Cancel: back"
                } else {
                    "Confirm: swap  Cancel: back"
                }
            }
            "jobs" => {
                if page == 0 {
                    "Confirm: select  Left/Right: actor  Cancel: back"
                } else if page == 1 {
                    "Confirm: equip  Cancel: back"
                } else {
                    "Confirm: purchase  Cancel: back"
                }
            }
            _ => "Cancel: back",
        },
    }
}

fn build_menu_stats_view(runtime: &GameRuntime) -> MenuPanelView {
    let current_session = runtime.start_time.elapsed().as_secs();
    let total_seconds = runtime.playtime + current_session;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    let currency_id = &runtime.content.rules.game.currency.id;
    let currency_symbol = &runtime.content.rules.game.currency.symbol;
    let currency_amount = runtime.inventory.currency_amount(currency_id);

    MenuPanelView {
        title: String::new(),
        lines: vec![
            panel_line(format!("Time: {:02}:{:02}:{:02}", hours, minutes, seconds)),
            panel_line(format!("{}: {}", currency_symbol, currency_amount)),
        ],
    }
}

#[derive(Clone, Debug)]
struct SaveSlotEntry {
    slot: u8,
    label: String,
    save: Option<SaveFile>,
    selectable: bool,
}

struct SaveMessage {
    text: String,
    style: PanelSpanStyle,
}

fn build_save_panel(
    runtime: &GameRuntime,
    save_dir: &Path,
    save_message: Option<&SaveMessage>,
) -> MenuPanelView {
    let slots = build_save_slots(runtime, save_dir);
    if slots.is_empty() {
        return MenuPanelView {
            title: "Save".to_string(),
            lines: vec![panel_line("No save slots configured.")],
        };
    }

    let selection = runtime
        .menu_state
        .detail_selection
        .min(slots.len().saturating_sub(1));
    let mut lines = Vec::new();
    for (index, slot) in slots.iter().enumerate() {
        lines.push(render_save_slot_line(runtime, slot, index == selection));
    }
    if let Some(message) = save_message {
        lines.push(panel_line_spans(vec![panel_span(
            message.text.clone(),
            message.style.clone(),
        )]));
    }

    MenuPanelView {
        title: "Save".to_string(),
        lines,
    }
}

fn render_save_slot_line(
    runtime: &GameRuntime,
    slot: &SaveSlotEntry,
    selected: bool,
) -> MenuPanelLine {
    let prefix = if selected { "> " } else { "  " };
    let style = if selected {
        tui::menu::PanelSpanStyle::Highlight
    } else if slot.selectable {
        tui::menu::PanelSpanStyle::Normal
    } else {
        tui::menu::PanelSpanStyle::Muted
    };

    let mut text = format!("{}{}", prefix, slot.label);
    if let Some(save) = &slot.save {
        let map_name =
            map_name_for_save(runtime, save).unwrap_or_else(|| save.world.map_id.clone());
        let playtime = format_playtime(save.metadata.play_time_seconds);
        text.push_str(" - ");
        text.push_str(map_name.as_str());
        text.push_str("  ");
        text.push_str(playtime.as_str());
    } else {
        text.push_str(" - Empty");
    }

    panel_line_spans(vec![panel_span(text, style)])
}

fn build_save_slots(runtime: &GameRuntime, save_dir: &Path) -> Vec<SaveSlotEntry> {
    let mut slots = Vec::new();
    if runtime.effective_autosave_enabled() {
        slots.push(build_save_slot_entry(save_dir, 0, "Autosave", false));
    }
    let max_slots = runtime.content.rules.save.slots_max.max(1) as u8;
    for slot in 1..=max_slots {
        slots.push(build_save_slot_entry(
            save_dir,
            slot,
            &format!("Slot {}", slot),
            true,
        ));
    }
    slots
}

fn build_save_slot_entry(
    save_dir: &Path,
    slot: u8,
    label: &str,
    selectable: bool,
) -> SaveSlotEntry {
    let save = load_save_slot(save_dir, slot);
    SaveSlotEntry {
        slot,
        label: label.to_string(),
        save,
        selectable,
    }
}

fn load_save_slot(save_dir: &Path, slot: u8) -> Option<SaveFile> {
    let path = save_slot_path(save_dir, slot);
    let save = SaveFile::load(path).ok()?;
    if save.version == 0 {
        return None;
    }
    Some(save)
}

fn save_slot_path(save_dir: &Path, slot: u8) -> PathBuf {
    save_dir.join(format!("slot_{}.json", slot))
}

fn write_save_slot(runtime: &GameRuntime, save_dir: &Path, slot: u8) -> Result<(), String> {
    std::fs::create_dir_all(save_dir).map_err(|err| format!("{}: {}", save_dir.display(), err))?;
    let save = SaveFile::from_runtime(runtime, slot);
    let path = save_slot_path(save_dir, slot);
    save.write(path)
}

fn first_selectable_save_slot(slots: &[SaveSlotEntry]) -> Option<usize> {
    slots.iter().position(|slot| slot.selectable)
}

fn move_save_selection(current: usize, slots: &[SaveSlotEntry], direction: i32) -> usize {
    if slots.is_empty() {
        return 0;
    }
    let mut index = current.min(slots.len().saturating_sub(1));
    let mut remaining = slots.len();
    while remaining > 0 {
        if direction < 0 {
            index = index.saturating_sub(1);
        } else {
            index = (index + 1).min(slots.len().saturating_sub(1));
        }
        if slots[index].selectable {
            return index;
        }
        remaining -= 1;
    }
    current
}

fn format_playtime(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

fn map_name_for_save(runtime: &GameRuntime, save: &SaveFile) -> Option<String> {
    let index = runtime.content.map_index.get(save.world.map_id.as_str())?;
    let map = runtime.content.maps.get(*index)?;
    if map.name.trim().is_empty() {
        None
    } else {
        Some(map.name.clone())
    }
}

fn build_party_summary(runtime: &GameRuntime) -> Vec<MenuPanelLine> {
    if runtime.party.active.is_empty() {
        return vec![panel_line("No party members.")];
    }
    let mut lines = Vec::new();
    let magic_system = runtime.content.rules.game.magic_system.clone();
    let rows_enabled = runtime.content.rules.battle.rows.enabled;
    for member_id in &runtime.party.active {
        if let Some(actor) = runtime.party.roster.get(member_id) {
            let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
            let job_name = runtime
                .content
                .jobs
                .jobs
                .iter()
                .find(|job| job.id == actor.job_id)
                .map(|job| job.name.as_str())
                .unwrap_or(actor.job_id.as_str());
            let summary_line = if magic_system == engine::rules::MagicSystem::TierCharges {
                let job = runtime
                    .content
                    .jobs
                    .jobs
                    .iter()
                    .find(|job| job.id == actor.job_id);
                let mut tiers = Vec::new();
                if let Some(job) = job {
                    if let Some(magic_slots) = &job.magic_slots {
                        for tier in magic_slots.keys() {
                            let current = actor.magic_tier_charges.get(tier).copied().unwrap_or(0);
                            let max = get_actor_max_charges(&runtime.content, actor, *tier);
                            if max > 0 {
                                tiers.push(format!("T{} {}/{}", tier, current, max));
                            }
                        }
                    }
                }
                let charge_text = if tiers.is_empty() {
                    "".to_string()
                } else {
                    format!("  {}", tiers.join("  "))
                };
                format!(
                    "{}  Lv{}  HP {}/{}{}",
                    actor.name, actor.level, actor.current_hp, max_hp, charge_text
                )
            } else {
                let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
                format!(
                    "{}  Lv{}  HP {}/{}  MP {}/{}",
                    actor.name, actor.level, actor.current_hp, max_hp, actor.current_mp, max_mp
                )
            };
            lines.push(panel_line(summary_line));
            lines.push(panel_line(format!("Job: {}", job_name)));
            if rows_enabled {
                lines.push(panel_line(format!("Row: {}", actor_row_label(actor))));
            }
            if runtime.content.rules.job_system.progression_mode == JobProgressionMode::JobPoints {
                lines.push(panel_line(format!("JP {}", job_jp(actor, &actor.job_id))));
            }
            lines.push(panel_line(""));
        }
    }
    lines
}

struct OverworldDestination {
    label: String,
    map_id: String,
    target_pos: (i32, i32),
    map_pos: (i32, i32),
    enabled: bool,
    reason: Option<String>,
    cost: Option<engine::maps::MapCurrencyStack>,
}

struct OverworldMapView {
    width: u32,
    height: u32,
    tiles: Vec<String>,
}

struct MapMarker {
    glyph: char,
    style: PanelSpanStyle,
}

struct PanelSize {
    width: u16,
    height: u16,
}

fn build_overworld_map_panel(
    runtime: &GameRuntime,
    panel_size: PanelSize,
    title: &str,
    allow_travel: bool,
) -> MenuPanelView {
    let Some(map) = overworld_base_map(runtime) else {
        return MenuPanelView {
            title: title.to_string(),
            lines: vec![panel_line("Overworld map unavailable.")],
        };
    };
    if allow_travel && !fast_travel_enabled(runtime) {
        return MenuPanelView {
            title: title.to_string(),
            lines: vec![
                panel_line("Fast travel unavailable."),
                panel_line("Unlock fast travel to use destinations."),
            ],
        };
    }
    let destinations = build_overworld_destinations(runtime, map);
    let selection = runtime
        .menu_state
        .detail_selection
        .min(destinations.len().saturating_sub(1));
    let mut list_lines =
        build_destination_list_lines(runtime, &destinations, selection, panel_size.width);
    let mut overview_line = if allow_travel {
        None
    } else {
        Some(panel_line_spans(vec![panel_span(
            "Overview only.",
            PanelSpanStyle::Muted,
        )]))
    };
    let mut reserved_lines = list_lines.len();
    if !list_lines.is_empty() {
        reserved_lines += 1;
    }
    if overview_line.is_some() {
        reserved_lines += 1;
    }
    let mut map_height = panel_size.height.saturating_sub(reserved_lines as u16);
    if map_height == 0 {
        map_height = panel_size.height;
        list_lines.clear();
        overview_line = None;
    }
    let map_view = build_overworld_map_view(map, panel_size.width, map_height);
    let markers = build_overworld_markers(runtime, map, &map_view, &destinations, selection);
    let mut lines = build_overworld_map_lines(&map_view, &markers);
    if map_height < panel_size.height && !list_lines.is_empty() {
        lines.push(panel_line(""));
    }
    lines.extend(list_lines);
    if let Some(line) = overview_line {
        lines.push(line);
    }
    MenuPanelView {
        title: title.to_string(),
        lines,
    }
}

fn build_overworld_map_lines(
    view: &OverworldMapView,
    markers: &HashMap<(i32, i32), MapMarker>,
) -> Vec<MenuPanelLine> {
    let mut lines = Vec::new();
    for y in 0..view.height as i32 {
        let row = view
            .tiles
            .get(y as usize)
            .map(|row| row.as_str())
            .unwrap_or("");
        let mut spans = Vec::new();
        for x in 0..view.width as i32 {
            let mut ch = row.chars().nth(x as usize).unwrap_or(' ');
            let mut style = PanelSpanStyle::Normal;
            if let Some(marker) = markers.get(&(x, y)) {
                ch = marker.glyph;
                style = marker.style;
            }
            spans.push(panel_span(ch.to_string(), style));
        }
        lines.push(panel_line_spans(spans));
    }
    lines
}

fn build_destination_list_lines(
    runtime: &GameRuntime,
    destinations: &[OverworldDestination],
    selection: usize,
    width: u16,
) -> Vec<MenuPanelLine> {
    if destinations.is_empty() {
        return vec![panel_line_spans(vec![panel_span(
            "No destinations available.",
            PanelSpanStyle::Muted,
        )])];
    }
    let selected_index = selection.min(destinations.len().saturating_sub(1));
    let destination = &destinations[selected_index];
    let header = format!("Destination {}/{}", selected_index + 1, destinations.len());
    let mut label = destination.label.clone();
    if let Some(cost) = destination.cost.as_ref() {
        label.push_str(" ");
        label.push_str(&format_currency_amount(&runtime.content.rules, cost));
    }
    if let Some(reason) = destination.reason.as_ref() {
        label.push_str(" (");
        label.push_str(reason);
        label.push_str(")");
    }
    let header_text = tui::utils::truncate_line(&header, width as usize);
    let label_text = tui::utils::truncate_line(&label, width as usize);
    let style = if destination.enabled {
        PanelSpanStyle::Highlight
    } else {
        PanelSpanStyle::Muted
    };
    vec![
        panel_line_spans(vec![panel_span(header_text, PanelSpanStyle::Accent)]),
        panel_line_spans(vec![panel_span(label_text, style)]),
    ]
}

fn build_overworld_markers(
    runtime: &GameRuntime,
    map: &MapFile,
    view: &OverworldMapView,
    destinations: &[OverworldDestination],
    selection: usize,
) -> HashMap<(i32, i32), MapMarker> {
    let mut markers = HashMap::new();
    for (index, destination) in destinations.iter().enumerate() {
        let view_pos = map_pos_to_view_pos(map, view, destination.map_pos);
        let selected = index == selection;
        let glyph = if selected { 'X' } else { '*' };
        let style = if selected {
            PanelSpanStyle::Highlight
        } else if destination.enabled {
            PanelSpanStyle::Accent
        } else {
            PanelSpanStyle::Muted
        };
        markers.insert(view_pos, MapMarker { glyph, style });
    }

    for vehicle in &map.vehicles {
        if let Some(flags) = vehicle.requires_flags.as_ref() {
            if !flags.iter().all(|flag| runtime.has_flag(flag)) {
                continue;
            }
        }
        let vehicle_def = match runtime
            .content
            .vehicles
            .vehicles
            .iter()
            .find(|entry| entry.id == vehicle.vehicle_id)
        {
            Some(vehicle_def) => vehicle_def,
            None => continue,
        };
        if !vehicle_def.unlock_flag.trim().is_empty() && !runtime.has_flag(&vehicle_def.unlock_flag)
        {
            continue;
        }
        let vehicle_position = runtime
            .vehicle_positions
            .get(&vehicle.vehicle_id)
            .map(|entry| (entry.map_id.clone(), entry.pos));
        let map_pos = if let Some((map_id, pos)) = vehicle_position {
            if map_id != map.id {
                continue;
            }
            (pos.0, pos.1)
        } else {
            (vehicle.pos[0], vehicle.pos[1])
        };
        let glyph = vehicle_def
            .glyph
            .as_ref()
            .and_then(|glyph| glyph.chars().next())
            .unwrap_or('V');
        let view_pos = map_pos_to_view_pos(map, view, map_pos);
        markers.entry(view_pos).or_insert(MapMarker {
            glyph,
            style: PanelSpanStyle::Accent,
        });
    }

    if let Some(player_pos) = player_marker_pos(runtime, map) {
        let view_pos = map_pos_to_view_pos(map, view, player_pos);
        markers.insert(
            view_pos,
            MapMarker {
                glyph: '@',
                style: PanelSpanStyle::Accent,
            },
        );
    }
    markers
}

fn player_marker_pos(runtime: &GameRuntime, map: &MapFile) -> Option<(i32, i32)> {
    if runtime.is_overworld_map(&runtime.world.map_id) && runtime.world.map_id == map.id {
        return Some(runtime.world.position);
    }
    map.transitions
        .iter()
        .find(|transition| transition.target_map == runtime.world.map_id)
        .map(|transition| (transition.pos[0], transition.pos[1]))
}

fn build_overworld_destinations(runtime: &GameRuntime, map: &MapFile) -> Vec<OverworldDestination> {
    let mut destinations = Vec::new();
    for transition in &map.transitions {
        let target_index = match runtime.content.map_index.get(&transition.target_map) {
            Some(index) => *index,
            None => continue,
        };
        if !map_visited(runtime, &transition.target_map) {
            continue;
        }
        let target_map = match runtime.content.maps.get(target_index) {
            Some(map) => map,
            None => continue,
        };
        let label = transition
            .label
            .as_ref()
            .filter(|label| !label.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| {
                if target_map.name.trim().is_empty() {
                    transition.target_map.clone()
                } else {
                    target_map.name.clone()
                }
            });
        let mut enabled = true;
        let mut reason = None;
        if let Some(flag) = transition
            .requires_flag
            .as_ref()
            .filter(|flag| !flag.trim().is_empty())
        {
            if !runtime.has_flag(flag) {
                enabled = false;
                reason = Some("Locked".to_string());
            }
        }
        if let Some(cost) = transition.cost.as_ref() {
            let available = runtime.inventory.currency_amount(&cost.id);
            if available < cost.amount {
                enabled = false;
                reason = Some(format!(
                    "Need {}",
                    format_currency_amount(&runtime.content.rules, cost)
                ));
            }
        }
        destinations.push(OverworldDestination {
            label,
            map_id: transition.target_map.clone(),
            target_pos: (transition.target_pos[0], transition.target_pos[1]),
            map_pos: (transition.pos[0], transition.pos[1]),
            enabled,
            reason,
            cost: transition.cost.clone(),
        });
    }
    destinations
}

fn overworld_destinations_for_runtime(runtime: &GameRuntime) -> Vec<OverworldDestination> {
    let Some(map) = overworld_base_map(runtime) else {
        return Vec::new();
    };
    build_overworld_destinations(runtime, map)
}

fn move_overworld_selection(current: usize, count: usize, direction: i32) -> usize {
    if count == 0 {
        return 0;
    }
    if direction < 0 {
        current.saturating_sub(1)
    } else {
        (current + 1).min(count.saturating_sub(1))
    }
}

fn overworld_base_map(runtime: &GameRuntime) -> Option<&MapFile> {
    let world = runtime
        .content
        .worlds
        .worlds
        .iter()
        .find(|world| world.id == runtime.world.world_id)?;
    let map_index = runtime.content.map_index.get(&world.overworld_map_id)?;
    runtime.content.maps.get(*map_index)
}

fn overworld_map_available(runtime: &GameRuntime) -> bool {
    overworld_base_map(runtime).is_some()
}

fn build_overworld_map_view(
    map: &MapFile,
    target_width: u16,
    target_height: u16,
) -> OverworldMapView {
    let width = map.width.max(1);
    let height = map.height.max(1);
    let target_width = target_width.max(1).min(width as u16) as u32;
    let target_height = target_height.max(1).min(height as u16) as u32;
    let mut tiles = Vec::new();
    if width == target_width && height == target_height {
        for y in 0..target_height {
            let row = map
                .tiles
                .get(y as usize)
                .map(|row| row.as_str())
                .unwrap_or("");
            let mut line = String::new();
            for x in 0..target_width {
                let ch = row.chars().nth(x as usize).unwrap_or(' ');
                line.push(ch);
            }
            tiles.push(line);
        }
    } else {
        for y in 0..target_height {
            let map_y = (y * height) / target_height;
            let mut line = String::new();
            for x in 0..target_width {
                let map_x = (x * width) / target_width;
                line.push(map_tile_at(map, map_x as i32, map_y as i32));
            }
            tiles.push(line);
        }
    }
    OverworldMapView {
        width: target_width,
        height: target_height,
        tiles,
    }
}

fn map_tile_at(map: &MapFile, x: i32, y: i32) -> char {
    if x < 0 || y < 0 {
        return ' ';
    }
    let row = match map.tiles.get(y as usize) {
        Some(row) => row,
        None => return ' ',
    };
    row.chars().nth(x as usize).unwrap_or(' ')
}

fn map_pos_to_view_pos(map: &MapFile, view: &OverworldMapView, pos: (i32, i32)) -> (i32, i32) {
    if map.width == 0 || map.height == 0 || view.width == 0 || view.height == 0 {
        return (0, 0);
    }
    let view_x = (pos.0.max(0) as i64 * view.width as i64 / map.width as i64) as i32;
    let view_y = (pos.1.max(0) as i64 * view.height as i64 / map.height as i64) as i32;
    (
        view_x.clamp(0, view.width.saturating_sub(1) as i32),
        view_y.clamp(0, view.height.saturating_sub(1) as i32),
    )
}

fn map_visited(runtime: &GameRuntime, map_id: &str) -> bool {
    runtime
        .map_states
        .get(map_id)
        .map(|state| state.flags.contains("visited"))
        .unwrap_or(false)
}

fn fast_travel_enabled(runtime: &GameRuntime) -> bool {
    let world = match runtime
        .content
        .worlds
        .worlds
        .iter()
        .find(|world| world.id == runtime.world.world_id)
    {
        Some(world) => world,
        None => return false,
    };
    if !world.fast_travel.enabled {
        return false;
    }
    if world.fast_travel.requires_flag.trim().is_empty() {
        return true;
    }
    runtime.has_flag(&world.fast_travel.requires_flag)
}

fn format_currency_amount(
    rules: &engine::rules::RulesFile,
    cost: &engine::maps::MapCurrencyStack,
) -> String {
    if cost.id == rules.game.currency.id {
        format!("{}{}", rules.game.currency.symbol, cost.amount)
    } else {
        format!("{} {}", cost.amount, cost.id)
    }
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
        Some(key) if !key.trim().is_empty() => runtime
            .content
            .rules
            .systems
            .get(key)
            .copied()
            .unwrap_or(false),
        _ => true,
    }
}

pub fn unlock_flag_enabled(runtime: &GameRuntime, unlock_flag: Option<&str>) -> bool {
    match unlock_flag {
        Some(flag) if !flag.trim().is_empty() => runtime.has_flag(flag),
        _ => true,
    }
}
