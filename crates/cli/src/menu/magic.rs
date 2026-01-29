use std::collections::HashMap;

use engine::rules::MagicSystem;
use engine::runtime::GameRuntime;
use tui::app::{MenuPanelLine, MenuPanelView, PanelSpanStyle};

use super::common::SpellEntry;
use super::equipment::detail_actor_id;
use super::inventory::{panel_line, panel_line_spans, panel_span};

pub fn build_magic_panel(runtime: &GameRuntime) -> MenuPanelView {
    let actor_id = detail_actor_id(runtime);
    let Some(actor_id) = actor_id else {
        return MenuPanelView {
            title: "Magic".to_string(),
            lines: vec![panel_line("No party members.")],
        };
    };
    let Some(actor) = runtime.party.roster.get(&actor_id) else {
        return MenuPanelView {
            title: "Magic".to_string(),
            lines: vec![panel_line("No party members.")],
        };
    };
    let entries = build_spell_entries(runtime);
    let mut lines = Vec::new();
    lines.push(magic_header_line(runtime, actor));
    if entries.is_empty() {
        lines.push(panel_line("------------------------------"));
        lines.push(panel_line("No spells learned."));
        lines.push(panel_line("Learn spells to use magic."));
        lines.push(panel_line("------------------------------"));
        lines.push(panel_line_spans(vec![panel_span(
            "Details",
            PanelSpanStyle::Accent,
        )]));
        lines.push(panel_line("Select a learned spell."));
        return MenuPanelView {
            title: "Magic".to_string(),
            lines,
        };
    }
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    let width = spell_list_width(&entries);
    for (index, entry) in entries.iter().enumerate() {
        lines.push(build_spell_line(entry, index == selection, width));
    }
    lines.push(panel_line("------------------------------"));
    if runtime.menu_state.detail_page == 1 {
        lines.extend(build_spell_target_panel(
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
    lines.extend(build_spell_description(
        runtime,
        entries.get(selection),
        actor,
    ));

    MenuPanelView {
        title: "Magic".to_string(),
        lines,
    }
}

pub fn build_spell_entries(runtime: &GameRuntime) -> Vec<SpellEntry> {
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Vec::new(),
    };
    let actor = match runtime.party.roster.get(&actor_id) {
        Some(actor) => actor,
        None => return Vec::new(),
    };
    let mut entries = Vec::new();
    let mut school_lookup = HashMap::new();
    for school in &runtime.content.spells.schools {
        school_lookup.insert(school.id.as_str(), school.name.as_str());
    }
    let mut spell_ids = collect_spell_ids(runtime, actor);
    spell_ids.sort();
    spell_ids.dedup();
    for spell_id in spell_ids {
        let Some(spell) = runtime
            .content
            .spells
            .spells
            .iter()
            .find(|spell| spell.id == spell_id)
        else {
            continue;
        };
        let school = school_lookup
            .get(spell.school.as_str())
            .copied()
            .unwrap_or(spell.school.as_str())
            .to_string();
        let (usable, reason) = spell_cast_status(runtime, actor, spell);
        entries.push(SpellEntry {
            id: spell.id.clone(),
            name: spell.name.clone(),
            school,
            tier: spell.tier,
            cost_type: spell.cost.r#type.clone(),
            cost_value: spell.cost.value,
            default_target: spell.default_target.clone(),
            allowed_targets: spell.allowed_targets.clone(),
            effect_type: spell.effect.r#type.clone(),
            effect_power: spell.effect.power,
            usable,
            reason,
        });
    }
    entries.sort_by(|left, right| {
        left.school
            .cmp(&right.school)
            .then(left.tier.cmp(&right.tier))
            .then(left.name.cmp(&right.name))
    });
    entries
}

pub fn selected_spell_targets(runtime: &GameRuntime) -> Vec<String> {
    let entries = build_spell_entries(runtime);
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Vec::new(),
    };
    match entries.get(selection) {
        Some(entry) => spell_targets_for_entry(runtime, entry, &actor_id),
        None => Vec::new(),
    }
}

pub fn spell_targets_for_entry(
    runtime: &GameRuntime,
    entry: &SpellEntry,
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

pub fn apply_spell_to_targets(
    runtime: &mut GameRuntime,
    entry: &SpellEntry,
    actor_id: &str,
    targets: &[String],
) -> bool {
    let magic_system = runtime.content.rules.game.magic_system.clone();
    let Some(actor) = runtime.party.roster.get(actor_id) else {
        return false;
    };
    if !spell_cost_available(
        magic_system.clone(),
        actor,
        entry.cost_type.as_str(),
        entry.tier,
        entry.cost_value,
    ) {
        return false;
    }
    let Some(actor) = runtime.party.roster.get_mut(actor_id) else {
        return false;
    };
    if !consume_spell_cost(magic_system, actor, entry) {
        return false;
    }
    for target_id in targets {
        apply_spell_to_actor(runtime, entry, target_id);
    }
    true
}

pub fn spell_cost_label(entry: &SpellEntry) -> String {
    match entry.cost_type.as_str() {
        "mp" => format!(" MP {}", entry.cost_value),
        "tier_charges" => format!(" T{} {}", entry.tier, entry.cost_value),
        _ => "".to_string(),
    }
}

pub fn spell_cost_available(
    magic_system: MagicSystem,
    actor: &engine::party::Actor,
    cost_type: &str,
    tier: u32,
    cost_value: i32,
) -> bool {
    if !spell_system_matches(magic_system.clone(), cost_type) {
        return false;
    }
    match magic_system {
        MagicSystem::Mp => actor.current_mp >= cost_value,
        MagicSystem::TierCharges => {
            actor.magic_tier_charges.get(&tier).copied().unwrap_or(0) >= cost_value
        }
    }
}

pub fn consume_spell_cost(
    magic_system: MagicSystem,
    actor: &mut engine::party::Actor,
    entry: &SpellEntry,
) -> bool {
    match magic_system {
        MagicSystem::Mp => {
            if actor.current_mp < entry.cost_value {
                return false;
            }
            actor.current_mp = actor.current_mp.saturating_sub(entry.cost_value);
            true
        }
        MagicSystem::TierCharges => {
            let charges = actor.magic_tier_charges.entry(entry.tier).or_insert(0);
            if *charges < entry.cost_value {
                return false;
            }
            *charges -= entry.cost_value;
            true
        }
    }
}

pub fn apply_spell_to_actor(runtime: &mut GameRuntime, entry: &SpellEntry, actor_id: &str) {
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

pub fn build_battle_spell_entries(runtime: &GameRuntime, actor_id: &str) -> Vec<SpellEntry> {
    let Some(actor) = runtime.party.roster.get(actor_id) else {
        return Vec::new();
    };
    let mut school_lookup = HashMap::new();
    for school in &runtime.content.spells.schools {
        school_lookup.insert(school.id.as_str(), school.name.as_str());
    }
    let mut spell_ids = collect_spell_ids(runtime, actor);
    spell_ids.sort();
    spell_ids.dedup();
    let mut entries = Vec::new();
    for spell_id in spell_ids {
        let Some(spell) = runtime
            .content
            .spells
            .spells
            .iter()
            .find(|spell| spell.id == spell_id)
        else {
            continue;
        };
        let school = school_lookup
            .get(spell.school.as_str())
            .copied()
            .unwrap_or(spell.school.as_str())
            .to_string();
        let (usable, reason) = spell_cast_status_battle(runtime, actor, spell);
        entries.push(SpellEntry {
            id: spell.id.clone(),
            name: spell.name.clone(),
            school,
            tier: spell.tier,
            cost_type: spell.cost.r#type.clone(),
            cost_value: spell.cost.value,
            default_target: spell.default_target.clone(),
            allowed_targets: spell.allowed_targets.clone(),
            effect_type: spell.effect.r#type.clone(),
            effect_power: spell.effect.power,
            usable,
            reason,
        });
    }
    entries.sort_by(|left, right| {
        left.school
            .cmp(&right.school)
            .then(left.tier.cmp(&right.tier))
            .then(left.name.cmp(&right.name))
    });
    entries
}

pub fn spell_cast_status_battle(
    runtime: &GameRuntime,
    actor: &engine::party::Actor,
    spell: &engine::entities::SpellDefinition,
) -> (bool, Option<String>) {
    if !spell_effect_allows_battle(spell.effect.r#type.as_str()) {
        return (false, Some("Unsupported effect.".to_string()));
    }
    let magic_system = runtime.content.rules.game.magic_system.clone();
    if !spell_system_matches(magic_system.clone(), spell.cost.r#type.as_str()) {
        return (false, Some("Cost system mismatch.".to_string()));
    }
    if !spell_cost_available(
        magic_system.clone(),
        actor,
        spell.cost.r#type.as_str(),
        spell.tier,
        spell.cost.value,
    ) {
        let reason = match magic_system {
            MagicSystem::Mp => "Not enough MP.",
            MagicSystem::TierCharges => "No tier charges.",
        };
        return (false, Some(reason.to_string()));
    }
    (true, None)
}

pub fn spell_system_matches(magic_system: MagicSystem, cost_type: &str) -> bool {
    match magic_system {
        MagicSystem::Mp => cost_type == "mp",
        MagicSystem::TierCharges => cost_type == "tier_charges",
    }
}

pub fn spell_effect_allows_field(effect: &str) -> bool {
    matches!(effect, "heal" | "revive")
}

pub fn spell_effect_allows_battle(effect: &str) -> bool {
    matches!(effect, "heal" | "revive" | "damage" | "scan")
}

fn magic_header_line(runtime: &GameRuntime, actor: &engine::party::Actor) -> MenuPanelLine {
    match runtime.content.rules.game.magic_system {
        MagicSystem::Mp => {
            let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
            panel_line_spans(vec![
                panel_span("Actor: ", PanelSpanStyle::Normal),
                panel_span(actor.name.clone(), PanelSpanStyle::Highlight),
                panel_span("  (Left/Right)", PanelSpanStyle::Muted),
                panel_span("  MP ", PanelSpanStyle::Normal),
                panel_span(
                    format!("{}/{}", actor.current_mp, max_mp),
                    PanelSpanStyle::Accent,
                ),
            ])
        }
        MagicSystem::TierCharges => {
            let mut tiers = Vec::new();
            for tier in &runtime.content.rules.magic_tiers {
                let current = actor
                    .magic_tier_charges
                    .get(&tier.tier)
                    .copied()
                    .unwrap_or(0);
                tiers.push(format!("T{} {}/{}", tier.tier, current, tier.max_charges));
            }
            let charge_text = if tiers.is_empty() {
                "No charges".to_string()
            } else {
                tiers.join("  ")
            };
            panel_line_spans(vec![
                panel_span("Actor: ", PanelSpanStyle::Normal),
                panel_span(actor.name.clone(), PanelSpanStyle::Highlight),
                panel_span("  (Left/Right)", PanelSpanStyle::Muted),
                panel_span("  ", PanelSpanStyle::Normal),
                panel_span(charge_text, PanelSpanStyle::Accent),
            ])
        }
    }
}

fn collect_spell_ids(runtime: &GameRuntime, actor: &engine::party::Actor) -> Vec<String> {
    let mut ids = Vec::new();
    ids.extend(actor.spells.clone());
    let job = runtime
        .content
        .jobs
        .jobs
        .iter()
        .find(|job| job.id == actor.job_id);
    if let Some(job) = job {
        for spell in &job.spells {
            if !job_spell_available(runtime, actor, spell) {
                continue;
            }
            if !ids.contains(&spell.id) {
                ids.push(spell.id.clone());
            }
        }
    }
    ids
}

fn job_spell_available(
    runtime: &GameRuntime,
    actor: &engine::party::Actor,
    spell: &engine::entities::JobSpell,
) -> bool {
    match spell.method.as_str() {
        "level" => spell.level.unwrap_or(0) <= actor.level,
        "tier" => spell.tier.unwrap_or(0) <= actor.level,
        "item" => match spell.item.as_deref() {
            Some(item_id) => runtime.inventory.item_qty(item_id) > 0,
            None => false,
        },
        _ => false,
    }
}

fn spell_cast_status(
    runtime: &GameRuntime,
    actor: &engine::party::Actor,
    spell: &engine::entities::SpellDefinition,
) -> (bool, Option<String>) {
    if spell.effect.r#type == "damage" {
        return (false, Some("Battle only.".to_string()));
    }
    if !spell_effect_allows_field(spell.effect.r#type.as_str()) {
        return (false, Some("Unsupported effect.".to_string()));
    }
    if !spell_target_allows_field(spell.default_target.as_str()) {
        return (false, Some("No field target.".to_string()));
    }
    let magic_system = runtime.content.rules.game.magic_system.clone();
    if !spell_system_matches(magic_system.clone(), spell.cost.r#type.as_str()) {
        return (false, Some("Cost system mismatch.".to_string()));
    }
    if !spell_cost_available(
        magic_system.clone(),
        actor,
        spell.cost.r#type.as_str(),
        spell.tier,
        spell.cost.value,
    ) {
        let reason = match magic_system {
            MagicSystem::Mp => "Not enough MP.",
            MagicSystem::TierCharges => "No tier charges.",
        };
        return (false, Some(reason.to_string()));
    }

    (true, None)
}

fn spell_target_allows_field(target: &str) -> bool {
    matches!(target, "self" | "ally" | "party")
}

fn spell_list_width(entries: &[SpellEntry]) -> usize {
    entries
        .iter()
        .map(|entry| entry.name.chars().count())
        .max()
        .unwrap_or(10)
        + 2
}

fn build_spell_line(entry: &SpellEntry, is_selected: bool, width: usize) -> MenuPanelLine {
    let prefix = if is_selected { "> " } else { "  " };
    let label = format!("{:width$}", entry.name, width = width);
    let cost_text = spell_cost_label(entry);
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

fn build_spell_target_panel(
    runtime: &GameRuntime,
    entry: Option<&SpellEntry>,
    actor_id: &str,
) -> Vec<MenuPanelLine> {
    let Some(entry) = entry else {
        return vec![panel_line("No target."), panel_line("")];
    };
    let targets = spell_targets_for_entry(runtime, entry, actor_id);
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

fn build_spell_description(
    runtime: &GameRuntime,
    entry: Option<&SpellEntry>,
    actor: &engine::party::Actor,
) -> Vec<MenuPanelLine> {
    let Some(entry) = entry else {
        return vec![panel_line("No selection.")];
    };
    let mut lines = Vec::new();
    lines.push(panel_line_spans(vec![
        panel_span("School: ", PanelSpanStyle::Normal),
        panel_span(entry.school.clone(), PanelSpanStyle::Accent),
        panel_span("  Tier: ", PanelSpanStyle::Normal),
        panel_span(entry.tier.to_string(), PanelSpanStyle::Accent),
    ]));
    lines.push(panel_line_spans(vec![
        panel_span("ID: ", PanelSpanStyle::Normal),
        panel_span(entry.id.clone(), PanelSpanStyle::Accent),
    ]));
    match runtime.content.rules.game.magic_system {
        MagicSystem::Mp => {
            let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
            lines.push(panel_line_spans(vec![
                panel_span("Cost: ", PanelSpanStyle::Normal),
                panel_span(format!("MP {}", entry.cost_value), PanelSpanStyle::Accent),
                panel_span("  MP: ", PanelSpanStyle::Normal),
                panel_span(
                    format!("{}/{}", actor.current_mp, max_mp),
                    PanelSpanStyle::Accent,
                ),
            ]));
        }
        MagicSystem::TierCharges => {
            let current = actor
                .magic_tier_charges
                .get(&entry.tier)
                .copied()
                .unwrap_or(0);
            let max = runtime
                .content
                .rules
                .magic_tiers
                .iter()
                .find(|tier| tier.tier == entry.tier)
                .map(|tier| tier.max_charges)
                .unwrap_or(0);
            lines.push(panel_line_spans(vec![
                panel_span("Cost: ", PanelSpanStyle::Normal),
                panel_span(
                    format!("T{} {}", entry.tier, entry.cost_value),
                    PanelSpanStyle::Accent,
                ),
                panel_span("  Charges: ", PanelSpanStyle::Normal),
                panel_span(format!("{}/{}", current, max), PanelSpanStyle::Accent),
            ]));
        }
    }
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
    if let Some(reason) = &entry.reason {
        lines.push(panel_line_spans(vec![
            panel_span("Unavailable: ", PanelSpanStyle::Muted),
            panel_span(reason.clone(), PanelSpanStyle::Muted),
        ]));
    }
    lines
}
