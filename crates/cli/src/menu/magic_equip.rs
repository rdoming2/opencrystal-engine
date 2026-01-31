use engine::party::recompute_derived_stats;
use engine::runtime::GameRuntime;
use tui::menu::{MenuPanelLine, MenuPanelView, PanelSpanStyle};

use super::common::{InventoryEntry, InventoryKind};
use super::equipment::{build_equipped_counts, build_equipped_map, detail_actor_id};
use super::inventory::{
    build_list_line, equipped_label, list_line_width, panel_line, panel_line_spans, panel_span,
};

pub fn build_magic_equip_panel(runtime: &GameRuntime) -> MenuPanelView {
    let actor_id = detail_actor_id(runtime);
    let Some(actor_id) = actor_id else {
        return MenuPanelView {
            title: "Magic Equip".to_string(),
            lines: vec![panel_line("No party members.")],
        };
    };
    let actor = match runtime.party.roster.get(&actor_id) {
        Some(actor) => actor,
        None => {
            return MenuPanelView {
                title: "Magic Equip".to_string(),
                lines: vec![panel_line("No party members.")],
            };
        }
    };
    let header = magic_equip_header_line(actor.name.as_str());
    let slots = magic_equip_slots_for_menu(runtime);
    if slots.is_empty() {
        return MenuPanelView {
            title: "Magic Equip".to_string(),
            lines: vec![panel_line("No magic slots available.")],
        };
    }
    let mut lines = Vec::new();
    lines.push(header);
    if runtime.menu_state.detail_page == 0 {
        let selection = runtime
            .menu_state
            .detail_selection
            .min(slots.len().saturating_sub(1));
        for (index, slot) in slots.iter().enumerate() {
            let equipped = actor
                .equipment
                .get(slot)
                .and_then(|item_id| {
                    runtime
                        .content
                        .equipment
                        .equipment
                        .iter()
                        .find(|item| item.id == *item_id)
                        .map(|item| item.name.as_str())
                })
                .unwrap_or("Empty");
            let is_selected = index == selection;
            let mut spans = Vec::new();
            spans.push(panel_span(
                if is_selected { "> " } else { "  " },
                if is_selected {
                    PanelSpanStyle::Highlight
                } else {
                    PanelSpanStyle::Normal
                },
            ));
            spans.push(panel_span(
                format!("Slot {}: {}", index + 1, equipped),
                if is_selected {
                    PanelSpanStyle::Highlight
                } else {
                    PanelSpanStyle::Normal
                },
            ));
            lines.push(panel_line_spans(spans));
        }
        lines.push(panel_line("------------------------------"));
        let detail_slot = slots.get(selection).cloned().unwrap_or_default();
        lines.extend(build_magic_equipped_detail(runtime, actor, &detail_slot));
    } else {
        let entries = magic_equip_entries_for_menu(runtime);
        if entries.is_empty() {
            return MenuPanelView {
                title: "Magic Equip".to_string(),
                lines: vec![panel_line("No magic items available.")],
            };
        }
        let selection = runtime
            .menu_state
            .detail_selection
            .min(entries.len().saturating_sub(1));
        let width = list_line_width(&entries);
        for (index, entry) in entries.iter().enumerate() {
            lines.push(build_list_line(entry, index == selection, width));
        }
        lines.push(panel_line("------------------------------"));
        lines.push(panel_line_spans(vec![panel_span(
            "Details",
            PanelSpanStyle::Accent,
        )]));
        if let Some(entry) = entries.get(selection) {
            let slot = magic_equip_slot_for_menu(runtime).unwrap_or_default();
            lines.extend(build_magic_item_detail(runtime, &actor_id, &slot, entry).lines);
        }
    }

    MenuPanelView {
        title: "Magic Equip".to_string(),
        lines,
    }
}

pub fn magic_equip_slots_for_menu(runtime: &GameRuntime) -> Vec<String> {
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Vec::new(),
    };
    let actor = match runtime.party.roster.get(&actor_id) {
        Some(actor) => actor,
        None => return Vec::new(),
    };
    let job = runtime
        .content
        .jobs
        .jobs
        .iter()
        .find(|job| job.id == actor.job_id);

    if let Some(job) = job {
        if let Some(progression) = &job.magic_equip_progression {
            let mut max_slots = 0;
            for (req_level, slots) in &progression.slots {
                if actor.level >= *req_level {
                    max_slots = max_slots.max(*slots);
                }
            }
            return (1..=max_slots).map(|i| format!("magic_{}", i)).collect();
        }
    }
    Vec::new()
}

pub fn magic_equip_slot_for_menu(runtime: &GameRuntime) -> Option<String> {
    let slots = magic_equip_slots_for_menu(runtime);
    slots.get(runtime.menu_state.detail_slot).cloned()
}

