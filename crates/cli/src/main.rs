mod battle;
mod build;
mod dialog;
mod events;
mod menu;
mod overworld;
mod party;
mod shop;
mod utils;

use std::env;
use std::fs;
use std::path::PathBuf;

use engine::{
    content::Content,
    party::PartyState,
    rules::{PartyMode, RulesFile, Ruleset},
    runtime::{GameRuntime, GameState},
    save::SaveFile,
    world::WorldState,
    Engine,
};
use tui::input::{InputBindings, InputFile};
use tui::menu::{run_content_menu, ContentMenuEntry};
use tui::renderer::RenderMode;
use tui::session::TuiSession;
use tui::title::{
    run_gameover, run_load_menu, run_title, GameOverAction, GameOverOptions, LoadSlotEntry,
    TitleAction,
};
use tui::ui::{BattleUiFile, DialogUiFile, MenuUiFile, ProgressUiFile, TitleUiFile};

use crate::battle::{run_battle, BattleOutcome, BattleSource, LastBattleContext};
use crate::dialog::default_dialog_ui;
use crate::events::{run_event_loop, run_event_loop_console, EventLoopOutcome};
use crate::overworld::{
    build_map_view, find_spawn, record_death_marker, run_overworld_loop, OverworldOutcome,
};
use crate::party::{default_party_names, run_party_create_flow};

struct SessionGuard(Option<TuiSession>);

enum SessionExit {
    ReturnTitle,
    Exit,
}

struct DefeatCarryover {
    map_id: String,
    pos: (i32, i32),
}

impl SessionGuard {
    fn start() -> Self {
        let session = match TuiSession::start() {
            Ok(session) => Some(session),
            Err(err) => {
                eprintln!("Failed to start TUI: {}", err);
                None
            }
        };
        Self(session)
    }

    fn as_mut(&mut self) -> Option<&mut TuiSession> {
        self.0.as_mut()
    }
}

