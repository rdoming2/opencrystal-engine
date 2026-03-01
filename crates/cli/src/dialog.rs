use engine::runtime::GameRuntime;
use tui::dialog::ChoiceView;
use tui::input::InputBindings;
use tui::overworld::{show_dialog_on_map, show_dialog_with_choices_on_map};
use tui::session::TuiSession;
use tui::ui::DialogUiFile;

use crate::shop::open_shop;

pub fn run_dialog(
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    bindings: &InputBindings,
    session: &mut TuiSession,
    dialog_id: &str,
) -> std::io::Result<()> {
    let dialog = match runtime.get_dialog(dialog_id).cloned() {
        Some(dialog) => dialog,
        None => {
            println!("Dialog not found: {}", dialog_id);
            return Ok(());
        }
    };

    let mut current = "start".to_string();
    loop {
        let node = dialog.nodes.iter().find(|node| node.id == current);
        let Some(node) = node else {
            println!("Dialog node not found: {}", current);
            break;
        };

        let speaker = node.speaker.as_deref().unwrap_or("");
        let filtered_choices = node.choices.as_ref().map(|choices| {
            choices
                .iter()
                .filter(|choice| dialog_choice_visible(runtime, choice))
                .collect::<Vec<_>>()
        });
        let choice_views = filtered_choices.as_ref().map(|choices| {
            choices
                .iter()
                .map(|choice| ChoiceView {
                    label: choice.label.clone(),
                    show_next: choice.next.as_str() != "end",
                })
                .collect::<Vec<_>>()
        });

        let selection = if let Some(choices) = &choice_views {
            if choices.is_empty() {
                tui::dialog::show_dialog(session, dialog_ui, bindings, speaker, &node.text)?;
                return Ok(());
            }
            tui::dialog::show_dialog_with_choices(
                session, dialog_ui, bindings, speaker, &node.text, choices,
            )?
        } else {
            tui::dialog::show_dialog(session, dialog_ui, bindings, speaker, &node.text)?;
            None
        };

        if let Some(actions) = &node.actions {
            for action in actions {
                if handle_dialog_action(runtime, session, bindings, action)? {
                    return Ok(());
                }
            }
        }

        if let (Some(selection), Some(choices)) = (selection, filtered_choices.as_ref()) {
            let next = choices
                .get(selection)
                .map(|choice| choice.next.clone())
                .unwrap_or_else(|| "end".to_string());
            if next == "end" {
                return Ok(());
            }
            current = next;
        } else {
            return Ok(());
        }
    }
    Ok(())
}

pub fn run_dialog_on_map(
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    bindings: &InputBindings,
    session: &mut TuiSession,
    dialog_id: &str,
    map: &tui::overworld::MapView,
    player_pos: (i32, i32),
) -> std::io::Result<()> {
    let dialog = match runtime.get_dialog(dialog_id).cloned() {
        Some(dialog) => dialog,
        None => {
            println!("Dialog not found: {}", dialog_id);
            return Ok(());
        }
    };

    let mut current = "start".to_string();
    loop {
        let node = dialog.nodes.iter().find(|node| node.id == current);
        let Some(node) = node else {
            println!("Dialog node not found: {}", current);
            break;
        };

        let speaker = node.speaker.as_deref().unwrap_or("");
        let filtered_choices = node.choices.as_ref().map(|choices| {
            choices
                .iter()
                .filter(|choice| dialog_choice_visible(runtime, choice))
                .collect::<Vec<_>>()
        });
        let choice_views = filtered_choices.as_ref().map(|choices| {
            choices
                .iter()
                .map(|choice| ChoiceView {
                    label: choice.label.clone(),
                    show_next: choice.next.as_str() != "end",
                })
                .collect::<Vec<_>>()
        });

        let selection = if let Some(choices) = &choice_views {
            if choices.is_empty() {
                show_dialog_on_map(
                    session, map, player_pos, dialog_ui, bindings, speaker, &node.text,
                )?;
                return Ok(());
            }
            show_dialog_with_choices_on_map(
                session, map, player_pos, dialog_ui, bindings, speaker, &node.text, choices,
            )?
        } else {
            show_dialog_on_map(
                session, map, player_pos, dialog_ui, bindings, speaker, &node.text,
            )?;
            None
        };

        if let Some(actions) = &node.actions {
            for action in actions {
                if handle_dialog_action(runtime, session, bindings, action)? {
                    return Ok(());
                }
            }
        }

        if let (Some(selection), Some(choices)) = (selection, filtered_choices.as_ref()) {
            let next = choices
                .get(selection)
                .map(|choice| choice.next.clone())
                .unwrap_or_else(|| "end".to_string());
            if next == "end" {
                return Ok(());
            }
            current = next;
        } else {
            return Ok(());
        }
    }
    Ok(())
}

