use std::collections::HashMap;

use engine::party::{actor_magic_tiers, exp_for_level, get_actor_max_charges, job_jp};
use engine::rules::{JobProgressionMode, MagicSystem};
use engine::runtime::GameRuntime;
use tui::menu::{MenuPanelLine, MenuPanelSpan, PanelSpanStyle};

pub fn build_status_panel(runtime: &GameRuntime, page: usize) -> Vec<MenuPanelLine> {
    if runtime.party.active.is_empty() {
        return vec![panel_line("No party members.")];
    }
    let mut lines = Vec::new();
    for member_id in &runtime.party.active {
        if let Some(actor) = runtime.party.roster.get(member_id) {
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
                let exp_next = exp_for_level(&runtime.content.rules.exp_curve, actor.level + 1)
                    .unwrap_or(actor.exp);
                let exp_remaining = exp_next.saturating_sub(actor.exp);
                let status_line = if magic_system == MagicSystem::TierCharges {
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
                lines.push(panel_line(format!("Job: {}", job_name)));
                if runtime.content.rules.job_system.progression_mode
                    == JobProgressionMode::JobPoints
                {
                    lines.push(panel_line(format!("JP {}", job_jp(actor, &actor.job_id))));
                }
                lines.push(panel_line(format!(
                    "EXP {} (next {})",
                    actor.exp, exp_remaining
                )));
            } else {
                lines.push(panel_line(format!("{}  Lv{}", actor.name, actor.level)));
                lines.push(panel_line(format!("Job: {}", job_name)));
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

fn panel_line(text: impl Into<String>) -> MenuPanelLine {
    MenuPanelLine {
        spans: vec![MenuPanelSpan {
            text: text.into(),
            style: PanelSpanStyle::Normal,
        }],
    }
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
