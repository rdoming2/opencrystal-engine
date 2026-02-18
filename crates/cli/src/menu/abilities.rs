use engine::party::job_level;
use engine::rules::{AbilityAcquisition, JpMode};
use engine::runtime::GameRuntime;
use tui::menu::{MenuPanelLine, MenuPanelView, PanelSpanStyle};

use super::common::AbilityEntry;
use super::equipment::detail_actor_id;
use super::inventory::{panel_line, panel_line_spans, panel_span};

pub fn build_abilities_panel(runtime: &GameRuntime) -> MenuPanelView {
    let actor_id = detail_actor_id(runtime);
    let Some(actor_id) = actor_id else {
        return MenuPanelView {
            title: "Abilities".to_string(),
            lines: vec![panel_line("No party members.")],
        };
    };
    let Some(actor) = runtime.party.roster.get(&actor_id) else {
        return MenuPanelView {
            title: "Abilities".to_string(),
            lines: vec![panel_line("No party members.")],
        };
    };
    let entries = build_ability_entries(runtime);
    let mut lines = Vec::new();
    lines.push(ability_header_line(actor));
    if entries.is_empty() {
        lines.push(panel_line("------------------------------"));
        lines.push(panel_line("No abilities learned."));
        lines.push(panel_line("Gain levels to unlock abilities."));
        lines.push(panel_line("------------------------------"));
        lines.push(panel_line_spans(vec![panel_span(
            "Details",
            PanelSpanStyle::Accent,
        )]));
        lines.push(panel_line("Select a learned ability."));
        return MenuPanelView {
            title: "Abilities".to_string(),
            lines,
        };
    }
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    let width = ability_list_width(&entries);
    for (index, entry) in entries.iter().enumerate() {
        lines.push(build_ability_line(
            runtime,
            entry,
            index == selection,
            width,
        ));
    }
    lines.push(panel_line("------------------------------"));
    if runtime.menu_state.detail_page == 1 {
        lines.extend(build_ability_target_panel(
            runtime,
            entries.get(selection),
            &actor_id,
        ));
        lines.push(panel_line("------------------------------"));
    }
    lines.push(panel_line_spans(vec![panel_span(
        "Details",
        PanelSpanStyle::Accent,
    )]));
    lines.extend(build_ability_description(
        runtime,
        entries.get(selection),
        actor,
    ));

    MenuPanelView {
        title: "Abilities".to_string(),
        lines,
    }
}

pub fn build_ability_entries(runtime: &GameRuntime) -> Vec<AbilityEntry> {
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Vec::new(),
    };
    let actor = match runtime.party.roster.get(&actor_id) {
        Some(actor) => actor,
        None => return Vec::new(),
    };
    let mut entries = Vec::new();
    let mut ability_ids = collect_ability_ids(runtime, actor);
    ability_ids.sort();
    ability_ids.dedup();
    for ability_id in ability_ids {
        let Some(ability) = runtime
            .content
            .abilities
            .abilities
            .iter()
            .find(|ability| ability.id == ability_id)
        else {
            continue;
        };
        let (cost_type, cost_value, cost_item_id, cost_currency_id) =
            if let Some(cost) = &ability.cost {
                (
                    cost.r#type.clone(),
                    cost.value,
                    cost.item_id.clone(),
                    cost.currency_id.clone(),
                )
            } else {
                ("none".to_string(), 0, None, None)
            };
        let (usable, reason) = ability_cost_available(
            runtime,
            actor,
            &cost_type,
            cost_value,
            cost_item_id.as_deref(),
            cost_currency_id.as_deref(),
        );
        entries.push(AbilityEntry {
            id: ability.id.clone(),
            name: ability.name.clone(),
            default_target: ability.default_target.clone(),
            allowed_targets: ability.allowed_targets.clone(),
            target_mode: ability.target_mode.clone(),
            multi_attenuation: ability.multi_attenuation,
            effect_type: ability.effect.r#type.clone(),
            effect_power: ability.effect.power,
            cost_type,
            cost_value,
            cost_item_id,
            cost_currency_id,
            usable,
            reason,
        });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}

pub fn selected_ability_targets(runtime: &GameRuntime) -> Vec<String> {
    let entries = build_ability_entries(runtime);
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Vec::new(),
    };
    match entries.get(selection) {
        Some(entry) => ability_targets_for_entry(runtime, entry, &actor_id),
        None => Vec::new(),
    }
}

