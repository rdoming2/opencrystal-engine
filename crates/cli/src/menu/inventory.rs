use engine::rules::MagicAcquisition;
use std::collections::{HashMap, HashSet};

use engine::runtime::GameRuntime;
use tui::menu::{MenuPanelLine, MenuPanelSpan, MenuPanelView, PanelSpanStyle};

use super::common::{
    filter_from_index, sort_from_index, InventoryEntry, InventoryFilter, InventoryKind,
    InventorySort,
};
use super::equipment::build_equipped_map;

pub fn build_items_panel(runtime: &GameRuntime) -> MenuPanelView {
    let filter = filter_from_index(runtime.menu_state.detail_filter);
    let sort = sort_from_index(runtime.menu_state.detail_sort);
    let entries = build_inventory_entries(runtime, &filter);
    if entries.is_empty() {
        let header = inventory_filter_line(&filter, &sort);
        let mut lines = Vec::new();
        lines.push(header);
        lines.push(panel_line("No items available."));
        lines.push(panel_line("------------------------------"));
        lines.push(panel_line_spans(vec![panel_span(
            "Details",
            PanelSpanStyle::Accent,
        )]));
        lines.push(panel_line("Select an item to view details."));
        return MenuPanelView {
            title: "Items".to_string(),
            lines,
        };
    }
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    let list_selection = if runtime.menu_state.detail_page == 0 {
        selection
    } else {
        runtime
            .menu_state
            .detail_slot
            .min(entries.len().saturating_sub(1))
    };
    let header = inventory_filter_line(&filter, &sort);
    let mut lines = Vec::new();
    let width = list_line_width(&entries);
    lines.push(header);
    for (index, entry) in entries.iter().enumerate() {
        lines.push(build_list_line(entry, index == list_selection, width));
    }
    lines.push(panel_line("------------------------------"));
    if runtime.menu_state.detail_page == 1 {
        lines.extend(build_item_action_panel(
            runtime,
            entries.get(list_selection),
        ));
        lines.push(panel_line("------------------------------"));
    } else if runtime.menu_state.detail_page == 2 {
        lines.extend(build_item_target_panel(
            runtime,
            entries.get(list_selection),
        ));
        lines.push(panel_line("------------------------------"));
    } else if runtime.menu_state.detail_page == 3 {
        lines.extend(build_item_move_panel(runtime, &entries));
        lines.push(panel_line("------------------------------"));
    }
    lines.push(panel_line_spans(vec![panel_span(
        "Details",
        PanelSpanStyle::Accent,
    )]));
    lines.extend(build_item_description(runtime, entries.get(list_selection)));

    MenuPanelView {
        title: "Items".to_string(),
        lines,
    }
}

