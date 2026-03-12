use std::path::Path;

use engine::menu::MenuFocus;
use engine::runtime::GameRuntime;
use tui::input::InputBindings;
use tui::menu::{MenuEntryView, MenuPane, MenuPanelView, PanelSpanStyle};
use tui::session::TuiSession;
use tui::ui::{DialogUiFile, MenuUiFile};

use super::abilities::{ability_targets_for_entry, build_ability_entries};
use super::common::MenuEntryState;
use super::equipment::{
    detail_actor_id, equip_item, equipment_entries_for_menu, equipment_slot_for_menu,
};
use super::inventory::{
    apply_item_to_targets, build_inventory_entries, item_action_for_entry_with_runtime,
    item_targets_for_entry, move_inventory_entry, ItemActionId,
};
use super::jobs::{
    apply_learn_purchase, apply_primary_change, apply_secondary_change, equipped_job_selection,
    job_menu_options, JobMenuOption,
};
use super::magic::{apply_spell_to_targets, build_spell_entries, spell_targets_for_entry};
use super::magic_equip::{magic_equip_entries_for_menu, magic_equip_slot_for_menu};
use super::overworld::{overworld_destinations_for_runtime, overworld_travel_allowed};
use super::panels::build_menu_stats_view;
use super::party::{party_actions, party_list_entries, party_member_id, PartyActionId, PartyList};
use super::save::{build_save_slots, default_save_selection, write_save_slot, SaveMessage};
use super::settings::apply_settings_confirm;
use super::{map_save_allowed, MenuOutcome};

pub(super) fn handle_confirm(
    session: &mut TuiSession,
    runtime: &mut GameRuntime,
    menu_ui: &MenuUiFile,
    dialog_ui: &DialogUiFile,
    bindings: &InputBindings,
    entries: &[MenuEntryState],
    entry_views: &[MenuEntryView],
    focus: MenuPane,
    right_panel: &MenuPanelView,
    footer_text: &'static str,
    save_dir: &Path,
    map_id: &str,
    player_pos: (i32, i32),
    submenu_action: &str,
    save_message: &mut Option<SaveMessage>,
) -> std::io::Result<Option<MenuOutcome>> {
    if matches!(focus, MenuPane::List) {
        if let Some(entry) = entries.get(runtime.menu_state.selected) {
            if entry.selectable {
                runtime.menu_state.detail_scroll = 0;
                if entry.action == "exit" {
                    let confirm_stats = build_menu_stats_view(runtime);
                    if tui::menu::confirm_menu_exit(session, |frame| {
                        tui::menu::draw_menu_frame(
                            frame,
                            menu_ui,
                            entry_views,
                            runtime.menu_state.selected,
                            focus,
                            right_panel,
                            Some(&confirm_stats),
                            footer_text,
                        );
                    })? {
                        runtime.close_menu();
                        return Ok(Some(MenuOutcome::ReturnTitle));
                    }
                } else {
                    enter_submenu_from_list(runtime, entry.action.as_str(), save_dir, save_message);
                }
            } else if submenu_action == "jobs" {
                apply_secondary_change(runtime);
            }
        }
    } else if submenu_action == "settings" {
        apply_settings_confirm(runtime, runtime.menu_state.detail_selection);
    } else if submenu_action == "save" {
        confirm_save(runtime, save_dir, save_message);
    } else if submenu_action == "overworld_map" {
        if let Some(outcome) = confirm_overworld_map(runtime) {
            return Ok(Some(outcome));
        }
    } else if submenu_action == "items" {
        confirm_items(
            session,
            runtime,
            menu_ui,
            dialog_ui,
            bindings,
            entry_views,
            focus,
            right_panel,
            footer_text,
        )?;
    } else if submenu_action == "magic" {
        confirm_magic(runtime)?;
    } else if submenu_action == "abilities" {
        confirm_abilities(runtime)?;
    } else if submenu_action == "equipment" {
        confirm_equipment(runtime);
    } else if submenu_action == "party" {
        confirm_party(runtime, map_id, player_pos)?;
    } else if submenu_action == "magic_equip" {
        confirm_magic_equip(runtime);
    } else if submenu_action == "journal" {
        confirm_journal(runtime);
    } else if submenu_action == "jobs" {
        confirm_jobs(runtime);
    }

    Ok(None)
}

