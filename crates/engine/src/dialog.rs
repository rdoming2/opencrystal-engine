use crate::events::EventExecutionResult;
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
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DialogChoice {
    pub label: String,
    pub next: String,
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
        _ => EventExecutionResult::Continue,
    }
}