pub fn run_dialog_console(runtime: &mut GameRuntime, dialog_ui: &DialogUiFile, dialog_id: &str) {
    let dialog = match runtime.get_dialog(dialog_id).cloned() {
        Some(dialog) => dialog,
        None => {
            println!("Dialog not found: {}", dialog_id);
            return;
        }
    };

    let mut current = "start".to_string();
    loop {
        let node = dialog.nodes.iter().find(|node| node.id == current);
        let Some(node) = node else {
            println!("Dialog node not found: {}", current);
            break;
        };

        show_dialog_console(dialog_ui, node.speaker.as_deref().unwrap_or(""), &node.text);
        if let Some(actions) = &node.actions {
            for action in actions {
                handle_dialog_action_console(runtime, action);
            }
        }

        if let Some(choices) = &node.choices {
            let filtered_choices = choices
                .iter()
                .filter(|choice| dialog_choice_visible(runtime, choice))
                .collect::<Vec<_>>();
            if filtered_choices.is_empty() {
                break;
            }
            for (index, choice) in filtered_choices.iter().enumerate() {
                println!("  {}. {}", index + 1, choice.label);
            }
            let selection = read_choice_console(filtered_choices.len());
            let next = filtered_choices
                .get(selection.saturating_sub(1))
                .map(|choice| choice.next.clone())
                .unwrap_or_else(|| "end".to_string());
            if next == "end" {
                break;
            }
            current = next;
        } else {
            break;
        }
    }
}

fn dialog_choice_visible(runtime: &GameRuntime, choice: &engine::dialog::DialogChoice) -> bool {
    choice.requires_flags.as_ref().map_or(true, |flags| {
        flags.iter().all(|flag| {
            if let Some(negated) = flag.strip_prefix('!') {
                !runtime.has_flag(negated)
            } else {
                runtime.has_flag(flag)
            }
        })
    })
}

pub fn show_dialog_console(dialog_ui: &DialogUiFile, speaker: &str, text: &str) {
    let width = 60usize;
    let height = dialog_ui.height.max(2) as usize;
    let mut lines = wrap_text_console(text, width);

    if dialog_ui.show_speaker && !speaker.is_empty() {
        println!("{}", speaker);
    }

    while !lines.is_empty() {
        let page: Vec<String> = lines.drain(0..lines.len().min(height - 1)).collect();
        for line in page {
            println!("{}", line);
        }
        if !lines.is_empty() {
            println!("{}", dialog_ui.continue_marker);
            let _ = read_line_console();
        }
    }
}

fn wrap_text_console(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
            continue;
        }
        if current.len() + word.len() + 1 > width {
            lines.push(current);
            current = word.to_string();
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn read_choice_console(max: usize) -> usize {
    loop {
        println!("Choose 1-{} and press Enter:", max);
        let input = read_line_console();
        if let Ok(value) = input.trim().parse::<usize>() {
            if value >= 1 && value <= max {
                return value;
            }
        }
    }
}

fn read_line_console() -> String {
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    input
}

pub fn default_dialog_ui() -> DialogUiFile {
    DialogUiFile {
        version: 1,
        position: "bottom".to_string(),
        height: 4,
        show_speaker: true,
        continue_marker: "▼".to_string(),
    }
}

fn handle_dialog_action(
    runtime: &mut GameRuntime,
    session: &mut TuiSession,
    bindings: &InputBindings,
    action: &engine::dialog::DialogAction,
) -> std::io::Result<bool> {
    let result = engine::dialog::apply_dialog_action(runtime, action);
    match result {
        engine::events::EventExecutionResult::OpenShop { shop_id } => {
            open_shop(runtime, session, bindings, &shop_id)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn handle_dialog_action_console(runtime: &mut GameRuntime, action: &engine::dialog::DialogAction) {
    let result = engine::dialog::apply_dialog_action(runtime, action);
    match result {
        engine::events::EventExecutionResult::OpenShop { shop_id } => {
            println!("Open shop: {}", shop_id);
        }
        _ => {}
    }
}