impl Drop for SessionGuard {
    fn drop(&mut self) {
        if let Some(session) = self.0.take() {
            if let Err(err) = session.finish() {
                eprintln!("Failed to close TUI: {}", err);
            }
        }
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let command = args.next();
    match command.as_deref() {
        Some("play") => run_play(args.collect()),
        Some("validate") => run_validate(),
        Some("new-project") => run_new_project(args.collect()),
        Some("build") => run_build(args.collect()),
        _ => print_usage(),
    }
}

fn run_play(args: Vec<String>) {
    let render_mode = parse_render_mode(&args).unwrap_or(RenderMode::Auto);
    let mut session_guard = SessionGuard::start();
    let content_dir_arg = parse_content_dir(&args);
    let content_base_arg = parse_content_base_dir(&args);
    let content_dir = match content_dir_arg {
        Some(dir) => dir,
        None => {
            let base_dir = content_base_arg.unwrap_or_else(|| {
                let base_dir = default_content_base_dir();
                if content_dir_arg.is_none() {
                    if let Err(err) = fs::create_dir_all(&base_dir) {
                        eprintln!(
                            "Failed to create default content directory {}: {}",
                            base_dir.display(),
                            err
                        );
                    }
                }
                base_dir
            });
            if base_dir.join("rules.json").exists() {
                base_dir
            } else if let Some(session) = session_guard.as_mut() {
                let bindings = InputBindings::default_bindings();
                match choose_content_dir(session, &bindings, &base_dir) {
                    Some(dir) => dir,
                    None => return,
                }
            } else {
                base_dir.join("demo")
            }
        }
    };
    let input_path = content_dir.join("input.json");
    let title_ui_path = content_dir.join("ui").join("title.json");
    let menu_ui_path = content_dir.join("ui").join("menu.json");
    let battle_ui_path = content_dir.join("ui").join("battle.json");
    let dialog_ui_path = content_dir.join("ui").join("dialog.json");
    let progress_ui_path = content_dir.join("ui").join("gameplay_stats.json");

    let content = match Content::load(&content_dir) {
        Ok(content) => content,
        Err(errors) => {
            for error in errors {
                eprintln!("Content error: {}", error);
            }
            return;
        }
    };

    let rules = Ruleset::from_file(content.rules.clone());
    let world = WorldState::new(
        &rules.start_location.world,
        &rules.start_location.map,
        (rules.start_location.x, rules.start_location.y),
    );

    let input_bindings = match InputFile::load(&input_path) {
        Ok(file) => match InputBindings::from_file(file) {
            Ok(bindings) => bindings,
            Err(err) => {
                eprintln!("Failed to parse input bindings: {}", err);
                InputBindings::default_bindings()
            }
        },
        Err(err) => {
            eprintln!("Failed to load input bindings: {}", err);
            InputBindings::default_bindings()
        }
    };

    let title_ui = match TitleUiFile::load(&title_ui_path) {
        Ok(title_ui) => title_ui,
        Err(err) => {
            eprintln!("Failed to load title UI: {}", err);
            return;
        }
    };

    let menu_ui = match MenuUiFile::load(&menu_ui_path) {
        Ok(menu_ui) => menu_ui,
        Err(err) => {
            eprintln!("Failed to load menu UI: {}", err);
            return;
        }
    };

    let battle_ui = match BattleUiFile::load(&battle_ui_path) {
        Ok(battle_ui) => battle_ui,
        Err(err) => {
            eprintln!("Failed to load battle UI: {}", err);
            return;
        }
    };

    let dialog_ui = match DialogUiFile::load(&dialog_ui_path) {
        Ok(dialog_ui) => dialog_ui,
        Err(err) => {
            eprintln!("Failed to load dialog UI: {}", err);
            default_dialog_ui()
        }
    };

    let progress_ui = match ProgressUiFile::load(&progress_ui_path) {
        Ok(progress_ui) => progress_ui,
        Err(err) => {
            eprintln!("Failed to load gameplay stats UI: {}", err);
            ProgressUiFile {
                version: 1,
                panels: Vec::new(),
            }
        }
    };

    let _engine = Engine::new(rules.clone(), world.clone());
    let mut runtime = GameRuntime::new(content);
    let save_dir = default_save_dir(&content_dir);
    let mut pending_carryover: Option<DefeatCarryover> = None;

    if session_guard.as_mut().is_none() {
        match render_mode {
            RenderMode::Auto => println!("Starting OpenCrystal (render: auto)..."),
            RenderMode::Wide => println!("Starting OpenCrystal (render: wide)..."),
            RenderMode::Modern => println!("Starting OpenCrystal (render: modern)..."),
        }
    }

    let mut running = true;
    while running {
        let action = if let Some(session) = session_guard.as_mut() {
            if let Err(err) = session.terminal_mut().clear() {
                eprintln!("Failed to clear TUI: {}", err);
            }
            let load_enabled = has_loadable_saves(&runtime, &save_dir);
            let default_selected = if load_enabled {
                menu_index(&title_ui, "load_game").or_else(|| menu_index(&title_ui, "new_game"))
            } else {
                menu_index(&title_ui, "new_game")
            }
            .unwrap_or(0);
            match run_title(
                session,
                &title_ui,
                &input_bindings,
                load_enabled,
                default_selected,
            ) {
                Ok(action) => action,
                Err(err) => {
                    eprintln!("Failed to run title UI: {}", err);
                    TitleAction::Exit
                }
            }
        } else {
            TitleAction::NewGame
        };

        match action {
            TitleAction::NewGame => {
                pending_carryover = None;
                if let Some(session) = session_guard.as_mut() {
                    match rules.party_mode {
                        PartyMode::Create => {
                            if let Err(err) = run_party_create_flow(
                                session,
                                &mut runtime,
                                &rules,
                                &input_bindings,
                            ) {
                                if err.kind() == std::io::ErrorKind::Interrupted {
                                    return;
                                }
                            }
                        }
                        PartyMode::Preset => {
                            runtime.party = PartyState::from_content(&runtime.content, &rules);
                        }
                        PartyMode::PresetRename => {
                            runtime.party = PartyState::from_content(&runtime.content, &rules);
                            if let Err(err) = party::run_preset_rename_flow(
                                session,
                                &mut runtime,
                                &rules,
                                &input_bindings,
                            ) {
                                if err.kind() == std::io::ErrorKind::Interrupted {
                                    return;
                                }
                            }
                        }
                    }

                    runtime.start_new_game(&rules);
                    match run_session_with_gameover(
                        session,
                        &mut runtime,
                        &dialog_ui,
                        &battle_ui,
                        &menu_ui,
                        &progress_ui,
                        &input_bindings,
                        &title_ui,
                        &save_dir,
                        &mut pending_carryover,
                    ) {
                        Ok(SessionExit::ReturnTitle) => {}
                        Ok(SessionExit::Exit) => return,
                        Err(err) => {
                            if err.kind() == std::io::ErrorKind::Interrupted {
                                return;
                            }
                        }
                    }
                } else {
                    match rules.party_mode {
                        PartyMode::Create => {
                            runtime.party = PartyState::from_created(
                                &runtime.content,
                                &rules,
                                default_party_names(&runtime, &rules),
                            );
                        }
                        PartyMode::Preset | PartyMode::PresetRename => {
                            runtime.party = PartyState::from_content(&runtime.content, &rules);
                        }
                    }
                    runtime.start_new_game(&rules);
                    run_event_loop_console(&mut runtime, &dialog_ui);
                    running = false;
                }
            }
            TitleAction::Load => {
                if let Some(session) = session_guard.as_mut() {
                    match run_load_flow(
                        session,
                        &mut runtime,
                        &title_ui,
                        &input_bindings,
                        &save_dir,
                    ) {
                        Ok(true) => {
                            if let Some(carryover) = pending_carryover.take() {
                                apply_defeat_carryover(&mut runtime, &carryover);
                            }
                            match run_session_with_gameover(
                                session,
                                &mut runtime,
                                &dialog_ui,
                                &battle_ui,
                                &menu_ui,
                                &progress_ui,
                                &input_bindings,
                                &title_ui,
                                &save_dir,
                                &mut pending_carryover,
                            ) {
                                Ok(SessionExit::ReturnTitle) => {}
                                Ok(SessionExit::Exit) => return,
                                Err(err) => {
                                    if err.kind() == std::io::ErrorKind::Interrupted {
                                        return;
                                    }
                                }
                            }
                        }
                        Ok(false) => {}
                        Err(err) => {
                            eprintln!("Failed to load save: {}", err);
                        }
                    }
                } else {
                    println!("Load not implemented.");
                }
            }
            TitleAction::Settings => println!("Settings not implemented."),
            TitleAction::Exit => running = false,
        }
    }
}

fn run_validate() {
    let args: Vec<String> = env::args().skip(2).collect();
    let content_dir_arg = parse_content_dir(&args);
    let content_base_arg = parse_content_base_dir(&args);
    let base_dir = content_base_arg.clone().unwrap_or_else(|| {
        let base_dir = default_content_base_dir();
        if content_dir_arg.is_none() && content_base_arg.is_none() {
            if let Err(err) = fs::create_dir_all(&base_dir) {
                eprintln!(
                    "Failed to create default content directory {}: {}",
                    base_dir.display(),
                    err
                );
            }
        }
        base_dir
    });
    let content_dir = content_dir_arg.unwrap_or_else(|| {
        if base_dir.join("rules.json").exists() {
            base_dir.clone()
        } else {
            base_dir.join("demo")
        }
    });
    let mut errors = engine::validate::validate_content(&content_dir);

    let input_path = content_dir.join("input.json");
    let title_ui_path = content_dir.join("ui").join("title.json");
    let menu_ui_path = content_dir.join("ui").join("menu.json");
    let battle_ui_path = content_dir.join("ui").join("battle.json");
    let dialog_ui_path = content_dir.join("ui").join("dialog.json");
    let progress_ui_path = content_dir.join("ui").join("gameplay_stats.json");

    if let Err(err) = InputFile::load(&input_path) {
        errors.push(format!("input.json: {}", err));
    }
    if let Err(err) = TitleUiFile::load(&title_ui_path) {
        errors.push(format!("ui/title.json: {}", err));
    }
    if let Err(err) = MenuUiFile::load(&menu_ui_path) {
        errors.push(format!("ui/menu.json: {}", err));
    }
    if let Err(err) = BattleUiFile::load(&battle_ui_path) {
        errors.push(format!("ui/battle.json: {}", err));
    }
    if let Err(err) = DialogUiFile::load(&dialog_ui_path) {
        errors.push(format!("ui/dialog.json: {}", err));
    }
    if let Err(err) = ProgressUiFile::load(&progress_ui_path) {
        errors.push(format!("ui/gameplay_stats.json: {}", err));
    }

    if errors.is_empty() {
        println!("Content validation passed.");
    } else {
        eprintln!("Content validation failed ({} errors):", errors.len());
        for error in errors {
            eprintln!("- {}", error);
        }
    }
}

fn run_new_project(args: Vec<String>) {
    let mut forwarded = vec!["new-project".to_string()];
    forwarded.extend(args);
    build::run_build(forwarded);
}

fn run_build(args: Vec<String>) {
    build::run_build(args);
}

fn choose_content_dir(
    session: &mut TuiSession,
    bindings: &InputBindings,
    base_dir: &PathBuf,
) -> Option<PathBuf> {
    let entries = discover_content_entries(base_dir);
    match run_content_menu(session, bindings, &entries) {
        Ok(Some(index)) => entries
            .get(index)
            .filter(|entry| entry.enabled)
            .map(|entry| PathBuf::from(entry.path.clone())),
        Ok(None) => None,
        Err(err) => {
            eprintln!("Failed to run content chooser: {}", err);
            None
        }
    }
}

fn discover_content_entries(base_dir: &PathBuf) -> Vec<ContentMenuEntry> {
    let mut entries = Vec::new();
    let Ok(dir_entries) = fs::read_dir(base_dir) else {
        entries.push(ContentMenuEntry {
            label: "No content found".to_string(),
            title: "No content found".to_string(),
            description: None,
            author: None,
            path: base_dir.display().to_string(),
            enabled: false,
            error: Some(format!("{} not found", base_dir.display())),
        });
        return entries;
    };

    for entry in dir_entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let label = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("content")
            .to_string();
        let rules_path = path.join("rules.json");
        if !rules_path.exists() {
            entries.push(ContentMenuEntry {
                label: label.clone(),
                title: "Missing rules.json".to_string(),
                description: None,
                author: None,
                path: path.display().to_string(),
                enabled: false,
                error: Some("rules.json not found".to_string()),
            });
            continue;
        }
        match RulesFile::load(&rules_path) {
            Ok(rules) => entries.push(ContentMenuEntry {
                label,
                title: rules.game.title,
                description: rules
                    .game
                    .description
                    .filter(|text| !text.trim().is_empty()),
                author: rules.game.author.filter(|text| !text.trim().is_empty()),
                path: path.display().to_string(),
                enabled: true,
                error: None,
            }),
            Err(err) => entries.push(ContentMenuEntry {
                label,
                title: "Invalid rules.json".to_string(),
                description: None,
                author: None,
                path: path.display().to_string(),
                enabled: false,
                error: Some(err),
            }),
        }
    }

