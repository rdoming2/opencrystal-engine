use engine::runtime::GameRuntime;
use tui::menu::{MenuPanelView, PanelSpanStyle};

use crate::menu::inventory::{panel_line, panel_line_spans, panel_span};

#[derive(Clone, Copy, Debug)]
enum SettingKind {
    Autosave,
    ReadinessSpeed,
    BattleMode,
}

#[derive(Clone, Debug)]
struct SettingsEntry {
    label: &'static str,
    value: String,
    locked: bool,
    kind: SettingKind,
}

pub fn build_settings_panel(runtime: &GameRuntime, selected: usize) -> MenuPanelView {
    let entries = settings_entries(runtime);
    if entries.is_empty() {
        return MenuPanelView {
            title: "Settings".to_string(),
            lines: vec![panel_line("No settings available.")],
        };
    }

    let mut lines = Vec::new();
    let mut has_locked = false;
    for (index, entry) in entries.iter().enumerate() {
        let is_selected = index == selected;
        let prefix = if is_selected { "> " } else { "  " };
        let label_style = if entry.locked {
            PanelSpanStyle::Muted
        } else if is_selected {
            PanelSpanStyle::Highlight
        } else {
            PanelSpanStyle::Normal
        };
        let value_style = if entry.locked {
            PanelSpanStyle::Muted
        } else {
            PanelSpanStyle::Accent
        };
        let mut spans = vec![
            panel_span(prefix, label_style),
            panel_span(format!("{:<18}", entry.label), label_style),
            panel_span(entry.value.clone(), value_style),
        ];
        if entry.locked {
            spans.push(panel_span("  [Locked]", PanelSpanStyle::Muted));
            has_locked = true;
        }
        lines.push(panel_line_spans(spans));
    }

    if has_locked {
        lines.push(panel_line_spans(vec![panel_span(
            "Locked by rules.json settings.",
            PanelSpanStyle::Muted,
        )]));
    }

    MenuPanelView {
        title: "Settings".to_string(),
        lines,
    }
}

pub fn settings_entry_count(runtime: &GameRuntime) -> usize {
    settings_entries(runtime).len()
}

pub fn apply_settings_confirm(runtime: &mut GameRuntime, selected: usize) {
    let entries = settings_entries(runtime);
    let Some(entry) = entries.get(selected) else {
        return;
    };
    if entry.locked {
        return;
    }
    match entry.kind {
        SettingKind::Autosave => {
            runtime.settings.autosave_enabled = !runtime.settings.autosave_enabled;
        }
        SettingKind::ReadinessSpeed | SettingKind::BattleMode => {}
    }
}

pub fn adjust_settings(runtime: &mut GameRuntime, selected: usize, direction: i32) {
    let entries = settings_entries(runtime);
    let Some(entry) = entries.get(selected) else {
        return;
    };
    if entry.locked {
        return;
    }
    match entry.kind {
        SettingKind::Autosave => {
            runtime.settings.autosave_enabled = direction >= 0;
        }
        SettingKind::ReadinessSpeed => {
            let setting = runtime.readiness_setting();
            let mut value = runtime.settings.readiness_speed + setting.step * direction as f32;
            if setting.step > 0.0 {
                value = (value / setting.step).round() * setting.step;
            }
            runtime.settings.readiness_speed = value.clamp(setting.min, setting.max);
        }
        SettingKind::BattleMode => {
            let setting = runtime.battle_mode_setting();
            let options = choice_options(&setting);
            if options.is_empty() {
                return;
            }
            let current = if options.contains(&runtime.settings.battle_mode) {
                runtime.settings.battle_mode.clone()
            } else {
                setting.value
            };
            let current_index = options
                .iter()
                .position(|option| option == &current)
                .unwrap_or(0);
            let len = options.len() as i32;
            let next_index = (current_index as i32 + direction).rem_euclid(len) as usize;
            runtime.settings.battle_mode = options[next_index].clone();
        }
    }
}

fn settings_entries(runtime: &GameRuntime) -> Vec<SettingsEntry> {
    let mut entries = Vec::new();

    let autosave_setting = runtime.autosave_setting();
    if autosave_setting.visible {
        let autosave_locked = !autosave_setting.editable;
        let autosave_value = if runtime.effective_autosave_enabled() {
            "On"
        } else {
            "Off"
        };
        entries.push(SettingsEntry {
            label: "Autosave",
            value: autosave_value.to_string(),
            locked: autosave_locked,
            kind: SettingKind::Autosave,
        });
    }

    let readiness_setting = runtime.readiness_setting();
    if readiness_setting.visible {
        let readiness_locked = !readiness_setting.editable;
        let readiness_value = format!("{:.1}", runtime.effective_readiness_speed());
        entries.push(SettingsEntry {
            label: "Readiness Speed",
            value: readiness_value,
            locked: readiness_locked,
            kind: SettingKind::ReadinessSpeed,
        });
    }

    let battle_setting = runtime.battle_mode_setting();
    if battle_setting.visible {
        let options = choice_options(&battle_setting);
        let battle_locked = !battle_setting.editable || options.len() <= 1;
        let battle_value = battle_mode_label(&runtime.effective_battle_mode());
        entries.push(SettingsEntry {
            label: "Battle Mode",
            value: battle_value.to_string(),
            locked: battle_locked,
            kind: SettingKind::BattleMode,
        });
    }

    entries
}

fn choice_options<T: Clone>(setting: &engine::rules::ChoiceSetting<T>) -> Vec<T> {
    if setting.options.is_empty() {
        vec![setting.value.clone()]
    } else {
        setting.options.clone()
    }
}

fn battle_mode_label(mode: &engine::rules::BattleMode) -> &'static str {
    match mode {
        engine::rules::BattleMode::Turn => "Turn",
        engine::rules::BattleMode::Dynamic => "Dynamic",
        engine::rules::BattleMode::DynamicWait => "Dynamic Wait",
    }
}
