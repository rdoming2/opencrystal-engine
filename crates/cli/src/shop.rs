use std::collections::BTreeSet;

use engine::runtime::GameRuntime;
use tui::input::InputBindings;
use tui::menu::{MenuPanelView, PanelSpanStyle};
use tui::session::TuiSession;
use tui::shop::{ShopItemKind, ShopMode, ShopView};

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
        let selection = match tui::shop::show_shop(session, &shop, bindings)? {
            Some(selection) => selection,
            None => break,
        };
        let shop_def = match runtime
            .content
            .shops
            .shops
            .iter()
            .find(|entry| entry.id == shop_id)
        {
            Some(entry) => entry,
            None => break,
        };
        let currency_id = shop.currency_id.as_str();
        let max_stack = runtime.content.rules.inventory.max_stack;
        match selection.mode {
            ShopMode::Buy => {
                let item = &shop.buy_items[selection.index];
                let price = item.price;
                let currency = runtime.inventory.currency_amount(currency_id);
                let max_afford = if price > 0 {
                    currency / price
                } else {
                    max_stack
                };
                let max_space = max_stack - item.owned;
                let stock_limit = item.stock.unwrap_or(i32::MAX);
                let limit = max_afford.min(max_space).min(stock_limit);

                if limit <= 0 {
                    if stock_limit == 0 {
                        tui::shop::show_info_popup(
                            session,
                            &shop,
                            ShopMode::Buy,
                            selection.index,
                            bindings,
                            "Out of Stock",
                            "This item is sold out.",
                        )?;
                    } else if max_afford == 0 {
                        tui::shop::show_info_popup(
                            session,
                            &shop,
                            ShopMode::Buy,
                            selection.index,
                            bindings,
                            "Insufficient Funds",
                            "You cannot afford this item.",
                        )?;
                    } else {
                        tui::shop::show_info_popup(
                            session,
                            &shop,
                            ShopMode::Buy,
                            selection.index,
                            bindings,
                            "Inventory Full",
                            "You cannot carry any more of this item.",
                        )?;
                    }
                    continue;
                }

                if let Some(qty) = tui::shop::show_quantity_picker(
                    session,
                    &shop,
                    ShopMode::Buy,
                    selection.index,
                    bindings,
                    "Purchase",
                    "Price",
                    limit,
                )? {
                    runtime.inventory.add_currency(currency_id, -price * qty);

                    match item.kind {
                        ShopItemKind::Item => {
                            runtime.inventory.add_item(&item.id, qty, max_stack);
                        }
                        ShopItemKind::Equipment => {
                            runtime.inventory.add_equipment(&item.id, qty, max_stack);
                        }
                    }

                    if shop_def.currency_pool == "tracked" {
                        let entry = runtime.shop_states.entry(shop_def.id.clone()).or_default();
                        entry.currency = entry.currency.saturating_add(price * qty);
                    }
                    if let Some(stock) = item.stock {
                        let entry = runtime.shop_states.entry(shop_def.id.clone()).or_default();
                        let remaining = entry.stock.entry(item.id.clone()).or_insert(stock);
                        *remaining = (*remaining - qty).max(0);
                        if *remaining == 0 {
                            entry.stock.remove(&item.id);
                        }
                    }
                }
            }
            ShopMode::Sell => {
                let item = &shop.sell_items[selection.index];
                let price = item.price;
                let owned = item.owned;
                let merchant_currency = shop.merchant_currency_amount.unwrap_or(i32::MAX);
                let max_afford = if price > 0 {
                    merchant_currency / price
                } else {
                    owned
                };
                let limit = owned.min(max_afford);

                if limit <= 0 {
                    tui::shop::show_info_popup(
                        session,
                        &shop,
                        ShopMode::Sell,
                        selection.index,
                        bindings,
                        "Merchant Funds",
                        "The merchant cannot afford this item.",
                    )?;
                    continue;
                }

                if let Some(qty) = tui::shop::show_quantity_picker(
                    session,
                    &shop,
                    ShopMode::Sell,
                    selection.index,
                    bindings,
                    "Sell",
                    "Sell Price",
                    limit,
                )? {
                    let removed = match item.kind {
                        ShopItemKind::Item => runtime.inventory.remove_item(&item.id, qty),
                        ShopItemKind::Equipment => {
                            runtime.inventory.remove_equipment(&item.id, qty)
                        }
                    };
                    if !removed {
                        continue;
                    }
                    runtime.inventory.add_currency(currency_id, price * qty);

                    if shop_def.currency_pool == "tracked" {
                        let entry = runtime.shop_states.entry(shop_def.id.clone()).or_default();
                        entry.currency = entry.currency.saturating_sub(price * qty);
                    }
                    if shop_def.sell_behavior == "stock" {
                        let should_track_stock = shop_def
                            .inventory
                            .iter()
                            .find(|entry| entry.item == item.id)
                            .map(|entry| entry.stock.is_some())
                            .unwrap_or(true);
                        if should_track_stock {
                            let entry = runtime.shop_states.entry(shop_def.id.clone()).or_default();
                            let stock_entry = entry.stock.entry(item.id.clone()).or_insert(0);
                            *stock_entry = stock_entry.saturating_add(qty);
                        }
                    }
                }
            }
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

    let currency_id = shop.currency.as_str();
    let currency_amount = runtime.inventory.currency_amount(currency_id);
    let (currency_name, currency_symbol) = currency_display(&runtime.content.rules, currency_id);
    let max_stack = runtime.content.rules.inventory.max_stack;
    let merchant_currency_amount = if shop.currency_pool == "tracked" {
        Some(
            runtime
                .shop_states
                .get(shop_id)
                .map(|state| state.currency)
                .unwrap_or_else(|| shop.currency_amount.unwrap_or(0)),
        )
    } else {
        None
    };
    let stock_state = runtime.shop_states.get(shop_id).map(|state| &state.stock);

    let mut buy_items = Vec::new();
    let mut buy_categories = BTreeSet::new();
    let mut known_items = std::collections::HashSet::new();
    for entry in &shop.inventory {
        let item_id = &entry.item;
        known_items.insert(item_id.clone());

        let mut name = item_id.clone();
        let mut kind = ShopItemKind::Item;
        let mut inv_kind = InventoryKind::Item;
        let mut slot = None;
        let mut category = entry.category.clone().unwrap_or_default();

        if let Some(item) = runtime
            .content
            .items
            .items
            .iter()
            .find(|i| i.id == *item_id)
        {
            name = item.name.clone();
            kind = ShopItemKind::Item;
            inv_kind = InventoryKind::Item;
            if category.is_empty() {
                category = item.r#type.clone();
            }
        } else if let Some(equipment) = runtime
            .content
            .equipment
            .equipment
            .iter()
            .find(|i| i.id == *item_id)
        {
            name = equipment.name.clone();
            kind = ShopItemKind::Equipment;
            inv_kind = InventoryKind::Equipment;
            slot = Some(equipment.slot.clone());
            if category.is_empty() {
                category = equipment.category.clone();
            }
        }

        let owned = match kind {
            ShopItemKind::Item => runtime.inventory.item_qty(item_id),
            ShopItemKind::Equipment => runtime.inventory.equipment_qty(item_id),
        };

        let inv_entry = InventoryEntry {
            id: item_id.clone(),
            label: name.clone(),
            available_qty: owned,
            total_qty: owned,
            kind: inv_kind.clone(),
            slot,
            category: if category.is_empty() {
                None
            } else {
                Some(category.clone())
            },
            usable: false,
            equipped_by: Vec::new(),
            usage_target: String::new(),
        };

        let mut lines = match inv_kind {
            InventoryKind::Item => build_item_description(runtime, Some(&inv_entry)),
            InventoryKind::Equipment => build_equipment_detail(runtime, "", "", &inv_entry).lines,
        };

        let price = apply_multiplier(entry.price, shop.buy_price_multiplier);
        let stock = entry.stock.map(|initial| {
            stock_state
                .and_then(|map| map.get(item_id))
                .copied()
                .unwrap_or(initial)
        });

        if let Some(count) = stock {
            lines.insert(
                0,
                panel_line_spans(vec![
                    panel_span("Stock: ", PanelSpanStyle::Normal),
                    panel_span(count.to_string(), PanelSpanStyle::Muted),
                ]),
            );
        }
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
                panel_span(
                    format_currency_amount(&runtime.content.rules, currency_id, price),
                    PanelSpanStyle::Accent,
                ),
            ]),
        );

        let details = MenuPanelView {
            title: name.clone(),
            lines,
        };

        let enabled = stock.map(|count| count > 0).unwrap_or(true);
        buy_categories.insert(category.clone());
        buy_items.push(tui::shop::ShopItem {
            id: item_id.clone(),
            name,
            price,
            details,
            owned,
            max: max_stack,
            category,
            stock,
            enabled,
            kind,
        });
    }

    if let Some(state_stock) = stock_state {
        for (item_id, stock) in state_stock {
            if *stock <= 0 || known_items.contains(item_id) {
                continue;
            }
            let mut name = item_id.clone();
            let mut kind = ShopItemKind::Item;
            let mut inv_kind = InventoryKind::Item;
            let mut slot = None;
            let mut category = String::new();
            let mut base_price = None;

            if let Some(item) = runtime
                .content
                .items
                .items
                .iter()
                .find(|i| i.id == *item_id)
            {
                name = item.name.clone();
                kind = ShopItemKind::Item;
                inv_kind = InventoryKind::Item;
                category = item.r#type.clone();
                base_price = price_for_currency(&item.price, currency_id);
            } else if let Some(equipment) = runtime
                .content
                .equipment
                .equipment
                .iter()
                .find(|i| i.id == *item_id)
            {
                name = equipment.name.clone();
                kind = ShopItemKind::Equipment;
                inv_kind = InventoryKind::Equipment;
                slot = Some(equipment.slot.clone());
                category = equipment.category.clone();
                base_price = price_for_currency(&equipment.price, currency_id);
            }

            let Some(base_price) = base_price else {
                continue;
            };
            let price = apply_multiplier(base_price, shop.buy_price_multiplier);
            let owned = match kind {
                ShopItemKind::Item => runtime.inventory.item_qty(item_id),
                ShopItemKind::Equipment => runtime.inventory.equipment_qty(item_id),
            };
            let inv_entry = InventoryEntry {
                id: item_id.clone(),
                label: name.clone(),
                available_qty: owned,
                total_qty: owned,
                kind: inv_kind.clone(),
                slot,
                category: if category.is_empty() {
                    None
                } else {
                    Some(category.clone())
                },
                usable: false,
                equipped_by: Vec::new(),
                usage_target: String::new(),
            };
            let mut lines = match inv_kind {
                InventoryKind::Item => build_item_description(runtime, Some(&inv_entry)),
                InventoryKind::Equipment => {
                    build_equipment_detail(runtime, "", "", &inv_entry).lines
                }
            };
            lines.insert(
                0,
                panel_line_spans(vec![
                    panel_span("Stock: ", PanelSpanStyle::Normal),
                    panel_span(stock.to_string(), PanelSpanStyle::Muted),
                ]),
            );
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
                    panel_span(
                        format_currency_amount(&runtime.content.rules, currency_id, price),
                        PanelSpanStyle::Accent,
                    ),
                ]),
            );
            let details = MenuPanelView {
                title: name.clone(),
                lines,
            };
            buy_categories.insert(category.clone());
            buy_items.push(tui::shop::ShopItem {
                id: item_id.clone(),
                name,
                price,
                details,
                owned,
                max: max_stack,
                category,
                stock: Some(*stock),
                enabled: true,
                kind,
            });
        }
    }

    let mut sell_items = Vec::new();
    let mut sell_categories = BTreeSet::new();
    for (item_id, qty) in runtime.inventory.items.iter() {
        if *qty <= 0 {
            continue;
        }
        let Some(item) = runtime
            .content
            .items
            .items
            .iter()
            .find(|entry| entry.id == *item_id)
        else {
            continue;
        };
        if !item_sellable(&item.price, item.sellable, currency_id) {
            continue;
        }
        let entry_override = shop.inventory.iter().find(|entry| entry.item == *item_id);
        let base_price = match price_for_currency(&item.price, currency_id) {
            Some(value) => value,
            None => continue,
        };
        let sell_price = entry_override
            .and_then(|entry| entry.sell_price)
            .unwrap_or_else(|| apply_multiplier(base_price, shop.sell_price_multiplier));
        let category = item.r#type.clone();
        let inv_entry = InventoryEntry {
            id: item_id.clone(),
            label: item.name.clone(),
            available_qty: *qty,
            total_qty: *qty,
            kind: InventoryKind::Item,
            slot: None,
            category: Some(category.clone()),
            usable: false,
            equipped_by: Vec::new(),
            usage_target: String::new(),
        };
        let mut lines = build_item_description(runtime, Some(&inv_entry));
        lines.insert(
            0,
            panel_line_spans(vec![
                panel_span("Owned: ", PanelSpanStyle::Normal),
                panel_span(qty.to_string(), PanelSpanStyle::Muted),
            ]),
        );
        lines.insert(
            0,
            panel_line_spans(vec![
                panel_span("Sell Price: ", PanelSpanStyle::Normal),
                panel_span(
                    format_currency_amount(&runtime.content.rules, currency_id, sell_price),
                    PanelSpanStyle::Accent,
                ),
            ]),
        );
        let details = MenuPanelView {
            title: item.name.clone(),
            lines,
        };
        let enabled = match merchant_currency_amount {
            Some(amount) => sell_price == 0 || amount >= sell_price,
            None => true,
        };
        sell_categories.insert(category.clone());
        sell_items.push(tui::shop::ShopItem {
            id: item_id.clone(),
            name: item.name.clone(),
            price: sell_price,
            details,
            owned: *qty,
            max: *qty,
            category,
            stock: None,
            enabled,
            kind: ShopItemKind::Item,
        });
    }
    for (item_id, qty) in runtime.inventory.equipment.iter() {
        if *qty <= 0 {
            continue;
        }
        let Some(equipment) = runtime
            .content
            .equipment
            .equipment
            .iter()
            .find(|entry| entry.id == *item_id)
        else {
            continue;
        };
        if !item_sellable(&equipment.price, equipment.sellable, currency_id) {
            continue;
        }
        let entry_override = shop.inventory.iter().find(|entry| entry.item == *item_id);
        let base_price = match price_for_currency(&equipment.price, currency_id) {
            Some(value) => value,
            None => continue,
        };
        let sell_price = entry_override
            .and_then(|entry| entry.sell_price)
            .unwrap_or_else(|| apply_multiplier(base_price, shop.sell_price_multiplier));
        let category = equipment.category.clone();
        let inv_entry = InventoryEntry {
            id: item_id.clone(),
            label: equipment.name.clone(),
            available_qty: *qty,
            total_qty: *qty,
            kind: InventoryKind::Equipment,
            slot: Some(equipment.slot.clone()),
            category: Some(category.clone()),
            usable: false,
            equipped_by: Vec::new(),
            usage_target: String::new(),
        };
        let mut lines = build_equipment_detail(runtime, "", "", &inv_entry).lines;
        lines.insert(
            0,
            panel_line_spans(vec![
                panel_span("Owned: ", PanelSpanStyle::Normal),
                panel_span(qty.to_string(), PanelSpanStyle::Muted),
            ]),
        );
        lines.insert(
            0,
            panel_line_spans(vec![
                panel_span("Sell Price: ", PanelSpanStyle::Normal),
                panel_span(
                    format_currency_amount(&runtime.content.rules, currency_id, sell_price),
                    PanelSpanStyle::Accent,
                ),
            ]),
        );
        let details = MenuPanelView {
            title: equipment.name.clone(),
            lines,
        };
        let enabled = match merchant_currency_amount {
            Some(amount) => sell_price == 0 || amount >= sell_price,
            None => true,
        };
        sell_categories.insert(category.clone());
        sell_items.push(tui::shop::ShopItem {
            id: item_id.clone(),
            name: equipment.name.clone(),
            price: sell_price,
            details,
            owned: *qty,
            max: *qty,
            category,
            stock: None,
            enabled,
            kind: ShopItemKind::Equipment,
        });
    }

    sell_items.sort_by(|left, right| left.name.cmp(&right.name));

    let mut buy_category_list = Vec::new();
    buy_category_list.push("All".to_string());
    buy_category_list.extend(buy_categories.into_iter());
    let mut sell_category_list = Vec::new();
    sell_category_list.push("All".to_string());
    sell_category_list.extend(sell_categories.into_iter());

    Some(ShopView {
        name: shop.name.clone(),
        currency_id: currency_id.to_string(),
        currency_name,
        currency_symbol,
        currency_amount,
        merchant_currency_amount,
        buy_categories: buy_category_list,
        sell_categories: sell_category_list,
        buy_items,
        sell_items,
    })
}

