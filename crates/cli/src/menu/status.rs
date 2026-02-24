use std::collections::HashMap;

use engine::party::{
    activity_proficiency, activity_rank_label, actor_magic_tiers, actor_row_label,
    actor_weapon_category, exp_for_level, get_actor_max_charges, job_jp, ActivityKind,
};
use engine::rules::{MagicSystem, ProgressionMode};
use engine::runtime::GameRuntime;
use tui::menu::{MenuPanelLine, MenuPanelSpan, PanelSpanStyle, StatusCard, StatusScreenView};

pub fn build_status_panel(runtime: &GameRuntime, page: usize) -> Vec<MenuPanelLine> {
    if runtime.party.active_count() == 0 {
        return vec![panel_line("No party members.")];
    }
    let mut lines = Vec::new();
    let rows_enabled = runtime.content.rules.battle.rows.enabled;
    for member_id in runtime.party.active_ids() {
        if let Some(actor) = runtime.party.roster.get(&member_id) {
            let job_name = runtime
                .content
                .jobs
                .jobs
                .iter()
                .find(|job| job.id == actor.job_id)
                .map(|job| job.name.as_str())
                .unwrap_or(actor.job_id.as_str());
            if page == 0 {
                let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
                let magic_system = runtime.content.rules.game.magic_system.clone();
                let status_labels = actor
                    .statuses
                    .iter()
                    .filter_map(|status| {
                        engine::battle::status_definition(&runtime.content, &status.id)
                            .map(|definition| definition.label.clone())
                    })
                    .collect::<Vec<_>>();
                let trait_labels = engine::party::actor_traits(&runtime.content, actor)
                    .iter()
                    .filter_map(|trait_id| engine::battle::trait_label(&runtime.content, trait_id))
                    .collect::<Vec<_>>();
                let status_line = if runtime.content.rules.progression_mode
                    == ProgressionMode::Activity
                {
                    if magic_system == MagicSystem::TierCharges {
                        let mut tiers = Vec::new();
                        for tier in actor_magic_tiers(&runtime.content, actor) {
                            let current = actor.magic_tier_charges.get(&tier).copied().unwrap_or(0);
                            let max = get_actor_max_charges(&runtime.content, actor, tier);
                            tiers.push(format!("T{} {}/{}", tier, current, max));
                        }
                        let charge_text = if tiers.is_empty() {
                            "No charges".to_string()
                        } else {
                            tiers.join("  ")
                        };
                        format!(
                            "{}  HP {}/{}  {}",
                            actor.name, actor.current_hp, max_hp, charge_text
                        )
                    } else {
                        let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
                        format!(
                            "{}  HP {}/{}  MP {}/{}",
                            actor.name, actor.current_hp, max_hp, actor.current_mp, max_mp
                        )
                    }
                } else if magic_system == MagicSystem::TierCharges {
                    let mut tiers = Vec::new();
                    for tier in actor_magic_tiers(&runtime.content, actor) {
                        let current = actor.magic_tier_charges.get(&tier).copied().unwrap_or(0);
                        let max = get_actor_max_charges(&runtime.content, actor, tier);
                        tiers.push(format!("T{} {}/{}", tier, current, max));
                    }
                    let charge_text = if tiers.is_empty() {
                        "No charges".to_string()
                    } else {
                        tiers.join("  ")
                    };
                    format!(
                        "{}  Lv{}  HP {}/{}  {}",
                        actor.name, actor.level, actor.current_hp, max_hp, charge_text
                    )
                } else {
                    let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
                    format!(
                        "{}  Lv{}  HP {}/{}  MP {}/{}",
                        actor.name, actor.level, actor.current_hp, max_hp, actor.current_mp, max_mp
                    )
                };
                lines.push(panel_line(status_line));
                if runtime.content.rules.progression_mode != ProgressionMode::Activity {
                    lines.push(panel_line(format!("Job: {}", job_name)));
                }
                if rows_enabled {
                    lines.push(panel_line(format!("Row: {}", actor_row_label(actor))));
                }
                if runtime.content.rules.progression_mode == ProgressionMode::JobPoints {
                    lines.push(panel_line(format!("JP {}", job_jp(actor, &actor.job_id))));
                }
                if runtime.content.rules.progression_mode != ProgressionMode::Activity {
                    let (exp_label, current_exp, exp_curve) =
                        match &runtime.content.rules.progression_mode {
                            ProgressionMode::Job => {
                                let job_exp = actor
                                    .job_progress
                                    .get(actor.job_id.as_str())
                                    .map(|progress| progress.exp)
                                    .unwrap_or(0);
                                (
                                    "Job EXP",
                                    job_exp,
                                    &runtime.content.rules.job_system.job_exp_curve,
                                )
                            }
                            ProgressionMode::Character | ProgressionMode::JobPoints => {
                                ("EXP", actor.exp, &runtime.content.rules.exp_curve)
                            }
                            ProgressionMode::Activity => {
                                ("EXP", 0, &runtime.content.rules.exp_curve)
                            }
                        };
                    let exp_next = exp_for_level(exp_curve, actor.level + 1).unwrap_or(current_exp);
                    let exp_remaining = exp_next.saturating_sub(current_exp);
                    lines.push(panel_line(format!(
                        "{} {} (next {})",
                        exp_label, current_exp, exp_remaining
                    )));
                } else {
                    let activity_rules = &runtime.content.rules.activity_progression;
                    let weapon_category = actor_weapon_category(
                        &runtime.content,
                        actor,
                        activity_rules.unarmed_category.as_str(),
                    );
                    if let Some(category) = weapon_category {
                        let prof = activity_proficiency(actor, ActivityKind::Weapon, &category);
                        let rank = activity_rank_label(activity_rules, prof).unwrap_or("Unranked");
                        lines.push(panel_line(format!(
                            "Weapon: {} {:.2} ({})",
                            category, prof, rank
                        )));
                    } else {
                        lines.push(panel_line("Weapon: None"));
                    }
                    let mut schools = actor
                        .spells
                        .iter()
                        .filter_map(|spell_id| {
                            runtime
                                .content
                                .spells
                                .spells
                                .iter()
                                .find(|spell| spell.id == *spell_id)
                                .map(|spell| spell.school.clone())
                        })
                        .collect::<Vec<_>>();
                    schools.sort();
                    schools.dedup();
                    if schools.is_empty() {
                        lines.push(panel_line("Magic: None"));
                    } else {
                        for school in schools {
                            let prof = activity_proficiency(actor, ActivityKind::Magic, &school);
                            let rank =
                                activity_rank_label(activity_rules, prof).unwrap_or("Unranked");
                            lines.push(panel_line(format!(
                                "Magic: {} {:.2} ({})",
                                school, prof, rank
                            )));
                        }
                    }
                }
                if status_labels.is_empty() {
                    lines.push(panel_line("Status: None"));
                } else {
                    lines.push(panel_line(format!("Status: {}", status_labels.join(", "))));
                }
                if trait_labels.is_empty() {
                    lines.push(panel_line("Traits: None"));
                } else {
                    lines.push(panel_line(format!("Traits: {}", trait_labels.join(", "))));
                }
            } else {
                if runtime.content.rules.progression_mode == ProgressionMode::Activity {
                    lines.push(panel_line(actor.name.clone()));
                } else {
                    lines.push(panel_line(format!("{}  Lv{}", actor.name, actor.level)));
                    lines.push(panel_line(format!("Job: {}", job_name)));
                }
                if rows_enabled {
                    lines.push(panel_line(format!("Row: {}", actor_row_label(actor))));
                }
                let base_entries =
                    if runtime.content.rules.game.magic_system == MagicSystem::TierCharges {
                        runtime
                            .content
                            .stats
                            .stats
                            .base
                            .iter()
                            .filter(|entry| entry.id != "mp")
                            .cloned()
                            .collect::<Vec<_>>()
                    } else {
                        runtime.content.stats.stats.base.clone()
                    };
                lines.push(panel_line(format!(
                    "Base: {}",
                    format_stat_block(&base_entries, &actor.base_stats)
                )));
                lines.push(panel_line(format!(
                    "Derived: {}",
                    format_stat_block(&runtime.content.stats.stats.derived, &actor.derived_stats)
                )));
            }
            lines.push(panel_line(""));
        }
    }
    lines
}

