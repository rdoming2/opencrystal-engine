use engine::runtime::GameRuntime;
use tui::input::InputBindings;
use tui::menu::{MenuPanelView, PanelSpanStyle};
use tui::session::TuiSession;
use tui::shop::ShopView;

use crate::menu::common::{InventoryEntry, InventoryKind};
use crate::menu::equipment::build_equipment_detail;
use crate::menu::inventory::{build_item_description, panel_line_spans, panel_span};

pub fn open_shop(
    runtime: &mut GameRuntime,
    session: &mut TuiSession,
    bindings: &InputBindings,
    shop_id: &str,
) -> std::io::Result<()> {
    loop {
        let shop = match build_shop_view(runtime, shop_id) {
            Some(shop) => shop,
            None => {
                println!("Shop not found: {}", shop_id);
                return Ok(());
            }
        };

        match tui::shop::show_shop(session, &shop, bindings)? {
            Some(index) => {
                let item = &shop.items[index];
                let currency_id = &runtime.content.rules.game.currency.id;
                let price = item.price;
                let max_stack = runtime.content.rules.inventory.max_stack;

                if runtime.inventory.currency_amount(currency_id) < price {
                    continue;
                }
                if item.owned >= max_stack {
                    continue;
                }

                runtime.inventory.add_currency(currency_id, -price);

                if runtime.content.items.items.iter().any(|i| i.id == item.id) {
                    runtime.inventory.add_item(&item.id, 1, max_stack);
                } else {
                    runtime.inventory.add_equipment(&item.id, 1, max_stack);
                }
            }
            None => break,
        }
    }
    Ok(())
}

pub fn build_shop_view(runtime: &GameRuntime, shop_id: &str) -> Option<ShopView> {
    let shop = runtime
        .content
        .shops
        .shops
        .iter()
        .find(|shop| shop.id == shop_id)?;

    let currency_id = &runtime.content.rules.game.currency.id;
    let currency_amount = runtime.inventory.currency_amount(currency_id);
    let max_stack = runtime.content.rules.inventory.max_stack;

    let items = shop
        .inventory
        .iter()
        .map(|entry| {
            let item_id = &entry.item;
            let mut name = item_id.clone();
            let mut kind = InventoryKind::Item;
            let mut slot = None;
            let mut category = None;

            if let Some(item) = runtime
                .content
                .items
                .items
                .iter()
                .find(|i| i.id == *item_id)
            {
                name = item.name.clone();
                kind = InventoryKind::Item;
            } else if let Some(equipment) = runtime
                .content
                .equipment
                .equipment
                .iter()
                .find(|i| i.id == *item_id)
            {
                name = equipment.name.clone();
                kind = InventoryKind::Equipment;
                slot = Some(equipment.slot.clone());
                category = Some(equipment.category.clone());
            }

            let owned =
                runtime.inventory.item_qty(item_id) + runtime.inventory.equipment_qty(item_id);

            let inv_entry = InventoryEntry {
                id: item_id.clone(),
                label: name.clone(),
                available_qty: owned,
                total_qty: owned,
                kind: kind.clone(),
                slot,
                category,
                usable: false,
                equipped_by: Vec::new(),
                usage_target: String::new(),
            };

            let mut lines = match kind {
                InventoryKind::Item => build_item_description(runtime, Some(&inv_entry)),
                InventoryKind::Equipment => {
                    build_equipment_detail(runtime, "", "", &inv_entry).lines
                }
            };

            lines.insert(
                0,
                panel_line_spans(vec![
                    panel_span("Owned: ", PanelSpanStyle::Normal),
                    panel_span(format!("{}/{}", owned, max_stack), PanelSpanStyle::Muted),
                ]),
            );
            lines.insert(
                0,
                panel_line_spans(vec![
                    panel_span("Price: ", PanelSpanStyle::Normal),
                    panel_span(format!("{} G", entry.price), PanelSpanStyle::Accent),
                ]),
            );

            let details = MenuPanelView {
                title: name.clone(),
                lines,
            };

            tui::shop::ShopItem {
                id: item_id.clone(),
                name,
                price: entry.price,
                details,
                owned,
                max: max_stack,
            }
        })
        .collect();

    Some(ShopView {
        name: shop.name.clone(),
        currency: currency_amount,
        items,
    })
}

pub fn lookup_item_name(runtime: &GameRuntime, item_id: &str) -> String {
    if let Some(item) = runtime
        .content
        .items
        .items
        .iter()
        .find(|item| item.id == item_id)
    {
        return item.name.clone();
    }
    if let Some(item) = runtime
        .content
        .equipment
        .equipment
        .iter()
        .find(|item| item.id == item_id)
    {
        return item.name.clone();
    }
    item_id.to_string()
}
