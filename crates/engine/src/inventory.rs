use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct InventoryState {
    pub items: HashMap<String, i32>,
    pub equipment: HashMap<String, i32>,
    pub currency: HashMap<String, i32>,
    pub items_order: Vec<String>,
    pub equipment_order: Vec<String>,
}

impl InventoryState {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty() && self.equipment.is_empty() && self.currency.is_empty()
    }

    pub fn add_item(&mut self, item_id: &str, qty: i32, max_stack: i32) {
        if qty <= 0 {
            return;
        }
        let is_new = !self.items.contains_key(item_id);
        let entry = self.items.entry(item_id.to_string()).or_insert(0);
        *entry = (*entry + qty).min(max_stack);
        if is_new {
            self.items_order.push(item_id.to_string());
        }
    }

    pub fn add_equipment(&mut self, item_id: &str, qty: i32, max_stack: i32) {
        if qty <= 0 {
            return;
        }
        let is_new = !self.equipment.contains_key(item_id);
        let entry = self.equipment.entry(item_id.to_string()).or_insert(0);
        *entry = (*entry + qty).min(max_stack);
        if is_new {
            self.equipment_order.push(item_id.to_string());
        }
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
            self.equipment_order.retain(|id| id != item_id);
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
            self.items_order.retain(|id| id != item_id);
        }
        true
    }

    pub fn item_qty(&self, item_id: &str) -> i32 {
        self.items.get(item_id).copied().unwrap_or(0)
    }

    pub fn equipment_qty(&self, item_id: &str) -> i32 {
        self.equipment.get(item_id).copied().unwrap_or(0)
    }

    pub fn normalize_orders(&mut self) {
        normalize_order(&mut self.items_order, &self.items);
        normalize_order(&mut self.equipment_order, &self.equipment);
    }
}

fn normalize_order(order: &mut Vec<String>, entries: &HashMap<String, i32>) {
    order.retain(|id| entries.get(id).copied().unwrap_or(0) > 0);
    let mut missing = entries
        .iter()
        .filter_map(|(id, qty)| {
            if *qty > 0 && !order.iter().any(|existing| existing == id) {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    missing.sort();
    order.extend(missing);
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InventoryStack {
    pub id: String,
    pub qty: i32,
}

#[cfg(test)]
mod tests {
    use super::InventoryState;

    #[test]
    fn add_item_respects_quantity_and_stack_limit() {
        let mut inventory = InventoryState::default();

        inventory.add_item("potion", 0, 99);
        assert_eq!(inventory.item_qty("potion"), 0);

        inventory.add_item("potion", 5, 8);
        inventory.add_item("potion", 10, 8);
        assert_eq!(inventory.item_qty("potion"), 8);
        assert_eq!(inventory.items_order, vec!["potion".to_string()]);
    }

    #[test]
    fn remove_item_and_equipment_updates_order() {
        let mut inventory = InventoryState::default();
        inventory.add_item("potion", 3, 99);
        inventory.add_equipment("sword", 1, 99);

        assert!(!inventory.remove_item("potion", 4));
        assert!(inventory.remove_item("potion", 2));
        assert_eq!(inventory.item_qty("potion"), 1);
        assert!(inventory.remove_item("potion", 1));
        assert_eq!(inventory.item_qty("potion"), 0);
        assert!(inventory.items_order.is_empty());

        assert!(inventory.remove_equipment("sword", 0));
        assert!(inventory.remove_equipment("sword", 1));
        assert_eq!(inventory.equipment_qty("sword"), 0);
        assert!(inventory.equipment_order.is_empty());
    }

    #[test]
    fn add_currency_saturates_at_bounds() {
        let mut inventory = InventoryState::default();
        inventory.add_currency("gold", i32::MAX);
        inventory.add_currency("gold", 1);
        assert_eq!(inventory.currency_amount("gold"), i32::MAX);

        inventory.add_currency("gold", -i32::MAX);
        inventory.add_currency("gold", -10);
        assert_eq!(inventory.currency_amount("gold"), -10);

        inventory.add_currency("gold", -i32::MAX);
        assert_eq!(inventory.currency_amount("gold"), i32::MIN);
    }

    #[test]
    fn normalize_orders_removes_stale_and_appends_missing_sorted() {
        let mut inventory = InventoryState::default();
        inventory.items.insert("ether".to_string(), 2);
        inventory.items.insert("potion".to_string(), 1);
        inventory.items.insert("zero".to_string(), 0);
        inventory.items_order = vec!["zero".to_string(), "unknown".to_string()];

        inventory.equipment.insert("axe".to_string(), 1);
        inventory.equipment.insert("bow".to_string(), 1);
        inventory.equipment_order = vec!["bow".to_string()];

        inventory.normalize_orders();

        assert_eq!(
            inventory.items_order,
            vec!["ether".to_string(), "potion".to_string()]
        );
        assert_eq!(
            inventory.equipment_order,
            vec!["bow".to_string(), "axe".to_string()]
        );
    }

    #[test]
    fn is_empty_checks_all_collections() {
        let mut inventory = InventoryState::default();
        assert!(inventory.is_empty());
        inventory.currency.insert("gold".to_string(), 1);
        assert!(!inventory.is_empty());
    }
}
