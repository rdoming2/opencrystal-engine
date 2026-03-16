use engine::quests::{QuestStatus, QuestStepState};
use engine::runtime::GameRuntime;
use tui::menu::{MenuPanelLine, MenuPanelSpan, PanelSpanStyle};

pub fn build_journal_panel(runtime: &GameRuntime, selected_index: usize) -> Vec<MenuPanelLine> {
    let mut lines = Vec::new();

    // Check if journal system is enabled
    let journal_enabled = runtime
        .content
        .rules
        .systems
        .get("journal")
        .copied()
        .unwrap_or(false);
    if !journal_enabled {
        return vec![panel_line("Journal system is disabled.")];
    }

    // Get all quest files
    if runtime.content.quests.is_empty() {
        return vec![panel_line("No quests defined.")];
    }

    // Resolve quests from all quest files
    let mut all_quest_states = Vec::new();
    for quest_file in &runtime.content.quests {
        let quest_states = quest_file.resolve_quests(&runtime.flags);
        all_quest_states.extend(quest_states);
    }

    if all_quest_states.is_empty() {
        return vec![panel_line("No active quests.")];
    }

    // Group quests by category
    let mut categories: std::collections::HashMap<String, Vec<&engine::quests::QuestState>> =
        std::collections::HashMap::new();
    for quest_state in &all_quest_states {
        let category_id = &quest_state.quest.category_id;
        categories
            .entry(category_id.clone())
            .or_default()
            .push(quest_state);
    }

    // Sort categories by sort_order
    let mut sorted_categories: Vec<_> = categories.into_iter().collect();
    sorted_categories.sort_by(|a, b| {
        let cat_a = find_category(&runtime.content.quests, &a.0);
        let cat_b = find_category(&runtime.content.quests, &b.0);
        let order_a = cat_a.map(|c| c.sort_order).unwrap_or(i32::MAX);
        let order_b = cat_b.map(|c| c.sort_order).unwrap_or(i32::MAX);
        order_a.cmp(&order_b)
    });

    // Render quests grouped by category
    let mut current_quest_idx = 0;
    for (category_id, quest_states) in sorted_categories {
        let category = find_category(&runtime.content.quests, &category_id);
        let category_label = category.map(|c| c.label.as_str()).unwrap_or("Unknown");

        lines.push(panel_line(format!("--- {} ---", category_label)));

        for quest_state in quest_states {
            let is_selected = current_quest_idx == selected_index;
            let status_text = match quest_state.status {
                QuestStatus::NotStarted => "[ ]",
                QuestStatus::InProgress => "[*]",
                QuestStatus::Complete => "[X]",
            };

            let prefix = if is_selected { "> " } else { "  " };
            let style = if is_selected {
                PanelSpanStyle::Highlight
            } else {
                PanelSpanStyle::Normal
            };

            lines.push(MenuPanelLine {
                spans: vec![MenuPanelSpan {
                    text: format!(
                        "{}{}{} {}",
                        prefix,
                        status_text,
                        if is_selected { "" } else { "" },
                        quest_state.quest.title
                    ),
                    style,
                    palette: None,
                }],
            });

            current_quest_idx += 1;
        }
        lines.push(panel_line(""));
    }

    lines
}

pub fn build_journal_detail_panel(
    runtime: &GameRuntime,
    quest_index: usize,
    page: usize,
) -> Vec<MenuPanelLine> {
    let mut lines = Vec::new();

    // Check if journal system is enabled
    let journal_enabled = runtime
        .content
        .rules
        .systems
        .get("journal")
        .copied()
        .unwrap_or(false);
    if !journal_enabled {
        return vec![panel_line("Journal system is disabled.")];
    }

    // Get all quest files
    if runtime.content.quests.is_empty() {
        return vec![panel_line("No quests defined.")];
    }

    // Resolve quests from all quest files
    let mut all_quest_states = Vec::new();
    for quest_file in &runtime.content.quests {
        let quest_states = quest_file.resolve_quests(&runtime.flags);
        all_quest_states.extend(quest_states);
    }

    if all_quest_states.is_empty() {
        return vec![panel_line("No active quests.")];
    }

    // Get the selected quest
    let quest_state = match all_quest_states.get(quest_index) {
        Some(state) => state,
        None => return vec![panel_line("Quest not found.")],
    };

    // Render quest details
    lines.push(panel_line(format!("Title: {}", quest_state.quest.title)));

    let status_text = match quest_state.status {
        QuestStatus::NotStarted => "Not Started",
        QuestStatus::InProgress => "In Progress",
        QuestStatus::Complete => "Complete",
    };
    lines.push(panel_line(format!("Status: {}", status_text)));
    lines.push(panel_line(""));

    if page == 0 {
        // Render steps
        lines.push(panel_line("--- Steps ---"));
        for step_state in &quest_state.steps {
            render_step(&mut lines, step_state, 0);
        }
    } else {
        // Render history
        lines.push(panel_line("--- History ---"));

        // Get history from all quest files
        let mut all_history = Vec::new();
        for quest_file in &runtime.content.quests {
            let history = quest_file.get_history(&runtime.flags);
            all_history.extend(history);
        }

        // Filter history for this quest
        let quest_history: Vec<_> = all_history
            .into_iter()
            .filter(|entry| entry.quest_id == quest_state.quest.id)
            .collect();

        if quest_history.is_empty() {
            lines.push(panel_line("No history yet."));
        } else {
            for entry in quest_history {
                lines.push(panel_line(format!("  - {}", entry.step_text)));
            }
        }
    }

    lines
}

fn render_step(lines: &mut Vec<MenuPanelLine>, step_state: &QuestStepState, indent: usize) {
    if !step_state.visible {
        return;
    }

    let indent_str = "  ".repeat(indent);
    let status_char = if step_state.complete { "X" } else { " " };
    lines.push(panel_line(format!(
        "{}[{}] {}",
        indent_str, status_char, step_state.step.text
    )));

    // Render substeps
    for substep_state in &step_state.substeps {
        render_step(lines, substep_state, indent + 1);
    }
}

fn find_category<'a>(
    quest_files: &'a [engine::quests::QuestsFile],
    category_id: &str,
) -> Option<&'a engine::quests::QuestCategory> {
    for quest_file in quest_files {
        if let Some(category) = quest_file
            .categories
            .iter()
            .find(|cat| cat.id == category_id)
        {
            return Some(category);
        }
    }
    None
}

fn panel_line(text: impl Into<String>) -> MenuPanelLine {
    MenuPanelLine {
        spans: vec![MenuPanelSpan {
            text: text.into(),
            style: PanelSpanStyle::Normal,
            palette: None,
        }],
    }
}
