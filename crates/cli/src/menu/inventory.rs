use engine::runtime::GameRuntime;
use tui::menu::{MenuPanelLine, MenuPanelSpan, MenuPanelView, PanelSpanStyle};

use super::common::{
    InventoryEntry, InventoryFilter, InventoryKind, InventorySort, filter_from_index,
    sort_from_index,
};
use super::equipment::build_equipped_map;

pub fn build_items_panel(runtime: &GameRuntime) -> MenuPanelView {
    let filter = filter_from_index(runtime.menu_state.detail_filter);
    let sort = sort_from_index(runtime.menu_state.detail_sort);
    let entries = build_inventory_entries(runtime, &filter, &sort);
    if entries.is_empty() {
        return MenuPanelView {
            title: "Items".to_string(),
            lines: vec![panel_line("No items available.")],
        };
    }
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    let header = inventory_filter_line(&filter, &sort);
    let mut lines = Vec::new();
    let width = list_line_width(&entries);
    lines.push(header);
    for (index, entry) in entries.iter().enumerate() {
        lines.push(build_list_line(entry, index == selection, width));
    }
    lines.push(panel_line("------------------------------"));
    if runtime.menu_state.detail_page == 1 {
        lines.extend(build_item_target_panel(runtime, entries.get(selection)));
        lines.push(panel_line("------------------------------"));
    }
    lines.push(panel_line_spans(vec![panel_span(
        "Details",
        PanelSpanStyle::Accent,
    )]));
    lines.extend(build_item_description(runtime, entries.get(selection)));

    MenuPanelView {
        title: "Items".to_string(),
        lines,
    }
}

pub fn build_inventory_entries(
    runtime: &GameRuntime,
    filter: &InventoryFilter,
    sort: &InventorySort,
) -> Vec<InventoryEntry> {
    let equipped_map = build_equipped_map(runtime);
    let mut entries = Vec::new();

    if matches!(filter, InventoryFilter::Items) {
        for item in &runtime.content.items.items {
            let qty = runtime.inventory.item_qty(&item.id);
            if qty <= 0 {
                continue;
            }
            let usable = item_usage_allows_field(&item.usage.context);
            entries.push(InventoryEntry {
                id: item.id.clone(),
                label: item.name.clone(),
                available_qty: qty,
                total_qty: qty,
                kind: InventoryKind::Item,
                slot: None,
                category: None,
                usable,
                equipped_by: Vec::new(),
                usage_target: item.usage.target.clone(),
            });
        }
    } else {
        for equipment in &runtime.content.equipment.equipment {
            if !matches_filter_equipment(filter, equipment) {
                continue;
            }
            let qty = runtime.inventory.equipment_qty(&equipment.id);
            let equipped_by = equipped_map.get(&equipment.id).cloned().unwrap_or_default();
            if qty <= 0 && equipped_by.is_empty() {
                continue;
            }
            let total_qty = qty + equipped_by.len() as i32;
            entries.push(InventoryEntry {
                id: equipment.id.clone(),
                label: equipment.name.clone(),
                available_qty: qty.max(0),
                total_qty,
                kind: InventoryKind::Equipment,
                slot: Some(equipment.slot.clone()),
                category: Some(equipment.category.clone()),
                usable: false,
                equipped_by,
                usage_target: String::new(),
            });
        }
    }

    entries.sort_by(|left, right| inventory_sort_key(left, right, sort));
    entries
}

pub fn item_targets_for_entry(runtime: &GameRuntime, entry: &InventoryEntry) -> Vec<String> {
    let item = match runtime
        .content
        .items
        .items
        .iter()
        .find(|item| item.id == entry.id)
    {
        Some(item) => item,
        None => return Vec::new(),
    };
    build_item_targets(runtime, item)
}

