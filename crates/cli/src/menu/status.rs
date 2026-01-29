use std::collections::HashMap;

use engine::party::exp_for_level;
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
                let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
                let exp_next = exp_for_level(&runtime.content.rules.exp_curve, actor.level + 1)
                    .unwrap_or(actor.exp);
                let exp_remaining = exp_next.saturating_sub(actor.exp);
                lines.push(panel_line(format!(
                    "{}  Lv{}  HP {}/{}  MP {}/{}",
                    actor.name, actor.level, actor.current_hp, max_hp, actor.current_mp, max_mp
                )));
                lines.push(panel_line(format!("Job: {}", job_name)));
                lines.push(panel_line(format!(
                    "EXP {} (next {})",
                    actor.exp, exp_remaining
                )));
            } else {
                lines.push(panel_line(format!("{}  Lv{}", actor.name, actor.level)));
                lines.push(panel_line(format!("Job: {}", job_name)));
                lines.push(panel_line(format!(
                    "Base: {}",
                    format_stat_block(&runtime.content.stats.stats.base, &actor.base_stats)
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