pub fn ability_targets_for_entry(
    runtime: &GameRuntime,
    entry: &AbilityEntry,
    actor_id: &str,
) -> Vec<String> {
    let mut targets = match entry.default_target.as_str() {
        "self" => vec![actor_id.to_string()],
        "party" => runtime.party.active_ids(),
        "ally" => runtime.party.active_ids(),
        _ => Vec::new(),
    };
    if entry.effect_type == "revive" {
        targets.retain(|id| {
            runtime
                .party
                .roster
                .get(id)
                .map(|actor| actor.current_hp <= 0)
                .unwrap_or(false)
        });
    }
    targets
}

pub fn apply_ability_to_actor(runtime: &mut GameRuntime, entry: &AbilityEntry, actor_id: &str) {
    let Some(actor) = runtime.party.roster.get_mut(actor_id) else {
        return;
    };
    let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
    match entry.effect_type.as_str() {
        "heal" => {
            actor.current_hp = (actor.current_hp + entry.effect_power).clamp(0, max_hp);
        }
        "revive" => {
            if actor.current_hp <= 0 {
                let amount = if entry.effect_power > 0 {
                    entry.effect_power
                } else {
                    max_hp
                };
                actor.current_hp = amount.clamp(1, max_hp);
            }
        }
        _ => {}
    }
}

pub fn ability_cost_available(
    runtime: &GameRuntime,
    actor: &engine::party::Actor,
    cost_type: &str,
    cost_value: i32,
    cost_item_id: Option<&str>,
    cost_currency_id: Option<&str>,
) -> (bool, Option<String>) {
    match cost_type {
        "none" => (true, None),
        "mp" => {
            if cost_value <= 0 || actor.current_mp >= cost_value {
                (true, None)
            } else {
                (false, Some("Not enough MP.".to_string()))
            }
        }
        "hp" => {
            if cost_value <= 0 || actor.current_hp >= cost_value {
                (true, None)
            } else {
                (false, Some("Not enough HP.".to_string()))
            }
        }
        "currency" => {
            let Some(currency_id) = cost_currency_id else {
                return (false, Some("No currency specified.".to_string()));
            };
            let amount = runtime.inventory.currency_amount(currency_id);
            if cost_value <= 0 || amount >= cost_value {
                (true, None)
            } else {
                (false, Some("Not enough currency.".to_string()))
            }
        }
        "item" => {
            if let Some(item_id) = cost_item_id {
                let qty = runtime.inventory.item_qty(item_id);
                if cost_value <= 0 || qty >= cost_value {
                    (true, None)
                } else {
                    (false, Some("Not enough items.".to_string()))
                }
            } else {
                (false, Some("No item specified.".to_string()))
            }
        }
        "death" => (true, None),
        "random" => (true, None),
        _ => (false, Some("Unknown cost type.".to_string())),
    }
}

