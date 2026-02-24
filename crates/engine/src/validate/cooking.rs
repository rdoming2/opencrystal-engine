use std::collections::HashSet;

use super::helpers::{check_non_empty, find_recipe};
use super::ValidationContext;

pub(crate) fn validate_items_learn_recipe(context: &ValidationContext, errors: &mut Vec<String>) {
    let Some(items) = context.items else {
        return;
    };
    for item in &items.items {
        if item.effect.r#type != "learn_recipe" {
            continue;
        }
        let Some(recipe_id) = item.effect.target.as_deref() else {
            errors.push(format!(
                "items.json: item '{}' learn_recipe requires target recipe id",
                item.id
            ));
            continue;
        };
        let Some(recipe) = find_recipe(
            errors,
            context.cooking,
            recipe_id,
            || {
                format!(
                    "items.json: item '{}' learn_recipe requires cooking.json",
                    item.id
                )
            },
            || {
                format!(
                    "items.json: item '{}' learn_recipe references unknown recipe '{}'",
                    item.id, recipe_id
                )
            },
        ) else {
            continue;
        };
        if recipe
            .unlock_flag
            .as_deref()
            .map(|flag| flag.trim().is_empty())
            .unwrap_or(true)
        {
            errors.push(format!(
                "items.json: item '{}' learn_recipe requires recipe unlock_flag",
                item.id
            ));
        }
    }
}

pub(crate) fn validate_cooking(context: &ValidationContext, errors: &mut Vec<String>) {
    let Some(cooking) = context.cooking else {
        return;
    };
    let mut recipe_ids = HashSet::new();
    let mut campfire_ids = HashSet::new();
    let item_ids: HashSet<&str> = context
        .items
        .as_ref()
        .map(|items| items.items.iter().map(|item| item.id.as_str()).collect())
        .unwrap_or_default();
    let equipment_ids: HashSet<&str> = context
        .equipment
        .as_ref()
        .map(|equipment| {
            equipment
                .equipment
                .iter()
                .map(|item| item.id.as_str())
                .collect()
        })
        .unwrap_or_default();
    for recipe in &cooking.recipes {
        if !recipe_ids.insert(recipe.id.as_str()) {
            errors.push(format!("cooking.json: duplicate recipe id '{}'", recipe.id));
        }
        check_non_empty(errors, recipe.name.as_str(), || {
            format!("cooking.json: recipe '{}' requires name", recipe.id)
        });
        if recipe
            .unlock_flag
            .as_deref()
            .map(|flag| flag.trim().is_empty())
            .unwrap_or(false)
        {
            errors.push(format!(
                "cooking.json: recipe '{}' has empty unlock_flag",
                recipe.id
            ));
        }
        if recipe.ingredients.is_empty() {
            errors.push(format!(
                "cooking.json: recipe '{}' requires ingredients",
                recipe.id
            ));
        }
        for ingredient in &recipe.ingredients {
            if ingredient.qty <= 0 {
                errors.push(format!(
                    "cooking.json: recipe '{}' ingredient '{}' must have qty > 0",
                    recipe.id, ingredient.id
                ));
            }
            if !item_ids.contains(ingredient.id.as_str()) {
                errors.push(format!(
                    "cooking.json: recipe '{}' references unknown item '{}'",
                    recipe.id, ingredient.id
                ));
            }
        }
        for item in &recipe.results.items {
            if item.qty <= 0 {
                errors.push(format!(
                    "cooking.json: recipe '{}' result item '{}' must have qty > 0",
                    recipe.id, item.id
                ));
            }
            if !item_ids.contains(item.id.as_str()) {
                errors.push(format!(
                    "cooking.json: recipe '{}' result item '{}' not found in items.json",
                    recipe.id, item.id
                ));
            }
        }
        for item in &recipe.results.equipment {
            if item.qty <= 0 {
                errors.push(format!(
                    "cooking.json: recipe '{}' result equipment '{}' must have qty > 0",
                    recipe.id, item.id
                ));
            }
            if !equipment_ids.contains(item.id.as_str()) {
                errors.push(format!(
                    "cooking.json: recipe '{}' result equipment '{}' not found in equipment.json",
                    recipe.id, item.id
                ));
            }
        }
        for currency in &recipe.results.currency {
            if currency.amount <= 0 {
                errors.push(format!(
                    "cooking.json: recipe '{}' result currency '{}' must have amount > 0",
                    recipe.id, currency.id
                ));
            }
            if !context.ids.currency_ids.contains(currency.id.as_str()) {
                errors.push(format!(
                    "cooking.json: recipe '{}' result currency '{}' not found in rules.json",
                    recipe.id, currency.id
                ));
            }
        }
    }

    for campfire in &cooking.campfires {
        if !campfire_ids.insert(campfire.id.as_str()) {
            errors.push(format!(
                "cooking.json: duplicate campfire id '{}'",
                campfire.id
            ));
        }
        check_non_empty(errors, campfire.label.as_str(), || {
            format!("cooking.json: campfire '{}' requires label", campfire.id)
        });
        if campfire.recipes.is_empty() {
            errors.push(format!(
                "cooking.json: campfire '{}' requires recipes",
                campfire.id
            ));
        }
        for recipe_id in &campfire.recipes {
            if !recipe_ids.contains(recipe_id.as_str()) {
                errors.push(format!(
                    "cooking.json: campfire '{}' references unknown recipe '{}'",
                    campfire.id, recipe_id
                ));
            }
        }
    }
}

pub(crate) fn validate_map_campfires(context: &ValidationContext, errors: &mut Vec<String>) {
    if context.cooking.is_none() {
        for map in context.maps {
            if !map.campfires.is_empty() {
                errors.push(format!(
                    "maps/{}: campfires defined but cooking.json is missing",
                    map.id
                ));
            }
        }
        return;
    }
    let Some(cooking) = context.cooking else {
        return;
    };
    let campfire_ids: HashSet<&str> = cooking
        .campfires
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    for map in context.maps {
        for campfire in &map.campfires {
            if !campfire_ids.contains(campfire.campfire_id.as_str()) {
                errors.push(format!(
                    "maps/{}: campfire '{}' references unknown campfire_id '{}'",
                    map.id, campfire.id, campfire.campfire_id
                ));
            }
        }
    }
}
