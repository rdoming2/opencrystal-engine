use std::collections::HashMap;

use engine::party::{actor_slots, recompute_derived_stats};
use engine::runtime::GameRuntime;
use tui::app::{MenuPanelLine, MenuPanelView, PanelSpanStyle};

use super::common::{InventoryEntry, InventoryKind};
use super::inventory::{
    build_list_line, equipped_label, list_line_width, panel_line, panel_line_spans, panel_span,
};

pub fn build_equipment_panel(runtime: &GameRuntime) -> MenuPanelView {
    let actor_id = detail_actor_id(runtime);
    let Some(actor_id) = actor_id else {
        return MenuPanelView {
            title: "Equipment".to_string(),
            lines: vec![panel_line("No party members.")],
        };
    };
    let actor = match runtime.party.roster.get(&actor_id) {
        Some(actor) => actor,
        None => {
            return MenuPanelView {
                title: "Equipment".to_string(),
                lines: vec![panel_line("No party members.")],
            };
        }
    };
    let header = equipment_header_line(actor.name.as_str());
    let slots = equipment_slots_for_menu(runtime);
    if slots.is_empty() {
        return MenuPanelView {
            title: "Equipment".to_string(),
            lines: vec![panel_line("No equipment slots.")],
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
                format!("{}: {}", slot, equipped),
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
        lines.extend(build_equipped_slot_detail(runtime, actor, &detail_slot));
    } else {
        let entries = equipment_entries_for_menu(runtime);
        if entries.is_empty() {
            return MenuPanelView {
                title: "Equipment".to_string(),
                lines: vec![panel_line("No equipment available.")],
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
            let slot = equipment_slot_for_menu(runtime).unwrap_or_default();
            lines.extend(build_equipment_detail(runtime, &actor_id, &slot, entry).lines);
        }
    }

    MenuPanelView {
        title: "Equipment".to_string(),
        lines,
    }
}

pub fn detail_actor_id(runtime: &GameRuntime) -> Option<String> {
    runtime
        .party
        .active
        .get(runtime.menu_state.detail_actor)
        .cloned()
}

pub fn equipment_slots_for_menu(runtime: &GameRuntime) -> Vec<String> {
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Vec::new(),
    };
    match runtime.party.roster.get(&actor_id) {
        Some(actor) => actor_slots(&runtime.content, actor),
        None => Vec::new(),
    }
}

pub fn equipment_slot_for_menu(runtime: &GameRuntime) -> Option<String> {
    let slots = equipment_slots_for_menu(runtime);
    slots.get(runtime.menu_state.detail_slot).cloned()
}

pub fn equipment_entries_for_menu(runtime: &GameRuntime) -> Vec<InventoryEntry> {
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Vec::new(),
    };
    let slots = equipment_slots_for_menu(runtime);
    let slot_index = runtime
        .menu_state
        .detail_slot
        .min(slots.len().saturating_sub(1));
    let slot = slots.get(slot_index).cloned().unwrap_or_default();
    build_equipment_entries(runtime, &actor_id, &slot)
}

pub fn build_equipped_map(runtime: &GameRuntime) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    for actor in runtime.party.roster.values() {
        for item_id in actor.equipment.values() {
            map.entry(item_id.clone())
                .or_insert_with(Vec::new)
                .push(actor.name.clone());
        }
    }
    map
}

pub fn equip_item(runtime: &mut GameRuntime, actor_id: &str, slot: &str, entry: &InventoryEntry) {
    let target_id = actor_id.to_string();
    if entry.id.is_empty() {
        if let Some(actor) = runtime.party.roster.get_mut(&target_id) {
            actor.equipment.remove(slot);
            recompute_derived_stats(&runtime.content, actor);
        }
        return;
    }
    let mut owner_to_clear = None;
    for (id, actor) in &runtime.party.roster {
        for (equip_slot, item_id) in &actor.equipment {
            if item_id == &entry.id && id != &target_id {
                owner_to_clear = Some((id.clone(), equip_slot.clone()));
                break;
            }
        }
        if owner_to_clear.is_some() {
            break;
        }
    }
    if let Some((owner_id, equip_slot)) = owner_to_clear {
        if let Some(owner) = runtime.party.roster.get_mut(&owner_id) {
            owner.equipment.remove(&equip_slot);
            recompute_derived_stats(&runtime.content, owner);
        }
    }
    if let Some(actor) = runtime.party.roster.get_mut(&target_id) {
        actor.equipment.insert(slot.to_string(), entry.id.clone());
        recompute_derived_stats(&runtime.content, actor);
    }
}

fn equipment_header_line(name: &str) -> MenuPanelLine {
    panel_line_spans(vec![
        panel_span("Actor: ", PanelSpanStyle::Normal),
        panel_span(name, PanelSpanStyle::Highlight),
        panel_span("  (Left/Right)", PanelSpanStyle::Muted),
    ])
}

