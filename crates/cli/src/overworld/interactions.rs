use engine::runtime::{GameRuntime, GameState};
use tui::dialog::ChoiceView;
use tui::input::InputBindings;
use tui::menu::{MenuPanelView, PanelSpanStyle};
use tui::overworld::{show_centered_dialog_on_map, MapView};
use tui::session::TuiSession;
use tui::ui::{BattleUiFile, DialogUiFile};

use crate::events::{run_event_loop, EventLoopOutcome};
use crate::menu::inventory::{panel_line_spans, panel_span};
use crate::shop::lookup_item_name;

use super::OverworldOutcome;

pub(crate) fn run_pending_events(
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    battle_ui: &BattleUiFile,
    bindings: &InputBindings,
    session: &mut TuiSession,
    map_view: Option<MapView>,
) -> std::io::Result<OverworldOutcome> {
    if runtime.event_queue.is_empty() {
        return Ok(OverworldOutcome::Continue);
    }
    runtime.state = GameState::Event;
    runtime.start_next_event();
    let outcome = run_event_loop(runtime, dialog_ui, battle_ui, bindings, session, map_view)?;
    if let EventLoopOutcome::Defeat(context) = outcome {
        return Ok(OverworldOutcome::Defeat(context));
    }
    Ok(OverworldOutcome::Continue)
}

pub(crate) fn is_on_save_point(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    map.save_points
        .iter()
        .any(|save_pos| (save_pos[0], save_pos[1]) == pos)
}

pub(crate) fn write_autosave(
    runtime: &GameRuntime,
    save_dir: &std::path::Path,
) -> Result<(), String> {
    std::fs::create_dir_all(save_dir).map_err(|err| format!("{}: {}", save_dir.display(), err))?;
    let save = engine::save::SaveFile::from_runtime(runtime, 0);
    let path = save_dir.join("slot_0.json");
    save.write(path)
}

pub(crate) fn open_chest(runtime: &mut GameRuntime, chest: &engine::maps::MapChest) -> String {
    if runtime.has_flag(&chest.opened_flag) {
        return "The chest is empty.".to_string();
    }

    let max_stack = runtime.content.rules.inventory.max_stack;
    let mut found = Vec::new();

    for item in &chest.loot.items {
        if item.qty <= 0 {
            continue;
        }
        runtime.inventory.add_item(&item.id, item.qty, max_stack);
        found.push(format!(
            "{} x{}",
            lookup_item_name(runtime, &item.id),
            item.qty
        ));
    }

    for item in &chest.loot.equipment {
        if item.qty <= 0 {
            continue;
        }
        runtime
            .inventory
            .add_equipment(&item.id, item.qty, max_stack);
        found.push(format!(
            "{} x{}",
            lookup_item_name(runtime, &item.id),
            item.qty
        ));
    }

    for currency in &chest.loot.currency {
        if currency.amount <= 0 {
            continue;
        }
        runtime
            .inventory
            .add_currency(&currency.id, currency.amount);
        found.push(format_currency_stack(&runtime.content.rules, currency));
    }

    runtime.set_flag(&chest.opened_flag);

    if found.is_empty() {
        "The chest is empty.".to_string()
    } else {
        format!("Found: {}.", found.join(", "))
    }
}

pub(crate) fn format_currency_stack(
    rules: &engine::rules::RulesFile,
    currency: &engine::maps::MapCurrencyStack,
) -> String {
    if let Some(definition) = rules.game.currency(&currency.id) {
        if definition.symbol.trim().is_empty() {
            format!("{} {}", currency.amount, definition.name)
        } else {
            format!("{}{}", definition.symbol, currency.amount)
        }
    } else {
        format!("{} {}", currency.amount, currency.id)
    }
}

pub(crate) fn find_chest(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<engine::maps::MapChest> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let chest = map.chests.iter().find(|chest| {
        let dx = (chest.pos[0] - pos.0).abs();
        let dy = (chest.pos[1] - pos.1).abs();
        (dx == 1 && dy == 0) || (dx == 0 && dy == 1)
    })?;
    Some(chest.clone())
}

pub(crate) fn find_sign_text(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<String> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let sign = map.signs.iter().find(|sign| {
        let dx = (sign.pos[0] - pos.0).abs();
        let dy = (sign.pos[1] - pos.1).abs();
        (dx == 1 && dy == 0) || (dx == 0 && dy == 1)
    })?;
    Some(sign.text.clone())
}