pub fn magic_equip_entries_for_menu(runtime: &GameRuntime) -> Vec<InventoryEntry> {
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Vec::new(),
    };
    let slot = match magic_equip_slot_for_menu(runtime) {
        Some(slot) => slot,
        None => return Vec::new(),
    };

    let equipped_map = build_equipped_map(runtime);
    let equipped_counts = build_equipped_counts(runtime);
    let actor = match runtime.party.roster.get(&actor_id) {
        Some(actor) => actor,
        None => return Vec::new(),
    };

    let mut entries = Vec::new();
    entries.push(InventoryEntry {
        id: "".to_string(),
        label: "Unequip".to_string(),
        available_qty: 0,
        total_qty: 0,
        kind: InventoryKind::Equipment,
        slot: Some(slot.clone()),
        category: None,
        usable: true,
        equipped_by: Vec::new(),
        usage_target: String::new(),
    });

    for equipment in &runtime.content.equipment.equipment {
        if equipment.slot != "magic" {
            continue;
        }

        let inventory_qty = runtime.inventory.equipment_qty(&equipment.id);
        let equipped_count = equipped_counts.get(&equipment.id).copied().unwrap_or(0);
        let mut available = inventory_qty;
        let equipped_by = equipped_map.get(&equipment.id).cloned().unwrap_or_default();
        let already_equipped = actor
            .equipment
            .values()
            .any(|item_id| item_id == &equipment.id);
        if already_equipped {
            available += 1;
        }
        if available <= 0 && equipped_by.is_empty() {
            continue;
        }
        let usable = available > 0 || already_equipped;
        entries.push(InventoryEntry {
            id: equipment.id.clone(),
            label: equipment.name.clone(),
            available_qty: inventory_qty.max(0),
            total_qty: (inventory_qty + equipped_count).max(0),
            kind: InventoryKind::Equipment,
            slot: Some(equipment.slot.clone()),
            category: Some(equipment.category.clone()),
            usable,
            equipped_by,
            usage_target: String::new(),
        });
    }

    entries
}

fn magic_equip_header_line(name: &str) -> MenuPanelLine {
    panel_line_spans(vec![
        panel_span("Actor: ", PanelSpanStyle::Normal),
        panel_span(name, PanelSpanStyle::Highlight),
        panel_span("  (Left/Right)", PanelSpanStyle::Muted),
    ])
}

fn build_magic_equipped_detail(
    runtime: &GameRuntime,
    actor: &engine::party::Actor,
    slot: &str,
) -> Vec<MenuPanelLine> {
    let Some(item_id) = actor.equipment.get(slot) else {
        return vec![panel_line("Empty slot."), panel_line("Confirm to equip.")];
    };
    let entry = runtime
        .content
        .equipment
        .equipment
        .iter()
        .find(|item| item.id == *item_id);
    if let Some(item) = entry {
        let mut lines = Vec::new();
        lines.push(panel_line_spans(vec![
            panel_span("Equipped: ", PanelSpanStyle::Normal),
            panel_span(item.name.clone(), PanelSpanStyle::Accent),
        ]));
        if !item.spells.is_empty() {
            lines.push(panel_line("Spells:"));
            for spell in &item.spells {
                lines.push(panel_line(format!("  - {}", spell)));
            }
        }
        lines
    } else {
        vec![panel_line("Item not found.")]
    }
}

fn build_magic_item_detail(
    runtime: &GameRuntime,
    actor_id: &str,
    slot: &str,
    entry: &InventoryEntry,
) -> MenuPanelView {
    if entry.id.is_empty() {
        return MenuPanelView {
            title: "Unequip".to_string(),
            lines: vec![panel_line("Remove item from slot.")],
        };
    }
    let equipment = runtime
        .content
        .equipment
        .equipment
        .iter()
        .find(|item| item.id == entry.id);

    if let Some(item) = equipment {
        let mut lines = Vec::new();
        if !item.spells.is_empty() {
            lines.push(panel_line("Grants Spells:"));
            for spell in &item.spells {
                lines.push(panel_line(format!("  - {}", spell)));
            }
        }
        if let Some(owner) = equipped_label(entry) {
            lines.push(panel_line_spans(vec![panel_span(
                owner,
                PanelSpanStyle::Accent,
            )]));
        }

        if !item.stats.is_empty() {
            if let Some(actor) = runtime.party.roster.get(actor_id) {
                let mut clone = actor.clone();
                clone.equipment.insert(slot.to_string(), item.id.clone());
                recompute_derived_stats(&runtime.content, &mut clone);

                lines.push(panel_line(""));
                lines.push(panel_line("Stat changes:"));
                for stat in runtime
                    .content
                    .stats
                    .stats
                    .base
                    .iter()
                    .chain(runtime.content.stats.stats.derived.iter())
                {
                    let current = actor.derived_stats.get(&stat.id).copied().unwrap_or(0);
                    let next = clone.derived_stats.get(&stat.id).copied().unwrap_or(0);
                    if current != next {
                        let diff = next - current;
                        lines.push(panel_line(format!(
                            "{} {} ({} {:+})",
                            stat.name, next, current, diff
                        )));
                    }
                }
            }
        }

        MenuPanelView {
            title: item.name.clone(),
            lines,
        }
    } else {
        MenuPanelView {
            title: "Magic Item".to_string(),
            lines: vec![panel_line("Item not found.")],
        }
    }
}