    if entries.is_empty() {
        entries.push(ContentMenuEntry {
            label: "No content found".to_string(),
            title: "No content found".to_string(),
            description: None,
            author: None,
            path: base_dir.display().to_string(),
            enabled: false,
            error: Some("No subdirectories under content/".to_string()),
        });
        return entries;
    }

    entries.sort_by(|a, b| {
        a.label
            .to_ascii_lowercase()
            .cmp(&b.label.to_ascii_lowercase())
    });
    entries
}

fn parse_render_mode(args: &[String]) -> Option<RenderMode> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(value) = arg.strip_prefix("--render=") {
            return RenderMode::from_arg(value);
        }
        if arg == "--render" {
            return iter.next().and_then(|value| RenderMode::from_arg(value));
        }
    }
    None
}

fn parse_content_dir(args: &[String]) -> Option<PathBuf> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(value) = arg.strip_prefix("--content=") {
            return Some(PathBuf::from(value));
        }
        if arg == "--content" {
            return iter.next().map(PathBuf::from);
        }
    }
    None
}

fn parse_content_base_dir(args: &[String]) -> Option<PathBuf> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if let Some(value) = arg.strip_prefix("--content-dir=") {
            return Some(PathBuf::from(value));
        }
        if arg == "--content-dir" {
            return iter.next().map(PathBuf::from);
        }
    }
    None
}