fn enter_submenu_from_list(
    runtime: &mut GameRuntime,
    action: &str,
    save_dir: &Path,
    save_message: &mut Option<SaveMessage>,
) {
    runtime.menu_state.focus = MenuFocus::Detail;
    runtime.menu_state.active_submenu = Some(action.to_string());
    match action {
        "items" => {
            runtime.menu_state.detail_page = 0;
            runtime.menu_state.detail_selection = 0;
            runtime.menu_state.detail_filter = 0;
            runtime.menu_state.detail_sort = 0;
            runtime.menu_state.detail_target = 0;
            runtime.menu_state.detail_slot = usize::MAX;
        }
        "magic" => {
            runtime.menu_state.detail_page = 0;
            runtime.menu_state.detail_selection = 0;
            runtime.menu_state.detail_actor = 0;
            runtime.menu_state.detail_target = 0;
        }
        "equipment" => {
            runtime.menu_state.detail_page = 0;
            runtime.menu_state.detail_selection = 0;
            runtime.menu_state.detail_actor = 0;
            runtime.menu_state.detail_slot = 0;
        }
        "magic_equip" => {
            runtime.menu_state.detail_page = 0;
            runtime.menu_state.detail_selection = 0;
            runtime.menu_state.detail_actor = 0;
            runtime.menu_state.detail_slot = 0;
        }
        "abilities" => {
            runtime.menu_state.detail_page = 0;
            runtime.menu_state.detail_selection = 0;
            runtime.menu_state.detail_actor = 0;
            runtime.menu_state.detail_target = 0;
        }
        "party" => {
            runtime.menu_state.detail_page = 0;
            runtime.menu_state.detail_selection = 0;
            runtime.menu_state.detail_slot = usize::MAX;
            runtime.menu_state.detail_target = 0;
        }
        "journal" => {
            runtime.menu_state.detail_page = 0;
            runtime.menu_state.detail_selection = 0;
        }
        "overworld_map" => {
            runtime.menu_state.detail_page = 0;
            runtime.menu_state.detail_selection = 0;
        }
        "save" => {
            runtime.menu_state.detail_page = 0;
            let slots = build_save_slots(runtime, save_dir);
            runtime.menu_state.detail_selection =
                default_save_selection(runtime, &slots).unwrap_or(0);
            *save_message = None;
        }
        "settings" => {
            runtime.menu_state.detail_page = 0;
            runtime.menu_state.detail_selection = 0;
        }
        "status" => {
            runtime.menu_state.detail_page = 0;
            runtime.menu_state.detail_actor = 0;
            runtime.menu_state.detail_scroll = 0;
        }
        "jobs" => {
            runtime.menu_state.detail_page = 0;
            runtime.menu_state.detail_slot = 0;
            runtime.menu_state.detail_selection = 0;
            runtime.menu_state.detail_target = 0;
        }
        _ => {
            runtime.menu_state.detail_page = 0;
        }
    }
}

fn confirm_save(
    runtime: &mut GameRuntime,
    save_dir: &Path,
    save_message: &mut Option<SaveMessage>,
) {
    let slots = build_save_slots(runtime, save_dir);
    if let Some(slot) = slots.get(runtime.menu_state.detail_selection) {
        if slot.selectable {
            match write_save_slot(runtime, save_dir, slot.slot) {
                Ok(()) => {
                    runtime.last_manual_save_slot = Some(slot.slot);
                    *save_message = Some(SaveMessage {
                        text: "Saved.".to_string(),
                        style: PanelSpanStyle::Accent,
                    });
                }
                Err(err) => {
                    eprintln!("Failed to save: {}", err);
                    *save_message = Some(SaveMessage {
                        text: "Save failed.".to_string(),
                        style: PanelSpanStyle::Muted,
                    });
                }
            }
        }
    }
}