pub fn build_status_screen_view(runtime: &GameRuntime, actor_index: usize) -> StatusScreenView {
    let title = "Status".to_string();
    let active_ids = runtime.party.active_ids();
    if active_ids.is_empty() {
        return StatusScreenView {
            title,
            cards: vec![StatusCard {
                title: "Party".to_string(),
                lines: vec![panel_line("No party members.")],
            }],
        };
    }

    let index = actor_index.min(active_ids.len().saturating_sub(1));
    let Some(actor_id) = active_ids.get(index) else {
        return StatusScreenView {
            title,
            cards: vec![StatusCard {
                title: "Party".to_string(),
                lines: vec![panel_line("No party members.")],
            }],
        };
    };
    let Some(actor) = runtime.party.roster.get(actor_id) else {
        return StatusScreenView {
            title,
            cards: vec![StatusCard {
                title: "Party".to_string(),
                lines: vec![panel_line("No party members.")],
            }],
        };
    };

    let magic_system = runtime.content.rules.game.magic_system.clone();
    let rows_enabled = runtime.content.rules.battle.rows.enabled;
    let progression_mode = runtime.content.rules.progression_mode.clone();
    let job_label = job_name(runtime, actor.job_id.as_str());
    let secondary_job = actor
        .secondary_job_id
        .as_ref()
        .map(|id| job_name(runtime, id.as_str()));

    let mut cards = Vec::new();

    let mut overview_lines = Vec::new();
    overview_lines.push(panel_line_with_style(
        actor.name.clone(),
        PanelSpanStyle::Highlight,
    ));
    if progression_mode != ProgressionMode::Activity {
        overview_lines.push(panel_line(format!("Lv {}", actor.level)));
    }
    overview_lines.push(panel_line(format!("Job: {}", job_label)));
    if let Some(secondary) = secondary_job {
        overview_lines.push(panel_line(format!("Secondary: {}", secondary)));
    }
    if rows_enabled {
        overview_lines.push(panel_line(format!("Row: {}", actor_row_label(actor))));
    }
    cards.push(StatusCard {
        title: "Overview".to_string(),
        lines: overview_lines,
    });

    let equipment_lines = build_equipment_lines(runtime, actor);
    if !equipment_lines.is_empty() {
        cards.push(StatusCard {
            title: "Equipment".to_string(),
            lines: equipment_lines,
        });
    }

    let mut vitals_lines = Vec::new();
    let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
    vitals_lines.push(panel_line(format!("HP: {}/{}", actor.current_hp, max_hp)));
    if magic_system == MagicSystem::TierCharges {
        let tiers = actor_magic_tiers(&runtime.content, actor);
        if tiers.is_empty() {
            vitals_lines.push(panel_line("Charges: None"));
        } else {
            for tier in tiers {
                let current = actor.magic_tier_charges.get(&tier).copied().unwrap_or(0);
                let max = get_actor_max_charges(&runtime.content, actor, tier);
                vitals_lines.push(panel_line(format!("T{}: {}/{}", tier, current, max)));
            }
        }
    } else {
        let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
        vitals_lines.push(panel_line(format!("MP: {}/{}", actor.current_mp, max_mp)));
    }
    cards.push(StatusCard {
        title: "Vitals".to_string(),
        lines: vitals_lines,
    });

    if progression_mode == ProgressionMode::Activity {
        let mut prof_lines = Vec::new();
        let activity_rules = &runtime.content.rules.activity_progression;
        let weapon_category = actor_weapon_category(
            &runtime.content,
            actor,
            activity_rules.unarmed_category.as_str(),
        );
        if let Some(category) = weapon_category {
            let prof = activity_proficiency(actor, ActivityKind::Weapon, &category);
            let rank = activity_rank_label(activity_rules, prof).unwrap_or("Unranked");
            prof_lines.push(panel_line(format!(
                "Weapon: {} {:.2} ({})",
                category, prof, rank
            )));
        } else {
            prof_lines.push(panel_line("Weapon: None"));
        }

        let mut schools = actor
            .spells
            .iter()
            .filter_map(|spell_id| {
                runtime
                    .content
                    .spells
                    .spells
                    .iter()
                    .find(|spell| spell.id == *spell_id)
                    .map(|spell| spell.school.clone())
            })
            .collect::<Vec<_>>();
        schools.sort();
        schools.dedup();
        if schools.is_empty() {
            prof_lines.push(panel_line("Magic: None"));
        } else {
            for school in schools {
                let prof = activity_proficiency(actor, ActivityKind::Magic, &school);
                let rank = activity_rank_label(activity_rules, prof).unwrap_or("Unranked");
                prof_lines.push(panel_line(format!(
                    "Magic: {} {:.2} ({})",
                    school, prof, rank
                )));
            }
        }

        cards.push(StatusCard {
            title: "Proficiencies".to_string(),
            lines: prof_lines,
        });
    } else {
        let mut progress_lines = Vec::new();
        if progression_mode == ProgressionMode::JobPoints {
            progress_lines.push(panel_line(format!("JP: {}", job_jp(actor, &actor.job_id))));
        }
        let (exp_label, current_exp, exp_curve) = match &progression_mode {
            ProgressionMode::Job => {
                let job_exp = actor
                    .job_progress
                    .get(actor.job_id.as_str())
                    .map(|progress| progress.exp)
                    .unwrap_or(0);
                (
                    "Job EXP",
                    job_exp,
                    &runtime.content.rules.job_system.job_exp_curve,
                )
            }
            ProgressionMode::Character | ProgressionMode::JobPoints => {
                ("EXP", actor.exp, &runtime.content.rules.exp_curve)
            }
            ProgressionMode::Activity => ("EXP", 0, &runtime.content.rules.exp_curve),
        };
        let exp_next = exp_for_level(exp_curve, actor.level + 1).unwrap_or(current_exp);
        let exp_remaining = exp_next.saturating_sub(current_exp);
        progress_lines.push(panel_line(format!("{}: {}", exp_label, current_exp)));
        progress_lines.push(panel_line(format!("To next: {}", exp_remaining)));
        cards.push(StatusCard {
            title: "Progress".to_string(),
            lines: progress_lines,
        });
    }

    let status_labels = actor
        .statuses
        .iter()
        .filter_map(|status| {
            engine::battle::status_definition(&runtime.content, &status.id)
                .map(|definition| definition.label.clone())
        })
        .collect::<Vec<_>>();
    let trait_labels = engine::party::actor_traits(&runtime.content, actor)
        .iter()
        .filter_map(|trait_id| engine::battle::trait_label(&runtime.content, trait_id))
        .collect::<Vec<_>>();
    let status_text = if status_labels.is_empty() {
        "Status: None".to_string()
    } else {
        format!("Status: {}", status_labels.join(", "))
    };
    let trait_text = if trait_labels.is_empty() {
        "Traits: None".to_string()
    } else {
        format!("Traits: {}", trait_labels.join(", "))
    };
    cards.push(StatusCard {
        title: "Conditions".to_string(),
        lines: vec![panel_line(status_text), panel_line(trait_text)],
    });

    let base_entries = if magic_system == MagicSystem::TierCharges {
        runtime
            .content
            .stats
            .stats
            .base
            .iter()
            .filter(|entry| entry.id != "mp")
            .cloned()
            .collect::<Vec<_>>()
    } else {
        runtime.content.stats.stats.base.clone()
    };
    cards.push(StatusCard {
        title: "Base Stats".to_string(),
        lines: format_stat_lines(&base_entries, &actor.base_stats, 2),
    });
    cards.push(StatusCard {
        title: "Derived Stats".to_string(),
        lines: format_stat_lines(
            &runtime.content.stats.stats.derived,
            &actor.derived_stats,
            2,
        ),
    });

    let sprite_lines = actor_sprite_lines(runtime, actor);
    if !sprite_lines.is_empty() {
        let lines = sprite_lines
            .into_iter()
            .map(|line| panel_line_with_style(line, PanelSpanStyle::Accent))
            .collect::<Vec<_>>();
        cards.push(StatusCard {
            title: "Sprite".to_string(),
            lines,
        });
    }

    StatusScreenView { title, cards }
}

