use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct InventoryState {
    pub items: HashMap<String, i32>,
    pub equipment: HashMap<String, i32>,
    pub currency: HashMap<String, i32>,
}

impl InventoryState {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.equipment.is_empty() && self.currency.is_empty()
    }

    pub fn add_item(&mut self, item_id: &str, qty: i32, max_stack: i32) {
        if qty <= 0 {
            return;
        }
        let entry = self.items.entry(item_id.to_string()).or_insert(0);
        *entry = (*entry + qty).min(max_stack);
    }

    pub fn add_equipment(&mut self, item_id: &str, qty: i32, max_stack: i32) {
        if qty <= 0 {
            return;
        }
        let entry = self.equipment.entry(item_id.to_string()).or_insert(0);
        *entry = (*entry + qty).min(max_stack);
    }

    pub fn remove_equipment(&mut self, item_id: &str, qty: i32) -> bool {
        if qty <= 0 {
            return true;
        }
        let Some(entry) = self.equipment.get_mut(item_id) else {
            return false;
        };
        if *entry < qty {
            return false;
        }
        *entry -= qty;
        if *entry == 0 {
            self.equipment.remove(item_id);
        }
        true
    }

    pub fn add_currency(&mut self, currency_id: &str, amount: i32) {
        let entry = self.currency.entry(currency_id.to_string()).or_insert(0);
        if amount >= 0 {
            *entry = entry.saturating_add(amount);
        } else {
            *entry = entry.saturating_sub(amount.abs());
        }
    }

    pub fn currency_amount(&self, currency_id: &str) -> i32 {
        self.currency.get(currency_id).copied().unwrap_or(0)
    }

    pub fn remove_item(&mut self, item_id: &str, qty: i32) -> bool {
        if qty <= 0 {
            return true;
        }
        let Some(entry) = self.items.get_mut(item_id) else {
            return false;
        };
        if *entry < qty {
            return false;
        }
        *entry -= qty;
        if *entry == 0 {
            self.items.remove(item_id);
        }
        true
    }

    pub fn item_qty(&self, item_id: &str) -> i32 {
        self.items.get(item_id).copied().unwrap_or(0)
    }

    pub fn equipment_qty(&self, item_id: &str) -> i32 {
        self.equipment.get(item_id).copied().unwrap_or(0)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InventoryStack {
    pub id: String,
    pub qty: i32,
}