fn default_content_base_dir() -> PathBuf {
    if let Ok(path) = env::var("XDG_DATA_HOME") {
        return PathBuf::from(path).join("opencrystal").join("content");
    }
    if let Ok(path) = env::var("LOCALAPPDATA") {
        return PathBuf::from(path).join("opencrystal").join("content");
    }
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("opencrystal")
            .join("content");
    }
    PathBuf::from("content")
}

fn default_save_dir(content_dir: &PathBuf) -> PathBuf {
    let base_dir = if let Ok(path) = env::var("XDG_DATA_HOME") {
        PathBuf::from(path)
    } else if let Ok(path) = env::var("LOCALAPPDATA") {
        PathBuf::from(path)
    } else if let Ok(home) = env::var("HOME") {
        PathBuf::from(home).join(".local").join("share")
    } else {
        PathBuf::from(".")
    };
    let content_slug = content_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(slugify)
        .unwrap_or_else(|| "content".to_string());
    base_dir
        .join("opencrystal")
        .join("saves")
        .join(content_slug)
}

fn run_load_flow(
    session: &mut TuiSession,
    runtime: &mut GameRuntime,
    title_ui: &TitleUiFile,
    bindings: &InputBindings,
    save_dir: &PathBuf,
) -> Result<bool, String> {
    let slots = build_load_slots(runtime, save_dir);
    let selection =
        run_load_menu(session, title_ui, bindings, &slots).map_err(|err| err.to_string())?;
    let Some(index) = selection else {
        return Ok(false);
    };
    let entry = slots
        .get(index)
        .filter(|entry| entry.enabled)
        .ok_or_else(|| "Selected slot is empty".to_string())?;
    load_save_slot(runtime, save_dir, entry.slot)?;
    Ok(true)
}