fn currency_display(rules: &engine::rules::RulesFile, currency_id: &str) -> (String, String) {
    if let Some(currency) = rules.game.currency(currency_id) {
        (currency.name.clone(), currency.symbol.clone())
    } else {
        (currency_id.to_string(), String::new())
    }
}

fn format_currency_amount(
    rules: &engine::rules::RulesFile,
    currency_id: &str,
    amount: i32,
) -> String {
    let (name, symbol) = currency_display(rules, currency_id);
    if symbol.trim().is_empty() {
        format!("{} {}", amount, name)
    } else {
        format!("{}{}", symbol, amount)
    }
}

fn apply_multiplier(price: i32, multiplier: f32) -> i32 {
    if price <= 0 {
        return price.max(0);
    }
    ((price as f32) * multiplier).round().max(0.0) as i32
}

fn price_for_currency(
    prices: &Option<std::collections::HashMap<String, i32>>,
    currency_id: &str,
) -> Option<i32> {
    prices
        .as_ref()
        .and_then(|map| map.get(currency_id))
        .copied()
}

fn item_sellable(
    prices: &Option<std::collections::HashMap<String, i32>>,
    sellable: Option<bool>,
    currency_id: &str,
) -> bool {
    if sellable == Some(false) {
        return false;
    }
    price_for_currency(prices, currency_id).is_some()
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
