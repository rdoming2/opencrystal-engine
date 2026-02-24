use tui::menu::MenuEntryView;

pub struct MenuEntryState {
    pub view: MenuEntryView,
    pub action: String,
    pub selectable: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InventoryFilter {
    Items,
    KeyItems,
    Equipment,
    Weapons,
    Armor,
    Accessory,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InventorySort {
    Name,
    Type,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InventoryKind {
    Item,
    Equipment,
}

pub struct InventoryEntry {
    pub id: String,
    pub label: String,
    pub available_qty: i32,
    pub total_qty: i32,
    pub kind: InventoryKind,
    pub slot: Option<String>,
    pub category: Option<String>,
    pub usable: bool,
    pub equipped_by: Vec<String>,
    pub usage_target: String,
}

#[derive(Clone, Debug)]
pub struct SpellEntry {
    pub id: String,
    pub name: String,
    pub school: String,
    pub tier: u32,
    pub cost_type: String,
    pub cost_value: i32,
    pub default_target: String,
    pub allowed_targets: Vec<String>,
    pub target_mode: String,
    pub multi_attenuation: Option<f32>,
    pub effect_type: String,
    pub effect_power: i32,
    pub usable: bool,
    pub reason: Option<String>,
    pub tier_current: i32,
    pub tier_max: i32,
}

#[derive(Clone, Debug)]
pub struct AbilityEntry {
    pub id: String,
    pub name: String,
    pub default_target: String,
    pub allowed_targets: Vec<String>,
    pub target_mode: String,
    pub multi_attenuation: Option<f32>,
    pub effect_type: String,
    pub effect_power: i32,
    pub cost_type: String,
    pub cost_value: i32,
    pub cost_item_id: Option<String>,
    pub cost_currency_id: Option<String>,
    pub usable: bool,
    pub reason: Option<String>,
}

pub fn filter_from_index(index: usize) -> InventoryFilter {
    match index % 6 {
        0 => InventoryFilter::Items,
        1 => InventoryFilter::KeyItems,
        2 => InventoryFilter::Equipment,
        3 => InventoryFilter::Weapons,
        4 => InventoryFilter::Armor,
        _ => InventoryFilter::Accessory,
    }
}

pub fn sort_from_index(index: usize) -> InventorySort {
    if index % 2 == 0 {
        InventorySort::Name
    } else {
        InventorySort::Type
    }
}

pub fn next_filter_index(index: usize) -> usize {
    (index + 1) % 6
}

pub fn prev_filter_index(index: usize) -> usize {
    if index == 0 {
        5
    } else {
        index - 1
    }
}

pub fn toggle_sort_index(index: usize) -> usize {
    if index == 0 {
        1
    } else {
        0
    }
}
