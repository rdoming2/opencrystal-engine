use engine::runtime::GameRuntime;
use tui::input::InputBindings;
use tui::session::TuiSession;
use tui::shop::ShopView;

pub fn open_shop(
    runtime: &GameRuntime,
    session: &mut TuiSession,
    bindings: &InputBindings,
    shop_id: &str,
) -> std::io::Result<()> {
    let shop = match build_shop_view(runtime, shop_id) {
        Some(shop) => shop,
        None => {
            println!("Shop not found: {}", shop_id);
            return Ok(());
        }
    };
    let _ = tui::shop::show_shop(session, &shop, bindings)?;
    Ok(())
}

pub fn build_shop_view(runtime: &GameRuntime, shop_id: &str) -> Option<ShopView> {
    let shop = runtime
        .content
        .shops
        .shops
        .iter()
        .find(|shop| shop.id == shop_id)?;

    let items = shop
        .inventory
        .iter()
        .map(|entry| tui::shop::ShopItem {
            name: lookup_item_name(runtime, &entry.item),
            price: entry.price,
        })
        .collect();

    Some(ShopView {
        name: shop.name.clone(),
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
