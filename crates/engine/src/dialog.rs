use crate::events::EventExecutionResult;
use crate::maps::MapCurrencyStack;
use crate::party::rest_party;
use crate::rules::RulesFile;
use crate::runtime::GameRuntime;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DialogFile {
    pub version: u32,
    pub id: String,
    pub nodes: Vec<DialogNode>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DialogNode {
    pub id: String,
    pub speaker: Option<String>,
    pub text: String,
    pub actions: Option<Vec<DialogAction>>,
    pub choices: Option<Vec<DialogChoice>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DialogAction {
    pub r#type: String,
    pub shop: Option<String>,
    pub flag: Option<String>,
    pub event: Option<String>,
    pub item: Option<String>,
    pub qty: Option<i32>,
    pub recipe: Option<String>,
    pub cost: Option<MapCurrencyStack>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DialogChoice {
    pub label: String,
    pub next: String,
    pub requires_flags: Option<Vec<String>>,
}

impl DialogFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

pub fn apply_dialog_action(
    runtime: &mut GameRuntime,
    action: &DialogAction,
) -> EventExecutionResult {
    match action.r#type.as_str() {
        "start_event" => {
            if let Some(event_id) = &action.event {
                runtime.queue_event(event_id);
            }
            EventExecutionResult::Continue
        }
        "open_shop" => {
            if let Some(shop_id) = &action.shop {
                EventExecutionResult::OpenShop {
                    shop_id: shop_id.clone(),
                }
            } else {
                EventExecutionResult::Continue
            }
        }
        "set_flag" => {
            if let Some(flag) = &action.flag {
                runtime.set_flag(flag);
            }
            EventExecutionResult::Continue
        }
        "give_item" => {
            if let Some(item) = &action.item {
                let qty = action.qty.unwrap_or(1);
                let max_stack = runtime.content.rules.inventory.max_stack;
                runtime.inventory.add_item(item, qty, max_stack);
            }
            EventExecutionResult::Continue
        }
        "rest_party" => {
            if let Some(cost) = action.cost.as_ref() {
                if cost.amount > 0 && !cost.id.trim().is_empty() {
                    let available = runtime.inventory.currency_amount(&cost.id);
                    if available < cost.amount {
                        return EventExecutionResult::Narration {
                            text: format!(
                                "Need {} to rest.",
                                format_currency_cost(&runtime.content.rules, cost)
                            ),
                        };
                    }
                    runtime.inventory.add_currency(&cost.id, -cost.amount);
                }
            }
            rest_party(&mut runtime.party, &runtime.content, &runtime.content.rules);
            EventExecutionResult::Continue
        }
        "learn_recipe" => {
            if let Some(recipe_id) = &action.recipe {
                unlock_recipe(runtime, recipe_id);
            }
            EventExecutionResult::Continue
        }
        _ => EventExecutionResult::Continue,
    }
}

fn format_currency_cost(rules: &RulesFile, cost: &MapCurrencyStack) -> String {
    if let Some(currency) = rules.game.currency(&cost.id) {
        if currency.symbol.trim().is_empty() {
            format!("{} {}", cost.amount, currency.name)
        } else {
            format!("{}{}", currency.symbol, cost.amount)
        }
    } else {
        format!("{} {}", cost.amount, cost.id)
    }
}

fn unlock_recipe(runtime: &mut GameRuntime, recipe_id: &str) {
    let flag = runtime
        .content
        .cooking
        .as_ref()
        .and_then(|cooking| cooking.recipes.iter().find(|recipe| recipe.id == recipe_id))
        .and_then(|recipe| recipe.unlock_flag.clone())
        .filter(|flag| !flag.trim().is_empty());
    if let Some(flag) = flag {
        runtime.set_flag(&flag);
    }
}