fn build_equipped_slot_detail(
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
        lines.push(panel_line_spans(vec![
            panel_span("Slot: ", PanelSpanStyle::Normal),
            panel_span(item.slot.clone(), PanelSpanStyle::Accent),
        ]));
        lines.push(panel_line_spans(vec![
            panel_span("Category: ", PanelSpanStyle::Normal),
            panel_span(item.category.clone(), PanelSpanStyle::Accent),
        ]));
        lines.push(panel_line(""));
        for (stat, value) in &item.stats {
            lines.push(panel_line(format!("{} +{}", stat, value)));
        }
        lines
    } else {
        vec![panel_line("Item not found.")]
    }
}

fn build_equipment_entries(
    runtime: &GameRuntime,
    actor_id: &str,
    slot: &str,
) -> Vec<InventoryEntry> {
    let equipped_map = build_equipped_map(runtime);
    let equipped_counts = build_equipped_counts(runtime);
    let actor = match runtime.party.roster.get(actor_id) {
        Some(actor) => actor,
        None => return Vec::new(),
    };
    let job = runtime
        .content
        .jobs
        .jobs
        .iter()
        .find(|job| job.id == actor.job_id);

    let mut entries = Vec::new();
    entries.push(InventoryEntry {
        id: "".to_string(),
        label: "Unequip".to_string(),
        available_qty: 0,
        total_qty: 0,
        kind: InventoryKind::Equipment,
        slot: Some(slot.to_string()),
        category: None,
        usable: true,
        equipped_by: Vec::new(),
        usage_target: String::new(),
    });

    for equipment in &runtime.content.equipment.equipment {
        if !equipment_slot_matches(slot, &equipment.slot) {
            continue;
        }
        if let Some(job) = job {
            if !equipment_allowed(job, equipment) {
                continue;
            }
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

fn build_equipped_counts(runtime: &GameRuntime) -> HashMap<String, i32> {
    let mut map = HashMap::new();
    for actor in runtime.party.roster.values() {
        for item_id in actor.equipment.values() {
            *map.entry(item_id.clone()).or_insert(0) += 1;
        }
    }
    map
}

fn equipment_slot_matches(slot: &str, equipment_slot: &str) -> bool {
    if slot.starts_with("accessory") {
        equipment_slot == "accessory"
    } else {
        slot == equipment_slot
    }
}

fn equipment_allowed(
    job: &engine::entities::JobDefinition,
    equipment: &engine::entities::EquipmentDefinition,
) -> bool {
    if let Some(allowed) = &equipment.allowed_jobs {
        if !allowed.contains(&job.id) {
            return false;
        }
    }
    match equipment.slot.as_str() {
        "weapon" => job.equipment.weapons.contains(&equipment.category),
        "armor" => job.equipment.armor.contains(&equipment.category),
        _ => true,
    }
}

fn build_equipment_detail(
    runtime: &GameRuntime,
    actor_id: &str,
    slot: &str,
    entry: &InventoryEntry,
) -> MenuPanelView {
    if entry.id.is_empty() {
        return MenuPanelView {
            title: "Unequip".to_string(),
            lines: vec![panel_line("Remove equipment from slot.")],
        };
    }
    let equipment = runtime
        .content
        .equipment
        .equipment
        .iter()
        .find(|item| item.id == entry.id);
    let mut lines = Vec::new();
    if let Some(equipment) = equipment {
        lines.push(panel_line(format!("Slot: {}", equipment.slot)));
        lines.push(panel_line(format!("Category: {}", equipment.category)));
        if let Some(owner) = equipped_label(entry) {
            lines.push(panel_line_spans(vec![panel_span(
                owner,
                PanelSpanStyle::Accent,
            )]));
        }
        if !actor_id.is_empty() {
            if let Some(actor) = runtime.party.roster.get(actor_id) {
                let preview = preview_equipment_delta(runtime, actor, slot, equipment);
                lines.extend(preview);
            }
        } else {
            lines.push(panel_line(""));
            lines.push(panel_line("Stats:"));
            for (stat, value) in &equipment.stats {
                lines.push(panel_line(format!("{} +{}", stat, value)));
            }
        }
        MenuPanelView {
            title: equipment.name.clone(),
            lines,
        }
    } else {
        MenuPanelView {
            title: "Equipment".to_string(),
            lines: vec![panel_line("Equipment not found.")],
        }
    }
}

fn preview_equipment_delta(
    runtime: &GameRuntime,
    actor: &engine::party::Actor,
    slot: &str,
    equipment: &engine::entities::EquipmentDefinition,
) -> Vec<MenuPanelLine> {
    let mut lines = Vec::new();
    let mut clone = actor.clone();
    clone
        .equipment
        .insert(slot.to_string(), equipment.id.clone());
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
    if lines.len() == 2 {
        lines.push(panel_line("No stat changes."));
    }
    lines
}
