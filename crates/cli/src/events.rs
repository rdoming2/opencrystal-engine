use engine::runtime::{GameRuntime, GameState};
use tui::app::{show_dialog, show_dialog_on_map, MapView, TuiSession};
use tui::input::InputBindings;
use tui::ui::{BattleUiFile, DialogUiFile};

use crate::battle::{run_event_battle_with_result, BattleOutcome};
use crate::dialog::{run_dialog, run_dialog_on_map, show_dialog_console};
use crate::overworld::build_map_view;
use crate::shop::open_shop;

pub fn run_event_loop(
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    battle_ui: &BattleUiFile,
    bindings: &InputBindings,
    session: &mut TuiSession,
    initial_map_view: Option<MapView>,
) -> std::io::Result<()> {
    let mut current_map_id = runtime.world.map_id.clone();
    let mut map_view = initial_map_view;

    while runtime.state == GameState::Event {
        // Check if map has changed (e.g., due to warp event)
        if runtime.world.map_id != current_map_id {
            current_map_id = runtime.world.map_id.clone();
            map_view = build_map_view(runtime, &current_map_id);
        }

        match runtime.next_event_step() {
            Some(step) => {
                let result = runtime.apply_event_step(&step);
                handle_event_result(
                    runtime,
                    dialog_ui,
                    battle_ui,
                    bindings,
                    session,
                    result,
                    map_view.as_ref(),
                )?
            }
            None => {}
        }
    }
    Ok(())
}

pub fn run_event_loop_console(runtime: &mut GameRuntime, dialog_ui: &DialogUiFile) {
    while runtime.state == GameState::Event {
        match runtime.next_event_step() {
            Some(step) => handle_event_step_console(runtime, dialog_ui, &step),
            None => {}
        }
    }
}

fn handle_event_result(
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    battle_ui: &BattleUiFile,
    bindings: &InputBindings,
    session: &mut TuiSession,
    result: engine::events::EventExecutionResult,
    map_view: Option<&MapView>,
) -> std::io::Result<()> {
    match result {
        engine::events::EventExecutionResult::Continue => {}
        engine::events::EventExecutionResult::Dialog { speaker, text } => {
            if let Some(map) = map_view {
                show_dialog_on_map(
                    session,
                    map,
                    runtime.world.position,
                    dialog_ui,
                    bindings,
                    &speaker,
                    &text,
                )?;
            } else {
                show_dialog(session, dialog_ui, bindings, &speaker, &text)?;
            }
        }
        engine::events::EventExecutionResult::Narration { text } => {
            if let Some(map) = map_view {
                show_dialog_on_map(
                    session,
                    map,
                    runtime.world.position,
                    dialog_ui,
                    bindings,
                    "",
                    &text,
                )?;
            } else {
                show_dialog(session, dialog_ui, bindings, "", &text)?;
            }
        }
        engine::events::EventExecutionResult::StartDialog { dialog_id } => {
            if let Some(map) = map_view {
                run_dialog_on_map(
                    runtime,
                    dialog_ui,
                    bindings,
                    session,
                    &dialog_id,
                    map,
                    runtime.world.position,
                )?;
            } else {
                run_dialog(runtime, dialog_ui, bindings, session, &dialog_id)?;
            }
        }
        engine::events::EventExecutionResult::StartBattle {
            encounter,
            formation,
        } => {
            let outcome = run_event_battle_with_result(
                runtime, battle_ui, bindings, session, &encounter, &formation,
            )?;
            if matches!(outcome, BattleOutcome::Defeat) {
                if let Some(map) = map_view {
                    show_dialog_on_map(
                        session,
                        map,
                        runtime.world.position,
                        dialog_ui,
                        bindings,
                        "",
                        "The party was defeated.",
                    )?;
                } else {
                    show_dialog(session, dialog_ui, bindings, "", "The party was defeated.")?;
                }
            }
        }
        engine::events::EventExecutionResult::OpenShop { shop_id } => {
            open_shop(runtime, session, bindings, &shop_id)?;
        }
        engine::events::EventExecutionResult::Completed => {}
        engine::events::EventExecutionResult::Abort => {
            println!("Event execution aborted.");
        }
    }
    Ok(())
}

fn handle_event_step_console(
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    step: &engine::events::EventStep,
) {
    match step.r#type.as_str() {
        "dialog" => {
            let speaker = step.speaker.as_deref().unwrap_or("Narrator");
            let text = step.text.as_deref().unwrap_or("");
            show_dialog_console(dialog_ui, speaker, text);
        }
        "narration" => {
            let text = step.text.as_deref().unwrap_or("");
            show_dialog_console(dialog_ui, "", text);
        }
        "start_dialog" => {
            if let Some(dialog) = &step.dialog {
                crate::dialog::run_dialog_console(runtime, dialog_ui, dialog);
            }
        }
        "set_flag" => {
            if let Some(flag) = &step.flag {
                runtime.set_flag(flag);
                println!("Set flag: {}", flag);
            }
        }
        "require_flags" => {
            if let Some(flags) = &step.flags {
                let missing = flags
                    .iter()
                    .filter(|flag| !runtime.has_flag(flag))
                    .cloned()
                    .collect::<Vec<_>>();
                if missing.is_empty() {
                    println!("Require flags met: {}", flags.join(", "));
                } else {
                    println!("Require flags missing: {}", missing.join(", "));
                }
            }
        }
        _ => {
            println!("Event step: {}", step.r#type);
        }
    }
}
