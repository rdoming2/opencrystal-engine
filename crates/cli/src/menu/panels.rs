use std::collections::{HashMap, HashSet};
use std::path::Path;

use engine::party::{actor_row_label, get_actor_max_charges, job_jp};
use engine::rules::ProgressionMode;
use engine::runtime::GameRuntime;
use tui::menu::{MenuPane, MenuPanelLine, MenuPanelView, PanelSpanStyle};
use tui::session::TuiSession;
use tui::ui::{MenuUiFile, ProgressUiFile};

use super::abilities::{build_abilities_panel, build_ability_entries, selected_ability_targets};
use super::equipment::{
    build_equipment_panel, equipment_entries_for_menu, equipment_slots_for_menu,
};
use super::inventory::{
    build_inventory_entries, build_items_panel, item_targets_for_entry, panel_line,
    panel_line_spans, panel_span,
};
use super::jobs::{
    build_job_picker, build_jobs_dashboard, build_learn_panel, job_menu_options, learnable_count,
};
use super::journal::{build_journal_detail_panel, build_journal_panel};
use super::magic::{build_magic_panel, build_spell_entries, selected_spell_targets};
use super::magic_equip::{
    build_magic_equip_panel, magic_equip_entries_for_menu, magic_equip_slots_for_menu,
};
use super::overworld::{build_overworld_map_panel, overworld_travel_allowed};
use super::party::{build_party_panel, party_actions, party_list_entries, PartyList};
use super::save::{build_save_panel, build_save_slots, SaveMessage};
use super::settings::{build_settings_panel, settings_entry_count};
use super::status::build_status_panel;
use super::{journal_entry_count, map_save_allowed};

#[derive(Clone, Copy)]
pub struct PanelSize {
    pub width: u16,
    pub height: u16,
}

pub(super) fn menu_detail_panel(
    label: &str,
    action: &str,
    runtime: &GameRuntime,
    progress_ui: &ProgressUiFile,
    page: usize,
    save_dir: &Path,
    save_message: Option<&SaveMessage>,
    panel_size: PanelSize,
) -> MenuPanelView {
    let mut panel = if action == "status" {
        MenuPanelView {
            title: "Status".to_string(),
            lines: build_status_panel(runtime, page),
        }
    } else if action == "items" {
        build_items_panel(runtime)
    } else if action == "equipment" {
        build_equipment_panel(runtime)
    } else if action == "magic_equip" {
        build_magic_equip_panel(runtime)
    } else if action == "jobs" {
        if page == 2 {
            build_learn_panel(runtime)
        } else if page == 1 {
            build_job_picker(runtime)
        } else {
            build_jobs_dashboard(runtime)
        }
    } else if action == "magic" {
        build_magic_panel(runtime)
    } else if action == "abilities" {
        build_abilities_panel(runtime)
    } else if action == "party" {
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
        build_party_panel(
            runtime,
            list,
            runtime.menu_state.detail_page,
            runtime.menu_state.detail_selection,
            runtime.menu_state.detail_selection,
            selected_member,
            swap_allowed,
        )
    } else if action == "journal" {
        let lines = if page == 0 {
            build_journal_panel(runtime, runtime.menu_state.detail_selection)
        } else {
            build_journal_detail_panel(runtime, runtime.menu_state.detail_selection, page - 1)
        };
        MenuPanelView {
            title: "Journal".to_string(),
            lines,
        }
    } else if action == "overworld_map" {
        build_overworld_map_panel(
            runtime,
            panel_size,
            "Overworld Map",
            overworld_travel_allowed(runtime),
        )
    } else if action == "settings" {
        build_settings_panel(runtime, runtime.menu_state.detail_selection)
    } else if action == "save" {
        build_save_panel(runtime, save_dir, save_message)
    } else if action == "gameplay_stats" {
        build_progress_panel(label.to_string(), progress_ui, runtime)
    } else {
        MenuPanelView {
            title: label.to_string(),
            lines: vec![
                panel_line(format!("{} menu not implemented.", label)),
                panel_line(format!("TODO: implement '{}' submenu.", action)),
            ],
        }
    };

    let selected_line = panel_selected_line(action, runtime, save_dir);
    let page_override = if action == "status" || action == "gameplay_stats" {
        Some(runtime.menu_state.detail_scroll)
    } else {
        None
    };
    panel.lines = apply_panel_paging(panel.lines, panel_size, selected_line, page_override);
    panel
}