pub fn apply_item_to_targets(
    runtime: &mut GameRuntime,
    entry: &InventoryEntry,
    targets: &[String],
) -> bool {
    let item = match runtime
        .content
        .items
        .items
        .iter()
        .find(|item| item.id == entry.id)
        .cloned()
    {
        Some(item) => item,
        None => return false,
    };
    if !item_usage_allows_field(&item.usage.context) {
        return false;
    }
    for target_id in targets {
        apply_item_to_actor(runtime, &item, target_id);
    }
    true
}

pub fn list_line_width(entries: &[InventoryEntry]) -> usize {
    entries
        .iter()
        .map(|entry| entry.label.chars().count())
        .max()
        .unwrap_or(10)
        + 2
}

pub fn build_list_line(entry: &InventoryEntry, is_selected: bool, width: usize) -> MenuPanelLine {
    let prefix = if is_selected { "> " } else { "  " };
    let mut spans = Vec::new();
    let label_style = if is_selected {
        PanelSpanStyle::Highlight
    } else if entry.usable {
        PanelSpanStyle::Normal
    } else {
        PanelSpanStyle::Muted
    };
    let count_text = match entry.kind {
        InventoryKind::Item => format!("x{}", entry.total_qty),
        InventoryKind::Equipment => format!("{}/{}", entry.available_qty, entry.total_qty),
    };
    spans.push(panel_span(
        prefix,
        if is_selected {
            PanelSpanStyle::Highlight
        } else {
            PanelSpanStyle::Normal
        },
    ));
    spans.push(panel_span(
        format!("{:<width$}", entry.label, width = width),
        label_style,
    ));
    spans.push(panel_span(
        format!("{:>6}", count_text),
        PanelSpanStyle::Accent,
    ));
    if let Some(owner) = equipped_label(entry) {
        spans.push(panel_span(format!(" {}", owner), PanelSpanStyle::Accent));
    }
    panel_line_spans(spans)
}

pub fn panel_line(text: impl Into<String>) -> MenuPanelLine {
    MenuPanelLine {
        spans: vec![MenuPanelSpan {
            text: text.into(),
            style: PanelSpanStyle::Normal,
        }],
    }
}

pub fn panel_line_spans(spans: Vec<MenuPanelSpan>) -> MenuPanelLine {
    MenuPanelLine { spans }
}

pub fn panel_span(text: impl Into<String>, style: PanelSpanStyle) -> MenuPanelSpan {
    MenuPanelSpan {
        text: text.into(),
        style,
    }
}

pub fn equipped_label(entry: &InventoryEntry) -> Option<String> {
    if entry.equipped_by.is_empty() {
        None
    } else {
        Some(format!("Equipped: {}", entry.equipped_by.join(", ")))
    }
}

pub fn item_usage_allows_field(context: &str) -> bool {
    matches!(context, "field" | "both")
}

pub fn item_usage_allows_battle(context: &str) -> bool {
    matches!(context, "battle" | "both")
}

pub fn apply_item_to_actor(
    runtime: &mut GameRuntime,
    item: &engine::entities::ItemDefinition,
    actor_id: &str,
) {
    if item.effect.r#type == "learn_spell" {
        if let Some(spell) = &item.effect.target {
            engine::party::learn_spell_event(&mut runtime.party, actor_id, spell);
        }
        return;
    }
    let Some(actor) = runtime.party.roster.get_mut(actor_id) else {
        return;
    };
    let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
    let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
    let power = item.effect.power.unwrap_or(0);
    match item.effect.r#type.as_str() {
        "heal_hp" => {
            actor.current_hp = (actor.current_hp + power).clamp(0, max_hp);
        }
        "heal_mp" => {
            actor.current_mp = (actor.current_mp + power).clamp(0, max_mp);
        }
        "revive" => {
            if actor.current_hp <= 0 {
                let amount = if power > 0 { power } else { max_hp };
                actor.current_hp = amount.clamp(1, max_hp);
            }
        }
        _ => {}
    }
}

