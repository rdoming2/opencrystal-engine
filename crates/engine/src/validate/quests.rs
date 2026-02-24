use std::collections::HashSet;

use super::ValidationContext;

pub(crate) fn validate_quests(context: &ValidationContext, errors: &mut Vec<String>) {
    for quest_file in context.quests {
        let category_ids: HashSet<&str> = quest_file
            .categories
            .iter()
            .map(|cat| cat.id.as_str())
            .collect();

        let mut seen_categories = HashSet::new();
        for category in &quest_file.categories {
            if !seen_categories.insert(&category.id) {
                errors.push(format!("quests: duplicate category id '{}'", category.id));
            }
        }

        let mut seen_quests = HashSet::new();
        for quest in &quest_file.quests {
            if !seen_quests.insert(&quest.id) {
                errors.push(format!("quests: duplicate quest id '{}'", quest.id));
            }
        }

        for quest in &quest_file.quests {
            if !category_ids.contains(quest.category_id.as_str()) {
                errors.push(format!(
                    "quests: quest '{}' references unknown category '{}'",
                    quest.id, quest.category_id
                ));
            }
        }

        for quest in &quest_file.quests {
            let mut seen_step_ids: HashSet<String> = HashSet::new();
            for step in &quest.steps {
                validate_step_flags(errors, &quest.id, step, &mut seen_step_ids);
            }
        }
    }
}

fn validate_step_flags(
    errors: &mut Vec<String>,
    quest_id: &str,
    step: &crate::quests::QuestStep,
    seen_step_ids: &mut HashSet<String>,
) {
    if !step.flag.starts_with("quest.") {
        errors.push(format!(
            "quests: quest '{}' step '{}' has invalid flag format '{}', should be 'quest.<quest_id>.<step_id>'",
            quest_id, step.id, step.flag
        ));
    }

    if let Some(show_flag) = &step.show_flag {
        if show_flag.trim().is_empty() {
            errors.push(format!(
                "quests: quest '{}' step '{}' has empty show_flag",
                quest_id, step.id
            ));
        }
    }

    if !seen_step_ids.insert(step.id.clone()) {
        errors.push(format!(
            "quests: quest '{}' has duplicate step id '{}'",
            quest_id, step.id
        ));
    }

    for substep in &step.substeps {
        validate_step_flags(errors, quest_id, substep, seen_step_ids);
    }
}