fn apply_panel_paging(
    lines: Vec<MenuPanelLine>,
    panel_size: PanelSize,
    selected_line: Option<usize>,
    page_override: Option<usize>,
) -> Vec<MenuPanelLine> {
    let page_size = panel_size.height as usize;
    if page_size == 0 || lines.len() <= page_size {
        return lines;
    }
    let total_pages = (lines.len() + page_size - 1) / page_size;
    let mut page = page_override.unwrap_or(0);
    if let Some(selected) = selected_line {
        page = selected / page_size;
    } else if total_pages > 0 {
        page = page.min(total_pages.saturating_sub(1));
    }
    let start = page.saturating_mul(page_size);
    let end = (start + page_size).min(lines.len());
    lines[start..end].to_vec()
}

fn panel_selected_line(action: &str, runtime: &GameRuntime, save_dir: &Path) -> Option<usize> {
    match action {
        "items" => items_selected_line(runtime),
        "magic" => magic_selected_line(runtime),
        "abilities" => abilities_selected_line(runtime),
        "equipment" => equipment_selected_line(runtime),
        "magic_equip" => magic_equip_selected_line(runtime),
        "party" => party_selected_line(runtime),
        "journal" => journal_selected_line(runtime),
        "settings" => settings_selected_line(runtime),
        "save" => save_selected_line(runtime, save_dir),
        "jobs" => jobs_selected_line(runtime),
        _ => None,
    }
}

fn items_selected_line(runtime: &GameRuntime) -> Option<usize> {
    let entries = build_inventory_entries(
        runtime,
        &super::common::filter_from_index(runtime.menu_state.detail_filter),
        &super::common::sort_from_index(runtime.menu_state.detail_sort),
    );
    if entries.is_empty() {
        return None;
    }
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    if runtime.menu_state.detail_page == 1 {
        let targets = entries
            .get(selection)
            .map(|entry| item_targets_for_entry(runtime, entry))
            .unwrap_or_default();
        if !targets.is_empty() {
            let target_selection = runtime
                .menu_state
                .detail_target
                .min(targets.len().saturating_sub(1));
            return Some(1 + entries.len() + 2 + target_selection);
        }
    }
    Some(1 + selection)
}

fn magic_selected_line(runtime: &GameRuntime) -> Option<usize> {
    let entries = build_spell_entries(runtime);
    if entries.is_empty() {
        return None;
    }
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    if runtime.menu_state.detail_page == 1 {
        let targets = selected_spell_targets(runtime);
        if !targets.is_empty() {
            let target_selection = runtime
                .menu_state
                .detail_target
                .min(targets.len().saturating_sub(1));
            return Some(1 + entries.len() + 2 + target_selection);
        }
    }
    Some(1 + selection)
}

fn abilities_selected_line(runtime: &GameRuntime) -> Option<usize> {
    let entries = build_ability_entries(runtime);
    if entries.is_empty() {
        return None;
    }
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    if runtime.menu_state.detail_page == 1 {
        let targets = selected_ability_targets(runtime);
        if !targets.is_empty() {
            let target_selection = runtime
                .menu_state
                .detail_target
                .min(targets.len().saturating_sub(1));
            return Some(1 + entries.len() + 2 + target_selection);
        }
    }
    Some(1 + selection)
}

fn equipment_selected_line(runtime: &GameRuntime) -> Option<usize> {
    if runtime.menu_state.detail_page == 0 {
        let slots = equipment_slots_for_menu(runtime);
        if slots.is_empty() {
            return None;
        }
        let selection = runtime
            .menu_state
            .detail_selection
            .min(slots.len().saturating_sub(1));
        return Some(1 + selection);
    }
    let entries = equipment_entries_for_menu(runtime);
    if entries.is_empty() {
        return None;
    }
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    Some(1 + selection)
}

fn magic_equip_selected_line(runtime: &GameRuntime) -> Option<usize> {
    if runtime.menu_state.detail_page == 0 {
        let slots = magic_equip_slots_for_menu(runtime);
        if slots.is_empty() {
            return None;
        }
        let selection = runtime
            .menu_state
            .detail_selection
            .min(slots.len().saturating_sub(1));
        return Some(1 + selection);
    }
    let entries = magic_equip_entries_for_menu(runtime);
    if entries.is_empty() {
        return None;
    }
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    Some(1 + selection)
}

