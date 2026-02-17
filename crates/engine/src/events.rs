use crate::maps::EntityState;
use crate::runtime::GameRuntime;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct EventQueue {
    pub pending: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventFile {
    pub version: u32,
    pub id: String,
    pub steps: Vec<EventStep>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventStep {
    pub r#type: String,
    pub speaker: Option<String>,
    pub text: Option<String>,
    pub flag: Option<String>,
    pub flags: Option<Vec<String>>,
    pub requires: Option<Vec<String>>,
    pub item: Option<String>,
    pub qty: Option<i32>,
    pub shop: Option<String>,
    pub stat: Option<String>,
    pub value: Option<i32>,
    pub target: Option<EventTarget>,
    pub encounter: Option<String>,
    pub formation: Option<Vec<FormationMember>>,
    pub npc: Option<String>,
    pub pos: Option<[i32; 2]>,
    pub sprite: Option<String>,
    pub dialog: Option<String>,
    pub spell: Option<String>,
    pub member: Option<String>,
    pub recipe: Option<String>,
    pub ms: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventTarget {
    pub map: String,
    pub pos: [i32; 2],
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct FormationMember {
    pub enemy: String,
    pub pos: [i32; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub enum EventExecutionResult {
    Continue,
    Dialog {
        speaker: String,
        text: String,
    },
    Narration {
        text: String,
    },
    StartDialog {
        dialog_id: String,
    },
    StartBattle {
        encounter: String,
        formation: Vec<FormationMember>,
    },
    OpenShop {
        shop_id: String,
    },
    Wait {
        ms: u64,
    },
    Completed,
    Abort,
}

impl EventFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }
}

pub fn apply_event_step(runtime: &mut GameRuntime, step: &EventStep) -> EventExecutionResult {
    match step.r#type.as_str() {
        "dialog" => {
            let speaker = step.speaker.as_deref().unwrap_or("Narrator");
            let text = step.text.as_deref().unwrap_or("");
            EventExecutionResult::Dialog {
                speaker: speaker.to_string(),
                text: text.to_string(),
            }
        }
        "narration" => {
            let text = step.text.as_deref().unwrap_or("");
            EventExecutionResult::Narration {
                text: text.to_string(),
            }
        }
        "start_dialog" => {
            if let Some(dialog) = &step.dialog {
                EventExecutionResult::StartDialog {
                    dialog_id: dialog.clone(),
                }
            } else {
                EventExecutionResult::Continue
            }
        }
        "set_flag" => {
            if let Some(flag) = &step.flag {
                runtime.set_flag(flag);
            }
            EventExecutionResult::Continue
        }
        "require_flags" => {
            if let Some(flags) = &step.flags {
                let missing = flags
                    .iter()
                    .filter(|flag| !runtime.has_flag(flag))
                    .cloned()
                    .collect::<Vec<_>>();
                if !missing.is_empty() {
                    runtime.abort_event();
                    return EventExecutionResult::Abort;
                }
            }
            EventExecutionResult::Continue
        }
        "give_item" => {
            if let Some(item) = &step.item {
                let qty = step.qty.unwrap_or(1);
                let max_stack = runtime.content.rules.inventory.max_stack;
                runtime.inventory.add_item(item, qty, max_stack);
            }
            EventExecutionResult::Continue
        }
        "give_equipment" => {
            if let Some(item) = &step.item {
                let qty = step.qty.unwrap_or(1);
                let max_stack = runtime.content.rules.inventory.max_stack;
                runtime.inventory.add_equipment(item, qty, max_stack);
            }
            EventExecutionResult::Continue
        }
        "warp" => {
            if let Some(target) = &step.target {
                if target.map == "last_overworld" {
                    runtime.warp_to_last_overworld();
                } else {
                    runtime.warp_to_map(&target.map, (target.pos[0], target.pos[1]));
                }
            }
            EventExecutionResult::Continue
        }
        "start_battle" => {
            let encounter = step.encounter.clone().unwrap_or_default();
            let formation = step.formation.clone().unwrap_or_default();
            EventExecutionResult::StartBattle {
                encounter,
                formation,
            }
        }
        "open_shop" => {
            if let Some(shop) = &step.shop {
                EventExecutionResult::OpenShop {
                    shop_id: shop.clone(),
                }
            } else {
                EventExecutionResult::Continue
            }
        }
        "npc_show" | "npc_hide" | "npc_move" | "npc_set_sprite" => {
            if let Some(npc_id) = &step.npc {
                let map_id = runtime.world.map_id.clone();
                let map_state = runtime.map_states.entry(map_id).or_default();
                let entity_state =
                    map_state
                        .entities
                        .entry(npc_id.clone())
                        .or_insert(EntityState {
                            pos: None,
                            state: None,
                            visible: None,
                            sprite: None,
                        });

                match step.r#type.as_str() {
                    "npc_show" => entity_state.visible = Some(true),
                    "npc_hide" => entity_state.visible = Some(false),
                    "npc_move" => {
                        if let Some(pos) = step.pos {
                            entity_state.pos = Some((pos[0], pos[1]));
                        }
                    }
                    "npc_set_sprite" => {
                        if let Some(sprite) = &step.sprite {
                            entity_state.sprite = Some(sprite.clone());
                        }
                    }
                    _ => {}
                }
            }
            EventExecutionResult::Continue
        }
        "wait" => {
            let ms = step.ms.unwrap_or(0).max(0) as u64;
            EventExecutionResult::Wait { ms }
        }
        "learn_spell" => {
            if let (Some(member), Some(spell)) = (&step.member, &step.spell) {
                crate::party::learn_spell_event(&mut runtime.party, member, spell);
            }
            EventExecutionResult::Continue
        }
        "party_add" => {
            if let Some(member) = &step.member {
                if crate::party::add_party_member_event(
                    &mut runtime.party,
                    &runtime.content,
                    &runtime.content.rules,
                    member,
                )
                .is_err()
                {
                    runtime.abort_event();
                    return EventExecutionResult::Abort;
                }
            } else {
                runtime.abort_event();
                return EventExecutionResult::Abort;
            }
            EventExecutionResult::Continue
        }
        "party_remove" => {
            if let Some(member) = &step.member {
                if crate::party::remove_party_member_event(&mut runtime.party, member).is_err() {
                    runtime.abort_event();
                    return EventExecutionResult::Abort;
                }
            } else {
                runtime.abort_event();
                return EventExecutionResult::Abort;
            }
            EventExecutionResult::Continue
        }
        "learn_recipe" => {
            if let Some(recipe_id) = &step.recipe {
                unlock_recipe(runtime, recipe_id);
            }
            EventExecutionResult::Continue
        }
        "stat_set" => {
            if let (Some(stat), Some(value)) = (&step.stat, step.value) {
                runtime.set_stat(stat, value);
            }
            EventExecutionResult::Continue
        }
        "stat_add" => {
            if let Some(stat) = &step.stat {
                let value = step.value.unwrap_or(1);
                runtime.add_stat(stat, value);
            }
            EventExecutionResult::Continue
        }
        "stat_max" => {
            if let (Some(stat), Some(value)) = (&step.stat, step.value) {
                runtime.track_max_stat(stat, value);
            }
            EventExecutionResult::Continue
        }
        _ => EventExecutionResult::Continue,
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