fn defeat_carryover_from_context(context: &LastBattleContext) -> DefeatCarryover {
    DefeatCarryover {
        map_id: context.defeat_map_id.clone(),
        pos: context.defeat_pos,
    }
}

fn apply_defeat_carryover(runtime: &mut GameRuntime, carryover: &DefeatCarryover) {
    record_death_marker(runtime, &carryover.map_id, carryover.pos);
    runtime.add_stat("player_deaths", 1);
}

fn run_session_with_gameover(
    session: &mut TuiSession,
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    battle_ui: &BattleUiFile,
    menu_ui: &MenuUiFile,
    progress_ui: &ProgressUiFile,
    bindings: &InputBindings,
    title_ui: &TitleUiFile,
    save_dir: &PathBuf,
    pending_carryover: &mut Option<DefeatCarryover>,
) -> std::io::Result<SessionExit> {
    let mut pending_defeat: Option<LastBattleContext> = None;

    loop {
        if let Some(context) = pending_defeat.take() {
            let latest_slot = latest_manual_save_slot(runtime, save_dir);
            let autosave_available =
                runtime.effective_autosave_enabled() && loadable_save_exists(save_dir, 0);
            let options = GameOverOptions {
                retry_enabled: true,
                load_latest_enabled: latest_slot.is_some(),
                load_autosave_enabled: autosave_available,
            };
            let localized_title_ui = localize_gameover_ui(runtime, title_ui);
            let action = run_gameover(session, &localized_title_ui, bindings, options)?;
            match action {
                GameOverAction::RetryBattle => {
                    apply_battle_snapshot(runtime, &context.snapshot);
                    apply_defeat_carryover(runtime, &defeat_carryover_from_context(&context));
                    match context.source {
                        BattleSource::Random => {
                            let mut rng = rand::thread_rng();
                            let outcome = run_battle(
                                runtime,
                                battle_ui,
                                bindings,
                                session,
                                &context.formation,
                                &mut rng,
                            )?;
                            if matches!(outcome, BattleOutcome::Defeat) {
                                pending_defeat = Some(context);
                            }
                        }
                        BattleSource::Event {
                            event_id,
                            event_step,
                        } => {
                            runtime.state = GameState::Event;
                            runtime.active_event = Some(event_id.clone());
                            runtime.event_step = event_step;
                            if runtime
                                .event_queue
                                .first()
                                .map(|id| id != &event_id)
                                .unwrap_or(true)
                            {
                                runtime.event_queue.insert(0, event_id.clone());
                            }
                            let initial_map_view = build_map_view(runtime, &runtime.world.map_id);
                            let outcome = run_event_loop(
                                runtime,
                                dialog_ui,
                                battle_ui,
                                bindings,
                                session,
                                initial_map_view,
                            )?;
                            if let EventLoopOutcome::Defeat(context) = outcome {
                                pending_defeat = Some(context);
                            }
                        }
                    }
                }
                GameOverAction::LoadLatest => {
                    if let Some(slot) = latest_slot {
                        if let Err(err) = load_save_slot(runtime, save_dir, slot) {
                            eprintln!("Failed to load save: {}", err);
                            pending_defeat = Some(context);
                        } else {
                            apply_defeat_carryover(
                                runtime,
                                &defeat_carryover_from_context(&context),
                            );
                        }
                    } else {
                        pending_defeat = Some(context);
                    }
                }
                GameOverAction::LoadAutosave => {
                    if autosave_available {
                        if let Err(err) = load_save_slot(runtime, save_dir, 0) {
                            eprintln!("Failed to load autosave: {}", err);
                            pending_defeat = Some(context);
                        } else {
                            apply_defeat_carryover(
                                runtime,
                                &defeat_carryover_from_context(&context),
                            );
                        }
                    } else {
                        pending_defeat = Some(context);
                    }
                }
                GameOverAction::ReturnTitle => {
                    *pending_carryover = Some(defeat_carryover_from_context(&context));
                    return Ok(SessionExit::ReturnTitle);
                }
                GameOverAction::Exit => return Ok(SessionExit::Exit),
            }
            continue;
        }

        if runtime.state == GameState::Event {
            let initial_map_view = build_map_view(runtime, &runtime.world.map_id);
            let outcome = run_event_loop(
                runtime,
                dialog_ui,
                battle_ui,
                bindings,
                session,
                initial_map_view,
            )?;
            if let EventLoopOutcome::Defeat(context) = outcome {
                pending_defeat = Some(context);
                continue;
            }
        }

        let map_id = runtime.world.map_id.clone();
        let spawn = find_spawn(runtime, &map_id, runtime.world.position);
        match run_overworld_loop(
            session,
            runtime,
            dialog_ui,
            battle_ui,
            menu_ui,
            progress_ui,
            bindings,
            &map_id,
            spawn,
            save_dir,
        )? {
            OverworldOutcome::Defeat(context) => {
                pending_defeat = Some(context);
            }
            OverworldOutcome::Quit => return Ok(SessionExit::Exit),
            OverworldOutcome::ReturnTitle => return Ok(SessionExit::ReturnTitle),
            OverworldOutcome::Continue => {}
        }
    }
}