fn panel_line(text: impl Into<String>) -> MenuPanelLine {
    MenuPanelLine {
        spans: vec![MenuPanelSpan {
            text: text.into(),
            style: PanelSpanStyle::Normal,
        }],
    }
}

fn panel_line_with_style(text: impl Into<String>, style: PanelSpanStyle) -> MenuPanelLine {
    MenuPanelLine {
        spans: vec![MenuPanelSpan {
            text: text.into(),
            style,
        }],
    }
}

fn panel_line_spans(spans: Vec<MenuPanelSpan>) -> MenuPanelLine {
    MenuPanelLine { spans }
}

fn panel_span(text: impl Into<String>, style: PanelSpanStyle) -> MenuPanelSpan {
    MenuPanelSpan {
        text: text.into(),
        style,
    }
}

fn format_stat_lines(
    entries: &[engine::stats::StatEntry],
    stats: &HashMap<String, i32>,
    per_line: usize,
) -> Vec<MenuPanelLine> {
    if entries.is_empty() {
        return Vec::new();
    }
    let per_line = per_line.max(1);
    let mut lines = Vec::new();
    let mut buffer = Vec::new();
    for entry in entries {
        let value = stats.get(&entry.id).copied().unwrap_or(0);
        buffer.push(format!("{} {}", entry.name, value));
        if buffer.len() >= per_line {
            lines.push(panel_line(buffer.join("  ")));
            buffer.clear();
        }
    }
    if !buffer.is_empty() {
        lines.push(panel_line(buffer.join("  ")));
    }
    lines
}

