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
        // Check if any step flag is set (quest is visible)
        let has_any_flag = quest.steps.iter().any(|step| flags.contains(&step.flag));
        if !has_any_flag {
            return None;
        }

        let mut step_states = Vec::new();
        let mut all_complete = true;
        let mut any_complete = false;

        for step in &quest.steps {
            let step_state = self.resolve_step(step, flags, true);
            if !step_state.complete {
                all_complete = false;
            }
            if step_state.complete {
                any_complete = true;
            }
            step_states.push(step_state);
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

    fn resolve_step(
        &self,
        step: &QuestStep,
        flags: &HashSet<String>,
        parent_visible: bool,
    ) -> QuestStepState {
        let complete = flags.contains(&step.flag);
        let visible = parent_visible || complete;

        let mut substep_states = Vec::new();
        for substep in &step.substeps {
            let substep_state = self.resolve_step(substep, flags, visible);
            substep_states.push(substep_state);
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
                if flags.contains(&step.flag) {
                    history.push(QuestHistoryEntry {
                        quest_id: quest.id.clone(),
                        quest_title: quest.title.clone(),
                        step_id: step.id.clone(),
                        step_text: step.text.clone(),
                    });
                }
                // Check substeps
                for substep in &step.substeps {
                    if flags.contains(&substep.flag) {
                        history.push(QuestHistoryEntry {
                            quest_id: quest.id.clone(),
                            quest_title: quest.title.clone(),
                            step_id: substep.id.clone(),
                            step_text: substep.text.clone(),
                        });
                    }
                }
            }
        }

        history
    }
}
