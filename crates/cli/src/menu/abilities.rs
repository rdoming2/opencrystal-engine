use engine::runtime::GameRuntime;
use tui::app::{MenuPanelLine, MenuPanelView, PanelSpanStyle};

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
        lines.push(build_ability_line(entry, index == selection, width));
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
        entries.push(AbilityEntry {
            id: ability.id.clone(),
            name: ability.name.clone(),
            default_target: ability.default_target.clone(),
            allowed_targets: ability.allowed_targets.clone(),
            effect_type: ability.effect.r#type.clone(),
            effect_power: ability.effect.power,
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
        "party" => runtime.party.active.clone(),
        "ally" => runtime.party.active.clone(),
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

pub fn build_battle_ability_entries(runtime: &GameRuntime, actor_id: &str) -> Vec<AbilityEntry> {
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
        entries.push(AbilityEntry {
            id: ability.id.clone(),
            name: ability.name.clone(),
            default_target: ability.default_target.clone(),
            allowed_targets: ability.allowed_targets.clone(),
            effect_type: ability.effect.r#type.clone(),
            effect_power: ability.effect.power,
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
    let job = runtime
        .content
        .jobs
        .jobs
        .iter()
        .find(|job| job.id == actor.job_id);
    if let Some(job) = job {
        for ability in &job.abilities {
            if !job_ability_available(actor, ability) {
                continue;
            }
            if !ids.contains(&ability.id) {
                ids.push(ability.id.clone());
            }
        }
    }
    ids
}

fn job_ability_available(
    actor: &engine::party::Actor,
    ability: &engine::entities::JobAbility,
) -> bool {
    match ability.method.as_str() {
        "level" => ability.level.unwrap_or(0) <= actor.level,
        _ => false,
    }
}

fn ability_list_width(entries: &[AbilityEntry]) -> usize {
    entries
        .iter()
        .map(|entry| entry.name.chars().count())
        .max()
        .unwrap_or(10)
        + 2
}

fn build_ability_line(entry: &AbilityEntry, is_selected: bool, width: usize) -> MenuPanelLine {
    let prefix = if is_selected { "> " } else { "  " };
    let label = format!("{:width$}", entry.name, width = width);
    let style = if is_selected {
        PanelSpanStyle::Highlight
    } else {
        PanelSpanStyle::Normal
    };
    panel_line_spans(vec![panel_span(prefix, style), panel_span(label, style)])
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
    lines
}