fn party_selected_line(runtime: &GameRuntime) -> Option<usize> {
    let list = if runtime.menu_state.detail_target == 0 {
        PartyList::Active
    } else {
        PartyList::Reserve
    };
    if runtime.menu_state.detail_page == 1 {
        let entries = party_list_entries(runtime, list);
        let member_index = runtime
            .menu_state
            .detail_slot
            .min(entries.len().saturating_sub(1));
        let swap_allowed = map_save_allowed(runtime, &runtime.world.map_id, runtime.world.position);
        let actions = party_actions(runtime, list, member_index, swap_allowed);
        let selection = if actions.is_empty() {
            0
        } else {
            runtime
                .menu_state
                .detail_selection
                .min(actions.len().saturating_sub(1))
        };
        return Some(3 + selection);
    }
    let active_entries = party_list_entries(runtime, PartyList::Active);
    let reserve_entries = party_list_entries(runtime, PartyList::Reserve);
    let active_block = active_entries.len().max(1);
    if runtime.menu_state.detail_page == 2 {
        let target_list = list.toggle();
        let selection = runtime.menu_state.detail_selection.min(match target_list {
            PartyList::Active => active_entries.len().saturating_sub(1),
            PartyList::Reserve => reserve_entries.len().saturating_sub(1),
        });
        return match target_list {
            PartyList::Active => Some(1 + selection),
            PartyList::Reserve => Some(3 + active_block + selection),
        };
    }
    let selection = runtime.menu_state.detail_selection.min(match list {
        PartyList::Active => active_entries.len().saturating_sub(1),
        PartyList::Reserve => reserve_entries.len().saturating_sub(1),
    });
    match list {
        PartyList::Active => Some(1 + selection),
        PartyList::Reserve => Some(3 + active_block + selection),
    }
}

fn journal_selected_line(runtime: &GameRuntime) -> Option<usize> {
    if runtime.menu_state.detail_page > 0 {
        return None;
    }
    let count = journal_entry_count(runtime);
    if count == 0 {
        return None;
    }
    let mut all_quest_states = Vec::new();
    for quest_file in &runtime.content.quests {
        let quest_states = quest_file.resolve_quests(&runtime.flags);
        all_quest_states.extend(quest_states);
    }
    if all_quest_states.is_empty() {
        return None;
    }
    let mut categories: HashMap<String, Vec<&engine::quests::QuestState>> = HashMap::new();
    for quest_state in &all_quest_states {
        categories
            .entry(quest_state.quest.category_id.clone())
            .or_default()
            .push(quest_state);
    }
    let mut sorted_categories: Vec<_> = categories.into_iter().collect();
    sorted_categories.sort_by(|a, b| {
        let cat_a = runtime
            .content
            .quests
            .iter()
            .find_map(|file| file.categories.iter().find(|cat| cat.id == a.0));
        let cat_b = runtime
            .content
            .quests
            .iter()
            .find_map(|file| file.categories.iter().find(|cat| cat.id == b.0));
        let order_a = cat_a.map(|cat| cat.sort_order).unwrap_or(i32::MAX);
        let order_b = cat_b.map(|cat| cat.sort_order).unwrap_or(i32::MAX);
        order_a.cmp(&order_b)
    });
    let selected = runtime
        .menu_state
        .detail_selection
        .min(count.saturating_sub(1));
    let mut line_index = 0usize;
    let mut quest_index = 0usize;
    for (_category_id, quest_states) in sorted_categories {
        line_index += 1;
        for _quest in quest_states {
            if quest_index == selected {
                return Some(line_index);
            }
            quest_index += 1;
            line_index += 1;
        }
        line_index += 1;
    }
    None
}

fn settings_selected_line(runtime: &GameRuntime) -> Option<usize> {
    let count = settings_entry_count(runtime);
    if count == 0 {
        return None;
    }
    Some(
        runtime
            .menu_state
            .detail_selection
            .min(count.saturating_sub(1)),
    )
}

