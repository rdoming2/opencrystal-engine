use engine::entities::JobDefinition;
use engine::party::{
    job_jp, job_level, set_primary_job, set_secondary_job, unlock_ability, unlock_spell,
};
use engine::rules::JobProgressionMode;
use engine::runtime::GameRuntime;
use tui::menu::{MenuPanelLine, MenuPanelSpan, MenuPanelView, PanelSpanStyle};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JobMenuOption {
    Primary,
    Secondary,
    Learn,
}

pub fn job_menu_options(runtime: &GameRuntime) -> Vec<JobMenuOption> {
    let mut options = vec![JobMenuOption::Primary];
    if runtime.content.rules.job_system.secondary_jobs {
        options.push(JobMenuOption::Secondary);
    }
    if runtime.content.rules.job_system.progression_mode == JobProgressionMode::JobPoints {
        options.push(JobMenuOption::Learn);
    }
    options
}

pub fn build_jobs_dashboard(runtime: &GameRuntime) -> MenuPanelView {
    let actor_id = runtime
        .party
        .active
        .get(runtime.menu_state.detail_actor)
        .cloned()
        .or_else(|| runtime.party.active.first().cloned());
    let actor = actor_id.and_then(|actor_id| runtime.party.roster.get(&actor_id));

    let mut lines = Vec::new();
    if let Some(actor) = actor {
        lines.push(panel_line(format!("Actor: {}", actor.name)));
        lines.push(panel_line(format!(
            "Progression: {}",
            match runtime.content.rules.job_system.progression_mode {
                JobProgressionMode::Character => "Character",
                JobProgressionMode::Job => "Job",
                JobProgressionMode::JobPoints => "Job Points",
            }
        )));
        if runtime.content.rules.job_system.progression_mode == JobProgressionMode::JobPoints {
            lines.push(panel_line(format!(
                "JP ({}): {}",
                actor.job_id,
                job_jp(actor, &actor.job_id)
            )));
        }
        lines.push(panel_line(""));

        let options = job_menu_options(runtime);
        let selected = runtime
            .menu_state
            .detail_slot
            .min(options.len().saturating_sub(1));
        for (index, option) in options.iter().enumerate() {
            let style = if index == selected {
                PanelSpanStyle::Highlight
            } else {
                PanelSpanStyle::Normal
            };
            let label = match option {
                JobMenuOption::Primary => format!(
                    "Primary: {} (Lv {})",
                    actor.job_id,
                    job_level(actor, &actor.job_id)
                ),
                JobMenuOption::Secondary => format!(
                    "Secondary: {}",
                    actor.secondary_job_id.as_deref().unwrap_or("None")
                ),
                JobMenuOption::Learn => "Learn Abilities".to_string(),
            };
            lines.push(MenuPanelLine {
                spans: vec![MenuPanelSpan { text: label, style }],
            });
        }
    } else {
        lines.push(panel_line("No active actor."));
    }

    MenuPanelView {
        title: "Jobs".to_string(),
        lines,
    }
}

pub fn build_job_picker(runtime: &GameRuntime) -> MenuPanelView {
    let actor_id = runtime
        .party
        .active
        .get(runtime.menu_state.detail_actor)
        .cloned()
        .or_else(|| runtime.party.active.first().cloned());
    let actor = actor_id.and_then(|actor_id| runtime.party.roster.get(&actor_id));

    let jobs = &runtime.content.jobs.jobs;
    let selection = runtime
        .menu_state
        .detail_selection
        .min(jobs.len().saturating_sub(1));
    let job = jobs.get(selection);

    let mut lines = Vec::new();
    if let Some(actor) = actor {
        lines.push(panel_line(format!("Actor: {}", actor.name)));
        let options = job_menu_options(runtime);
        let selected = runtime
            .menu_state
            .detail_slot
            .min(options.len().saturating_sub(1));
        let label = match options.get(selected) {
            Some(JobMenuOption::Primary) => "Selecting: Primary",
            Some(JobMenuOption::Secondary) => "Selecting: Secondary",
            Some(JobMenuOption::Learn) | None => "Selecting: Job",
        };
        lines.push(panel_line(label));
        lines.push(panel_line(""));
    } else {
        lines.push(panel_line("No active actor."));
        return MenuPanelView {
            title: "Jobs".to_string(),
            lines,
        };
    }

    if let Some(job) = job {
        let mut status = Vec::new();
        if let Some(actor) = actor {
            if actor.job_id == job.id {
                status.push("Primary");
            }
            if actor.secondary_job_id.as_deref() == Some(&job.id) {
                status.push("Secondary");
            }
        }
        if status.is_empty() {
            status.push("Selectable");
        }
        lines.push(panel_line(format!("{} [{}]", job.name, status.join(", "))));
        if let Some(description) = &job.description {
            lines.push(panel_line(description.clone()));
        }
        if !job.magic_schools.is_empty() {
            lines.push(panel_line(format!(
                "Magic schools: {}",
                job.magic_schools.join(", ")
            )));
        }
        let job_level_text = actor.map(|actor| job_level(actor, &job.id)).unwrap_or(1);
        let job_jp_text = actor.map(|actor| job_jp(actor, &job.id)).unwrap_or(0);
        if runtime.content.rules.job_system.progression_mode == JobProgressionMode::JobPoints {
            lines.push(panel_line(format!(
                "Job Lv: {}  JP: {}",
                job_level_text, job_jp_text
            )));
        } else {
            lines.push(panel_line(format!("Job Lv: {}", job_level_text)));
        }
    } else {
        lines.push(panel_line("No jobs defined."));
    }

    MenuPanelView {
        title: "Select Job".to_string(),
        lines,
    }
}