fn job_name(runtime: &GameRuntime, job_id: &str) -> String {
    runtime
        .content
        .jobs
        .jobs
        .iter()
        .find(|job| job.id == job_id)
        .map(|job| job.name.clone())
        .unwrap_or_else(|| job_id.to_string())
}

fn actor_sprite_lines(runtime: &GameRuntime, actor: &engine::party::Actor) -> Vec<String> {
    let job = runtime
        .content
        .jobs
        .jobs
        .iter()
        .find(|job| job.id == actor.job_id);
    if let Some(job) = job {
        if let Some(art) = &job.art {
            if !art.lines.is_empty() {
                return art.lines.clone();
            }
        }
        if let Some(glyph) = job.sprite.glyph.chars().next() {
            return vec![glyph.to_string()];
        }
    }
    vec!["@".to_string()]
}

fn build_equipment_lines(
    runtime: &GameRuntime,
    actor: &engine::party::Actor,
) -> Vec<MenuPanelLine> {
    let slots = actor_equipment_slots(runtime, actor);
    if slots.is_empty() {
        return vec![panel_line("No equipment slots.")];
    }

    let mut lines = Vec::new();
    for slot in slots {
        let slot_label = format_slot_label(&slot);
        let equipped = actor.equipment.get(&slot).and_then(|item_id| {
            runtime
                .content
                .equipment
                .equipment
                .iter()
                .find(|item| item.id == *item_id)
                .map(|item| item.name.clone())
                .or_else(|| Some("Unknown".to_string()))
        });
        let (item_text, style) = match equipped {
            Some(item_name) => (item_name, PanelSpanStyle::Accent),
            None => ("Empty".to_string(), PanelSpanStyle::Muted),
        };
        lines.push(panel_line_spans(vec![
            panel_span(format!("{}: ", slot_label), PanelSpanStyle::Normal),
            panel_span(item_text, style),
        ]));
    }
    lines
}