fn save_selected_line(runtime: &GameRuntime, save_dir: &Path) -> Option<usize> {
    let slots = build_save_slots(runtime, save_dir);
    if slots.is_empty() {
        return None;
    }
    Some(
        runtime
            .menu_state
            .detail_selection
            .min(slots.len().saturating_sub(1)),
    )
}

fn jobs_selected_line(runtime: &GameRuntime) -> Option<usize> {
    if runtime.party.active_count() == 0 {
        return None;
    }
    if runtime.menu_state.detail_page == 2 {
        let actor_present = runtime.party.active_count() > 0;
        let mut line_index = if actor_present { 3 } else { 2 };
        if !runtime.content.jobs.jobs.is_empty() {
            line_index += 2;
        }
        let learn_count = learnable_count(runtime);
        if learn_count == 0 {
            return Some(line_index);
        }
        let learn_selection = runtime
            .menu_state
            .detail_target
            .min(learn_count.saturating_sub(1));
        return Some(line_index + learn_selection);
    }
    if runtime.menu_state.detail_page > 0 {
        return None;
    }
    let job_points = runtime.content.rules.progression_mode == ProgressionMode::JobPoints;
    let line_index = 2 + if job_points { 1 } else { 0 } + 1;
    let options = job_menu_options(runtime);
    if options.is_empty() {
        return None;
    }
    let selection = runtime
        .menu_state
        .detail_slot
        .min(options.len().saturating_sub(1));
    Some(line_index + selection)
}

pub(super) fn menu_default_panel(
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

pub(super) fn build_progress_panel(
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

pub(super) fn menu_panel_size(
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

pub(super) fn menu_footer_text(
    focus: MenuPane,
    submenu: &str,
    page: usize,
    allow_overworld_travel: bool,
) -> &'static str {
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
            "overworld_map" => {
                if allow_overworld_travel {
                    "Confirm: travel  Up/Down: select  Cancel: back"
                } else {
                    "Up/Down: select  Cancel: back"
                }
            }
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

pub(super) fn build_menu_stats_view(runtime: &GameRuntime) -> MenuPanelView {
    let current_session = runtime.start_time.elapsed().as_secs();
    let total_seconds = runtime.playtime + current_session;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    let (pos_x, pos_y) = runtime.world.position;
    let mut lines = Vec::new();
    lines.push(panel_line(format!(
        "Time: {:02}:{:02}:{:02}",
        hours, minutes, seconds
    )));

    let mut seen_currency_ids = HashSet::new();
    for currency in &runtime.content.rules.game.currencies {
        let amount = runtime.inventory.currency_amount(currency.id.as_str());
        if amount <= 0 {
            continue;
        }
        seen_currency_ids.insert(currency.id.as_str());
        let label = if currency.symbol.trim().is_empty() {
            currency.name.as_str()
        } else {
            currency.symbol.as_str()
        };
        lines.push(panel_line(format!("{}: {}", label, amount)));
    }

    let mut extra_currency_ids: Vec<_> = runtime
        .inventory
        .currency
        .iter()
        .filter_map(|(id, amount)| {
            if *amount > 0 && !seen_currency_ids.contains(id.as_str()) {
                Some(id.as_str())
            } else {
                None
            }
        })
        .collect();
    extra_currency_ids.sort();
    for currency_id in extra_currency_ids {
        let amount = runtime.inventory.currency_amount(currency_id);
        lines.push(panel_line(format!("{}: {}", currency_id, amount)));
    }

    MenuPanelView {
        title: String::new(),
        lines: {
            lines.push(panel_line(format!("Pos: {},{}", pos_x, pos_y)));
            lines
        },
    }
}

fn build_party_summary(runtime: &GameRuntime) -> Vec<MenuPanelLine> {
    if runtime.party.active_count() == 0 {
        return vec![panel_line("No party members.")];
    }
    let mut lines = Vec::new();
    let magic_system = runtime.content.rules.game.magic_system.clone();
    let rows_enabled = runtime.content.rules.battle.rows.enabled;
    for member_id in runtime.party.active_ids() {
        if let Some(actor) = runtime.party.roster.get(&member_id) {
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
            if runtime.content.rules.progression_mode == ProgressionMode::JobPoints {
                lines.push(panel_line(format!("JP {}", job_jp(actor, &actor.job_id))));
            }
            lines.push(panel_line(""));
        }
    }
    lines
}