fn matches_filter_equipment(
    filter: &InventoryFilter,
    equipment: &engine::entities::EquipmentDefinition,
) -> bool {
    match filter {
        InventoryFilter::Equipment => true,
        InventoryFilter::Weapons => equipment.slot == "weapon",
        InventoryFilter::Armor => equipment.slot == "armor",
        InventoryFilter::Accessory => equipment.slot == "accessory",
        InventoryFilter::Items => false,
    }
}

fn inventory_sort_key(
    left: &InventoryEntry,
    right: &InventoryEntry,
    sort: &InventorySort,
) -> std::cmp::Ordering {
    match sort {
        InventorySort::Name => left.label.cmp(&right.label),
        InventorySort::Type => {
            let left_kind = match left.kind {
                InventoryKind::Item => 0,
                InventoryKind::Equipment => 1,
            };
            let right_kind = match right.kind {
                InventoryKind::Item => 0,
                InventoryKind::Equipment => 1,
            };
            left_kind
                .cmp(&right_kind)
                .then_with(|| left.slot.cmp(&right.slot))
                .then_with(|| left.category.cmp(&right.category))
                .then_with(|| left.label.cmp(&right.label))
        }
    }
}

fn inventory_filters() -> Vec<String> {
    vec![
        "Items".to_string(),
        "Equipment".to_string(),
        "Weapons".to_string(),
        "Armor".to_string(),
        "Accessory".to_string(),
    ]
}

fn filter_label(filter: &InventoryFilter) -> String {
    match filter {
        InventoryFilter::Items => "Items",
        InventoryFilter::Equipment => "Equipment",
        InventoryFilter::Weapons => "Weapons",
        InventoryFilter::Armor => "Armor",
        InventoryFilter::Accessory => "Accessory",
    }
    .to_string()
}

fn sort_label(sort: &InventorySort) -> &'static str {
    match sort {
        InventorySort::Name => "Name",
        InventorySort::Type => "Type",
    }
}

fn inventory_filter_line(filter: &InventoryFilter, sort: &InventorySort) -> MenuPanelLine {
    let mut spans = Vec::new();
    spans.push(panel_span("Filter: ", PanelSpanStyle::Normal));
    for (index, entry) in inventory_filters().into_iter().enumerate() {
        if index > 0 {
            spans.push(panel_span(" | ", PanelSpanStyle::Muted));
        }
        let style = if entry == filter_label(filter) {
            PanelSpanStyle::Highlight
        } else {
            PanelSpanStyle::Muted
        };
        spans.push(panel_span(entry, style));
    }
    spans.push(panel_span("  Sort: ", PanelSpanStyle::Normal));
    spans.push(panel_span(sort_label(sort), PanelSpanStyle::Accent));
    panel_line_spans(spans)
}

fn build_item_target_panel(
    runtime: &GameRuntime,
    entry: Option<&InventoryEntry>,
) -> Vec<MenuPanelLine> {
    let Some(entry) = entry else {
        return vec![panel_line("No target."), panel_line("")];
    };
    let targets = item_targets_for_entry(runtime, entry);
    if targets.is_empty() {
        return vec![panel_line("No valid targets."), panel_line("")];
    }
    let selection = runtime
        .menu_state
        .detail_target
        .min(targets.len().saturating_sub(1));
    let mut lines = Vec::new();
    lines.push(panel_line_spans(vec![panel_span(
        "Target",
        PanelSpanStyle::Accent,
    )]));
    for (index, target_id) in targets.iter().enumerate() {
        let name = runtime
            .party
            .roster
            .get(target_id)
            .map(|actor| actor.name.as_str())
            .unwrap_or(target_id.as_str());
        let is_selected = index == selection;
        lines.push(panel_line_spans(vec![
            panel_span(
                if is_selected { "> " } else { "  " },
                if is_selected {
                    PanelSpanStyle::Highlight
                } else {
                    PanelSpanStyle::Normal
                },
            ),
            panel_span(
                name,
                if is_selected {
                    PanelSpanStyle::Highlight
                } else {
                    PanelSpanStyle::Normal
                },
            ),
        ]));
    }
    lines
}