fn confirm_overworld_map(runtime: &mut GameRuntime) -> Option<MenuOutcome> {
    if !overworld_travel_allowed(runtime) {
        return None;
    }
    let destinations = overworld_destinations_for_runtime(runtime);
    if destinations.is_empty() {
        return None;
    }
    let selection = runtime
        .menu_state
        .detail_selection
        .min(destinations.len().saturating_sub(1));
    let destination = destinations.get(selection)?;
    if !destination.enabled {
        return None;
    }
    if let Some(cost) = &destination.cost {
        runtime.inventory.add_currency(&cost.id, -cost.amount);
    }
    runtime.warp_to_map(&destination.map_id, destination.target_pos);
    runtime.close_menu();
    Some(MenuOutcome::Continue)
}

fn confirm_items(
    session: &mut TuiSession,
    runtime: &mut GameRuntime,
    menu_ui: &MenuUiFile,
    dialog_ui: &DialogUiFile,
    bindings: &InputBindings,
    entry_views: &[MenuEntryView],
    focus: MenuPane,
    right_panel: &MenuPanelView,
    footer_text: &'static str,
) -> std::io::Result<()> {
    let entries = build_inventory_entries(
        runtime,
        &super::common::filter_from_index(runtime.menu_state.detail_filter),
    );
    if entries.is_empty() {
        return Ok(());
    }
    let list_selection = if runtime.menu_state.detail_page == 0 {
        runtime
            .menu_state
            .detail_selection
            .min(entries.len().saturating_sub(1))
    } else {
        runtime
            .menu_state
            .detail_slot
            .min(entries.len().saturating_sub(1))
    };
    let Some(entry) = entries.get(list_selection) else {
        return Ok(());
    };
    match runtime.menu_state.detail_page {
        0 => {
            runtime.menu_state.detail_page = 1;
            runtime.menu_state.detail_slot = list_selection;
            runtime.menu_state.detail_selection = 0;
            runtime.menu_state.detail_target = 0;
        }
        1 => {
            let action_selection = runtime.menu_state.detail_selection;
            let Some((action_id, enabled)) =
                item_action_for_entry_with_runtime(runtime, Some(entry), action_selection)
            else {
                return Ok(());
            };
            if !enabled {
                return Ok(());
            }
            match action_id {
                ItemActionId::Use => {
                    if entry.kind != super::common::InventoryKind::Item {
                        return Ok(());
                    }
                    let targets = item_targets_for_entry(runtime, entry);
                    if entry.usage_target == "party" {
                        let result = apply_item_to_targets(runtime, entry, &targets);
                        if result.consumed {
                            runtime.inventory.remove_item(&entry.id, 1);
                        }
                        if let Some(message) = result.message {
                            if message == "No valid targets." {
                                show_no_valid_targets_modal(
                                    session,
                                    runtime,
                                    menu_ui,
                                    bindings,
                                    entry_views,
                                    focus,
                                    right_panel,
                                    footer_text,
                                )?;
                            } else {
                                tui::dialog::show_dialog(
                                    session, dialog_ui, bindings, "", &message,
                                )?;
                            }
                        }
                        if result.consumed {
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = list_selection;
                            runtime.menu_state.detail_slot = usize::MAX;
                            runtime.menu_state.detail_target = 0;
                        } else {
                            runtime.menu_state.detail_page = 1;
                            runtime.menu_state.detail_slot = list_selection;
                            runtime.menu_state.detail_selection = action_selection;
                            runtime.menu_state.detail_target = 0;
                        }
                    } else if targets.is_empty() {
                        show_no_valid_targets_modal(
                            session,
                            runtime,
                            menu_ui,
                            bindings,
                            entry_views,
                            focus,
                            right_panel,
                            footer_text,
                        )?;
                        runtime.menu_state.detail_page = 1;
                        runtime.menu_state.detail_slot = list_selection;
                        runtime.menu_state.detail_selection = action_selection;
                        runtime.menu_state.detail_target = 0;
                    } else {
                        runtime.menu_state.detail_page = 2;
                        runtime.menu_state.detail_target = 0;
                    }
                }
                ItemActionId::Drop => {
                    match entry.kind {
                        super::common::InventoryKind::Item => {
                            runtime.inventory.remove_item(&entry.id, 1);
                        }
                        super::common::InventoryKind::Equipment => {
                            if entry.available_qty > 0 {
                                runtime.inventory.remove_equipment(&entry.id, 1);
                            }
                        }
                    }
                    runtime.menu_state.detail_page = 0;
                    runtime.menu_state.detail_selection = list_selection;
                    runtime.menu_state.detail_slot = usize::MAX;
                    runtime.menu_state.detail_target = 0;
                }
                ItemActionId::Move => {
                    runtime.menu_state.detail_page = 3;
                    runtime.menu_state.detail_target = list_selection;
                }
            }
        }
        2 => {
            let targets = item_targets_for_entry(runtime, entry);
            let mut consumed = false;
            if let Some(target_id) = targets.get(runtime.menu_state.detail_target) {
                let result = apply_item_to_targets(runtime, entry, &[target_id.clone()]);
                if result.consumed {
                    runtime.inventory.remove_item(&entry.id, 1);
                    consumed = true;
                }
                if let Some(message) = result.message {
                    if message == "No valid targets." {
                        show_no_valid_targets_modal(
                            session,
                            runtime,
                            menu_ui,
                            bindings,
                            entry_views,
                            focus,
                            right_panel,
                            footer_text,
                        )?;
                    } else {
                        tui::dialog::show_dialog(session, dialog_ui, bindings, "", &message)?;
                    }
                }
            } else {
                show_no_valid_targets_modal(
                    session,
                    runtime,
                    menu_ui,
                    bindings,
                    entry_views,
                    focus,
                    right_panel,
                    footer_text,
                )?;
            }
            if consumed {
                runtime.menu_state.detail_page = 0;
                runtime.menu_state.detail_selection =
                    list_selection.min(entries.len().saturating_sub(1));
                runtime.menu_state.detail_slot = usize::MAX;
            } else {
                runtime.menu_state.detail_page = 1;
                runtime.menu_state.detail_slot = list_selection;
                runtime.menu_state.detail_selection = 0;
            }
            runtime.menu_state.detail_target = 0;
        }
        3 => {
            let target_index = runtime
                .menu_state
                .detail_target
                .min(entries.len().saturating_sub(1));
            move_inventory_entry(runtime, &entries, list_selection, target_index);
            runtime.menu_state.detail_page = 0;
            runtime.menu_state.detail_selection = target_index;
            runtime.menu_state.detail_slot = usize::MAX;
            runtime.menu_state.detail_target = 0;
            runtime.menu_state.detail_sort = 0;
        }
        _ => {}
    }
    if runtime.menu_state.detail_page == 0 {
        let refreshed = build_inventory_entries(
            runtime,
            &super::common::filter_from_index(runtime.menu_state.detail_filter),
        );
        if refreshed.is_empty() {
            runtime.menu_state.detail_selection = 0;
        } else if runtime.menu_state.detail_selection >= refreshed.len() {
            runtime.menu_state.detail_selection = refreshed.len() - 1;
        }
    }
    Ok(())
}