fn panel_line(content: impl Into<String>) -> MenuPanelLine {
    MenuPanelLine {
        spans: vec![MenuPanelSpan {
            text: content.into(),
            style: PanelSpanStyle::Normal,
        }],
    }
}

#[derive(Clone, Debug)]
struct LearnEntry {
    kind: &'static str,
    id: String,
    cost: i32,
    locked: bool,
    label: String,
}

fn build_learn_entries(actor: &engine::party::Actor, job: &JobDefinition) -> Vec<LearnEntry> {
    let mut entries = Vec::new();
    let current_level = job_level(actor, &job.id);

    for spell in &job.spells {
        let is_learned = actor.spells.iter().any(|s| s == &spell.id);
        if is_learned {
            continue;
        }
        let jp_cost = spell.jp_cost.unwrap_or(0);
        let unlock_level = spell.unlock_level.unwrap_or(0);
        let level_locked = unlock_level > 0 && current_level < unlock_level;

        if jp_cost > 0 {
            let label = if level_locked {
                format!("{} (Locked Lv {})", spell.id, unlock_level)
            } else {
                format!("{} (JP {})", spell.id, jp_cost)
            };
            entries.push(LearnEntry {
                kind: "spell",
                id: spell.id.clone(),
                cost: jp_cost,
                locked: level_locked,
                label,
            });
        } else if unlock_level > 0 {
            let label = if level_locked {
                format!("{} (Locked Lv {})", spell.id, unlock_level)
            } else {
                format!("{} (Lv {})", spell.id, unlock_level)
            };
            entries.push(LearnEntry {
                kind: "spell",
                id: spell.id.clone(),
                cost: 0,
                locked: level_locked,
                label,
            });
        }
    }

    for ability in &job.abilities {
        let is_learned = actor.unlocked_abilities.contains(&ability.id);
        if is_learned {
            continue;
        }
        let jp_cost = ability.jp_cost.unwrap_or(0);
        let unlock_level = ability.unlock_level.unwrap_or(0);
        let level_locked = unlock_level > 0 && current_level < unlock_level;

        if jp_cost > 0 {
            let label = if level_locked {
                format!("{} (Locked Lv {})", ability.id, unlock_level)
            } else {
                format!("{} (JP {})", ability.id, jp_cost)
            };
            entries.push(LearnEntry {
                kind: "ability",
                id: ability.id.clone(),
                cost: jp_cost,
                locked: level_locked,
                label,
            });
        } else if unlock_level > 0 {
            let label = if level_locked {
                format!("{} (Locked Lv {})", ability.id, unlock_level)
            } else {
                format!("{} (Lv {})", ability.id, unlock_level)
            };
            entries.push(LearnEntry {
                kind: "ability",
                id: ability.id.clone(),
                cost: 0,
                locked: level_locked,
                label,
            });
        }
    }

    entries
}

pub fn apply_primary_change(runtime: &mut GameRuntime) {
    if runtime.party.active.is_empty() {
        return;
    }
    let actor_id = runtime
        .party
        .active
        .get(runtime.menu_state.detail_actor)
        .cloned()
        .unwrap_or_else(|| runtime.party.active[0].clone());
    if let Some(actor) = runtime.party.roster.get_mut(&actor_id) {
        if let Some(job) = runtime
            .content
            .jobs
            .jobs
            .get(runtime.menu_state.detail_selection)
        {
            set_primary_job(actor, &job.id, &runtime.content);
        }
    }
}