fn load_save_slot(runtime: &mut GameRuntime, save_dir: &PathBuf, slot: u8) -> Result<(), String> {
    let save = SaveFile::load(save_slot_path(save_dir, slot))?;
    save.apply_to_runtime(runtime);
    if slot > 0 {
        runtime.last_manual_save_slot = Some(slot);
    }
    runtime.state = GameState::Overworld;
    runtime.event_queue.clear();
    runtime.active_event = None;
    runtime.event_step = 0;
    Ok(())
}

fn apply_battle_snapshot(runtime: &mut GameRuntime, snapshot: &crate::battle::BattleSnapshot) {
    snapshot.save.apply_to_runtime(runtime);
    runtime.event_queue = snapshot.event_queue.clone();
    runtime.active_event = snapshot.active_event.clone();
    runtime.event_step = snapshot.event_step;
    runtime.state = snapshot.state.clone();
}

fn localize_gameover_ui(runtime: &GameRuntime, title_ui: &TitleUiFile) -> TitleUiFile {
    let mut localized = title_ui.clone();
    let Some(gameover) = localized.gameover.as_mut() else {
        return localized;
    };

    let fallback_title = gameover
        .title
        .clone()
        .unwrap_or_else(|| "Game Over".to_string());
    let title = runtime
        .content
        .ui_text("gameover.title")
        .unwrap_or(fallback_title.as_str())
        .to_string();
    gameover.title = Some(title);

    if let Some(subtitle) = runtime.content.ui_text("gameover.subtitle") {
        gameover.subtitle = Some(subtitle.to_string());
    }

    for item in &mut gameover.menu {
        if let Some(key) = gameover_menu_key(item.id.as_str()) {
            if let Some(label) = runtime.content.ui_text(key) {
                item.label = label.to_string();
            }
        }
    }

    localized
}

fn gameover_menu_key(id: &str) -> Option<&'static str> {
    match id {
        "retry_battle" => Some("gameover.retry_battle"),
        "load_latest" => Some("gameover.load_latest"),
        "load_autosave" => Some("gameover.load_autosave"),
        "return_title" => Some("gameover.return_title"),
        "exit" => Some("gameover.exit"),
        _ => None,
    }
}