pub(crate) fn find_adjacent_door(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<engine::maps::MapDoor> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let door = map.doors.iter().find(|door| {
        let dx = (door.pos[0] - pos.0).abs();
        let dy = (door.pos[1] - pos.1).abs();
        (dx == 1 && dy == 0) || (dx == 0 && dy == 1)
    })?;
    Some(door.clone())
}

pub(crate) fn door_locked(runtime: &GameRuntime, door: &engine::maps::MapDoor) -> bool {
    door.requires_flag
        .as_ref()
        .map(|flag| !runtime.has_flag(flag))
        .unwrap_or(false)
}

pub(crate) fn find_adjacent_puzzle(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<engine::maps::MapPuzzle> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let puzzle = map.puzzles.iter().find(|puzzle| {
        if !super::puzzle_visible(runtime, puzzle) {
            return false;
        }
        let dx = (puzzle.pos[0] - pos.0).abs();
        let dy = (puzzle.pos[1] - pos.1).abs();
        dx + dy <= 1
    })?;
    Some(puzzle.clone())
}

pub(crate) fn find_adjacent_campfire(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<engine::maps::MapCampfire> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let campfire = map.campfires.iter().find(|campfire| {
        if !super::requires_flags_met(runtime, &campfire.requires_flags) {
            return false;
        }
        let dx = (campfire.pos[0] - pos.0).abs();
        let dy = (campfire.pos[1] - pos.1).abs();
        (dx == 1 && dy == 0) || (dx == 0 && dy == 1)
    })?;
    Some(campfire.clone())
}

pub(crate) fn open_campfire(
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    bindings: &InputBindings,
    session: &mut TuiSession,
    map: &tui::overworld::MapView,
    player_pos: (i32, i32),
    campfire: &engine::maps::MapCampfire,
) -> std::io::Result<()> {
    let cooking_enabled = runtime
        .content
        .rules
        .systems
        .get("cooking")
        .copied()
        .unwrap_or(false);
    if !cooking_enabled {
        show_centered_dialog_on_map(
            session,
            map,
            player_pos,
            dialog_ui,
            bindings,
            "Cooking is unavailable.",
        )?;
        return Ok(());
    }

    let recipe = {
        let Some(cooking) = runtime.content.cooking.as_ref() else {
            show_centered_dialog_on_map(
                session,
                map,
                player_pos,
                dialog_ui,
                bindings,
                "No recipes are available.",
            )?;
            return Ok(());
        };

        let Some(campfire_def) = cooking
            .campfires
            .iter()
            .find(|entry| entry.id == campfire.campfire_id)
        else {
            show_centered_dialog_on_map(
                session,
                map,
                player_pos,
                dialog_ui,
                bindings,
                "No recipes are available.",
            )?;
            return Ok(());
        };

        let recipes = campfire_def
            .recipes
            .iter()
            .filter_map(|recipe_id| {
                cooking
                    .recipes
                    .iter()
                    .find(|recipe| recipe.id == *recipe_id)
            })
            .filter(|recipe| recipe_unlocked(runtime, recipe))
            .collect::<Vec<_>>();

        if recipes.is_empty() {
            show_centered_dialog_on_map(
                session,
                map,
                player_pos,
                dialog_ui,
                bindings,
                "No recipes are available.",
            )?;
            return Ok(());
        }

        let choices = recipes
            .iter()
            .map(|recipe| ChoiceView {
                label: recipe.name.clone(),
                show_next: false,
            })
            .collect::<Vec<_>>();

        let details = recipes
            .iter()
            .map(|recipe| build_recipe_detail_panel(runtime, recipe))
            .collect::<Vec<_>>();

        let selection = tui::overworld::show_dialog_with_choices_and_details_on_map(
            session,
            map,
            player_pos,
            dialog_ui,
            bindings,
            "Campfire",
            &format!("{} Recipes", campfire_def.label),
            &choices,
            &details,
        )?;

        let Some(selection) = selection else {
            return Ok(());
        };

        match recipes.get(selection) {
            Some(recipe) => (*recipe).clone(),
            None => return Ok(()),
        }
    };

    if !can_cook_recipe(runtime, &recipe) {
        show_centered_dialog_on_map(
            session,
            map,
            player_pos,
            dialog_ui,
            bindings,
            "You lack the ingredients.",
        )?;
        return Ok(());
    }

    apply_cooking_recipe(runtime, &recipe);
    let result_text = format_cooking_results(runtime, &recipe);
    show_centered_dialog_on_map(session, map, player_pos, dialog_ui, bindings, &result_text)?;
    Ok(())
}