pub fn apply_secondary_change(runtime: &mut GameRuntime) {
    if !runtime.content.rules.job_system.secondary_jobs {
        return;
    }
    if runtime.party.active.is_empty() {
        return;
    }
    let actor_id = runtime
        .party
        .active
        .get(runtime.menu_state.detail_actor)
        .cloned()
        .unwrap_or_else(|| runtime.party.active[0].clone());
    if let Some(actor) = runtime.party.roster.get_mut(&actor_id) {
        let job = runtime
            .content
            .jobs
            .jobs
            .get(runtime.menu_state.detail_selection);
        let job_id = job.map(|job| job.id.clone());
        set_secondary_job(actor, job_id);
    }
}

pub fn build_learn_panel(runtime: &GameRuntime) -> MenuPanelView {
    let actor_id = runtime
        .party
        .active
        .get(runtime.menu_state.detail_actor)
        .cloned()
        .or_else(|| runtime.party.active.first().cloned());
    let actor = actor_id.and_then(|actor_id| runtime.party.roster.get(&actor_id));

    let jobs = &runtime.content.jobs.jobs;
    let selection = runtime
        .menu_state
        .detail_selection
        .min(jobs.len().saturating_sub(1));
    let job = jobs.get(selection);

    let mut lines = Vec::new();
    if let Some(actor) = actor {
        lines.push(panel_line(format!("Actor: {}", actor.name)));
        lines.push(panel_line(format!(
            "JP ({}): {}",
            actor.job_id,
            job_jp(actor, &actor.job_id)
        )));
    } else {
        lines.push(panel_line("No active actor."));
    }
    lines.push(panel_line(""));

    if let Some(job) = job {
        lines.push(panel_line(format!("{} - Learn Abilities", job.name)));
        lines.push(panel_line(""));

        let learnables = actor
            .map(|actor| build_learn_entries(actor, job))
            .unwrap_or_default();

        if learnables.is_empty() {
            lines.push(panel_line("No abilities to learn."));
        } else {
            let learn_selection = runtime
                .menu_state
                .detail_target
                .min(learnables.len().saturating_sub(1));
            for (index, learnable) in learnables.iter().enumerate() {
                let prefix = if index == learn_selection { "> " } else { "  " };
                lines.push(panel_line(format!("{}{}", prefix, learnable.label)));
            }
        }

        lines.push(panel_line(""));
        lines.push(panel_line("Confirm: purchase selected ability."));
        lines.push(panel_line("Cancel: return to job list."));
    } else {
        lines.push(panel_line("No jobs defined."));
    }

    MenuPanelView {
        title: "Learn".to_string(),
        lines,
    }
}

pub fn learnable_count(runtime: &GameRuntime) -> usize {
    let actor_id = runtime
        .party
        .active
        .get(runtime.menu_state.detail_actor)
        .cloned()
        .or_else(|| runtime.party.active.first().cloned());
    let actor = actor_id.and_then(|actor_id| runtime.party.roster.get(&actor_id));

    let jobs = &runtime.content.jobs.jobs;
    let selection = runtime
        .menu_state
        .detail_selection
        .min(jobs.len().saturating_sub(1));
    let job = jobs.get(selection);

    let Some(actor) = actor else {
        return 0;
    };
    let Some(job) = job else {
        return 0;
    };

    build_learn_entries(actor, job).len()
}

pub fn apply_learn_purchase(runtime: &mut GameRuntime) {
    if runtime.party.active.is_empty() {
        return;
    }
    let actor_id = runtime
        .party
        .active
        .get(runtime.menu_state.detail_actor)
        .cloned()
        .unwrap_or_else(|| runtime.party.active[0].clone());
    let Some(actor) = runtime.party.roster.get_mut(&actor_id) else {
        return;
    };

    let jobs = &runtime.content.jobs.jobs;
    let selection = runtime
        .menu_state
        .detail_selection
        .min(jobs.len().saturating_sub(1));
    let Some(job) = jobs.get(selection) else {
        return;
    };

    let learnables = build_learn_entries(actor, job);

    let learn_selection = runtime
        .menu_state
        .detail_target
        .min(learnables.len().saturating_sub(1));

    if let Some(entry) = learnables.get(learn_selection) {
        if entry.locked {
            return;
        }
        let current_jp = job_jp(actor, &job.id);
        if entry.cost > 0 && current_jp < entry.cost {
            return; // Not enough JP
        }

        if entry.cost > 0 {
            let progress = actor
                .job_progress
                .entry(job.id.clone())
                .or_insert_with(engine::party::JobProgress::default);
            progress.jp = progress.jp.saturating_sub(entry.cost);
        }

        match entry.kind {
            "spell" => unlock_spell(actor, &entry.id),
            "ability" => unlock_ability(actor, &entry.id),
            _ => {}
        }
    }
}