fn show_no_valid_targets_modal(
    session: &mut TuiSession,
    runtime: &GameRuntime,
    menu_ui: &MenuUiFile,
    bindings: &InputBindings,
    entry_views: &[MenuEntryView],
    focus: MenuPane,
    right_panel: &MenuPanelView,
    footer_text: &'static str,
) -> std::io::Result<()> {
    let stats = build_menu_stats_view(runtime);
    tui::menu::show_menu_notice_modal(
        session,
        bindings,
        |frame| {
            tui::menu::draw_menu_frame(
                frame,
                menu_ui,
                entry_views,
                runtime.menu_state.selected,
                focus,
                right_panel,
                Some(&stats),
                footer_text,
            );
        },
        "No valid targets.",
    )
}

fn confirm_magic(runtime: &mut GameRuntime) -> std::io::Result<()> {
    let entries = build_spell_entries(runtime);
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Ok(()),
    };
    if let Some(entry) = entries.get(selection) {
        if entry.usable {
            if runtime.menu_state.detail_page == 0 {
                let targets = spell_targets_for_entry(runtime, entry, &actor_id);
                if entry.default_target == "party" || entry.default_target == "self" {
                    apply_spell_to_targets(runtime, entry, &actor_id, &targets);
                } else if targets.is_empty() {
                    runtime.menu_state.detail_page = 0;
                } else {
                    runtime.menu_state.detail_page = 1;
                    runtime.menu_state.detail_target = 0;
                }
            } else {
                let targets = spell_targets_for_entry(runtime, entry, &actor_id);
                if let Some(target_id) = targets.get(runtime.menu_state.detail_target) {
                    apply_spell_to_targets(runtime, entry, &actor_id, &[target_id.clone()]);
                }
                runtime.menu_state.detail_page = 0;
                runtime.menu_state.detail_target = 0;
            }
        }
        runtime.menu_state.detail_selection = selection;
    }
    Ok(())
}