fn can_cook_recipe(runtime: &GameRuntime, recipe: &engine::content::CookingRecipe) -> bool {
    recipe
        .ingredients
        .iter()
        .all(|ingredient| runtime.inventory.item_qty(&ingredient.id) >= ingredient.qty)
}

fn apply_cooking_recipe(runtime: &mut GameRuntime, recipe: &engine::content::CookingRecipe) {
    for ingredient in &recipe.ingredients {
        runtime
            .inventory
            .remove_item(&ingredient.id, ingredient.qty);
    }

    let max_stack = runtime.content.rules.inventory.max_stack;
    for item in &recipe.results.items {
        runtime.inventory.add_item(&item.id, item.qty, max_stack);
    }
    for item in &recipe.results.equipment {
        runtime
            .inventory
            .add_equipment(&item.id, item.qty, max_stack);
    }
    for currency in &recipe.results.currency {
        runtime
            .inventory
            .add_currency(&currency.id, currency.amount);
    }
}

fn format_cooking_results(
    runtime: &GameRuntime,
    recipe: &engine::content::CookingRecipe,
) -> String {
    let cost = recipe
        .ingredients
        .iter()
        .filter(|ingredient| ingredient.qty > 0)
        .map(|ingredient| {
            format!(
                "{} x{}",
                lookup_item_name(runtime, &ingredient.id),
                ingredient.qty
            )
        })
        .collect::<Vec<_>>();
    let mut found = Vec::new();
    for item in &recipe.results.items {
        if item.qty <= 0 {
            continue;
        }
        found.push(format!(
            "{} x{}",
            lookup_item_name(runtime, &item.id),
            item.qty
        ));
    }
    for item in &recipe.results.equipment {
        if item.qty <= 0 {
            continue;
        }
        found.push(format!(
            "{} x{}",
            lookup_item_name(runtime, &item.id),
            item.qty
        ));
    }
    for currency in &recipe.results.currency {
        if currency.amount <= 0 {
            continue;
        }
        found.push(format_currency_stack(&runtime.content.rules, currency));
    }

    let cost_text = if cost.is_empty() {
        "Cost: None.".to_string()
    } else {
        format!("Cost: {}.", cost.join(", "))
    };
    let result_text = if found.is_empty() {
        format!("Cooked: {}.", recipe.name)
    } else {
        format!("Cooked: {}.", found.join(", "))
    };
    format!("{}\n{}", cost_text, result_text)
}

fn build_recipe_detail_panel(
    runtime: &GameRuntime,
    recipe: &engine::content::CookingRecipe,
) -> MenuPanelView {
    let mut lines = Vec::new();
    lines.push(panel_line_spans(vec![panel_span(
        "Ingredients",
        PanelSpanStyle::Accent,
    )]));

    let mut ready = true;
    for ingredient in &recipe.ingredients {
        let have = runtime.inventory.item_qty(&ingredient.id);
        let need = ingredient.qty;
        if have < need {
            ready = false;
        }
        let count_style = if have >= need {
            PanelSpanStyle::Accent
        } else {
            PanelSpanStyle::Muted
        };
        lines.push(panel_line_spans(vec![
            panel_span(
                format!("{}: ", lookup_item_name(runtime, &ingredient.id)),
                PanelSpanStyle::Normal,
            ),
            panel_span(format!("{}/{}", have, need), count_style),
        ]));
    }

    let status_text = if ready {
        "Ready"
    } else {
        "Missing ingredients"
    };
    let status_style = if ready {
        PanelSpanStyle::Accent
    } else {
        PanelSpanStyle::Muted
    };
    lines.push(panel_line_spans(vec![
        panel_span("Status: ", PanelSpanStyle::Normal),
        panel_span(status_text, status_style),
    ]));

    MenuPanelView {
        title: recipe.name.clone(),
        lines,
    }
}

fn recipe_unlocked(runtime: &GameRuntime, recipe: &engine::content::CookingRecipe) -> bool {
    recipe
        .unlock_flag
        .as_ref()
        .map(|flag| runtime.has_flag(flag))
        .unwrap_or(true)
}
