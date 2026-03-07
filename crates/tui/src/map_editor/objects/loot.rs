use std::io;

use crate::dialog::{prompt_choice, prompt_text};
use crate::input::InputBindings;
use crate::session::TuiSession;

use super::super::prompts::choose_from_list_or_custom;
use super::super::{InventoryStack, MapChestLoot, MapCurrencyStack};

pub(super) fn edit_chest_loot(
    session: &mut TuiSession,
    bindings: &InputBindings,
    loot: &mut MapChestLoot,
    item_ids: &[String],
    equipment_ids: &[String],
    currency_ids: &[String],
) -> io::Result<()> {
    loop {
        let options = vec![
            "Items".to_string(),
            "Equipment".to_string(),
            "Currency".to_string(),
            "Done".to_string(),
        ];
        let selection = prompt_choice(
            session,
            bindings,
            "Chest Loot",
            "Select section:",
            &options,
            0,
        )?;
        let Some(selection) = selection else {
            break;
        };
        match selection {
            0 => edit_loot_items(session, bindings, &mut loot.items, item_ids)?,
            1 => edit_loot_equipment(session, bindings, &mut loot.equipment, equipment_ids)?,
            2 => edit_loot_currency(session, bindings, &mut loot.currency, currency_ids)?,
            _ => break,
        }
    }
    Ok(())
}

fn edit_loot_items(
    session: &mut TuiSession,
    bindings: &InputBindings,
    items: &mut Vec<InventoryStack>,
    item_ids: &[String],
) -> io::Result<()> {
    loop {
        let options = vec![
            "Add or update".to_string(),
            "Remove".to_string(),
            "Back".to_string(),
        ];
        let selection = prompt_choice(
            session,
            bindings,
            "Loot Items",
            "Select action:",
            &options,
            0,
        )?;
        let Some(selection) = selection else {
            break;
        };
        match selection {
            0 => {
                let item_id = choose_from_list_or_custom(
                    session,
                    bindings,
                    "Loot Items",
                    "Item id:",
                    item_ids,
                    "",
                )?;
                let Some(item_id) = item_id else {
                    continue;
                };
                let default_qty = items
                    .iter()
                    .find(|item| item.id == item_id)
                    .map(|item| item.qty.to_string())
                    .unwrap_or_else(|| "1".to_string());
                let qty_text = prompt_text(session, "Loot Items", "Qty:", &default_qty, 8)?;
                let Some(qty_text) = qty_text else {
                    continue;
                };
                let qty: i32 = qty_text.trim().parse().unwrap_or(0);
                if qty <= 0 {
                    continue;
                }
                upsert_inventory_stack(items, item_id, qty);
            }
            1 => {
                if items.is_empty() {
                    continue;
                }
                let options = items
                    .iter()
                    .map(|item| format!("{} x{}", item.id, item.qty))
                    .collect::<Vec<_>>();
                let selection = prompt_choice(
                    session,
                    bindings,
                    "Loot Items",
                    "Remove which?",
                    &options,
                    0,
                )?;
                if let Some(index) = selection {
                    items.remove(index);
                }
            }
            _ => break,
        }
    }
    Ok(())
}

fn edit_loot_equipment(
    session: &mut TuiSession,
    bindings: &InputBindings,
    equipment: &mut Vec<InventoryStack>,
    equipment_ids: &[String],
) -> io::Result<()> {
    loop {
        let options = vec![
            "Add or update".to_string(),
            "Remove".to_string(),
            "Back".to_string(),
        ];
        let selection = prompt_choice(
            session,
            bindings,
            "Loot Equipment",
            "Select action:",
            &options,
            0,
        )?;
        let Some(selection) = selection else {
            break;
        };
        match selection {
            0 => {
                let equipment_id = choose_from_list_or_custom(
                    session,
                    bindings,
                    "Loot Equipment",
                    "Equipment id:",
                    equipment_ids,
                    "",
                )?;
                let Some(equipment_id) = equipment_id else {
                    continue;
                };
                let default_qty = equipment
                    .iter()
                    .find(|item| item.id == equipment_id)
                    .map(|item| item.qty.to_string())
                    .unwrap_or_else(|| "1".to_string());
                let qty_text = prompt_text(session, "Loot Equipment", "Qty:", &default_qty, 8)?;
                let Some(qty_text) = qty_text else {
                    continue;
                };
                let qty: i32 = qty_text.trim().parse().unwrap_or(0);
                if qty <= 0 {
                    continue;
                }
                upsert_inventory_stack(equipment, equipment_id, qty);
            }
            1 => {
                if equipment.is_empty() {
                    continue;
                }
                let options = equipment
                    .iter()
                    .map(|item| format!("{} x{}", item.id, item.qty))
                    .collect::<Vec<_>>();
                let selection = prompt_choice(
                    session,
                    bindings,
                    "Loot Equipment",
                    "Remove which?",
                    &options,
                    0,
                )?;
                if let Some(index) = selection {
                    equipment.remove(index);
                }
            }
            _ => break,
        }
    }
    Ok(())
}

fn edit_loot_currency(
    session: &mut TuiSession,
    bindings: &InputBindings,
    currency: &mut Vec<MapCurrencyStack>,
    currency_ids: &[String],
) -> io::Result<()> {
    loop {
        let options = vec![
            "Add or update".to_string(),
            "Remove".to_string(),
            "Back".to_string(),
        ];
        let selection = prompt_choice(
            session,
            bindings,
            "Loot Currency",
            "Select action:",
            &options,
            0,
        )?;
        let Some(selection) = selection else {
            break;
        };
        match selection {
            0 => {
                let currency_id = choose_from_list_or_custom(
                    session,
                    bindings,
                    "Loot Currency",
                    "Currency id:",
                    currency_ids,
                    "",
                )?;
                let Some(currency_id) = currency_id else {
                    continue;
                };
                let default_amount = currency
                    .iter()
                    .find(|item| item.id == currency_id)
                    .map(|item| item.amount.to_string())
                    .unwrap_or_else(|| "1".to_string());
                let amount_text =
                    prompt_text(session, "Loot Currency", "Amount:", &default_amount, 8)?;
                let Some(amount_text) = amount_text else {
                    continue;
                };
                let amount: i32 = amount_text.trim().parse().unwrap_or(0);
                if amount <= 0 {
                    continue;
                }
                upsert_currency_stack(currency, currency_id, amount);
            }
            1 => {
                if currency.is_empty() {
                    continue;
                }
                let options = currency
                    .iter()
                    .map(|item| format!("{} x{}", item.id, item.amount))
                    .collect::<Vec<_>>();
                let selection = prompt_choice(
                    session,
                    bindings,
                    "Loot Currency",
                    "Remove which?",
                    &options,
                    0,
                )?;
                if let Some(index) = selection {
                    currency.remove(index);
                }
            }
            _ => break,
        }
    }
    Ok(())
}

fn upsert_inventory_stack(items: &mut Vec<InventoryStack>, id: String, qty: i32) {
    if let Some(item) = items.iter_mut().find(|item| item.id == id) {
        item.qty = qty;
    } else {
        items.push(InventoryStack { id, qty });
    }
}

fn upsert_currency_stack(currency: &mut Vec<MapCurrencyStack>, id: String, amount: i32) {
    if let Some(item) = currency.iter_mut().find(|item| item.id == id) {
        item.amount = amount;
    } else {
        currency.push(MapCurrencyStack { id, amount });
    }
}