pub fn consume_ability_cost(
    runtime: &mut GameRuntime,
    entry: &AbilityEntry,
    actor_id: &str,
) -> bool {
    let Some(actor) = runtime.party.roster.get_mut(actor_id) else {
        return false;
    };
    match entry.cost_type.as_str() {
        "none" => true,
        "mp" => {
            if actor.current_mp >= entry.cost_value {
                actor.current_mp -= entry.cost_value;
                true
            } else {
                false
            }
        }
        "hp" => {
            if actor.current_hp >= entry.cost_value {
                actor.current_hp -= entry.cost_value;
                true
            } else {
                false
            }
        }
        "currency" => {
            let Some(currency_id) = entry.cost_currency_id.as_deref() else {
                return false;
            };
            let amount = runtime.inventory.currency_amount(currency_id);
            if amount < entry.cost_value {
                return false;
            }
            runtime
                .inventory
                .add_currency(currency_id, -entry.cost_value);
            true
        }
        "item" => {
            if let Some(item_id) = &entry.cost_item_id {
                if runtime.inventory.remove_item(item_id, entry.cost_value) {
                    true
                } else {
                    false
                }
            } else {
                false
            }
        }
        "death" => {
            actor.current_hp = 0;
            true
        }
        "random" => {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            if rng.gen_bool(0.5) {
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

pub fn build_battle_ability_entries(
    runtime: &GameRuntime,
    actor_id: &str,
    command_group: Option<&str>,
) -> Vec<AbilityEntry> {
    let Some(actor) = runtime.party.roster.get(actor_id) else {
        return Vec::new();
    };
    let mut ability_ids = collect_ability_ids(runtime, actor);
    ability_ids.sort();
    ability_ids.dedup();
    let mut entries = Vec::new();
    for ability_id in ability_ids {
        let Some(ability) = runtime
            .content
            .abilities
            .abilities
            .iter()
            .find(|ability| ability.id == ability_id)
        else {
            continue;
        };
        if let Some(group) = command_group {
            if ability.command_group.as_deref() != Some(group) {
                continue;
            }
        }
        let (cost_type, cost_value, cost_item_id, cost_currency_id) =
            if let Some(cost) = &ability.cost {
                (
                    cost.r#type.clone(),
                    cost.value,
                    cost.item_id.clone(),
                    cost.currency_id.clone(),
                )
            } else {
                ("none".to_string(), 0, None, None)
            };
        let (usable, reason) = ability_cost_available(
            runtime,
            actor,
            &cost_type,
            cost_value,
            cost_item_id.as_deref(),
            cost_currency_id.as_deref(),
        );
        entries.push(AbilityEntry {
            id: ability.id.clone(),
            name: ability.name.clone(),
            default_target: ability.default_target.clone(),
            allowed_targets: ability.allowed_targets.clone(),
            target_mode: ability.target_mode.clone(),
            multi_attenuation: ability.multi_attenuation,
            effect_type: ability.effect.r#type.clone(),
            effect_power: ability.effect.power,
            cost_type,
            cost_value,
            cost_item_id,
            cost_currency_id,
            usable,
            reason,
        });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}

fn ability_header_line(actor: &engine::party::Actor) -> MenuPanelLine {
    panel_line_spans(vec![
        panel_span("Actor: ", PanelSpanStyle::Normal),
        panel_span(actor.name.clone(), PanelSpanStyle::Highlight),
        panel_span("  (Left/Right)", PanelSpanStyle::Muted),
    ])
}

fn collect_ability_ids(runtime: &GameRuntime, actor: &engine::party::Actor) -> Vec<String> {
    let mut ids = Vec::new();
    let mut job_ids = vec![actor.job_id.as_str()];
    if runtime.content.rules.job_system.secondary_jobs {
        if let Some(job_id) = actor.secondary_job_id.as_deref() {
            job_ids.push(job_id);
        }
    }
    for job_id in job_ids {
        if let Some(job) = runtime
            .content
            .jobs
            .jobs
            .iter()
            .find(|job| job.id == job_id)
        {
            for ability in &job.abilities {
                if !job_ability_available(runtime, actor, job, ability) {
                    continue;
                }
                if !ids.contains(&ability.id) {
                    ids.push(ability.id.clone());
                }
            }
        }
    }
    ids
}

fn job_ability_available(
    runtime: &GameRuntime,
    actor: &engine::party::Actor,
    job: &engine::entities::JobDefinition,
    ability: &engine::entities::JobAbility,
) -> bool {
    let acquisition = resolve_ability_acquisition(runtime, job);
    let current_level = job_level(actor, &job.id);
    match acquisition {
        AbilityAcquisition::Level => ability.level.unwrap_or(0) <= current_level,
        AbilityAcquisition::Jp => {
            if runtime.content.rules.job_system.jp_mode == JpMode::Spend {
                actor.unlocked_abilities.contains(&ability.id)
            } else {
                ability.level.unwrap_or(0) <= current_level
            }
        }
        AbilityAcquisition::Item | AbilityAcquisition::Equip => {
            actor.unlocked_abilities.contains(&ability.id)
        }
    }
}

fn resolve_ability_acquisition(
    runtime: &GameRuntime,
    job: &engine::entities::JobDefinition,
) -> AbilityAcquisition {
    job.acquisition
        .as_ref()
        .and_then(|acquisition| acquisition.abilities.clone())
        .unwrap_or_else(|| runtime.content.rules.game.ability_acquisition.clone())
}

fn ability_list_width(entries: &[AbilityEntry]) -> usize {
    entries
        .iter()
        .map(|entry| entry.name.chars().count())
        .max()
        .unwrap_or(10)
        + 2
}

fn build_ability_line(
    runtime: &GameRuntime,
    entry: &AbilityEntry,
    is_selected: bool,
    width: usize,
) -> MenuPanelLine {
    let prefix = if is_selected { "> " } else { "  " };
    let label = format!("{:width$}", entry.name, width = width);
    let cost_text = ability_cost_label(runtime, entry);
    let base_style = if entry.usable {
        PanelSpanStyle::Normal
    } else {
        PanelSpanStyle::Muted
    };
    let style = if is_selected {
        PanelSpanStyle::Highlight
    } else {
        base_style
    };
    panel_line_spans(vec![
        panel_span(prefix, style),
        panel_span(label, style),
        panel_span(cost_text, style),
    ])
}

pub fn ability_cost_label(runtime: &GameRuntime, entry: &AbilityEntry) -> String {
    match entry.cost_type.as_str() {
        "mp" => format!(" MP {}", entry.cost_value),
        "hp" => format!(" HP {}", entry.cost_value),
        "currency" => format_currency_cost(runtime, entry),
        "item" => {
            if let Some(item_id) = &entry.cost_item_id {
                format!(" {} x{}", item_id, entry.cost_value)
            } else {
                "".to_string()
            }
        }
        "death" => " Death".to_string(),
        "random" => " Random".to_string(),
        _ => "".to_string(),
    }
}

fn format_currency_cost(runtime: &GameRuntime, entry: &AbilityEntry) -> String {
    let Some(currency_id) = entry.cost_currency_id.as_deref() else {
        return "".to_string();
    };
    if let Some(currency) = runtime.content.rules.game.currency(currency_id) {
        if currency.symbol.trim().is_empty() {
            format!(" {} {}", entry.cost_value, currency.name)
        } else {
            format!(" {}{}", currency.symbol, entry.cost_value)
        }
    } else {
        format!(" {} {}", entry.cost_value, currency_id)
    }
}

fn build_ability_target_panel(
    runtime: &GameRuntime,
    entry: Option<&AbilityEntry>,
    actor_id: &str,
) -> Vec<MenuPanelLine> {
    let Some(entry) = entry else {
        return vec![panel_line("No target."), panel_line("")];
    };
    let targets = ability_targets_for_entry(runtime, entry, actor_id);
    if targets.is_empty() {
        return vec![panel_line("No valid targets."), panel_line("")];
    }
    let selection = runtime
        .menu_state
        .detail_target
        .min(targets.len().saturating_sub(1));
    let mut lines = Vec::new();
    lines.push(panel_line_spans(vec![panel_span(
        "Target",
        PanelSpanStyle::Accent,
    )]));
    for (index, target_id) in targets.iter().enumerate() {
        let name = runtime
            .party
            .roster
            .get(target_id)
            .map(|actor| actor.name.as_str())
            .unwrap_or(target_id.as_str());
        let is_selected = index == selection;
        lines.push(panel_line_spans(vec![
            panel_span(
                if is_selected { "> " } else { "  " },
                if is_selected {
                    PanelSpanStyle::Highlight
                } else {
                    PanelSpanStyle::Normal
                },
            ),
            panel_span(
                name,
                if is_selected {
                    PanelSpanStyle::Highlight
                } else {
                    PanelSpanStyle::Normal
                },
            ),
        ]));
    }
    lines
}

fn build_ability_description(
    runtime: &GameRuntime,
    entry: Option<&AbilityEntry>,
    actor: &engine::party::Actor,
) -> Vec<MenuPanelLine> {
    let Some(entry) = entry else {
        return vec![panel_line("No selection.")];
    };
    let mut lines = Vec::new();
    lines.push(panel_line_spans(vec![
        panel_span("Actor: ", PanelSpanStyle::Normal),
        panel_span(actor.name.clone(), PanelSpanStyle::Highlight),
    ]));
    lines.push(panel_line_spans(vec![
        panel_span("ID: ", PanelSpanStyle::Normal),
        panel_span(entry.id.clone(), PanelSpanStyle::Accent),
    ]));
    let allowed_targets = if entry.allowed_targets.is_empty() {
        entry.default_target.clone()
    } else {
        entry.allowed_targets.join(", ")
    };
    lines.push(panel_line_spans(vec![
        panel_span("Target: ", PanelSpanStyle::Normal),
        panel_span(entry.default_target.clone(), PanelSpanStyle::Accent),
    ]));
    lines.push(panel_line_spans(vec![
        panel_span("Allowed: ", PanelSpanStyle::Normal),
        panel_span(allowed_targets, PanelSpanStyle::Accent),
    ]));
    lines.push(panel_line_spans(vec![
        panel_span("Effect: ", PanelSpanStyle::Normal),
        panel_span(entry.effect_type.clone(), PanelSpanStyle::Accent),
        panel_span("  Power: ", PanelSpanStyle::Normal),
        panel_span(entry.effect_power.to_string(), PanelSpanStyle::Accent),
    ]));
    match entry.cost_type.as_str() {
        "mp" => {
            lines.push(panel_line_spans(vec![
                panel_span("Cost: ", PanelSpanStyle::Normal),
                panel_span(format!("MP {}", entry.cost_value), PanelSpanStyle::Accent),
                panel_span("  MP: ", PanelSpanStyle::Normal),
                panel_span(
                    format!(
                        "{}/{}",
                        actor.current_mp,
                        actor.derived_stats.get("mp").copied().unwrap_or(0)
                    ),
                    PanelSpanStyle::Accent,
                ),
            ]));
        }
        "hp" => {
            lines.push(panel_line_spans(vec![
                panel_span("Cost: ", PanelSpanStyle::Normal),
                panel_span(format!("HP {}", entry.cost_value), PanelSpanStyle::Accent),
            ]));
        }
        "currency" => {
            let Some(currency_id) = entry.cost_currency_id.as_deref() else {
                return lines;
            };
            let currency_amount = runtime.inventory.currency_amount(currency_id);
            let (currency_label, cost_label) =
                if let Some(currency) = runtime.content.rules.game.currency(currency_id) {
                    if currency.symbol.trim().is_empty() {
                        (
                            currency.name.clone(),
                            format!("{} {}", entry.cost_value, currency.name),
                        )
                    } else {
                        (
                            currency.symbol.clone(),
                            format!("{}{}", currency.symbol, entry.cost_value),
                        )
                    }
                } else {
                    (
                        currency_id.to_string(),
                        format!("{} {}", entry.cost_value, currency_id),
                    )
                };
            lines.push(panel_line_spans(vec![
                panel_span("Cost: ", PanelSpanStyle::Normal),
                panel_span(cost_label, PanelSpanStyle::Accent),
                panel_span(format!("  {}: ", currency_label), PanelSpanStyle::Normal),
                panel_span(format!("{}", currency_amount), PanelSpanStyle::Accent),
            ]));
        }
        "item" => {
            if let Some(item_id) = &entry.cost_item_id {
                let item_name = runtime
                    .content
                    .items
                    .items
                    .iter()
                    .find(|item| item.id == *item_id)
                    .map(|item| item.name.as_str())
                    .unwrap_or(item_id);
                let item_qty = runtime.inventory.item_qty(item_id);
                lines.push(panel_line_spans(vec![
                    panel_span("Cost: ", PanelSpanStyle::Normal),
                    panel_span(
                        format!("{} x{}", item_name, entry.cost_value),
                        PanelSpanStyle::Accent,
                    ),
                    panel_span("  Qty: ", PanelSpanStyle::Normal),
                    panel_span(format!("{}", item_qty), PanelSpanStyle::Accent),
                ]));
            }
        }
        "death" => {
            lines.push(panel_line_spans(vec![
                panel_span("Cost: ", PanelSpanStyle::Normal),
                panel_span("Death", PanelSpanStyle::Accent),
            ]));
        }
        "random" => {
            lines.push(panel_line_spans(vec![
                panel_span("Cost: ", PanelSpanStyle::Normal),
                panel_span("Random (50% success)", PanelSpanStyle::Accent),
            ]));
        }
        _ => {}
    }
    let description = runtime
        .content
        .abilities
        .abilities
        .iter()
        .find(|ability| ability.id == entry.id)
        .and_then(|ability| ability.description.clone())
        .unwrap_or_else(|| "Battle-only ability.".to_string());
    lines.push(panel_line_spans(vec![
        panel_span("Description: ", PanelSpanStyle::Accent),
        panel_span(description, PanelSpanStyle::Normal),
    ]));
    if let Some(reason) = &entry.reason {
        lines.push(panel_line_spans(vec![
            panel_span("Unavailable: ", PanelSpanStyle::Muted),
            panel_span(reason.clone(), PanelSpanStyle::Muted),
        ]));
    }
    lines
}