fn build_item_targets(
    runtime: &GameRuntime,
    item: &engine::entities::ItemDefinition,
) -> Vec<String> {
    let mut targets = runtime.party.active.clone();
    match item.effect.r#type.as_str() {
        "revive" => {
            targets.retain(|id| {
                runtime
                    .party
                    .roster
                    .get(id)
                    .map(|actor| actor.current_hp <= 0)
                    .unwrap_or(false)
            });
        }
        "learn_spell" => {
            let spell_id = item.effect.target.as_deref().unwrap_or("");
            targets.retain(|id| {
                let Some(actor) = runtime.party.roster.get(id) else {
                    return false;
                };
                if actor.spells.iter().any(|s| s == spell_id) {
                    return false;
                }
                let Some(job) = runtime
                    .content
                    .jobs
                    .jobs
                    .iter()
                    .find(|j| j.id == actor.job_id)
                else {
                    return false;
                };
                job.spells.iter().any(|s| {
                    s.id == spell_id
                        && s.method == "item"
                        && (s.item.is_none() || s.item.as_deref() == Some(item.id.as_str()))
                })
            });
        }
        _ => {}
    }
    targets
}

pub fn build_item_description(
    runtime: &GameRuntime,
    entry: Option<&InventoryEntry>,
) -> Vec<MenuPanelLine> {
    let Some(entry) = entry else {
        return vec![panel_line("No selection.")];
    };
    match entry.kind {
        InventoryKind::Item => {
            let item = runtime
                .content
                .items
                .items
                .iter()
                .find(|item| item.id == entry.id);
            if let Some(item) = item {
                let mut lines = Vec::new();
                let power_text = item
                    .effect
                    .power
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string());
                lines.push(panel_line_spans(vec![
                    panel_span("Usage: ", PanelSpanStyle::Normal),
                    panel_span(item.usage.context.clone(), PanelSpanStyle::Accent),
                    panel_span("  Target: ", PanelSpanStyle::Normal),
                    panel_span(item.usage.target.clone(), PanelSpanStyle::Accent),
                ]));
                lines.push(panel_line_spans(vec![
                    panel_span("Effect: ", PanelSpanStyle::Normal),
                    panel_span(item.effect.r#type.clone(), PanelSpanStyle::Accent),
                    panel_span("  Power: ", PanelSpanStyle::Normal),
                    panel_span(power_text, PanelSpanStyle::Accent),
                ]));
                let description = item
                    .description
                    .clone()
                    .unwrap_or_else(|| "No description.".to_string());
                lines.push(panel_line_spans(vec![
                    panel_span("Description: ", PanelSpanStyle::Accent),
                    panel_span(description, PanelSpanStyle::Normal),
                ]));
                if !entry.usable {
                    lines.push(panel_line("Cannot use in field."));
                }
                lines
            } else {
                vec![panel_line("Item not found.")]
            }
        }
        InventoryKind::Equipment => vec![panel_line("Select equipment in Equipment menu.")],
    }
}

pub fn build_battle_item_entries(runtime: &GameRuntime) -> Vec<InventoryEntry> {
    let mut entries = Vec::new();
    for item in &runtime.content.items.items {
        let qty = runtime.inventory.item_qty(&item.id);
        if qty <= 0 {
            continue;
        }
        if !item_usage_allows_battle(&item.usage.context) {
            continue;
        }
        entries.push(InventoryEntry {
            id: item.id.clone(),
            label: item.name.clone(),
            available_qty: qty,
            total_qty: qty,
            kind: InventoryKind::Item,
            slot: None,
            category: None,
            usable: true,
            equipped_by: Vec::new(),
            usage_target: item.usage.target.clone(),
        });
    }
    entries.sort_by(|left, right| left.label.cmp(&right.label));
    entries
}
