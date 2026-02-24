use std::collections::HashSet;

use super::helpers::find_recipe;
use super::ValidationContext;

const EVENT_TYPES: [&str; 23] = [
    "dialog",
    "narration",
    "set_flag",
    "require_flags",
    "give_item",
    "give_equipment",
    "require_items",
    "remove_item",
    "warp",
    "start_battle",
    "start_dialog",
    "open_shop",
    "npc_show",
    "npc_hide",
    "npc_move",
    "npc_set_sprite",
    "party_add",
    "party_remove",
    "learn_recipe",
    "wait",
    "stat_set",
    "stat_add",
    "stat_max",
];

pub(crate) fn validate_events(context: &ValidationContext, errors: &mut Vec<String>) {
    let event_types: HashSet<&str> = EVENT_TYPES.iter().copied().collect();
    for event in context.events {
        for step in &event.steps {
            if !event_types.contains(step.r#type.as_str()) {
                errors.push(format!(
                    "events/{}: unknown step type '{}'",
                    event.id, step.r#type
                ));
            }
            if step.r#type == "require_items" || step.r#type == "remove_item" {
                let item_id = step.item.as_deref().unwrap_or("");
                if item_id.trim().is_empty() {
                    errors.push(format!(
                        "events/{}: {} step missing item",
                        event.id, step.r#type
                    ));
                } else if !context.ids.item_ids.contains(item_id) {
                    errors.push(format!(
                        "events/{}: {} step references unknown item '{}'",
                        event.id, step.r#type, item_id
                    ));
                }
                if step.qty.unwrap_or(1) <= 0 {
                    errors.push(format!(
                        "events/{}: {} step requires qty > 0",
                        event.id, step.r#type
                    ));
                }
            }
            if step.r#type == "warp" {
                let Some(target) = &step.target else {
                    errors.push(format!("events/{}: warp step missing target", event.id));
                    continue;
                };
                if target.map != "last_overworld" {
                    if !context.ids.map_ids.contains(target.map.as_str()) {
                        errors.push(format!(
                            "events/{}: warp target '{}' not found",
                            event.id, target.map
                        ));
                        continue;
                    }
                    if let Some((width, height)) = context.ids.map_dims.get(target.map.as_str()) {
                        if target.pos[0] < 0
                            || target.pos[1] < 0
                            || target.pos[0] >= *width as i32
                            || target.pos[1] >= *height as i32
                        {
                            errors.push(format!(
                                "events/{}: warp target_pos {:?} out of bounds",
                                event.id, target.pos
                            ));
                        }
                    }
                }
            }
            if step.r#type == "stat_set" || step.r#type == "stat_max" {
                if step.stat.as_deref().unwrap_or("").is_empty() {
                    errors.push(format!(
                        "events/{}: {} step missing stat",
                        event.id, step.r#type
                    ));
                }
                if step.value.is_none() {
                    errors.push(format!(
                        "events/{}: {} step missing value",
                        event.id, step.r#type
                    ));
                }
            }
            if step.r#type == "stat_add" {
                if step.stat.as_deref().unwrap_or("").is_empty() {
                    errors.push(format!("events/{}: stat_add step missing stat", event.id));
                }
            }
            if step.r#type == "party_add" {
                let member_id = step.member.as_deref().unwrap_or("");
                if member_id.trim().is_empty() {
                    errors.push(format!(
                        "events/{}: party_add step missing member",
                        event.id
                    ));
                }
                let Some(party) = context.party else {
                    errors.push(format!(
                        "events/{}: party_add '{}' requires party.json",
                        event.id, member_id
                    ));
                    continue;
                };
                if !member_id.trim().is_empty()
                    && !party.roster.iter().any(|actor| actor.id == member_id)
                {
                    errors.push(format!(
                        "events/{}: party_add '{}' not found in party roster",
                        event.id, member_id
                    ));
                }
            }
            if step.r#type == "party_remove" {
                if step.member.as_deref().unwrap_or("").trim().is_empty() {
                    errors.push(format!(
                        "events/{}: party_remove step missing member",
                        event.id
                    ));
                }
            }
            if step.r#type == "learn_recipe" {
                let Some(recipe_id) = step.recipe.as_deref() else {
                    errors.push(format!(
                        "events/{}: learn_recipe step missing recipe",
                        event.id
                    ));
                    continue;
                };
                let Some(recipe) = find_recipe(
                    errors,
                    context.cooking,
                    recipe_id,
                    || {
                        format!(
                            "events/{}: learn_recipe '{}' requires cooking.json",
                            event.id, recipe_id
                        )
                    },
                    || {
                        format!(
                            "events/{}: learn_recipe '{}' not found",
                            event.id, recipe_id
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
                        "events/{}: learn_recipe '{}' requires recipe unlock_flag",
                        event.id, recipe_id
                    ));
                }
            }
        }
    }
}