fn confirm_abilities(runtime: &mut GameRuntime) -> std::io::Result<()> {
    let entries = build_ability_entries(runtime);
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Ok(()),
    };
    if let Some(entry) = entries.get(selection) {
        if runtime.menu_state.detail_page == 0 {
            let targets = ability_targets_for_entry(runtime, entry, &actor_id);
            if entry.default_target == "party" || entry.default_target == "self" {
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
    Ok(())
}

fn confirm_equipment(runtime: &mut GameRuntime) {
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
}

fn confirm_party(
    runtime: &mut GameRuntime,
    map_id: &str,
    player_pos: (i32, i32),
) -> std::io::Result<()> {
    let swap_allowed = map_save_allowed(runtime, map_id, player_pos);
    let list = if runtime.menu_state.detail_target == 0 {
        PartyList::Active
    } else {
        PartyList::Reserve
    };
    if runtime.menu_state.detail_page == 0 {
        let entries = party_list_entries(runtime, list);
        if entries.is_empty() {
            return Ok(());
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
            return Ok(());
        }
        let actions = party_actions(runtime, list, member_index, swap_allowed);
        let Some(action) = actions.get(runtime.menu_state.detail_selection) else {
            return Ok(());
        };
        if !action.enabled {
            return Ok(());
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
                if list == PartyList::Active && member_index + 1 < runtime.party.active.len() {
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
                if let Some(actor_id) = party_member_id(runtime, list, member_index) {
                    if let Some(actor) = runtime.party.roster.get_mut(&actor_id) {
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
            return Ok(());
        }
        let member_index = runtime.menu_state.detail_slot;
        if member_index == usize::MAX {
            return Ok(());
        }
        let target_entries = party_list_entries(runtime, list.toggle());
        if target_entries.is_empty() {
            return Ok(());
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
                    let reserve_id = runtime.party.reserve[target_index].clone();
                    let active_id = runtime.party.active[member_index].take();
                    runtime.party.active[member_index] = Some(reserve_id);
                    if let Some(active_id) = active_id {
                        runtime.party.reserve[target_index] = active_id;
                    } else {
                        runtime.party.reserve.remove(target_index);
                    }
                }
            }
            PartyList::Reserve => {
                if member_index < runtime.party.reserve.len()
                    && target_index < runtime.party.active.len()
                {
                    let reserve_id = runtime.party.reserve[member_index].clone();
                    let active_id = runtime.party.active[target_index].take();
                    runtime.party.active[target_index] = Some(reserve_id);
                    if let Some(active_id) = active_id {
                        runtime.party.reserve[member_index] = active_id;
                    } else {
                        runtime.party.reserve.remove(member_index);
                    }
                }
            }
        }
        runtime.menu_state.detail_page = 0;
        runtime.menu_state.detail_selection =
            member_index.min(party_list_entries(runtime, list).len().saturating_sub(1));
        runtime.menu_state.detail_slot = usize::MAX;
    }
    Ok(())
}

fn confirm_magic_equip(runtime: &mut GameRuntime) {
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
}

fn confirm_journal(runtime: &mut GameRuntime) {
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
}

fn confirm_jobs(runtime: &mut GameRuntime) {
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
                    runtime.menu_state.detail_selection = equipped_job_selection(runtime, *option);
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