fn actor_equipment_slots(runtime: &GameRuntime, actor: &engine::party::Actor) -> Vec<String> {
    let Some(job) = runtime
        .content
        .jobs
        .jobs
        .iter()
        .find(|job| job.id == actor.job_id)
    else {
        return Vec::new();
    };

    let mut slots = if job.equipment_slots.is_empty() {
        let mut fallback = Vec::new();
        if !job.equipment.weapons.is_empty() {
            fallback.push("weapon".to_string());
        }
        if !job.equipment.armor.is_empty() {
            fallback.push("armor".to_string());
        }
        fallback
    } else {
        job.equipment_slots.clone()
    };

    for index in 1..=job.accessory_slots {
        slots.push(format!("accessory_{}", index));
    }

    if let Some(progression) = &job.magic_equip_progression {
        let mut max_slots = 0;
        for (req_level, count) in &progression.slots {
            if actor.level >= *req_level {
                max_slots = max_slots.max(*count);
            }
        }
        for index in 1..=max_slots {
            slots.push(format!("magic_{}", index));
        }
    }

    slots
}

fn format_slot_label(slot: &str) -> String {
    if slot == "weapon" {
        return "Weapon".to_string();
    }
    if slot == "armor" {
        return "Armor".to_string();
    }
    if let Some(index) = slot.strip_prefix("accessory_") {
        return format!("Accessory {}", index);
    }
    if let Some(index) = slot.strip_prefix("magic_") {
        return format!("Magic {}", index);
    }
    slot.to_string()
}

fn format_stat_block(entries: &[engine::stats::StatEntry], stats: &HashMap<String, i32>) -> String {
    entries
        .iter()
        .map(|entry| {
            let value = stats.get(&entry.id).copied().unwrap_or(0);
            format!("{} {}", entry.name, value)
        })
        .collect::<Vec<_>>()
        .join("  ")
}
