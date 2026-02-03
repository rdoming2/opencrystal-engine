use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QuestsFile {
    pub version: u32,
    pub categories: Vec<QuestCategory>,
    pub quests: Vec<Quest>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QuestCategory {
    pub id: String,
    pub label: String,
    pub sort_order: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Quest {
    pub id: String,
    pub title: String,
    pub category_id: String,
    pub steps: Vec<QuestStep>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QuestStep {
    pub id: String,
    pub text: String,
    pub flag: String,
    #[serde(default)]
    pub show_flag: Option<String>,
    #[serde(default)]
    pub substeps: Vec<QuestStep>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum QuestStatus {
    NotStarted,
    InProgress,
    Complete,
}

#[derive(Clone, Debug)]
pub struct QuestState {
    pub quest: Quest,
    pub status: QuestStatus,
    pub steps: Vec<QuestStepState>,
}

#[derive(Clone, Debug)]
pub struct QuestStepState {
    pub step: QuestStep,
    pub visible: bool,
    pub complete: bool,
    pub substeps: Vec<QuestStepState>,
}

#[derive(Clone, Debug)]
pub struct QuestHistoryEntry {
    pub quest_id: String,
    pub quest_title: String,
    pub step_id: String,
    pub step_text: String,
}

impl QuestsFile {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::load_json(path)
    }

    pub fn resolve_quests(&self, flags: &HashSet<String>) -> Vec<QuestState> {
        let mut quest_states = Vec::new();

        for quest in &self.quests {
            if let Some(state) = self.resolve_quest(quest, flags) {
                quest_states.push(state);
            }
        }

        quest_states
    }

    fn resolve_quest(&self, quest: &Quest, flags: &HashSet<String>) -> Option<QuestState> {
        let Some(first_step) = quest.steps.first() else {
            return None;
        };

        // Quest becomes visible only when the first step is set (quest acquired)
        if !flags.contains(&first_step.flag) {
            return None;
        }

        let step_states = self.resolve_step_list(&quest.steps, flags, true);
        let mut all_complete = true;
        let mut any_complete = false;

        for step_state in &step_states {
            if !step_state.complete {
                all_complete = false;
            }
            if step_state.complete {
                any_complete = true;
            }
        }

        let status = if all_complete {
            QuestStatus::Complete
        } else if any_complete {
            QuestStatus::InProgress
        } else {
            QuestStatus::InProgress
        };

        Some(QuestState {
            quest: quest.clone(),
            status,
            steps: step_states,
        })
    }

    fn resolve_step_list(
        &self,
        steps: &[QuestStep],
        flags: &HashSet<String>,
        parent_revealed: bool,
    ) -> Vec<QuestStepState> {
        let mut states = Vec::new();
        let mut previous_complete = false;

        for (index, step) in steps.iter().enumerate() {
            let can_reveal = if index == 0 {
                parent_revealed
            } else {
                parent_revealed && previous_complete
            };
            let step_state = self.resolve_step(step, flags, parent_revealed, can_reveal);
            previous_complete = step_state.complete;
            states.push(step_state);
        }

        states
    }

    fn resolve_step(
        &self,
        step: &QuestStep,
        flags: &HashSet<String>,
        parent_revealed: bool,
        can_reveal: bool,
    ) -> QuestStepState {
        let complete = flags.contains(&step.flag);
        let gate_revealed = step
            .show_flag
            .as_ref()
            .map(|flag| flags.contains(flag))
            .unwrap_or(false);
        let mut visible = complete
            || (parent_revealed
                && if step.show_flag.is_some() {
                    gate_revealed
                } else {
                    can_reveal
                });

        let substep_states = self.resolve_step_list(&step.substeps, flags, visible || complete);
        if !visible && substep_states.iter().any(|substep| substep.visible) {
            visible = true;
        }

        QuestStepState {
            step: step.clone(),
            visible,
            complete,
            substeps: substep_states,
        }
    }

    pub fn get_history(&self, flags: &HashSet<String>) -> Vec<QuestHistoryEntry> {
        let mut history = Vec::new();

        for quest in &self.quests {
            for step in &quest.steps {
                self.collect_history(&mut history, quest, step, flags);
            }
        }

        history
    }

    fn collect_history(
        &self,
        history: &mut Vec<QuestHistoryEntry>,
        quest: &Quest,
        step: &QuestStep,
        flags: &HashSet<String>,
    ) {
        if flags.contains(&step.flag) {
            history.push(QuestHistoryEntry {
                quest_id: quest.id.clone(),
                quest_title: quest.title.clone(),
                step_id: step.id.clone(),
                step_text: step.text.clone(),
            });
        }

        for substep in &step.substeps {
            self.collect_history(history, quest, substep, flags);
        }
    }
}
