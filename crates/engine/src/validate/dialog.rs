use std::collections::HashSet;

use super::helpers::find_recipe;
use super::ValidationContext;

pub(crate) fn validate_npc_dialogs(context: &ValidationContext, errors: &mut Vec<String>) {
    let Some(npcs) = context.npcs else {
        return;
    };
    if !context.dialogs.is_empty() {
        for npc in &npcs.npcs {
            if !context.ids.dialog_ids.contains(npc.dialog.as_str()) {
                errors.push(format!(
                    "npcs.json: npc '{}' references unknown dialog '{}'",
                    npc.id, npc.dialog
                ));
            }
        }
        return;
    }

    errors.push("dialog/: no dialog files found".to_string());
    for npc in &npcs.npcs {
        errors.push(format!(
            "npcs.json: npc '{}' references dialog '{}'",
            npc.id, npc.dialog
        ));
    }
}

pub(crate) fn validate_dialogs(context: &ValidationContext, errors: &mut Vec<String>) {
    if context.dialogs.is_empty() {
        return;
    }

    let event_ids: HashSet<&str> = context
        .events
        .iter()
        .map(|event| event.id.as_str())
        .collect();
    let shop_ids = &context.ids.shop_ids;

    for dialog in context.dialogs {
        for node in &dialog.nodes {
            if let Some(actions) = &node.actions {
                for action in actions {
                    match action.r#type.as_str() {
                        "start_event" => {
                            if let Some(event_id) = &action.event {
                                if !event_ids.contains(event_id.as_str()) {
                                    errors.push(format!(
                                        "dialog/{}: action references unknown event '{}'",
                                        dialog.id, event_id
                                    ));
                                }
                            } else {
                                errors.push(format!(
                                    "dialog/{}: start_event missing event id",
                                    dialog.id
                                ));
                            }
                        }
                        "open_shop" => {
                            if let Some(shop_id) = &action.shop {
                                if !shop_ids.contains(shop_id.as_str()) {
                                    errors.push(format!(
                                        "dialog/{}: action references unknown shop '{}'",
                                        dialog.id, shop_id
                                    ));
                                }
                            } else {
                                errors.push(format!(
                                    "dialog/{}: open_shop missing shop id",
                                    dialog.id
                                ));
                            }
                        }
                        "set_flag" => {
                            if action.flag.is_none() {
                                errors.push(format!("dialog/{}: set_flag missing flag", dialog.id));
                            }
                        }
                        "give_item" => {
                            if action.item.is_none() {
                                errors.push(format!(
                                    "dialog/{}: give_item missing item id",
                                    dialog.id
                                ));
                            }
                        }
                        "learn_recipe" => {
                            let Some(recipe_id) = action.recipe.as_deref() else {
                                errors.push(format!(
                                    "dialog/{}: learn_recipe missing recipe id",
                                    dialog.id
                                ));
                                continue;
                            };
                            let Some(recipe) = find_recipe(
                                errors,
                                context.cooking,
                                recipe_id,
                                || {
                                    format!(
                                        "dialog/{}: learn_recipe '{}' requires cooking.json",
                                        dialog.id, recipe_id
                                    )
                                },
                                || {
                                    format!(
                                        "dialog/{}: learn_recipe '{}' not found",
                                        dialog.id, recipe_id
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
                                    "dialog/{}: learn_recipe '{}' requires recipe unlock_flag",
                                    dialog.id, recipe_id
                                ));
                            }
                        }
                        "rest_party" => {}
                        _ => {
                            errors.push(format!(
                                "dialog/{}: unknown action type '{}'",
                                dialog.id, action.r#type
                            ));
                        }
                    }
                }
            }
            if let Some(choices) = &node.choices {
                for choice in choices {
                    if let Some(flags) = &choice.requires_flags {
                        if flags.iter().any(|flag| flag.trim().is_empty()) {
                            errors.push(format!(
                                "dialog/{}: choice '{}' has empty requires_flags entry",
                                dialog.id, choice.label
                            ));
                        }
                    }
                }
            }
        }
    }
}
