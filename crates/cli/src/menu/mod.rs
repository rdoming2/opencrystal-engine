pub mod abilities;
pub mod common;
pub mod equipment;
pub mod inventory;
pub mod jobs;
pub mod journal;
pub mod magic;
pub mod magic_equip;
pub mod status;

use engine::menu::MenuFocus;
use engine::party::{get_actor_max_charges, job_jp};
use engine::rules::JobProgressionMode;
use engine::runtime::GameRuntime;
use tui::input::{Action, InputBindings};
use tui::menu::{MenuEntryView, MenuPane, MenuPanelLine, MenuPanelView};
use tui::session::TuiSession;
use tui::ui::MenuUiFile;

use crate::utils::read_action;

use self::abilities::{
    ability_targets_for_entry, build_abilities_panel, build_ability_entries,
    selected_ability_targets,
};
use self::common::{
    InventoryKind, MenuEntryState, filter_from_index, next_filter_index, prev_filter_index,
    sort_from_index, toggle_sort_index,
};
use self::equipment::{
    build_equipment_panel, detail_actor_id, equip_item, equipment_entries_for_menu,
    equipment_slot_for_menu, equipment_slots_for_menu,
};
use self::inventory::{
    apply_item_to_targets, build_inventory_entries, build_items_panel, item_targets_for_entry,
    panel_line,
};
use self::jobs::{
    JobMenuOption, apply_learn_purchase, apply_primary_change, apply_secondary_change,
    build_job_picker, build_jobs_dashboard, build_learn_panel, job_menu_options, learnable_count,
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
use self::status::build_status_panel;

pub fn run_menu_loop(
    session: &mut TuiSession,
    runtime: &mut GameRuntime,
    menu_ui: &MenuUiFile,
    bindings: &InputBindings,
    map_id: &str,
    player_pos: (i32, i32),
) -> std::io::Result<()> {
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
            .unwrap_or("menu");
        let right_panel = if matches!(focus, MenuPane::Detail) {
            menu_detail_panel(
                label,
                submenu_action,
                runtime,
                runtime.menu_state.detail_page,
            )
        } else {
            menu_default_panel(menu_ui, runtime)
        };

        let footer_text = menu_footer_text(focus, submenu_action, runtime.menu_state.detail_page);
        let stats_view = build_menu_stats_view(runtime);
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
                    }
                }
                Action::MoveDown => {
                    if matches!(focus, MenuPane::List) {
                        if runtime.menu_state.selected + 1 < entry_views.len() {
                            runtime.menu_state.selected += 1;
                        }
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
                                    "journal" => {
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
                                        if apply_item_to_targets(runtime, entry, &targets) {
                                            runtime.inventory.remove_item(&entry.id, 1);
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
                                        if apply_item_to_targets(
                                            runtime,
                                            entry,
                                            &[target_id.clone()],
                                        ) {
                                            runtime.inventory.remove_item(&entry.id, 1);
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

fn menu_detail_panel(
    label: &str,
    action: &str,
    runtime: &GameRuntime,
    page: usize,
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
    MenuPanelView {
        title: label.to_string(),
        lines: vec![
            panel_line(format!("{} menu not implemented.", label)),
            panel_line(format!("TODO: implement '{}' submenu.", action)),
        ],
    }
}

fn menu_default_panel(menu_ui: &MenuUiFile, runtime: &GameRuntime) -> MenuPanelView {
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
        "progress" => MenuPanelView {
            title,
            lines: vec![
                panel_line("Progress panel (stub)."),
                panel_line("TODO: render ui/progress.json."),
            ],
        },
        _ => MenuPanelView {
            title,
            lines: vec![panel_line("Menu panel not configured.")],
        },
    }
}

fn menu_footer_text(focus: MenuPane, submenu: &str, page: usize) -> &'static str {
    match focus {
        MenuPane::List => "Confirm: open  Cancel: close",
        MenuPane::Detail => match submenu {
            "status" => {
                if page == 0 {
                    "Left/Right: details  Cancel: back"
                } else {
                    "Left/Right: summary  Cancel: back"
                }
            }
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

fn build_party_summary(runtime: &GameRuntime) -> Vec<MenuPanelLine> {
    if runtime.party.active.is_empty() {
        return vec![panel_line("No party members.")];
    }
    let mut lines = Vec::new();
    let magic_system = runtime.content.rules.game.magic_system.clone();
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
            if runtime.content.rules.job_system.progression_mode == JobProgressionMode::JobPoints {
                lines.push(panel_line(format!("JP {}", job_jp(actor, &actor.job_id))));
            }
            lines.push(panel_line(""));
        }
    }
    lines
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