pub fn build_inventory_entries(
    runtime: &GameRuntime,
    filter: &InventoryFilter,
) -> Vec<InventoryEntry> {
    let equipped_map = build_equipped_map(runtime);
    let mut entries = Vec::new();

    if matches!(filter, InventoryFilter::Items | InventoryFilter::KeyItems) {
        for item_id in ordered_item_ids(runtime) {
            let qty = runtime.inventory.item_qty(&item_id);
            if qty <= 0 {
                continue;
            }
            let Some(item) = find_item_definition(runtime, &item_id) else {
                continue;
            };
            if !matches_item_filter(filter, item) {
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
        for equipment_id in ordered_equipment_ids(runtime, &equipped_map) {
            let Some(equipment) = find_equipment_definition(runtime, &equipment_id) else {
                continue;
            };
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
    entries
}

pub fn apply_inventory_sort_action(runtime: &mut GameRuntime, sort: InventorySort) {
    match sort {
        InventorySort::Manual => {}
        InventorySort::Name => {
            let mut item_ids = runtime
                .inventory
                .items
                .iter()
                .filter_map(|(id, qty)| if *qty > 0 { Some(id.clone()) } else { None })
                .collect::<Vec<_>>();
            item_ids.sort_by(|a, b| {
                let left = find_item_definition(runtime, a)
                    .map(|item| item.name.as_str())
                    .unwrap_or(a.as_str());
                let right = find_item_definition(runtime, b)
                    .map(|item| item.name.as_str())
                    .unwrap_or(b.as_str());
                left.cmp(right)
            });
            runtime.inventory.items_order = item_ids;

            let mut equipment_ids = runtime
                .inventory
                .equipment
                .iter()
                .filter_map(|(id, qty)| if *qty > 0 { Some(id.clone()) } else { None })
                .collect::<Vec<_>>();
            equipment_ids.sort_by(|a, b| {
                let left = find_equipment_definition(runtime, a)
                    .map(|item| item.name.as_str())
                    .unwrap_or(a.as_str());
                let right = find_equipment_definition(runtime, b)
                    .map(|item| item.name.as_str())
                    .unwrap_or(b.as_str());
                left.cmp(right)
            });
            runtime.inventory.equipment_order = equipment_ids;
        }
        InventorySort::TypePower => {
            let mut item_ids = runtime
                .inventory
                .items
                .iter()
                .filter_map(|(id, qty)| if *qty > 0 { Some(id.clone()) } else { None })
                .collect::<Vec<_>>();
            item_ids.sort_by(|a, b| {
                let left = find_item_definition(runtime, a);
                let right = find_item_definition(runtime, b);
                let left_type = left.map(|item| item.r#type.as_str()).unwrap_or("");
                let right_type = right.map(|item| item.r#type.as_str()).unwrap_or("");
                let left_power = left.map(item_power).unwrap_or(0);
                let right_power = right.map(item_power).unwrap_or(0);
                left_type
                    .cmp(right_type)
                    .then_with(|| left_power.cmp(&right_power))
                    .then_with(|| {
                        let left_name = left.map(|item| item.name.as_str()).unwrap_or(a.as_str());
                        let right_name = right.map(|item| item.name.as_str()).unwrap_or(b.as_str());
                        left_name.cmp(right_name)
                    })
            });
            runtime.inventory.items_order = item_ids;

            let mut equipment_ids = runtime
                .inventory
                .equipment
                .iter()
                .filter_map(|(id, qty)| if *qty > 0 { Some(id.clone()) } else { None })
                .collect::<Vec<_>>();
            equipment_ids.sort_by(|a, b| {
                let left = find_equipment_definition(runtime, a);
                let right = find_equipment_definition(runtime, b);
                let left_slot = left.map(|item| item.slot.as_str()).unwrap_or("");
                let right_slot = right.map(|item| item.slot.as_str()).unwrap_or("");
                let left_category = left.map(|item| item.category.as_str()).unwrap_or("");
                let right_category = right.map(|item| item.category.as_str()).unwrap_or("");
                let left_power = left.map(equipment_power).unwrap_or(0);
                let right_power = right.map(equipment_power).unwrap_or(0);
                left_slot
                    .cmp(right_slot)
                    .then_with(|| left_category.cmp(right_category))
                    .then_with(|| left_power.cmp(&right_power))
                    .then_with(|| {
                        let left_name = left.map(|item| item.name.as_str()).unwrap_or(a.as_str());
                        let right_name = right.map(|item| item.name.as_str()).unwrap_or(b.as_str());
                        left_name.cmp(right_name)
                    })
            });
            runtime.inventory.equipment_order = equipment_ids;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ItemActionId {
    Use,
    Drop,
    Move,
}

#[derive(Clone, Debug)]
struct ItemActionEntry {
    id: ItemActionId,
    label: &'static str,
    enabled: bool,
}

fn can_drop_item(total_qty: i32, unique: bool) -> bool {
    total_qty > 0 && !unique
}

fn can_drop_inventory_entry(runtime: Option<&GameRuntime>, entry: &InventoryEntry) -> bool {
    match entry.kind {
        InventoryKind::Item => {
            let Some(runtime) = runtime else {
                return can_drop_item(entry.total_qty, false);
            };
            let unique = find_item_definition(runtime, &entry.id)
                .map(|item| item.unique)
                .unwrap_or(false);
            can_drop_item(entry.total_qty, unique)
        }
        InventoryKind::Equipment => entry.available_qty > 0,
    }
}

fn item_action_entries(
    runtime: Option<&GameRuntime>,
    entry: Option<&InventoryEntry>,
) -> Vec<ItemActionEntry> {
    let Some(entry) = entry else {
        return Vec::new();
    };
    let can_use = entry.kind == InventoryKind::Item && entry.usable;
    let can_drop = can_drop_inventory_entry(runtime, entry);
    vec![
        ItemActionEntry {
            id: ItemActionId::Use,
            label: "Use",
            enabled: can_use,
        },
        ItemActionEntry {
            id: ItemActionId::Drop,
            label: "Drop",
            enabled: can_drop,
        },
        ItemActionEntry {
            id: ItemActionId::Move,
            label: "Move",
            enabled: true,
        },
    ]
}

pub(super) fn item_actions_len(entry: Option<&InventoryEntry>) -> usize {
    item_action_entries(None, entry).len()
}

pub(super) fn item_action_for_entry_with_runtime(
    runtime: &GameRuntime,
    entry: Option<&InventoryEntry>,
    index: usize,
) -> Option<(ItemActionId, bool)> {
    let actions = item_action_entries(Some(runtime), entry);
    actions.get(index).map(|action| (action.id, action.enabled))
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

pub struct ItemUseResult {
    pub consumed: bool,
    pub message: Option<String>,
}

pub fn apply_item_to_targets(
    runtime: &mut GameRuntime,
    entry: &InventoryEntry,
    targets: &[String],
) -> ItemUseResult {
    let warp_message = Some("The party was whisked outside.".to_string());
    let item = match runtime
        .content
        .items
        .items
        .iter()
        .find(|item| item.id == entry.id)
        .cloned()
    {
        Some(item) => item,
        None => {
            return ItemUseResult {
                consumed: false,
                message: None,
            };
        }
    };
    if !item_usage_allows_field(&item.usage.context) {
        return ItemUseResult {
            consumed: false,
            message: None,
        };
    }
    if item.effect.r#type == "learn_recipe" {
        if let Some(recipe_id) = item.effect.target.as_deref() {
            if let Some(flag) = recipe_unlock_flag(runtime, recipe_id) {
                if runtime.has_flag(&flag) {
                    return ItemUseResult {
                        consumed: false,
                        message: Some("No valid targets.".to_string()),
                    };
                }
                runtime.set_flag(&flag);
                return ItemUseResult {
                    consumed: true,
                    message: None,
                };
            }
        }
        return ItemUseResult {
            consumed: false,
            message: Some("No valid targets.".to_string()),
        };
    }
    if item.effect.r#type == "warp" {
        if let Some(destination) = &item.effect.destination {
            runtime.warp_to_map(&destination.map, (destination.pos[0], destination.pos[1]));
            return ItemUseResult {
                consumed: true,
                message: warp_message,
            };
        }
        if item.effect.target.as_deref() == Some("last_overworld") {
            if runtime.warp_to_last_overworld() {
                return ItemUseResult {
                    consumed: true,
                    message: warp_message,
                };
            } else {
                return ItemUseResult {
                    consumed: false,
                    message: Some("The scroll has no effect.".to_string()),
                };
            }
        }
        return ItemUseResult {
            consumed: false,
            message: None,
        };
    }
    let valid_targets = targets
        .iter()
        .filter_map(|target_id| {
            let actor = runtime.party.roster.get(target_id)?;
            if item_has_effect_on_actor(runtime, &item, actor, false) {
                Some(target_id.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if valid_targets.is_empty() {
        return ItemUseResult {
            consumed: false,
            message: Some("No valid targets.".to_string()),
        };
    }
    for target_id in &valid_targets {
        apply_item_to_actor(runtime, &item, target_id);
    }
    ItemUseResult {
        consumed: true,
        message: None,
    }
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
            palette: None,
        }],
    }
}

pub fn panel_line_spans(spans: Vec<MenuPanelSpan>) -> MenuPanelLine {
    MenuPanelLine { spans }
}

pub fn panel_span(text: impl Into<String>, style: PanelSpanStyle) -> MenuPanelSpan {
    panel_span_with_palette(text, style, None)
}

pub fn panel_span_with_palette(
    text: impl Into<String>,
    style: PanelSpanStyle,
    palette: Option<String>,
) -> MenuPanelSpan {
    MenuPanelSpan {
        text: text.into(),
        style,
        palette,
    }
}

pub fn equipped_label(entry: &InventoryEntry) -> Option<String> {
    if entry.equipped_by.is_empty() {
        None
    } else {
        Some(format!("Equipped: {}", entry.equipped_by.join(", ")))
    }
}

fn ordered_item_ids(runtime: &GameRuntime) -> Vec<String> {
    let mut ids = runtime.inventory.items_order.clone();
    let seen = ids.iter().cloned().collect::<HashSet<_>>();
    let mut missing = runtime
        .inventory
        .items
        .iter()
        .filter_map(|(id, qty)| {
            if *qty > 0 && !seen.contains(id) {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    missing.sort();
    ids.extend(missing);
    ids
}

fn ordered_equipment_ids(
    runtime: &GameRuntime,
    equipped_map: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut ids = runtime.inventory.equipment_order.clone();
    let seen = ids.iter().cloned().collect::<HashSet<_>>();
    let mut missing = runtime
        .inventory
        .equipment
        .iter()
        .filter_map(|(id, qty)| {
            if *qty > 0 && !seen.contains(id) {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    for id in equipped_map.keys() {
        if !seen.contains(id) {
            missing.push(id.clone());
        }
    }
    missing.sort_by(|a, b| {
        let left = find_equipment_definition(runtime, a)
            .map(|item| item.name.as_str())
            .unwrap_or(a.as_str());
        let right = find_equipment_definition(runtime, b)
            .map(|item| item.name.as_str())
            .unwrap_or(b.as_str());
        left.cmp(right)
    });
    ids.extend(missing);
    ids
}

pub fn item_usage_allows_field(context: &str) -> bool {
    matches!(context, "field" | "both")
}

pub fn item_usage_allows_battle(context: &str) -> bool {
    matches!(context, "battle" | "both")
}

fn find_item_definition<'a>(
    runtime: &'a GameRuntime,
    item_id: &str,
) -> Option<&'a engine::entities::ItemDefinition> {
    runtime
        .content
        .items
        .items
        .iter()
        .find(|item| item.id == item_id)
}

fn find_equipment_definition<'a>(
    runtime: &'a GameRuntime,
    item_id: &str,
) -> Option<&'a engine::entities::EquipmentDefinition> {
    runtime
        .content
        .equipment
        .equipment
        .iter()
        .find(|item| item.id == item_id)
}

fn item_power(item: &engine::entities::ItemDefinition) -> i32 {
    item.effect.power.unwrap_or(0)
}

fn equipment_power(item: &engine::entities::EquipmentDefinition) -> i32 {
    item.stats
        .values()
        .filter(|value| **value > 0)
        .copied()
        .sum()
}

fn field_item_heal_has_effect(current_hp: i32, max_hp: i32, power: i32) -> bool {
    current_hp > 0 && current_hp < max_hp && power > 0
}

fn apply_field_item_heal(current_hp: i32, max_hp: i32, power: i32) -> i32 {
    if current_hp <= 0 {
        current_hp
    } else {
        (current_hp + power).clamp(0, max_hp)
    }
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
            actor.current_hp = apply_field_item_heal(actor.current_hp, max_hp, power);
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
        "cure_status" => {
            if !item.effect.statuses.is_empty() {
                actor
                    .statuses
                    .retain(|status| !item.effect.statuses.contains(&status.id));
            }
        }
        _ => {}
    }
}

fn recipe_unlock_flag(runtime: &GameRuntime, recipe_id: &str) -> Option<String> {
    let cooking = runtime.content.cooking.as_ref()?;
    cooking
        .recipes
        .iter()
        .find(|recipe| recipe.id == recipe_id)
        .and_then(|recipe| recipe.unlock_flag.clone())
        .filter(|flag| !flag.trim().is_empty())
}

fn matches_filter_equipment(
    filter: &InventoryFilter,
    equipment: &engine::entities::EquipmentDefinition,
) -> bool {
    match filter {
        InventoryFilter::KeyItems => false,
        InventoryFilter::Equipment => true,
        InventoryFilter::Weapons => equipment.slot == "weapon",
        InventoryFilter::Armor => equipment.slot == "armor",
        InventoryFilter::Accessory => equipment.slot == "accessory",
        InventoryFilter::Items => false,
    }
}

fn matches_item_filter(filter: &InventoryFilter, item: &engine::entities::ItemDefinition) -> bool {
    match filter {
        InventoryFilter::Items => item.r#type != "key_item",
        InventoryFilter::KeyItems => item.r#type == "key_item",
        _ => false,
    }
}

fn inventory_filters() -> Vec<String> {
    vec![
        "Items".to_string(),
        "Key Items".to_string(),
        "Equipment".to_string(),
        "Weapons".to_string(),
        "Armor".to_string(),
        "Accessory".to_string(),
    ]
}

fn filter_label(filter: &InventoryFilter) -> String {
    match filter {
        InventoryFilter::Items => "Items",
        InventoryFilter::KeyItems => "Key Items",
        InventoryFilter::Equipment => "Equipment",
        InventoryFilter::Weapons => "Weapons",
        InventoryFilter::Armor => "Armor",
        InventoryFilter::Accessory => "Accessory",
    }
    .to_string()
}

fn sort_label(sort: &InventorySort) -> &'static str {
    match sort {
        InventorySort::Manual => "Manual",
        InventorySort::Name => "Name",
        InventorySort::TypePower => "Type/Power",
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
    spans.push(panel_span("  Order: ", PanelSpanStyle::Normal));
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
    let name_width = targets
        .iter()
        .map(|target_id| {
            runtime
                .party
                .roster
                .get(target_id)
                .map(|actor| actor.name.chars().count())
                .unwrap_or_else(|| target_id.chars().count())
        })
        .max()
        .unwrap_or(8);
    let mut lines = Vec::new();
    lines.push(panel_line_spans(vec![panel_span(
        "Target",
        PanelSpanStyle::Accent,
    )]));
    for (index, target_id) in targets.iter().enumerate() {
        let (name, current_hp, max_hp, current_mp, max_mp, status_text) =
            if let Some(actor) = runtime.party.roster.get(target_id) {
                let status_labels = build_short_status_labels(runtime, actor);
                let status_text = format_status_inline(&status_labels);
                (
                    actor.name.as_str(),
                    actor.current_hp,
                    actor.derived_stats.get("hp").copied().unwrap_or(0),
                    actor.current_mp,
                    actor.derived_stats.get("mp").copied().unwrap_or(0),
                    status_text,
                )
            } else {
                (target_id.as_str(), 0, 0, 0, 0, None)
            };
        let is_selected = index == selection;
        let name_style = if is_selected {
            PanelSpanStyle::Highlight
        } else {
            PanelSpanStyle::Normal
        };
        let stat_style = if is_selected {
            PanelSpanStyle::Highlight
        } else {
            PanelSpanStyle::Accent
        };
        let status_style = if is_selected {
            PanelSpanStyle::Highlight
        } else {
            PanelSpanStyle::Accent
        };
        let mut spans = vec![
            panel_span(if is_selected { "> " } else { "  " }, name_style),
            panel_span(format!("{:<width$}", name, width = name_width), name_style),
            panel_span(
                format!(
                    " HP {}/{}  MP {}/{}",
                    current_hp, max_hp, current_mp, max_mp
                ),
                stat_style,
            ),
        ];
        if let Some(status_text) = status_text {
            spans.push(panel_span(format!("  {}", status_text), status_style));
        }
        lines.push(panel_line_spans(spans));
    }
    lines
}

fn build_short_status_labels(runtime: &GameRuntime, actor: &engine::party::Actor) -> Vec<String> {
    actor
        .statuses
        .iter()
        .filter_map(|status| {
            engine::battle::status_short_label(&runtime.content, &status.id).or_else(|| {
                engine::battle::status_definition(&runtime.content, &status.id)
                    .map(|definition| definition.label.clone())
            })
        })
        .collect()
}

fn format_status_inline(statuses: &[String]) -> Option<String> {
    if statuses.is_empty() {
        return None;
    }
    let cap = 3;
    let mut labels = statuses.iter().take(cap).cloned().collect::<Vec<_>>();
    if statuses.len() > cap {
        if let Some(last) = labels.pop() {
            labels.push(format!("{}+", last));
        }
    }
    Some(labels.join(", "))
}

fn build_item_action_panel(
    runtime: &GameRuntime,
    entry: Option<&InventoryEntry>,
) -> Vec<MenuPanelLine> {
    let actions = item_action_entries(Some(runtime), entry);
    if actions.is_empty() {
        return vec![panel_line("No actions available."), panel_line("")];
    }
    let selection = runtime
        .menu_state
        .detail_selection
        .min(actions.len().saturating_sub(1));
    let mut lines = Vec::new();
    lines.push(panel_line_spans(vec![panel_span(
        "Actions",
        PanelSpanStyle::Accent,
    )]));
    for (index, action) in actions.iter().enumerate() {
        let is_selected = index == selection;
        let mut style = if action.enabled {
            PanelSpanStyle::Normal
        } else {
            PanelSpanStyle::Muted
        };
        if is_selected {
            style = PanelSpanStyle::Highlight;
        }
        let prefix = if is_selected { "> " } else { "  " };
        lines.push(panel_line_spans(vec![panel_span(
            format!("{}{}", prefix, action.label),
            style,
        )]));
    }
    lines
}

fn build_item_move_panel(runtime: &GameRuntime, entries: &[InventoryEntry]) -> Vec<MenuPanelLine> {
    if entries.is_empty() {
        return vec![panel_line("No items available."), panel_line("")];
    }
    let selection = runtime
        .menu_state
        .detail_target
        .min(entries.len().saturating_sub(1));
    let mut lines = Vec::new();
    lines.push(panel_line_spans(vec![panel_span(
        "Move To",
        PanelSpanStyle::Accent,
    )]));
    for (index, entry) in entries.iter().enumerate() {
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
                entry.label.as_str(),
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

pub fn move_inventory_entry(
    runtime: &mut GameRuntime,
    entries: &[InventoryEntry],
    from_index: usize,
    to_index: usize,
) {
    if entries.is_empty() || from_index >= entries.len() || to_index >= entries.len() {
        return;
    }
    if from_index == to_index {
        return;
    }
    let kind = entries[from_index].kind.clone();
    let filtered_ids = entries
        .iter()
        .map(|entry| entry.id.clone())
        .collect::<Vec<_>>();
    let mut reordered = filtered_ids.clone();
    let moving = reordered.remove(from_index);
    reordered.insert(to_index, moving);
    match kind {
        InventoryKind::Item => {
            runtime.inventory.items_order =
                reorder_order_list(&runtime.inventory.items_order, &filtered_ids, &reordered);
        }
        InventoryKind::Equipment => {
            runtime.inventory.equipment_order = reorder_order_list(
                &runtime.inventory.equipment_order,
                &filtered_ids,
                &reordered,
            );
        }
    }
}

fn reorder_order_list(
    existing: &[String],
    filtered_ids: &[String],
    reordered: &[String],
) -> Vec<String> {
    let filtered_set = filtered_ids.iter().cloned().collect::<HashSet<_>>();
    let mut remaining = reordered.iter();
    let mut new_order = Vec::new();
    for id in existing {
        if filtered_set.contains(id) {
            if let Some(next_id) = remaining.next() {
                new_order.push(next_id.clone());
            }
        } else {
            new_order.push(id.clone());
        }
    }
    for id in remaining {
        new_order.push(id.clone());
    }
    new_order
}

fn build_item_targets(
    runtime: &GameRuntime,
    item: &engine::entities::ItemDefinition,
) -> Vec<String> {
    let mut targets = runtime.party.active_ids();
    targets.retain(|id| {
        runtime
            .party
            .roster
            .get(id)
            .map(|actor| item_has_effect_on_actor(runtime, item, actor, false))
            .unwrap_or(false)
    });
    targets
}

pub fn item_has_effect_on_actor(
    runtime: &GameRuntime,
    item: &engine::entities::ItemDefinition,
    actor: &engine::party::Actor,
    in_battle: bool,
) -> bool {
    let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
    let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
    let power = item.effect.power.unwrap_or(0);
    match item.effect.r#type.as_str() {
        "heal_hp" => {
            if in_battle {
                let inverted = engine::battle::healing_inverted(
                    &runtime.content,
                    &engine::party::actor_traits(&runtime.content, actor),
                );
                if inverted {
                    actor.current_hp > 0 && power.max(1) > 0
                } else {
                    actor.current_hp < max_hp && power > 0
                }
            } else {
                field_item_heal_has_effect(actor.current_hp, max_hp, power)
            }
        }
        "heal_mp" => actor.current_mp < max_mp && power > 0,
        "revive" => actor.current_hp <= 0 && max_hp > 0,
        "cure_status" => {
            !item.effect.statuses.is_empty()
                && actor
                    .statuses
                    .iter()
                    .any(|status| item.effect.statuses.contains(&status.id))
        }
        "learn_spell" => actor_can_learn_spell_from_item(runtime, item, actor),
        "learn_recipe" => item_recipe_can_unlock(runtime, item),
        _ => true,
    }
}

pub fn item_recipe_can_unlock(
    runtime: &GameRuntime,
    item: &engine::entities::ItemDefinition,
) -> bool {
    if item.effect.r#type != "learn_recipe" {
        return false;
    }
    let Some(recipe_id) = item.effect.target.as_deref() else {
        return false;
    };
    let Some(flag) = recipe_unlock_flag(runtime, recipe_id) else {
        return false;
    };
    !runtime.has_flag(&flag)
}

fn actor_can_learn_spell_from_item(
    runtime: &GameRuntime,
    item: &engine::entities::ItemDefinition,
    actor: &engine::party::Actor,
) -> bool {
    let spell_id = item.effect.target.as_deref().unwrap_or("");
    if spell_id.is_empty() || actor.spells.iter().any(|spell| spell == spell_id) {
        return false;
    }
    let mut job_ids = vec![actor.job_id.as_str()];
    if runtime.content.rules.job_system.secondary_jobs {
        if let Some(job_id) = actor.secondary_job_id.as_deref() {
            job_ids.push(job_id);
        }
    }
    for job_id in job_ids {
        let Some(job) = runtime
            .content
            .jobs
            .jobs
            .iter()
            .find(|job| job.id == job_id)
        else {
            continue;
        };
        let acquisition = resolve_magic_acquisition(runtime, job, spell_id);
        if acquisition != MagicAcquisition::Item {
            continue;
        }
        if let Some(entry) = job.spells.iter().find(|entry| entry.id == spell_id) {
            if entry.item.is_none() || entry.item.as_deref() == Some(item.id.as_str()) {
                return true;
            }
            continue;
        }
        let school_allowed = runtime
            .content
            .spells
            .spells
            .iter()
            .find(|spell| spell.id == spell_id)
            .map(|spell| {
                job.magic_schools
                    .iter()
                    .any(|school| school == &spell.school)
            })
            .unwrap_or(false);
        if school_allowed {
            return true;
        }
    }
    false
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

fn resolve_magic_acquisition(
    runtime: &GameRuntime,
    job: &engine::entities::JobDefinition,
    spell_id: &str,
) -> MagicAcquisition {
    let default = runtime.content.rules.game.magic_acquisition.clone();
    let Some(acquisition) = job
        .acquisition
        .as_ref()
        .and_then(|acquisition| acquisition.magic.as_ref())
    else {
        return default;
    };
    match acquisition {
        engine::entities::MagicAcquisitionOverride::Mode(mode) => mode.clone(),
        engine::entities::MagicAcquisitionOverride::BySchool(map) => {
            let school = runtime
                .content
                .spells
                .spells
                .iter()
                .find(|spell| spell.id == spell_id)
                .map(|spell| spell.school.as_str());
            match school.and_then(|school| map.get(school)) {
                Some(mode) => mode.clone(),
                None => default,
            }
        }
    }
}

pub fn build_battle_item_entries(runtime: &GameRuntime) -> Vec<InventoryEntry> {
    let mut entries = Vec::new();
    for item_id in ordered_item_ids(runtime) {
        let Some(item) = find_item_definition(runtime, &item_id) else {
            continue;
        };
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

#[cfg(test)]
mod tests {
    use super::{
        apply_field_item_heal, can_drop_inventory_entry, can_drop_item, field_item_heal_has_effect,
    };
    use crate::menu::common::{InventoryEntry, InventoryKind};

    #[test]
    fn can_drop_item_allows_non_unique_with_quantity() {
        assert!(can_drop_item(1, false));
    }

    #[test]
    fn can_drop_item_disallows_unique_items() {
        assert!(!can_drop_item(1, true));
    }

    #[test]
    fn can_drop_inventory_entry_allows_unequipped_equipment_only() {
        let mut equipment = inventory_entry(InventoryKind::Equipment, 2, 2);
        assert!(can_drop_inventory_entry(None, &equipment));
        equipment.available_qty = 0;
        assert!(!can_drop_inventory_entry(None, &equipment));
    }

    #[test]
    fn field_heal_item_ignores_ko_targets() {
        assert!(!field_item_heal_has_effect(0, 100, 30));
        assert_eq!(apply_field_item_heal(0, 100, 30), 0);
    }

    #[test]
    fn field_heal_item_still_heals_living_targets() {
        assert!(field_item_heal_has_effect(40, 100, 30));
        assert_eq!(apply_field_item_heal(40, 100, 30), 70);
    }

    fn inventory_entry(kind: InventoryKind, available_qty: i32, total_qty: i32) -> InventoryEntry {
        InventoryEntry {
            id: "id".to_string(),
            label: "label".to_string(),
            available_qty,
            total_qty,
            kind,
            slot: None,
            category: None,
            usable: false,
            equipped_by: Vec::new(),
            usage_target: String::new(),
        }
    }
}