fn latest_manual_save_slot(runtime: &GameRuntime, save_dir: &PathBuf) -> Option<u8> {
    let max_slots = runtime.content.rules.save.slots_max.max(1) as u8;
    let mut latest_slot: Option<(u8, u64)> = None;
    for slot in 1..=max_slots {
        let Ok(save) = SaveFile::load(save_slot_path(save_dir, slot)) else {
            continue;
        };
        if save.version == 0 {
            continue;
        }
        let timestamp = save.metadata.timestamp_seconds;
        if latest_slot
            .map(|(_, latest_timestamp)| timestamp > latest_timestamp)
            .unwrap_or(true)
        {
            latest_slot = Some((slot, timestamp));
        }
    }
    latest_slot.map(|(slot, _)| slot)
}

fn menu_index(title_ui: &TitleUiFile, id: &str) -> Option<usize> {
    title_ui.menu.iter().position(|item| item.id == id)
}

fn has_loadable_saves(runtime: &GameRuntime, save_dir: &PathBuf) -> bool {
    if runtime.effective_autosave_enabled() && loadable_save_exists(save_dir, 0) {
        return true;
    }
    let max_slots = runtime.content.rules.save.slots_max.max(1) as u8;
    for slot in 1..=max_slots {
        if loadable_save_exists(save_dir, slot) {
            return true;
        }
    }
    false
}

fn loadable_save_exists(save_dir: &PathBuf, slot: u8) -> bool {
    SaveFile::load(save_slot_path(save_dir, slot))
        .map(|save| save.version != 0)
        .unwrap_or(false)
}

fn build_load_slots(runtime: &GameRuntime, save_dir: &PathBuf) -> Vec<LoadSlotEntry> {
    let mut slots = Vec::new();
    if runtime.effective_autosave_enabled() {
        if let Some(entry) = build_load_slot_entry(runtime, save_dir, 0, "Autosave") {
            slots.push(entry);
        } else {
            slots.push(LoadSlotEntry {
                slot: 0,
                label: "Autosave - Empty".to_string(),
                enabled: false,
            });
        }
    }
    let max_slots = runtime.content.rules.save.slots_max.max(1) as u8;
    for slot in 1..=max_slots {
        if let Some(entry) =
            build_load_slot_entry(runtime, save_dir, slot, &format!("Slot {}", slot))
        {
            slots.push(entry);
        } else {
            slots.push(LoadSlotEntry {
                slot,
                label: format!("Slot {} - Empty", slot),
                enabled: false,
            });
        }
    }
    slots
}

fn build_load_slot_entry(
    runtime: &GameRuntime,
    save_dir: &PathBuf,
    slot: u8,
    label: &str,
) -> Option<LoadSlotEntry> {
    let save = SaveFile::load(save_slot_path(save_dir, slot)).ok()?;
    let map_name = runtime
        .content
        .map_index
        .get(save.world.map_id.as_str())
        .and_then(|index| runtime.content.maps.get(*index))
        .map(|map| map.name.clone())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| save.world.map_id.clone());
    let playtime = format_playtime(save.metadata.play_time_seconds);
    Some(LoadSlotEntry {
        slot,
        label: format!("{} - {}  {}", label, map_name, playtime),
        enabled: true,
    })
}

fn save_slot_path(save_dir: &PathBuf, slot: u8) -> PathBuf {
    save_dir.join(format!("slot_{}.json", slot))
}

fn format_playtime(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

fn slugify(value: &str) -> String {
    let mut out = String::new();
    let mut prev_underscore = false;
    for ch in value.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }
    while out.starts_with('_') {
        out.remove(0);
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() {
        "opencrystal".to_string()
    } else {
        out
    }
}

fn print_usage() {
    println!(
        "OpenCrystal\n\nUsage:\n  cryst play [--render=auto|wide|modern] [--content path] [--content-dir path]\n  cryst validate [--content path] [--content-dir path]\n  cryst new-project <name> [--path path]\n  cryst build new <kind> <id> [--content path] [--content-dir path] [--name label] [--force]\n  cryst build upgrade [--content path] [--content-dir path] [--dry-run]\n  cryst build new-project <name> [--path path]\n  cryst build docs [-s|--schemas] [-a|--architecture] [-c|--content-authoring] [-j|--jobs]"
    );
}
