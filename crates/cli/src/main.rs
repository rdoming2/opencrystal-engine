use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::{Duration, Instant};

use engine::{
    battle::{
        apply_damage_to_actor, apply_damage_to_enemy, build_battle_state, collect_rewards,
        is_enemies_defeated, is_party_defeated, next_living_party_index, physical_damage,
        roll_damage, BattleResult, BattleState, LevelUpDiff,
    },
    content::Content,
    party::{actor_slots, exp_for_level, gain_exp, recompute_derived_stats, PartyState},
    rules::{MagicSystem, PartyMode, Ruleset},
    runtime::{GameRuntime, GameState, MenuFocus},
    world::WorldState,
    Engine,
};
use rand::seq::SliceRandom;
use rand::Rng;
use tui::app::{
    draw_battle, draw_menu, draw_menu_frame, draw_overworld, draw_overworld_with_tooltip,
    prompt_choice, prompt_text, run_title, show_centered_dialog_on_map, show_dialog,
    show_dialog_on_map, show_dialog_with_choices, show_dialog_with_choices_on_map, show_shop,
    BattleCommandItem, BattleCommandPanelMode, BattleCommandPanelView, BattleEnemyView,
    BattleFocus, BattlePartyView, BattleRenderState, ChoiceView, MapView, MenuEntryView, MenuPane,
    MenuPanelLine, MenuPanelSpan, MenuPanelView, NpcView, PanelSpanStyle, ShopItem, ShopView,
    TileRender, TitleAction, TransitionView, TuiSession,
};
use tui::input::{Action, InputBindings, InputFile};
use tui::renderer::RenderMode;
use tui::ui::{BattleUiFile, DialogUiFile, MenuUiFile, ProgressUiFile, TitleUiFile};

struct SessionGuard(Option<TuiSession>);

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
        Some("new-project") => run_new_project(),
        Some("build") => run_build(),
        _ => print_usage(),
    }
}

fn run_play(args: Vec<String>) {
    let render_mode = parse_render_mode(&args).unwrap_or(RenderMode::Auto);
    let content_dir = parse_content_dir(&args).unwrap_or_else(|| PathBuf::from("content/demo"));
    let input_path = content_dir.join("input.json");
    let title_ui_path = content_dir.join("ui").join("title.json");
    let menu_ui_path = content_dir.join("ui").join("menu.json");
    let battle_ui_path = content_dir.join("ui").join("battle.json");
    let dialog_ui_path = content_dir.join("ui").join("dialog.json");
    let progress_ui_path = content_dir.join("ui").join("progress.json");

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
    let world = content
        .worlds
        .worlds
        .first()
        .map(|world| WorldState::new(&world.id, &world.starting_map, (0, 0)))
        .unwrap_or_else(|| WorldState::new("gaia", "overworld_gaia", (20, 14)));

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

    if let Err(err) = ProgressUiFile::load(&progress_ui_path) {
        eprintln!("Failed to load progress UI: {}", err);
    }

    let _engine = Engine::new(rules.clone(), world.clone());
    let mut runtime = GameRuntime::new(content);

    match render_mode {
        RenderMode::Auto => println!("Starting OpenCrystal (render: auto)..."),
        RenderMode::Wide => println!("Starting OpenCrystal (render: wide)..."),
        RenderMode::Modern => println!("Starting OpenCrystal (render: modern)..."),
    }

    let mut session_guard = SessionGuard::start();

    let action = if let Some(session) = session_guard.as_mut() {
        match run_title(session, &title_ui, &input_bindings) {
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
            if let Some(session) = session_guard.as_mut() {
                if rules.party_mode == PartyMode::Create {
                    if let Err(err) =
                        run_party_create_flow(session, &mut runtime, &rules, &input_bindings)
                    {
                        if err.kind() == std::io::ErrorKind::Interrupted {
                            return;
                        }
                    }
                }

                runtime.start_new_game(&rules);
                if let Err(err) = run_event_loop(
                    &mut runtime,
                    &dialog_ui,
                    &battle_ui,
                    &input_bindings,
                    session,
                ) {
                    if err.kind() == std::io::ErrorKind::Interrupted {
                        return;
                    }
                }
                let spawn = find_spawn(&runtime, &world.map_id, world.position);
                if let Err(err) = run_overworld_loop(
                    session,
                    &mut runtime,
                    &dialog_ui,
                    &battle_ui,
                    &menu_ui,
                    &input_bindings,
                    &world.map_id,
                    spawn,
                ) {
                    if err.kind() == std::io::ErrorKind::Interrupted {
                        return;
                    }
                }
            } else {
                if rules.party_mode == PartyMode::Create {
                    runtime.party = PartyState::from_created(
                        &runtime.content,
                        &rules,
                        default_party_names(&rules),
                    );
                }
                runtime.start_new_game(&rules);
                run_event_loop_console(&mut runtime, &dialog_ui);
            }
        }
        TitleAction::Load => println!("Load not implemented."),
        TitleAction::Settings => println!("Settings not implemented."),
        TitleAction::Exit => println!("Exit."),
    }
}

fn run_event_loop(
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    battle_ui: &BattleUiFile,
    bindings: &tui::input::InputBindings,
    session: &mut TuiSession,
) -> std::io::Result<()> {
    while runtime.state == GameState::Event {
        match runtime.next_event_step() {
            Some(step) => {
                let result = runtime.apply_event_step(&step);
                handle_event_result(runtime, dialog_ui, battle_ui, bindings, session, result)?
            }
            None => {}
        }
    }
    Ok(())
}

fn run_event_loop_console(runtime: &mut GameRuntime, dialog_ui: &DialogUiFile) {
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
    bindings: &tui::input::InputBindings,
    session: &mut TuiSession,
    result: engine::events::EventExecutionResult,
) -> std::io::Result<()> {
    match result {
        engine::events::EventExecutionResult::Continue => {}
        engine::events::EventExecutionResult::Dialog { speaker, text } => {
            show_dialog(session, dialog_ui, bindings, &speaker, &text)?;
        }
        engine::events::EventExecutionResult::Narration { text } => {
            show_dialog(session, dialog_ui, bindings, "", &text)?;
        }
        engine::events::EventExecutionResult::StartDialog { dialog_id } => {
            run_dialog(runtime, dialog_ui, bindings, session, &dialog_id)?;
        }
        engine::events::EventExecutionResult::StartBattle {
            encounter,
            formation,
        } => {
            let outcome = run_event_battle_with_result(
                runtime, battle_ui, bindings, session, &encounter, &formation,
            )?;
            if matches!(outcome, BattleOutcome::Defeat) {
                show_dialog(session, dialog_ui, bindings, "", "The party was defeated.")?;
            }
        }
        engine::events::EventExecutionResult::OpenShop { shop_id } => {
            open_shop(runtime, session, bindings, &shop_id)?;
        }
        engine::events::EventExecutionResult::Completed => {}
    }
    Ok(())
}

fn handle_event_step(
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    battle_ui: &BattleUiFile,
    bindings: &tui::input::InputBindings,
    session: &mut TuiSession,
    step: &engine::events::EventStep,
) -> std::io::Result<()> {
    match step.r#type.as_str() {
        "dialog" => {
            let speaker = step.speaker.as_deref().unwrap_or("Narrator");
            let text = step.text.as_deref().unwrap_or("");
            show_dialog(session, dialog_ui, bindings, speaker, text)?;
        }
        "narration" => {
            let text = step.text.as_deref().unwrap_or("");
            show_dialog(session, dialog_ui, bindings, "", text)?;
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
        "start_dialog" => {
            if let Some(dialog) = &step.dialog {
                run_dialog(runtime, dialog_ui, bindings, session, dialog)?;
            }
        }
        "start_battle" => {
            let outcome = run_event_battle(runtime, battle_ui, bindings, session, step)?;
            if matches!(outcome, BattleOutcome::Defeat) {
                show_dialog(session, dialog_ui, bindings, "", "The party was defeated.")?;
            }
        }
        "give_item" => {
            if let Some(item) = &step.item {
                let qty = step.qty.unwrap_or(1);
                println!("Give item: {} x{}", item, qty);
            }
        }
        "give_equipment" => {
            if let Some(item) = &step.item {
                let qty = step.qty.unwrap_or(1);
                println!("Give equipment: {} x{}", item, qty);
            }
        }
        "warp" => {
            if let Some(target) = &step.target {
                println!("Warp to {} at {:?}", target.map, target.pos);
            }
        }
        "open_shop" => {
            if let Some(shop) = &step.shop {
                open_shop(runtime, session, bindings, shop)?;
            }
        }
        "npc_show" | "npc_hide" | "npc_move" | "npc_set_sprite" => {
            println!("NPC action: {}", step.r#type);
        }
        other => {
            println!("Event step: {}", other);
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
                run_dialog_console(runtime, dialog_ui, dialog);
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

fn run_dialog(
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    bindings: &tui::input::InputBindings,
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
        let choice_views = node.choices.as_ref().map(|choices| {
            choices
                .iter()
                .map(|choice| ChoiceView {
                    label: choice.label.clone(),
                    show_next: choice.next.as_str() != "end",
                })
                .collect::<Vec<_>>()
        });

        let selection = if let Some(choices) = &choice_views {
            show_dialog_with_choices(session, dialog_ui, bindings, speaker, &node.text, choices)?
        } else {
            show_dialog(session, dialog_ui, bindings, speaker, &node.text)?;
            None
        };

        if let Some(actions) = &node.actions {
            for action in actions {
                if handle_dialog_action(runtime, session, bindings, action)? {
                    return Ok(());
                }
            }
        }

        if let (Some(selection), Some(choices)) = (selection, node.choices.as_ref()) {
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

fn run_dialog_on_map(
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    bindings: &tui::input::InputBindings,
    session: &mut TuiSession,
    dialog_id: &str,
    map: &MapView,
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
        let choice_views = node.choices.as_ref().map(|choices| {
            choices
                .iter()
                .map(|choice| ChoiceView {
                    label: choice.label.clone(),
                    show_next: choice.next.as_str() != "end",
                })
                .collect::<Vec<_>>()
        });

        let selection = if let Some(choices) = &choice_views {
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

        if let (Some(selection), Some(choices)) = (selection, node.choices.as_ref()) {
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

fn run_dialog_console(runtime: &mut GameRuntime, dialog_ui: &DialogUiFile, dialog_id: &str) {
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
            for (index, choice) in choices.iter().enumerate() {
                println!("  {}. {}", index + 1, choice.label);
            }
            let selection = read_choice_console(choices.len());
            let next = choices
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

fn show_dialog_console(dialog_ui: &DialogUiFile, speaker: &str, text: &str) {
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

fn run_party_create_flow(
    session: &mut TuiSession,
    runtime: &mut GameRuntime,
    rules: &Ruleset,
    bindings: &tui::input::InputBindings,
) -> std::io::Result<()> {
    let max_len = rules.party_create.name_length;
    let jobs_enabled = rules.systems.get("jobs").copied().unwrap_or(false);
    let job_options = if jobs_enabled {
        build_available_jobs(runtime)
    } else {
        Vec::new()
    };
    if jobs_enabled && job_options.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "no jobs available for party creation",
        ));
    }
    let default_job_index = job_options
        .iter()
        .position(|job| job.is_default)
        .unwrap_or(0);
    let mut members = Vec::new();
    for index in 0..rules.party_size {
        let default_name = format!("Hero {}", index + 1);
        let prompt = format!("Name character {}:", index + 1);
        let name = match prompt_text(session, "Create Party", &prompt, &default_name, max_len)? {
            Some(name) => name,
            None => return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "quit")),
        };
        let job_id = if jobs_enabled {
            let labels = job_options
                .iter()
                .map(|job| job.name.clone())
                .collect::<Vec<_>>();
            match prompt_choice(
                session,
                bindings,
                "Choose Job",
                "Select a job:",
                &labels,
                default_job_index,
            )? {
                Some(selection) => job_options[selection].id.clone(),
                None => return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "quit")),
            }
        } else {
            rules.party_create.default_job.clone()
        };
        members.push((name, job_id));
    }
    runtime.party = PartyState::from_created(&runtime.content, rules, members);
    Ok(())
}

fn default_party_names(rules: &Ruleset) -> Vec<(String, String)> {
    let max_len = rules.party_create.name_length;
    let default_job = rules.party_create.default_job.clone();
    (1..=rules.party_size)
        .map(|index| {
            let name = format!("Hero {}", index);
            let name = name.chars().take(max_len).collect::<String>();
            (name, default_job.clone())
        })
        .collect()
}

fn build_available_jobs(runtime: &GameRuntime) -> Vec<JobOption> {
    let mut jobs = runtime
        .content
        .jobs
        .jobs
        .iter()
        .filter(|job| job_unlock_available(runtime, job))
        .map(|job| JobOption {
            id: job.id.clone(),
            name: job.name.clone(),
            is_default: job.is_default,
            sort_order: job.sort_order,
        })
        .collect::<Vec<_>>();
    jobs.sort_by(|left, right| {
        let left_order = left.sort_order.unwrap_or(0);
        let right_order = right.sort_order.unwrap_or(0);
        left_order
            .cmp(&right_order)
            .then_with(|| left.name.cmp(&right.name))
    });
    jobs
}

fn job_unlock_available(runtime: &GameRuntime, job: &engine::entities::JobDefinition) -> bool {
    match job.unlock_flag.as_deref() {
        Some(flag) if !flag.trim().is_empty() => runtime.has_flag(flag),
        _ => true,
    }
}

struct JobOption {
    id: String,
    name: String,
    is_default: bool,
    sort_order: Option<i32>,
}

fn run_overworld_loop(
    session: &mut TuiSession,
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
    battle_ui: &BattleUiFile,
    menu_ui: &MenuUiFile,
    bindings: &tui::input::InputBindings,
    map_id: &str,
    start_pos: (i32, i32),
) -> std::io::Result<()> {
    let mut current_map_id = map_id.to_string();
    let mut player_pos = start_pos;
    let mut return_positions: HashMap<String, (String, (i32, i32))> = HashMap::new();
    let mut last_map_id = String::new();
    let mut area_name_active = false;
    let mut rng = rand::thread_rng();

    let mut running = true;
    while running {
        let map = match build_map_view(runtime, &current_map_id) {
            Some(map) => map,
            None => {
                println!("Map not found: {}", current_map_id);
                running = false;
                continue;
            }
        };

        if current_map_id != last_map_id {
            area_name_active = !map.hide_name && !map.name.is_empty();
            last_map_id = current_map_id.clone();
        }

        if area_name_active {
            draw_overworld_with_tooltip(session, &map, player_pos, dialog_ui, &map.name)?;
        } else {
            draw_overworld(session, &map, player_pos)?;
        }

        let previous_pos = player_pos;
        if let Some(action) = read_action(bindings) {
            match action {
                Action::MoveUp => {
                    player_pos.1 -= 1;
                    area_name_active = false;
                }
                Action::MoveDown => {
                    player_pos.1 += 1;
                    area_name_active = false;
                }
                Action::MoveLeft => {
                    player_pos.0 -= 1;
                    area_name_active = false;
                }
                Action::MoveRight => {
                    player_pos.0 += 1;
                    area_name_active = false;
                }
                Action::Confirm => {
                    if let Some(chest) = find_chest(runtime, &current_map_id, player_pos) {
                        let chest_text = open_chest(runtime, &chest);
                        show_centered_dialog_on_map(
                            session,
                            &map,
                            player_pos,
                            dialog_ui,
                            bindings,
                            &chest_text,
                        )?;
                    } else if let Some(text) = find_sign_text(runtime, &current_map_id, player_pos)
                    {
                        show_centered_dialog_on_map(
                            session, &map, player_pos, dialog_ui, bindings, &text,
                        )?;
                    } else if let Some(dialog_id) =
                        find_npc_dialog(runtime, &current_map_id, player_pos)
                    {
                        run_dialog_on_map(
                            runtime, dialog_ui, bindings, session, &dialog_id, &map, player_pos,
                        )?;
                    }
                }
                Action::Menu => {
                    runtime.open_menu();
                    if let Err(err) = run_menu_loop(
                        session,
                        runtime,
                        menu_ui,
                        bindings,
                        &current_map_id,
                        player_pos,
                    ) {
                        if err.kind() == std::io::ErrorKind::Interrupted {
                            return Err(err);
                        }
                    }
                }
                Action::Cancel => {}
                Action::Quit => {
                    if tui::app::confirm_quit(session, |frame| {
                        tui::app::draw_overworld_frame(frame, &map, player_pos);
                    })? {
                        return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "quit"));
                    }
                }
                _ => {}
            }
        }

        let mut transitioned = false;
        if let Some(transition) = find_transition(runtime, &current_map_id, player_pos) {
            let (next_map, next_pos) = if transition.return_to_last {
                return_positions
                    .get(&current_map_id)
                    .cloned()
                    .map(|(return_map, return_pos)| (return_map, return_pos))
                    .unwrap_or_else(|| {
                        (
                            transition.target_map.clone(),
                            (transition.target_pos[0], transition.target_pos[1]),
                        )
                    })
            } else {
                (
                    transition.target_map.clone(),
                    (transition.target_pos[0], transition.target_pos[1]),
                )
            };

            if !transition.return_to_last
                && !is_returning_from_child(&return_positions, &current_map_id, &next_map)
            {
                return_positions.insert(next_map.clone(), (current_map_id.clone(), player_pos));
            }
            current_map_id = next_map;
            player_pos = next_pos;
            transitioned = true;
        }

        if !is_passable(runtime, &current_map_id, player_pos)
            || npc_at(runtime, &current_map_id, player_pos)
            || sign_at(runtime, &current_map_id, player_pos)
            || chest_at(runtime, &current_map_id, player_pos)
        {
            player_pos = previous_pos;
            transitioned = false;
        }

        if transitioned {
            let on_enter_events = runtime.get_on_enter_events_for_map(&current_map_id);
            for event_id in on_enter_events {
                runtime.queue_event(&event_id);
            }
        }

        let moved = player_pos != previous_pos;
        if moved && !transitioned {
            runtime.world.map_id = current_map_id.clone();
            runtime.world.position = player_pos;
            let step_events_pos =
                runtime.get_on_step_events_for_position(&current_map_id, player_pos);
            let step_events_zone =
                runtime.get_on_step_events_for_zone(&current_map_id, player_pos, previous_pos);
            for event_id in step_events_pos.into_iter().chain(step_events_zone) {
                runtime.queue_event(&event_id);
            }
            if let Some(outcome) = try_start_random_battle(
                runtime,
                battle_ui,
                bindings,
                session,
                &current_map_id,
                player_pos,
                &mut rng,
            )? {
                if matches!(outcome, BattleOutcome::Defeat) {
                    show_dialog(session, dialog_ui, bindings, "", "The party was defeated.")?;
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "defeat",
                    ));
                }
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum BattleOutcome {
    Victory,
    Defeat,
    Escaped,
}

struct MenuEntryState {
    view: MenuEntryView,
    action: String,
    selectable: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum InventoryFilter {
    Items,
    Equipment,
    Weapons,
    Armor,
    Accessory,
}

#[derive(Clone, Debug, PartialEq)]
enum InventorySort {
    Name,
    Type,
}

#[derive(Clone, Debug, PartialEq)]
enum InventoryKind {
    Item,
    Equipment,
}

struct InventoryEntry {
    id: String,
    label: String,
    available_qty: i32,
    total_qty: i32,
    kind: InventoryKind,
    slot: Option<String>,
    category: Option<String>,
    usable: bool,
    equipped_by: Vec<String>,
    usage_target: String,
}

#[derive(Clone, Debug)]
struct SpellEntry {
    id: String,
    name: String,
    school: String,
    tier: u32,
    cost_type: String,
    cost_value: i32,
    default_target: String,
    allowed_targets: Vec<String>,
    effect_type: String,
    effect_power: i32,
    usable: bool,
    reason: Option<String>,
}

#[derive(Clone, Debug)]
struct AbilityEntry {
    id: String,
    name: String,
    default_target: String,
    allowed_targets: Vec<String>,
    effect_type: String,
    effect_power: i32,
}

fn run_menu_loop(
    session: &mut TuiSession,
    runtime: &mut GameRuntime,
    menu_ui: &MenuUiFile,
    bindings: &tui::input::InputBindings,
    map_id: &str,
    player_pos: (i32, i32),
) -> std::io::Result<()> {
    let entries = build_menu_entries(runtime, menu_ui, map_id, player_pos);
    if entries.is_empty() {
        runtime.close_menu();
        return Ok(());
    }
    let entry_views = entries
        .iter()
        .map(|entry| entry.view.clone())
        .collect::<Vec<_>>();

    if runtime.menu_state.selected >= entry_views.len() {
        runtime.menu_state.selected = 0;
    }

    loop {
        let focus = match runtime.menu_state.focus {
            MenuFocus::List => MenuPane::List,
            MenuFocus::Detail => MenuPane::Detail,
        };
        let selected = entries.get(runtime.menu_state.selected);
        let label = selected
            .map(|entry| entry.view.label.as_str())
            .unwrap_or("Menu");
        let submenu_action = runtime
            .menu_state
            .active_submenu
            .as_deref()
            .or_else(|| selected.map(|entry| entry.action.as_str()))
            .unwrap_or("menu");
        let right_panel = if matches!(focus, MenuPane::Detail) {
            menu_detail_panel(
                label,
                submenu_action,
                runtime,
                runtime.menu_state.detail_page,
            )
        } else {
            menu_default_panel(menu_ui, runtime)
        };

        let footer_text = menu_footer_text(focus, submenu_action, runtime.menu_state.detail_page);
        let stats_view = build_menu_stats_view(runtime);
        draw_menu(
            session,
            menu_ui,
            &entry_views,
            runtime.menu_state.selected,
            focus,
            &right_panel,
            Some(&stats_view),
            footer_text,
        )?;

        if let Some(action) = read_action(bindings) {
            match action {
                Action::MoveUp => {
                    if matches!(focus, MenuPane::List) {
                        if runtime.menu_state.selected > 0 {
                            runtime.menu_state.selected -= 1;
                        }
                    } else if submenu_action == "items" {
                        if runtime.menu_state.detail_page == 0 {
                            if runtime.menu_state.detail_selection > 0 {
                                runtime.menu_state.detail_selection -= 1;
                            }
                        } else if runtime.menu_state.detail_target > 0 {
                            runtime.menu_state.detail_target -= 1;
                        }
                    } else if submenu_action == "magic" {
                        if runtime.menu_state.detail_page == 0 {
                            if runtime.menu_state.detail_selection > 0 {
                                runtime.menu_state.detail_selection -= 1;
                            }
                        } else if runtime.menu_state.detail_target > 0 {
                            runtime.menu_state.detail_target -= 1;
                        }
                    } else if submenu_action == "abilities" {
                        if runtime.menu_state.detail_page == 0 {
                            if runtime.menu_state.detail_selection > 0 {
                                runtime.menu_state.detail_selection -= 1;
                            }
                        } else if runtime.menu_state.detail_target > 0 {
                            runtime.menu_state.detail_target -= 1;
                        }
                    } else if submenu_action == "equipment" {
                        if runtime.menu_state.detail_selection > 0 {
                            runtime.menu_state.detail_selection -= 1;
                        }
                    }
                }
                Action::MoveDown => {
                    if matches!(focus, MenuPane::List) {
                        if runtime.menu_state.selected + 1 < entry_views.len() {
                            runtime.menu_state.selected += 1;
                        }
                    } else if submenu_action == "items" {
                        if runtime.menu_state.detail_page == 0 {
                            let entries = build_inventory_entries(
                                runtime,
                                &filter_from_index(runtime.menu_state.detail_filter),
                                &sort_from_index(runtime.menu_state.detail_sort),
                            );
                            if runtime.menu_state.detail_selection + 1 < entries.len() {
                                runtime.menu_state.detail_selection += 1;
                            }
                        } else {
                            let entries = build_inventory_entries(
                                runtime,
                                &filter_from_index(runtime.menu_state.detail_filter),
                                &sort_from_index(runtime.menu_state.detail_sort),
                            );
                            let targets = entries
                                .get(runtime.menu_state.detail_selection)
                                .map(|entry| item_targets_for_entry(runtime, entry))
                                .unwrap_or_default();
                            if runtime.menu_state.detail_target + 1 < targets.len() {
                                runtime.menu_state.detail_target += 1;
                            }
                        }
                    } else if submenu_action == "magic" {
                        if runtime.menu_state.detail_page == 0 {
                            let entries = build_spell_entries(runtime);
                            if runtime.menu_state.detail_selection + 1 < entries.len() {
                                runtime.menu_state.detail_selection += 1;
                            }
                        } else {
                            let targets = selected_spell_targets(runtime);
                            if runtime.menu_state.detail_target + 1 < targets.len() {
                                runtime.menu_state.detail_target += 1;
                            }
                        }
                    } else if submenu_action == "abilities" {
                        if runtime.menu_state.detail_page == 0 {
                            let entries = build_ability_entries(runtime);
                            if runtime.menu_state.detail_selection + 1 < entries.len() {
                                runtime.menu_state.detail_selection += 1;
                            }
                        } else {
                            let targets = selected_ability_targets(runtime);
                            if runtime.menu_state.detail_target + 1 < targets.len() {
                                runtime.menu_state.detail_target += 1;
                            }
                        }
                    } else if submenu_action == "equipment" {
                        let limit = if runtime.menu_state.detail_page == 0 {
                            equipment_slots_for_menu(runtime).len()
                        } else {
                            equipment_entries_for_menu(runtime).len()
                        };
                        if runtime.menu_state.detail_selection + 1 < limit {
                            runtime.menu_state.detail_selection += 1;
                        }
                    }
                }
                Action::Confirm => {
                    if matches!(focus, MenuPane::List) {
                        if let Some(entry) = entries.get(runtime.menu_state.selected) {
                            if entry.selectable {
                                match entry.action.as_str() {
                                    "items" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = 0;
                                        runtime.menu_state.detail_filter = 0;
                                        runtime.menu_state.detail_sort = 0;
                                        runtime.menu_state.detail_target = 0;
                                    }
                                    "magic" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = 0;
                                        runtime.menu_state.detail_actor = 0;
                                        runtime.menu_state.detail_target = 0;
                                    }
                                    "equipment" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = 0;
                                        runtime.menu_state.detail_actor = 0;
                                        runtime.menu_state.detail_slot = 0;
                                    }
                                    "abilities" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = 0;
                                        runtime.menu_state.detail_actor = 0;
                                        runtime.menu_state.detail_target = 0;
                                    }
                                    _ => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                    }
                                }
                            }
                        }
                    } else if submenu_action == "items" {
                        let entries = build_inventory_entries(
                            runtime,
                            &filter_from_index(runtime.menu_state.detail_filter),
                            &sort_from_index(runtime.menu_state.detail_sort),
                        );
                        if let Some(entry) = entries.get(runtime.menu_state.detail_selection) {
                            if entry.kind == InventoryKind::Item && entry.usable {
                                if runtime.menu_state.detail_page == 0 {
                                    let targets = item_targets_for_entry(runtime, entry);
                                    if entry.usage_target == "party" {
                                        if apply_item_to_targets(runtime, entry, &targets) {
                                            runtime.inventory.remove_item(&entry.id, 1);
                                        }
                                    } else if targets.is_empty() {
                                        runtime.menu_state.detail_page = 0;
                                    } else {
                                        runtime.menu_state.detail_page = 1;
                                        runtime.menu_state.detail_target = 0;
                                    }
                                } else {
                                    let targets = item_targets_for_entry(runtime, entry);
                                    if let Some(target_id) =
                                        targets.get(runtime.menu_state.detail_target)
                                    {
                                        if apply_item_to_targets(
                                            runtime,
                                            entry,
                                            &[target_id.clone()],
                                        ) {
                                            runtime.inventory.remove_item(&entry.id, 1);
                                        }
                                    }
                                    runtime.menu_state.detail_page = 0;
                                    runtime.menu_state.detail_target = 0;
                                }
                                runtime.menu_state.detail_selection = runtime
                                    .menu_state
                                    .detail_selection
                                    .min(entries.len().saturating_sub(1));
                            }
                        }
                    } else if submenu_action == "magic" {
                        let entries = build_spell_entries(runtime);
                        let selection = runtime
                            .menu_state
                            .detail_selection
                            .min(entries.len().saturating_sub(1));
                        let actor_id = match detail_actor_id(runtime) {
                            Some(actor_id) => actor_id,
                            None => continue,
                        };
                        if let Some(entry) = entries.get(selection) {
                            if entry.usable {
                                if runtime.menu_state.detail_page == 0 {
                                    let targets =
                                        spell_targets_for_entry(runtime, entry, &actor_id);
                                    if entry.default_target == "party"
                                        || entry.default_target == "self"
                                    {
                                        apply_spell_to_targets(runtime, entry, &actor_id, &targets);
                                    } else if targets.is_empty() {
                                        runtime.menu_state.detail_page = 0;
                                    } else {
                                        runtime.menu_state.detail_page = 1;
                                        runtime.menu_state.detail_target = 0;
                                    }
                                } else {
                                    let targets =
                                        spell_targets_for_entry(runtime, entry, &actor_id);
                                    if let Some(target_id) =
                                        targets.get(runtime.menu_state.detail_target)
                                    {
                                        apply_spell_to_targets(
                                            runtime,
                                            entry,
                                            &actor_id,
                                            &[target_id.clone()],
                                        );
                                    }
                                    runtime.menu_state.detail_page = 0;
                                    runtime.menu_state.detail_target = 0;
                                }
                            }
                            runtime.menu_state.detail_selection = selection;
                        }
                    } else if submenu_action == "abilities" {
                        let entries = build_ability_entries(runtime);
                        let selection = runtime
                            .menu_state
                            .detail_selection
                            .min(entries.len().saturating_sub(1));
                        let actor_id = match detail_actor_id(runtime) {
                            Some(actor_id) => actor_id,
                            None => continue,
                        };
                        if let Some(entry) = entries.get(selection) {
                            if runtime.menu_state.detail_page == 0 {
                                let targets = ability_targets_for_entry(runtime, entry, &actor_id);
                                if entry.default_target == "party" || entry.default_target == "self"
                                {
                                    runtime.menu_state.detail_page = 0;
                                } else if targets.is_empty() {
                                    runtime.menu_state.detail_page = 0;
                                } else {
                                    runtime.menu_state.detail_page = 1;
                                    runtime.menu_state.detail_target = 0;
                                }
                            } else {
                                runtime.menu_state.detail_page = 0;
                                runtime.menu_state.detail_target = 0;
                            }
                            runtime.menu_state.detail_selection = selection;
                        }
                    } else if submenu_action == "equipment" {
                        if runtime.menu_state.detail_page == 0 {
                            runtime.menu_state.detail_slot = runtime.menu_state.detail_selection;
                            runtime.menu_state.detail_page = 1;
                            runtime.menu_state.detail_selection = 0;
                        } else {
                            let entries = equipment_entries_for_menu(runtime);
                            if let Some(entry) = entries.get(runtime.menu_state.detail_selection) {
                                if entry.usable {
                                    let slot = equipment_slot_for_menu(runtime);
                                    if let Some(slot) = slot {
                                        let actor_id = detail_actor_id(runtime);
                                        if let Some(actor_id) = actor_id {
                                            equip_item(runtime, &actor_id, &slot, entry);
                                        }
                                    }
                                }
                            }
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = runtime.menu_state.detail_slot;
                        }
                    }
                }
                Action::Cancel | Action::Menu => {
                    if matches!(focus, MenuPane::Detail) {
                        if submenu_action == "equipment" && runtime.menu_state.detail_page == 1 {
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = runtime.menu_state.detail_slot;
                        } else if submenu_action == "items" && runtime.menu_state.detail_page == 1 {
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_target = 0;
                        } else if submenu_action == "magic" && runtime.menu_state.detail_page == 1 {
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_target = 0;
                        } else if submenu_action == "abilities"
                            && runtime.menu_state.detail_page == 1
                        {
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_target = 0;
                        } else {
                            runtime.menu_state.focus = MenuFocus::List;
                            runtime.menu_state.active_submenu = None;
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = 0;
                        }
                    } else {
                        runtime.close_menu();
                        return Ok(());
                    }
                }
                Action::MoveLeft | Action::MoveRight => {
                    if matches!(focus, MenuPane::Detail) && submenu_action == "status" {
                        runtime.menu_state.detail_page = if runtime.menu_state.detail_page == 0 {
                            1
                        } else {
                            0
                        };
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "items" {
                        if runtime.menu_state.detail_page == 0 {
                            runtime.menu_state.detail_filter =
                                if matches!(action, Action::MoveRight) {
                                    next_filter_index(runtime.menu_state.detail_filter)
                                } else {
                                    prev_filter_index(runtime.menu_state.detail_filter)
                                };
                            runtime.menu_state.detail_selection = 0;
                        }
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "equipment" {
                        let actor_count = runtime.party.active.len();
                        if actor_count > 0 {
                            runtime.menu_state.detail_actor = if matches!(action, Action::MoveRight)
                            {
                                (runtime.menu_state.detail_actor + 1) % actor_count
                            } else if runtime.menu_state.detail_actor == 0 {
                                actor_count - 1
                            } else {
                                runtime.menu_state.detail_actor - 1
                            };
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = 0;
                        }
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "magic" {
                        let actor_count = runtime.party.active.len();
                        if actor_count > 0 {
                            runtime.menu_state.detail_actor = if matches!(action, Action::MoveRight)
                            {
                                (runtime.menu_state.detail_actor + 1) % actor_count
                            } else if runtime.menu_state.detail_actor == 0 {
                                actor_count - 1
                            } else {
                                runtime.menu_state.detail_actor - 1
                            };
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = 0;
                            runtime.menu_state.detail_target = 0;
                        }
                    } else if matches!(focus, MenuPane::Detail) && submenu_action == "abilities" {
                        let actor_count = runtime.party.active.len();
                        if actor_count > 0 {
                            runtime.menu_state.detail_actor = if matches!(action, Action::MoveRight)
                            {
                                (runtime.menu_state.detail_actor + 1) % actor_count
                            } else if runtime.menu_state.detail_actor == 0 {
                                actor_count - 1
                            } else {
                                runtime.menu_state.detail_actor - 1
                            };
                            runtime.menu_state.detail_page = 0;
                            runtime.menu_state.detail_selection = 0;
                            runtime.menu_state.detail_target = 0;
                        }
                    }
                }
                Action::Pause => {
                    if matches!(focus, MenuPane::Detail)
                        && submenu_action == "items"
                        && runtime.menu_state.detail_page == 0
                    {
                        runtime.menu_state.detail_sort =
                            toggle_sort_index(runtime.menu_state.detail_sort);
                    }
                }
                Action::Quit => {
                    let confirm_stats = build_menu_stats_view(runtime);
                    if tui::app::confirm_quit(session, |frame| {
                        draw_menu_frame(
                            frame,
                            menu_ui,
                            &entry_views,
                            runtime.menu_state.selected,
                            focus,
                            &right_panel,
                            Some(&confirm_stats),
                            footer_text,
                        );
                    })? {
                        return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "quit"));
                    }
                }
            }
        }
    }
}
fn menu_footer_text(focus: MenuPane, submenu: &str, page: usize) -> &'static str {
    match focus {
        MenuPane::List => "Confirm: open  Cancel: close",
        MenuPane::Detail => match submenu {
            "status" => {
                if page == 0 {
                    "Left/Right: details  Cancel: back"
                } else {
                    "Left/Right: summary  Cancel: back"
                }
            }
            "items" => {
                if page == 0 {
                    "Confirm: use  Left/Right: filter  Pause: sort  Cancel: back"
                } else {
                    "Confirm: use  Cancel: back"
                }
            }
            "magic" => {
                if page == 0 {
                    "Confirm: cast  Left/Right: actor  Cancel: back"
                } else {
                    "Confirm: cast  Cancel: back"
                }
            }
            "abilities" => {
                if page == 0 {
                    "Confirm: preview  Left/Right: actor  Cancel: back"
                } else {
                    "Confirm: back  Cancel: back"
                }
            }
            "equipment" => {
                if page == 0 {
                    "Confirm: pick slot  Left/Right: actor  Cancel: back"
                } else {
                    "Confirm: equip  Left/Right: actor  Cancel: back"
                }
            }
            _ => "Cancel: back",
        },
    }
}

fn filter_from_index(index: usize) -> InventoryFilter {
    match index % 5 {
        0 => InventoryFilter::Items,
        1 => InventoryFilter::Equipment,
        2 => InventoryFilter::Weapons,
        3 => InventoryFilter::Armor,
        _ => InventoryFilter::Accessory,
    }
}

fn sort_from_index(index: usize) -> InventorySort {
    if index % 2 == 0 {
        InventorySort::Name
    } else {
        InventorySort::Type
    }
}

fn next_filter_index(index: usize) -> usize {
    (index + 1) % 5
}

fn prev_filter_index(index: usize) -> usize {
    if index == 0 {
        4
    } else {
        index - 1
    }
}

fn toggle_sort_index(index: usize) -> usize {
    if index == 0 {
        1
    } else {
        0
    }
}

fn detail_actor_id(runtime: &GameRuntime) -> Option<String> {
    runtime
        .party
        .active
        .get(runtime.menu_state.detail_actor)
        .cloned()
}

fn equipment_slots_for_menu(runtime: &GameRuntime) -> Vec<String> {
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Vec::new(),
    };
    match runtime.party.roster.get(&actor_id) {
        Some(actor) => actor_slots(&runtime.content, actor),
        None => Vec::new(),
    }
}

fn equipment_slot_for_menu(runtime: &GameRuntime) -> Option<String> {
    let slots = equipment_slots_for_menu(runtime);
    slots.get(runtime.menu_state.detail_slot).cloned()
}

fn equipment_entries_for_menu(runtime: &GameRuntime) -> Vec<InventoryEntry> {
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Vec::new(),
    };
    let slots = equipment_slots_for_menu(runtime);
    let slot_index = runtime
        .menu_state
        .detail_slot
        .min(slots.len().saturating_sub(1));
    let slot = slots.get(slot_index).cloned().unwrap_or_default();
    build_equipment_entries(runtime, &actor_id, &slot)
}

fn inventory_filters() -> Vec<String> {
    vec![
        "Items".to_string(),
        "Equipment".to_string(),
        "Weapons".to_string(),
        "Armor".to_string(),
        "Accessory".to_string(),
    ]
}

fn filter_label(filter: &InventoryFilter) -> String {
    match filter {
        InventoryFilter::Items => "Items",
        InventoryFilter::Equipment => "Equipment",
        InventoryFilter::Weapons => "Weapons",
        InventoryFilter::Armor => "Armor",
        InventoryFilter::Accessory => "Accessory",
    }
    .to_string()
}

fn sort_label(sort: &InventorySort) -> &'static str {
    match sort {
        InventorySort::Name => "Name",
        InventorySort::Type => "Type",
    }
}

fn panel_line(text: impl Into<String>) -> MenuPanelLine {
    MenuPanelLine {
        spans: vec![MenuPanelSpan {
            text: text.into(),
            style: PanelSpanStyle::Normal,
        }],
    }
}

fn panel_line_spans(spans: Vec<MenuPanelSpan>) -> MenuPanelLine {
    MenuPanelLine { spans }
}

fn panel_span(text: impl Into<String>, style: PanelSpanStyle) -> MenuPanelSpan {
    MenuPanelSpan {
        text: text.into(),
        style,
    }
}

fn list_line_width(entries: &[InventoryEntry]) -> usize {
    entries
        .iter()
        .map(|entry| entry.label.chars().count())
        .max()
        .unwrap_or(10)
        + 2
}

fn build_list_line(entry: &InventoryEntry, is_selected: bool, width: usize) -> MenuPanelLine {
    let prefix = if is_selected { "> " } else { "  " };
    let mut spans = Vec::new();
    let label_style = if is_selected {
        PanelSpanStyle::Highlight
    } else if entry.usable {
        PanelSpanStyle::Normal
    } else {
        PanelSpanStyle::Muted
    };
    let count_text = match entry.kind {
        InventoryKind::Item => format!("x{}", entry.total_qty),
        InventoryKind::Equipment => format!("{}/{}", entry.available_qty, entry.total_qty),
    };
    spans.push(panel_span(
        prefix,
        if is_selected {
            PanelSpanStyle::Highlight
        } else {
            PanelSpanStyle::Normal
        },
    ));
    spans.push(panel_span(
        format!("{:<width$}", entry.label, width = width),
        label_style,
    ));
    spans.push(panel_span(
        format!("{:>6}", count_text),
        PanelSpanStyle::Accent,
    ));
    if let Some(owner) = equipped_label(entry) {
        spans.push(panel_span(format!(" {}", owner), PanelSpanStyle::Accent));
    }
    panel_line_spans(spans)
}

fn build_inventory_entries(
    runtime: &GameRuntime,
    filter: &InventoryFilter,
    sort: &InventorySort,
) -> Vec<InventoryEntry> {
    let equipped_map = build_equipped_map(runtime);
    let mut entries = Vec::new();

    if matches!(filter, InventoryFilter::Items) {
        for item in &runtime.content.items.items {
            let qty = runtime.inventory.item_qty(&item.id);
            if qty <= 0 {
                continue;
            }
            let usable = item_usage_allows_field(&item.usage.context);
            entries.push(InventoryEntry {
                id: item.id.clone(),
                label: item.name.clone(),
                available_qty: qty,
                total_qty: qty,
                kind: InventoryKind::Item,
                slot: None,
                category: None,
                usable,
                equipped_by: Vec::new(),
                usage_target: item.usage.target.clone(),
            });
        }
    } else {
        for equipment in &runtime.content.equipment.equipment {
            if !matches_filter_equipment(filter, equipment) {
                continue;
            }
            let qty = runtime.inventory.equipment_qty(&equipment.id);
            let equipped_by = equipped_map.get(&equipment.id).cloned().unwrap_or_default();
            if qty <= 0 && equipped_by.is_empty() {
                continue;
            }
            let total_qty = qty + equipped_by.len() as i32;
            entries.push(InventoryEntry {
                id: equipment.id.clone(),
                label: equipment.name.clone(),
                available_qty: qty.max(0),
                total_qty,
                kind: InventoryKind::Equipment,
                slot: Some(equipment.slot.clone()),
                category: Some(equipment.category.clone()),
                usable: false,
                equipped_by,
                usage_target: String::new(),
            });
        }
    }

    entries.sort_by(|left, right| inventory_sort_key(left, right, sort));
    entries
}

fn matches_filter_equipment(
    filter: &InventoryFilter,
    equipment: &engine::entities::EquipmentDefinition,
) -> bool {
    match filter {
        InventoryFilter::Equipment => true,
        InventoryFilter::Weapons => equipment.slot == "weapon",
        InventoryFilter::Armor => equipment.slot == "armor",
        InventoryFilter::Accessory => equipment.slot == "accessory",
        InventoryFilter::Items => false,
    }
}

fn inventory_sort_key(
    left: &InventoryEntry,
    right: &InventoryEntry,
    sort: &InventorySort,
) -> std::cmp::Ordering {
    match sort {
        InventorySort::Name => left.label.cmp(&right.label),
        InventorySort::Type => {
            let left_kind = match left.kind {
                InventoryKind::Item => 0,
                InventoryKind::Equipment => 1,
            };
            let right_kind = match right.kind {
                InventoryKind::Item => 0,
                InventoryKind::Equipment => 1,
            };
            left_kind
                .cmp(&right_kind)
                .then_with(|| left.slot.cmp(&right.slot))
                .then_with(|| left.category.cmp(&right.category))
                .then_with(|| left.label.cmp(&right.label))
        }
    }
}

fn equipped_label(entry: &InventoryEntry) -> Option<String> {
    if entry.equipped_by.is_empty() {
        None
    } else {
        Some(format!("Equipped: {}", entry.equipped_by.join(", ")))
    }
}

fn build_equipment_detail(
    runtime: &GameRuntime,
    actor_id: &str,
    slot: &str,
    entry: &InventoryEntry,
) -> MenuPanelView {
    if entry.id.is_empty() {
        return MenuPanelView {
            title: "Unequip".to_string(),
            lines: vec![panel_line("Remove equipment from slot.")],
        };
    }
    let equipment = runtime
        .content
        .equipment
        .equipment
        .iter()
        .find(|item| item.id == entry.id);
    let mut lines = Vec::new();
    if let Some(equipment) = equipment {
        lines.push(panel_line(format!("Slot: {}", equipment.slot)));
        lines.push(panel_line(format!("Category: {}", equipment.category)));
        if let Some(owner) = equipped_label(entry) {
            lines.push(panel_line_spans(vec![panel_span(
                owner,
                PanelSpanStyle::Accent,
            )]));
        }
        if !actor_id.is_empty() {
            if let Some(actor) = runtime.party.roster.get(actor_id) {
                let preview = preview_equipment_delta(runtime, actor, slot, equipment);
                lines.extend(preview);
            }
        } else {
            lines.push(panel_line(""));
            lines.push(panel_line("Stats:"));
            for (stat, value) in &equipment.stats {
                lines.push(panel_line(format!("{} +{}", stat, value)));
            }
        }
        MenuPanelView {
            title: equipment.name.clone(),
            lines,
        }
    } else {
        MenuPanelView {
            title: "Equipment".to_string(),
            lines: vec![panel_line("Equipment not found.")],
        }
    }
}

fn preview_equipment_delta(
    runtime: &GameRuntime,
    actor: &engine::party::Actor,
    slot: &str,
    equipment: &engine::entities::EquipmentDefinition,
) -> Vec<MenuPanelLine> {
    let mut lines = Vec::new();
    let mut clone = actor.clone();
    clone
        .equipment
        .insert(slot.to_string(), equipment.id.clone());
    recompute_derived_stats(&runtime.content, &mut clone);
    lines.push(panel_line(""));
    lines.push(panel_line("Stat changes:"));
    for stat in runtime
        .content
        .stats
        .stats
        .base
        .iter()
        .chain(runtime.content.stats.stats.derived.iter())
    {
        let current = actor.derived_stats.get(&stat.id).copied().unwrap_or(0);
        let next = clone.derived_stats.get(&stat.id).copied().unwrap_or(0);
        if current != next {
            let diff = next - current;
            lines.push(panel_line(format!(
                "{} {} ({} {:+})",
                stat.name, next, current, diff
            )));
        }
    }
    if lines.len() == 2 {
        lines.push(panel_line("No stat changes."));
    }
    lines
}

fn build_equipped_map(runtime: &GameRuntime) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    for actor in runtime.party.roster.values() {
        for item_id in actor.equipment.values() {
            map.entry(item_id.clone())
                .or_insert_with(Vec::new)
                .push(actor.name.clone());
        }
    }
    map
}

fn build_equipped_counts(runtime: &GameRuntime) -> HashMap<String, i32> {
    let mut map = HashMap::new();
    for actor in runtime.party.roster.values() {
        for item_id in actor.equipment.values() {
            *map.entry(item_id.clone()).or_insert(0) += 1;
        }
    }
    map
}

fn build_equipment_entries(
    runtime: &GameRuntime,
    actor_id: &str,
    slot: &str,
) -> Vec<InventoryEntry> {
    let equipped_map = build_equipped_map(runtime);
    let equipped_counts = build_equipped_counts(runtime);
    let actor = match runtime.party.roster.get(actor_id) {
        Some(actor) => actor,
        None => return Vec::new(),
    };
    let job = runtime
        .content
        .jobs
        .jobs
        .iter()
        .find(|job| job.id == actor.job_id);

    let mut entries = Vec::new();
    entries.push(InventoryEntry {
        id: "".to_string(),
        label: "Unequip".to_string(),
        available_qty: 0,
        total_qty: 0,
        kind: InventoryKind::Equipment,
        slot: Some(slot.to_string()),
        category: None,
        usable: true,
        equipped_by: Vec::new(),
        usage_target: String::new(),
    });

    for equipment in &runtime.content.equipment.equipment {
        if !equipment_slot_matches(slot, &equipment.slot) {
            continue;
        }
        if let Some(job) = job {
            if !equipment_allowed(job, equipment) {
                continue;
            }
        }
        let inventory_qty = runtime.inventory.equipment_qty(&equipment.id);
        let equipped_count = equipped_counts.get(&equipment.id).copied().unwrap_or(0);
        let mut available = inventory_qty;
        let equipped_by = equipped_map.get(&equipment.id).cloned().unwrap_or_default();
        let already_equipped = actor
            .equipment
            .values()
            .any(|item_id| item_id == &equipment.id);
        if already_equipped {
            available += 1;
        }
        if available <= 0 && equipped_by.is_empty() {
            continue;
        }
        let usable = available > 0 || already_equipped;
        entries.push(InventoryEntry {
            id: equipment.id.clone(),
            label: equipment.name.clone(),
            available_qty: inventory_qty.max(0),
            total_qty: (inventory_qty + equipped_count).max(0),
            kind: InventoryKind::Equipment,
            slot: Some(equipment.slot.clone()),
            category: Some(equipment.category.clone()),
            usable,
            equipped_by,
            usage_target: String::new(),
        });
    }

    entries
}

fn equipment_slot_matches(slot: &str, equipment_slot: &str) -> bool {
    if slot.starts_with("accessory") {
        equipment_slot == "accessory"
    } else {
        slot == equipment_slot
    }
}

fn equipment_allowed(
    job: &engine::entities::JobDefinition,
    equipment: &engine::entities::EquipmentDefinition,
) -> bool {
    if let Some(allowed) = &equipment.allowed_jobs {
        if !allowed.contains(&job.id) {
            return false;
        }
    }
    match equipment.slot.as_str() {
        "weapon" => job.equipment.weapons.contains(&equipment.category),
        "armor" => job.equipment.armor.contains(&equipment.category),
        _ => true,
    }
}

fn item_targets_for_entry(runtime: &GameRuntime, entry: &InventoryEntry) -> Vec<String> {
    let item = match runtime
        .content
        .items
        .items
        .iter()
        .find(|item| item.id == entry.id)
    {
        Some(item) => item,
        None => return Vec::new(),
    };
    build_item_targets(runtime, item)
}

fn apply_item_to_targets(
    runtime: &mut GameRuntime,
    entry: &InventoryEntry,
    targets: &[String],
) -> bool {
    let item = match runtime
        .content
        .items
        .items
        .iter()
        .find(|item| item.id == entry.id)
        .cloned()
    {
        Some(item) => item,
        None => return false,
    };
    if !item_usage_allows_field(&item.usage.context) {
        return false;
    }
    for target_id in targets {
        apply_item_to_actor(runtime, &item, target_id);
    }
    true
}

fn build_item_targets(
    runtime: &GameRuntime,
    item: &engine::entities::ItemDefinition,
) -> Vec<String> {
    let mut targets = runtime.party.active.clone();
    match item.effect.r#type.as_str() {
        "revive" => {
            targets.retain(|id| {
                runtime
                    .party
                    .roster
                    .get(id)
                    .map(|actor| actor.current_hp <= 0)
                    .unwrap_or(false)
            });
        }
        _ => {}
    }
    targets
}

fn apply_item_to_actor(
    runtime: &mut GameRuntime,
    item: &engine::entities::ItemDefinition,
    actor_id: &str,
) {
    let Some(actor) = runtime.party.roster.get_mut(actor_id) else {
        return;
    };
    let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
    let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
    let power = item.effect.power.unwrap_or(0);
    match item.effect.r#type.as_str() {
        "heal_hp" => {
            actor.current_hp = (actor.current_hp + power).clamp(0, max_hp);
        }
        "heal_mp" => {
            actor.current_mp = (actor.current_mp + power).clamp(0, max_mp);
        }
        "revive" => {
            if actor.current_hp <= 0 {
                let amount = if power > 0 { power } else { max_hp };
                actor.current_hp = amount.clamp(1, max_hp);
            }
        }
        _ => {}
    }
}

fn item_usage_allows_field(context: &str) -> bool {
    matches!(context, "field" | "both")
}

fn equip_item(runtime: &mut GameRuntime, actor_id: &str, slot: &str, entry: &InventoryEntry) {
    let target_id = actor_id.to_string();
    if entry.id.is_empty() {
        if let Some(actor) = runtime.party.roster.get_mut(&target_id) {
            actor.equipment.remove(slot);
            recompute_derived_stats(&runtime.content, actor);
        }
        return;
    }
    let mut owner_to_clear = None;
    for (id, actor) in &runtime.party.roster {
        for (equip_slot, item_id) in &actor.equipment {
            if item_id == &entry.id && id != &target_id {
                owner_to_clear = Some((id.clone(), equip_slot.clone()));
                break;
            }
        }
        if owner_to_clear.is_some() {
            break;
        }
    }
    if let Some((owner_id, equip_slot)) = owner_to_clear {
        if let Some(owner) = runtime.party.roster.get_mut(&owner_id) {
            owner.equipment.remove(&equip_slot);
            recompute_derived_stats(&runtime.content, owner);
        }
    }
    if let Some(actor) = runtime.party.roster.get_mut(&target_id) {
        actor.equipment.insert(slot.to_string(), entry.id.clone());
        recompute_derived_stats(&runtime.content, actor);
    }
}

fn build_menu_entries(
    runtime: &GameRuntime,
    menu_ui: &MenuUiFile,
    map_id: &str,
    player_pos: (i32, i32),
) -> Vec<MenuEntryState> {
    let save_allowed = map_save_allowed(runtime, map_id, player_pos);
    menu_ui
        .menu
        .iter()
        .filter_map(|entry| {
            let system_enabled = system_enabled(runtime, entry.system.as_deref());
            if !system_enabled {
                return None;
            }
            let unlock_enabled = unlock_flag_enabled(runtime, entry.unlock_flag.as_deref());
            let mut selectable = entry.enabled && unlock_enabled;
            if entry.action == "save" && !save_allowed {
                selectable = false;
            }
            let show = selectable
                || (!entry.enabled && entry.locked_behavior.as_deref() == Some("disable"))
                || (!unlock_enabled && entry.locked_behavior.as_deref() == Some("disable"))
                || (entry.action == "save"
                    && !save_allowed
                    && entry.locked_behavior.as_deref() == Some("disable"));
            if !show {
                return None;
            }
            Some(MenuEntryState {
                view: MenuEntryView {
                    id: entry.id.clone(),
                    label: entry.label.clone(),
                    enabled: selectable,
                },
                action: entry.action.clone(),
                selectable,
            })
        })
        .collect()
}

fn menu_detail_panel(
    label: &str,
    action: &str,
    runtime: &GameRuntime,
    page: usize,
) -> MenuPanelView {
    if action == "status" {
        return MenuPanelView {
            title: "Status".to_string(),
            lines: build_status_panel(runtime, page),
        };
    }
    if action == "items" {
        return build_items_panel(runtime);
    }
    if action == "equipment" {
        return build_equipment_panel(runtime);
    }
    if action == "magic" {
        return build_magic_panel(runtime);
    }
    if action == "abilities" {
        return build_abilities_panel(runtime);
    }
    MenuPanelView {
        title: label.to_string(),
        lines: vec![
            panel_line(format!("{} menu not implemented.", label)),
            panel_line(format!("TODO: implement '{}' submenu.", action)),
        ],
    }
}

fn build_status_panel(runtime: &GameRuntime, page: usize) -> Vec<MenuPanelLine> {
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

fn build_items_panel(runtime: &GameRuntime) -> MenuPanelView {
    let filter = filter_from_index(runtime.menu_state.detail_filter);
    let sort = sort_from_index(runtime.menu_state.detail_sort);
    let entries = build_inventory_entries(runtime, &filter, &sort);
    if entries.is_empty() {
        return MenuPanelView {
            title: "Items".to_string(),
            lines: vec![panel_line("No items available.")],
        };
    }
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    let header = inventory_filter_line(&filter, &sort);
    let mut lines = Vec::new();
    let width = list_line_width(&entries);
    lines.push(header);
    for (index, entry) in entries.iter().enumerate() {
        lines.push(build_list_line(entry, index == selection, width));
    }
    lines.push(panel_line("------------------------------"));
    if runtime.menu_state.detail_page == 1 {
        lines.extend(build_item_target_panel(runtime, entries.get(selection)));
        lines.push(panel_line("------------------------------"));
    }
    lines.push(panel_line_spans(vec![panel_span(
        "Details",
        PanelSpanStyle::Accent,
    )]));
    lines.extend(build_item_description(runtime, entries.get(selection)));

    MenuPanelView {
        title: "Items".to_string(),
        lines,
    }
}

fn build_item_target_panel(
    runtime: &GameRuntime,
    entry: Option<&InventoryEntry>,
) -> Vec<MenuPanelLine> {
    let Some(entry) = entry else {
        return vec![panel_line("No target."), panel_line("")];
    };
    let targets = item_targets_for_entry(runtime, entry);
    if targets.is_empty() {
        return vec![panel_line("No valid targets."), panel_line("")];
    }
    let selection = runtime
        .menu_state
        .detail_target
        .min(targets.len().saturating_sub(1));
    let mut lines = Vec::new();
    lines.push(panel_line_spans(vec![panel_span(
        "Target",
        PanelSpanStyle::Accent,
    )]));
    for (index, target_id) in targets.iter().enumerate() {
        let name = runtime
            .party
            .roster
            .get(target_id)
            .map(|actor| actor.name.as_str())
            .unwrap_or(target_id.as_str());
        let is_selected = index == selection;
        lines.push(panel_line_spans(vec![
            panel_span(
                if is_selected { "> " } else { "  " },
                if is_selected {
                    PanelSpanStyle::Highlight
                } else {
                    PanelSpanStyle::Normal
                },
            ),
            panel_span(
                name,
                if is_selected {
                    PanelSpanStyle::Highlight
                } else {
                    PanelSpanStyle::Normal
                },
            ),
        ]));
    }
    lines
}

fn build_equipment_panel(runtime: &GameRuntime) -> MenuPanelView {
    let actor_id = detail_actor_id(runtime);
    let Some(actor_id) = actor_id else {
        return MenuPanelView {
            title: "Equipment".to_string(),
            lines: vec![panel_line("No party members.")],
        };
    };
    let actor = match runtime.party.roster.get(&actor_id) {
        Some(actor) => actor,
        None => {
            return MenuPanelView {
                title: "Equipment".to_string(),
                lines: vec![panel_line("No party members.")],
            };
        }
    };
    let header = equipment_header_line(actor.name.as_str());
    let slots = equipment_slots_for_menu(runtime);
    if slots.is_empty() {
        return MenuPanelView {
            title: "Equipment".to_string(),
            lines: vec![panel_line("No equipment slots.")],
        };
    }
    let mut lines = Vec::new();
    lines.push(header);
    if runtime.menu_state.detail_page == 0 {
        let selection = runtime
            .menu_state
            .detail_selection
            .min(slots.len().saturating_sub(1));
        for (index, slot) in slots.iter().enumerate() {
            let equipped = actor
                .equipment
                .get(slot)
                .and_then(|item_id| {
                    runtime
                        .content
                        .equipment
                        .equipment
                        .iter()
                        .find(|item| item.id == *item_id)
                        .map(|item| item.name.as_str())
                })
                .unwrap_or("Empty");
            let is_selected = index == selection;
            let mut spans = Vec::new();
            spans.push(panel_span(
                if is_selected { "> " } else { "  " },
                if is_selected {
                    PanelSpanStyle::Highlight
                } else {
                    PanelSpanStyle::Normal
                },
            ));
            spans.push(panel_span(
                format!("{}: {}", slot, equipped),
                if is_selected {
                    PanelSpanStyle::Highlight
                } else {
                    PanelSpanStyle::Normal
                },
            ));
            lines.push(panel_line_spans(spans));
        }
        lines.push(panel_line("------------------------------"));
        let detail_slot = slots.get(selection).cloned().unwrap_or_default();
        lines.extend(build_equipped_slot_detail(runtime, actor, &detail_slot));
    } else {
        let entries = equipment_entries_for_menu(runtime);
        if entries.is_empty() {
            return MenuPanelView {
                title: "Equipment".to_string(),
                lines: vec![panel_line("No equipment available.")],
            };
        }
        let selection = runtime
            .menu_state
            .detail_selection
            .min(entries.len().saturating_sub(1));
        let width = list_line_width(&entries);
        for (index, entry) in entries.iter().enumerate() {
            lines.push(build_list_line(entry, index == selection, width));
        }
        lines.push(panel_line("------------------------------"));
        lines.push(panel_line_spans(vec![panel_span(
            "Details",
            PanelSpanStyle::Accent,
        )]));
        if let Some(entry) = entries.get(selection) {
            let slot = equipment_slot_for_menu(runtime).unwrap_or_default();
            lines.extend(build_equipment_detail(runtime, &actor_id, &slot, entry).lines);
        }
    }

    MenuPanelView {
        title: "Equipment".to_string(),
        lines,
    }
}

fn equipment_header_line(name: &str) -> MenuPanelLine {
    panel_line_spans(vec![
        panel_span("Actor: ", PanelSpanStyle::Normal),
        panel_span(name, PanelSpanStyle::Highlight),
        panel_span("  (Left/Right)", PanelSpanStyle::Muted),
    ])
}

fn build_equipped_slot_detail(
    runtime: &GameRuntime,
    actor: &engine::party::Actor,
    slot: &str,
) -> Vec<MenuPanelLine> {
    let Some(item_id) = actor.equipment.get(slot) else {
        return vec![panel_line("Empty slot."), panel_line("Confirm to equip.")];
    };
    let entry = runtime
        .content
        .equipment
        .equipment
        .iter()
        .find(|item| item.id == *item_id);
    if let Some(item) = entry {
        let mut lines = Vec::new();
        lines.push(panel_line_spans(vec![
            panel_span("Equipped: ", PanelSpanStyle::Normal),
            panel_span(item.name.clone(), PanelSpanStyle::Accent),
        ]));
        lines.push(panel_line_spans(vec![
            panel_span("Slot: ", PanelSpanStyle::Normal),
            panel_span(item.slot.clone(), PanelSpanStyle::Accent),
        ]));
        lines.push(panel_line_spans(vec![
            panel_span("Category: ", PanelSpanStyle::Normal),
            panel_span(item.category.clone(), PanelSpanStyle::Accent),
        ]));
        lines.push(panel_line(""));
        for (stat, value) in &item.stats {
            lines.push(panel_line(format!("{} +{}", stat, value)));
        }
        lines
    } else {
        vec![panel_line("Item not found.")]
    }
}

fn inventory_filter_line(filter: &InventoryFilter, sort: &InventorySort) -> MenuPanelLine {
    let mut spans = Vec::new();
    spans.push(panel_span("Filter: ", PanelSpanStyle::Normal));
    for (index, entry) in inventory_filters().into_iter().enumerate() {
        if index > 0 {
            spans.push(panel_span(" | ", PanelSpanStyle::Muted));
        }
        let style = if entry == filter_label(filter) {
            PanelSpanStyle::Highlight
        } else {
            PanelSpanStyle::Muted
        };
        spans.push(panel_span(entry, style));
    }
    spans.push(panel_span("  Sort: ", PanelSpanStyle::Normal));
    spans.push(panel_span(sort_label(sort), PanelSpanStyle::Accent));
    panel_line_spans(spans)
}

fn build_item_description(
    runtime: &GameRuntime,
    entry: Option<&InventoryEntry>,
) -> Vec<MenuPanelLine> {
    let Some(entry) = entry else {
        return vec![panel_line("No selection.")];
    };
    match entry.kind {
        InventoryKind::Item => {
            let item = runtime
                .content
                .items
                .items
                .iter()
                .find(|item| item.id == entry.id);
            if let Some(item) = item {
                let mut lines = Vec::new();
                let power_text = item
                    .effect
                    .power
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string());
                lines.push(panel_line_spans(vec![
                    panel_span("Usage: ", PanelSpanStyle::Normal),
                    panel_span(item.usage.context.clone(), PanelSpanStyle::Accent),
                    panel_span("  Target: ", PanelSpanStyle::Normal),
                    panel_span(item.usage.target.clone(), PanelSpanStyle::Accent),
                ]));
                lines.push(panel_line_spans(vec![
                    panel_span("Effect: ", PanelSpanStyle::Normal),
                    panel_span(item.effect.r#type.clone(), PanelSpanStyle::Accent),
                    panel_span("  Power: ", PanelSpanStyle::Normal),
                    panel_span(power_text, PanelSpanStyle::Accent),
                ]));
                let description = item
                    .description
                    .clone()
                    .unwrap_or_else(|| "No description.".to_string());
                lines.push(panel_line_spans(vec![
                    panel_span("Description: ", PanelSpanStyle::Accent),
                    panel_span(description, PanelSpanStyle::Normal),
                ]));
                if !entry.usable {
                    lines.push(panel_line("Cannot use in field."));
                }
                lines
            } else {
                vec![panel_line("Item not found.")]
            }
        }
        InventoryKind::Equipment => vec![panel_line("Select equipment in Equipment menu.")],
    }
}

fn build_magic_panel(runtime: &GameRuntime) -> MenuPanelView {
    let actor_id = detail_actor_id(runtime);
    let Some(actor_id) = actor_id else {
        return MenuPanelView {
            title: "Magic".to_string(),
            lines: vec![panel_line("No party members.")],
        };
    };
    let Some(actor) = runtime.party.roster.get(&actor_id) else {
        return MenuPanelView {
            title: "Magic".to_string(),
            lines: vec![panel_line("No party members.")],
        };
    };
    let entries = build_spell_entries(runtime);
    let mut lines = Vec::new();
    lines.push(magic_header_line(runtime, actor));
    if entries.is_empty() {
        lines.push(panel_line("------------------------------"));
        lines.push(panel_line("No spells learned."));
        lines.push(panel_line("Learn spells to use magic."));
        lines.push(panel_line("------------------------------"));
        lines.push(panel_line_spans(vec![panel_span(
            "Details",
            PanelSpanStyle::Accent,
        )]));
        lines.push(panel_line("Select a learned spell."));
        return MenuPanelView {
            title: "Magic".to_string(),
            lines,
        };
    }
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    let width = spell_list_width(&entries);
    for (index, entry) in entries.iter().enumerate() {
        lines.push(build_spell_line(entry, index == selection, width));
    }
    lines.push(panel_line("------------------------------"));
    if runtime.menu_state.detail_page == 1 {
        lines.extend(build_spell_target_panel(
            runtime,
            entries.get(selection),
            &actor_id,
        ));
        lines.push(panel_line("------------------------------"));
    }
    lines.push(panel_line_spans(vec![panel_span(
        "Details",
        PanelSpanStyle::Accent,
    )]));
    lines.extend(build_spell_description(
        runtime,
        entries.get(selection),
        actor,
    ));

    MenuPanelView {
        title: "Magic".to_string(),
        lines,
    }
}

fn build_abilities_panel(runtime: &GameRuntime) -> MenuPanelView {
    let actor_id = detail_actor_id(runtime);
    let Some(actor_id) = actor_id else {
        return MenuPanelView {
            title: "Abilities".to_string(),
            lines: vec![panel_line("No party members.")],
        };
    };
    let Some(actor) = runtime.party.roster.get(&actor_id) else {
        return MenuPanelView {
            title: "Abilities".to_string(),
            lines: vec![panel_line("No party members.")],
        };
    };
    let entries = build_ability_entries(runtime);
    let mut lines = Vec::new();
    lines.push(ability_header_line(actor));
    if entries.is_empty() {
        lines.push(panel_line("------------------------------"));
        lines.push(panel_line("No abilities learned."));
        lines.push(panel_line("Gain levels to unlock abilities."));
        lines.push(panel_line("------------------------------"));
        lines.push(panel_line_spans(vec![panel_span(
            "Details",
            PanelSpanStyle::Accent,
        )]));
        lines.push(panel_line("Select a learned ability."));
        return MenuPanelView {
            title: "Abilities".to_string(),
            lines,
        };
    }
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    let width = ability_list_width(&entries);
    for (index, entry) in entries.iter().enumerate() {
        lines.push(build_ability_line(entry, index == selection, width));
    }
    lines.push(panel_line("------------------------------"));
    if runtime.menu_state.detail_page == 1 {
        lines.extend(build_ability_target_panel(
            runtime,
            entries.get(selection),
            &actor_id,
        ));
        lines.push(panel_line("------------------------------"));
    }
    lines.push(panel_line_spans(vec![panel_span(
        "Details",
        PanelSpanStyle::Accent,
    )]));
    lines.extend(build_ability_description(
        runtime,
        entries.get(selection),
        actor,
    ));

    MenuPanelView {
        title: "Abilities".to_string(),
        lines,
    }
}

fn ability_header_line(actor: &engine::party::Actor) -> MenuPanelLine {
    panel_line_spans(vec![
        panel_span("Actor: ", PanelSpanStyle::Normal),
        panel_span(actor.name.clone(), PanelSpanStyle::Highlight),
        panel_span("  (Left/Right)", PanelSpanStyle::Muted),
    ])
}

fn magic_header_line(runtime: &GameRuntime, actor: &engine::party::Actor) -> MenuPanelLine {
    match runtime.content.rules.game.magic_system {
        MagicSystem::Mp => {
            let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
            panel_line_spans(vec![
                panel_span("Actor: ", PanelSpanStyle::Normal),
                panel_span(actor.name.clone(), PanelSpanStyle::Highlight),
                panel_span("  (Left/Right)", PanelSpanStyle::Muted),
                panel_span("  MP ", PanelSpanStyle::Normal),
                panel_span(
                    format!("{}/{}", actor.current_mp, max_mp),
                    PanelSpanStyle::Accent,
                ),
            ])
        }
        MagicSystem::TierCharges => {
            let mut tiers = Vec::new();
            for tier in &runtime.content.rules.magic_tiers {
                let current = actor
                    .magic_tier_charges
                    .get(&tier.tier)
                    .copied()
                    .unwrap_or(0);
                tiers.push(format!("T{} {}/{}", tier.tier, current, tier.max_charges));
            }
            let charge_text = if tiers.is_empty() {
                "No charges".to_string()
            } else {
                tiers.join("  ")
            };
            panel_line_spans(vec![
                panel_span("Actor: ", PanelSpanStyle::Normal),
                panel_span(actor.name.clone(), PanelSpanStyle::Highlight),
                panel_span("  (Left/Right)", PanelSpanStyle::Muted),
                panel_span("  ", PanelSpanStyle::Normal),
                panel_span(charge_text, PanelSpanStyle::Accent),
            ])
        }
    }
}

fn build_spell_entries(runtime: &GameRuntime) -> Vec<SpellEntry> {
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Vec::new(),
    };
    let actor = match runtime.party.roster.get(&actor_id) {
        Some(actor) => actor,
        None => return Vec::new(),
    };
    let mut entries = Vec::new();
    let mut school_lookup = HashMap::new();
    for school in &runtime.content.spells.schools {
        school_lookup.insert(school.id.as_str(), school.name.as_str());
    }
    let mut spell_ids = collect_spell_ids(runtime, actor);
    spell_ids.sort();
    spell_ids.dedup();
    for spell_id in spell_ids {
        let Some(spell) = runtime
            .content
            .spells
            .spells
            .iter()
            .find(|spell| spell.id == spell_id)
        else {
            continue;
        };
        let school = school_lookup
            .get(spell.school.as_str())
            .copied()
            .unwrap_or(spell.school.as_str())
            .to_string();
        let (usable, reason) = spell_cast_status(runtime, actor, spell);
        entries.push(SpellEntry {
            id: spell.id.clone(),
            name: spell.name.clone(),
            school,
            tier: spell.tier,
            cost_type: spell.cost.r#type.clone(),
            cost_value: spell.cost.value,
            default_target: spell.default_target.clone(),
            allowed_targets: spell.allowed_targets.clone(),
            effect_type: spell.effect.r#type.clone(),
            effect_power: spell.effect.power,
            usable,
            reason,
        });
    }
    entries.sort_by(|left, right| {
        left.school
            .cmp(&right.school)
            .then(left.tier.cmp(&right.tier))
            .then(left.name.cmp(&right.name))
    });
    entries
}

fn build_ability_entries(runtime: &GameRuntime) -> Vec<AbilityEntry> {
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Vec::new(),
    };
    let actor = match runtime.party.roster.get(&actor_id) {
        Some(actor) => actor,
        None => return Vec::new(),
    };
    let mut entries = Vec::new();
    let mut ability_ids = collect_ability_ids(runtime, actor);
    ability_ids.sort();
    ability_ids.dedup();
    for ability_id in ability_ids {
        let Some(ability) = runtime
            .content
            .abilities
            .abilities
            .iter()
            .find(|ability| ability.id == ability_id)
        else {
            continue;
        };
        entries.push(AbilityEntry {
            id: ability.id.clone(),
            name: ability.name.clone(),
            default_target: ability.default_target.clone(),
            allowed_targets: ability.allowed_targets.clone(),
            effect_type: ability.effect.r#type.clone(),
            effect_power: ability.effect.power,
        });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}

fn collect_spell_ids(runtime: &GameRuntime, actor: &engine::party::Actor) -> Vec<String> {
    let mut ids = Vec::new();
    ids.extend(actor.spells.clone());
    let job = runtime
        .content
        .jobs
        .jobs
        .iter()
        .find(|job| job.id == actor.job_id);
    if let Some(job) = job {
        for spell in &job.spells {
            if !job_spell_available(runtime, actor, spell) {
                continue;
            }
            if !ids.contains(&spell.id) {
                ids.push(spell.id.clone());
            }
        }
    }
    ids
}

fn collect_ability_ids(runtime: &GameRuntime, actor: &engine::party::Actor) -> Vec<String> {
    let mut ids = Vec::new();
    let job = runtime
        .content
        .jobs
        .jobs
        .iter()
        .find(|job| job.id == actor.job_id);
    if let Some(job) = job {
        for ability in &job.abilities {
            if !job_ability_available(actor, ability) {
                continue;
            }
            if !ids.contains(&ability.id) {
                ids.push(ability.id.clone());
            }
        }
    }
    ids
}

fn job_spell_available(
    runtime: &GameRuntime,
    actor: &engine::party::Actor,
    spell: &engine::entities::JobSpell,
) -> bool {
    match spell.method.as_str() {
        "level" => spell.level.unwrap_or(0) <= actor.level,
        "tier" => spell.tier.unwrap_or(0) <= actor.level,
        "item" => match spell.item.as_deref() {
            Some(item_id) => runtime.inventory.item_qty(item_id) > 0,
            None => false,
        },
        _ => false,
    }
}

fn job_ability_available(
    actor: &engine::party::Actor,
    ability: &engine::entities::JobAbility,
) -> bool {
    match ability.method.as_str() {
        "level" => ability.level.unwrap_or(0) <= actor.level,
        _ => false,
    }
}

fn spell_cast_status(
    runtime: &GameRuntime,
    actor: &engine::party::Actor,
    spell: &engine::entities::SpellDefinition,
) -> (bool, Option<String>) {
    if spell.effect.r#type == "damage" {
        return (false, Some("Battle only.".to_string()));
    }
    if !spell_effect_allows_field(spell.effect.r#type.as_str()) {
        return (false, Some("Unsupported effect.".to_string()));
    }
    if !spell_target_allows_field(spell.default_target.as_str()) {
        return (false, Some("No field target.".to_string()));
    }
    let magic_system = runtime.content.rules.game.magic_system.clone();
    if !spell_system_matches(magic_system.clone(), spell.cost.r#type.as_str()) {
        return (false, Some("Cost system mismatch.".to_string()));
    }
    if !spell_cost_available(
        magic_system.clone(),
        actor,
        spell.cost.r#type.as_str(),
        spell.tier,
        spell.cost.value,
    ) {
        let reason = match magic_system {
            MagicSystem::Mp => "Not enough MP.",
            MagicSystem::TierCharges => "No tier charges.",
        };
        return (false, Some(reason.to_string()));
    }

    (true, None)
}

fn spell_effect_allows_field(effect: &str) -> bool {
    matches!(effect, "heal" | "revive")
}

fn spell_target_allows_field(target: &str) -> bool {
    matches!(target, "self" | "ally" | "party")
}

fn spell_system_matches(magic_system: MagicSystem, cost_type: &str) -> bool {
    match magic_system {
        MagicSystem::Mp => cost_type == "mp",
        MagicSystem::TierCharges => cost_type == "tier_charges",
    }
}

fn spell_cost_available(
    magic_system: MagicSystem,
    actor: &engine::party::Actor,
    cost_type: &str,
    tier: u32,
    cost_value: i32,
) -> bool {
    if !spell_system_matches(magic_system.clone(), cost_type) {
        return false;
    }
    match magic_system {
        MagicSystem::Mp => actor.current_mp >= cost_value,
        MagicSystem::TierCharges => {
            actor.magic_tier_charges.get(&tier).copied().unwrap_or(0) >= cost_value
        }
    }
}

fn apply_spell_to_targets(
    runtime: &mut GameRuntime,
    entry: &SpellEntry,
    actor_id: &str,
    targets: &[String],
) -> bool {
    let magic_system = runtime.content.rules.game.magic_system.clone();
    let Some(actor) = runtime.party.roster.get(actor_id) else {
        return false;
    };
    if !spell_cost_available(
        magic_system.clone(),
        actor,
        entry.cost_type.as_str(),
        entry.tier,
        entry.cost_value,
    ) {
        return false;
    }
    let Some(actor) = runtime.party.roster.get_mut(actor_id) else {
        return false;
    };
    if !consume_spell_cost(magic_system, actor, entry) {
        return false;
    }
    for target_id in targets {
        apply_spell_to_actor(runtime, entry, target_id);
    }
    true
}

fn consume_spell_cost(
    magic_system: MagicSystem,
    actor: &mut engine::party::Actor,
    entry: &SpellEntry,
) -> bool {
    match magic_system {
        MagicSystem::Mp => {
            if actor.current_mp < entry.cost_value {
                return false;
            }
            actor.current_mp = actor.current_mp.saturating_sub(entry.cost_value);
            true
        }
        MagicSystem::TierCharges => {
            let charges = actor.magic_tier_charges.entry(entry.tier).or_insert(0);
            if *charges < entry.cost_value {
                return false;
            }
            *charges -= entry.cost_value;
            true
        }
    }
}

fn apply_spell_to_actor(runtime: &mut GameRuntime, entry: &SpellEntry, actor_id: &str) {
    let Some(actor) = runtime.party.roster.get_mut(actor_id) else {
        return;
    };
    let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
    match entry.effect_type.as_str() {
        "heal" => {
            actor.current_hp = (actor.current_hp + entry.effect_power).clamp(0, max_hp);
        }
        "revive" => {
            if actor.current_hp <= 0 {
                let amount = if entry.effect_power > 0 {
                    entry.effect_power
                } else {
                    max_hp
                };
                actor.current_hp = amount.clamp(1, max_hp);
            }
        }
        _ => {}
    }
}

fn apply_ability_to_actor(runtime: &mut GameRuntime, entry: &AbilityEntry, actor_id: &str) {
    let Some(actor) = runtime.party.roster.get_mut(actor_id) else {
        return;
    };
    let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
    match entry.effect_type.as_str() {
        "heal" => {
            actor.current_hp = (actor.current_hp + entry.effect_power).clamp(0, max_hp);
        }
        "revive" => {
            if actor.current_hp <= 0 {
                let amount = if entry.effect_power > 0 {
                    entry.effect_power
                } else {
                    max_hp
                };
                actor.current_hp = amount.clamp(1, max_hp);
            }
        }
        _ => {}
    }
}

fn spell_list_width(entries: &[SpellEntry]) -> usize {
    entries
        .iter()
        .map(|entry| entry.name.chars().count())
        .max()
        .unwrap_or(10)
        + 2
}

fn build_spell_line(entry: &SpellEntry, is_selected: bool, width: usize) -> MenuPanelLine {
    let prefix = if is_selected { "> " } else { "  " };
    let label = format!("{:width$}", entry.name, width = width);
    let cost_text = spell_cost_label(entry);
    let base_style = if entry.usable {
        PanelSpanStyle::Normal
    } else {
        PanelSpanStyle::Muted
    };
    let style = if is_selected {
        PanelSpanStyle::Highlight
    } else {
        base_style
    };
    panel_line_spans(vec![
        panel_span(prefix, style),
        panel_span(label, style),
        panel_span(cost_text, style),
    ])
}

fn spell_cost_label(entry: &SpellEntry) -> String {
    match entry.cost_type.as_str() {
        "mp" => format!(" MP {}", entry.cost_value),
        "tier_charges" => format!(" T{} {}", entry.tier, entry.cost_value),
        _ => "".to_string(),
    }
}

fn ability_list_width(entries: &[AbilityEntry]) -> usize {
    entries
        .iter()
        .map(|entry| entry.name.chars().count())
        .max()
        .unwrap_or(10)
        + 2
}

fn build_ability_line(entry: &AbilityEntry, is_selected: bool, width: usize) -> MenuPanelLine {
    let prefix = if is_selected { "> " } else { "  " };
    let label = format!("{:width$}", entry.name, width = width);
    let style = if is_selected {
        PanelSpanStyle::Highlight
    } else {
        PanelSpanStyle::Normal
    };
    panel_line_spans(vec![panel_span(prefix, style), panel_span(label, style)])
}

fn selected_spell_targets(runtime: &GameRuntime) -> Vec<String> {
    let entries = build_spell_entries(runtime);
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Vec::new(),
    };
    match entries.get(selection) {
        Some(entry) => spell_targets_for_entry(runtime, entry, &actor_id),
        None => Vec::new(),
    }
}

fn selected_ability_targets(runtime: &GameRuntime) -> Vec<String> {
    let entries = build_ability_entries(runtime);
    let selection = runtime
        .menu_state
        .detail_selection
        .min(entries.len().saturating_sub(1));
    let actor_id = match detail_actor_id(runtime) {
        Some(actor_id) => actor_id,
        None => return Vec::new(),
    };
    match entries.get(selection) {
        Some(entry) => ability_targets_for_entry(runtime, entry, &actor_id),
        None => Vec::new(),
    }
}

fn build_spell_target_panel(
    runtime: &GameRuntime,
    entry: Option<&SpellEntry>,
    actor_id: &str,
) -> Vec<MenuPanelLine> {
    let Some(entry) = entry else {
        return vec![panel_line("No target."), panel_line("")];
    };
    let targets = spell_targets_for_entry(runtime, entry, actor_id);
    if targets.is_empty() {
        return vec![panel_line("No valid targets."), panel_line("")];
    }
    let selection = runtime
        .menu_state
        .detail_target
        .min(targets.len().saturating_sub(1));
    let mut lines = Vec::new();
    lines.push(panel_line_spans(vec![panel_span(
        "Target",
        PanelSpanStyle::Accent,
    )]));
    for (index, target_id) in targets.iter().enumerate() {
        let name = runtime
            .party
            .roster
            .get(target_id)
            .map(|actor| actor.name.as_str())
            .unwrap_or(target_id.as_str());
        let is_selected = index == selection;
        lines.push(panel_line_spans(vec![
            panel_span(
                if is_selected { "> " } else { "  " },
                if is_selected {
                    PanelSpanStyle::Highlight
                } else {
                    PanelSpanStyle::Normal
                },
            ),
            panel_span(
                name,
                if is_selected {
                    PanelSpanStyle::Highlight
                } else {
                    PanelSpanStyle::Normal
                },
            ),
        ]));
    }
    lines
}

fn build_ability_target_panel(
    runtime: &GameRuntime,
    entry: Option<&AbilityEntry>,
    actor_id: &str,
) -> Vec<MenuPanelLine> {
    let Some(entry) = entry else {
        return vec![panel_line("No target."), panel_line("")];
    };
    let targets = ability_targets_for_entry(runtime, entry, actor_id);
    if targets.is_empty() {
        return vec![panel_line("No valid targets."), panel_line("")];
    }
    let selection = runtime
        .menu_state
        .detail_target
        .min(targets.len().saturating_sub(1));
    let mut lines = Vec::new();
    lines.push(panel_line_spans(vec![panel_span(
        "Target",
        PanelSpanStyle::Accent,
    )]));
    for (index, target_id) in targets.iter().enumerate() {
        let name = runtime
            .party
            .roster
            .get(target_id)
            .map(|actor| actor.name.as_str())
            .unwrap_or(target_id.as_str());
        let is_selected = index == selection;
        lines.push(panel_line_spans(vec![
            panel_span(
                if is_selected { "> " } else { "  " },
                if is_selected {
                    PanelSpanStyle::Highlight
                } else {
                    PanelSpanStyle::Normal
                },
            ),
            panel_span(
                name,
                if is_selected {
                    PanelSpanStyle::Highlight
                } else {
                    PanelSpanStyle::Normal
                },
            ),
        ]));
    }
    lines
}

fn spell_targets_for_entry(
    runtime: &GameRuntime,
    entry: &SpellEntry,
    actor_id: &str,
) -> Vec<String> {
    let mut targets = match entry.default_target.as_str() {
        "self" => vec![actor_id.to_string()],
        "party" => runtime.party.active.clone(),
        "ally" => runtime.party.active.clone(),
        _ => Vec::new(),
    };
    if entry.effect_type == "revive" {
        targets.retain(|id| {
            runtime
                .party
                .roster
                .get(id)
                .map(|actor| actor.current_hp <= 0)
                .unwrap_or(false)
        });
    }
    targets
}

fn ability_targets_for_entry(
    runtime: &GameRuntime,
    entry: &AbilityEntry,
    actor_id: &str,
) -> Vec<String> {
    let mut targets = match entry.default_target.as_str() {
        "self" => vec![actor_id.to_string()],
        "party" => runtime.party.active.clone(),
        "ally" => runtime.party.active.clone(),
        _ => Vec::new(),
    };
    if entry.effect_type == "revive" {
        targets.retain(|id| {
            runtime
                .party
                .roster
                .get(id)
                .map(|actor| actor.current_hp <= 0)
                .unwrap_or(false)
        });
    }
    targets
}

fn build_spell_description(
    runtime: &GameRuntime,
    entry: Option<&SpellEntry>,
    actor: &engine::party::Actor,
) -> Vec<MenuPanelLine> {
    let Some(entry) = entry else {
        return vec![panel_line("No selection.")];
    };
    let mut lines = Vec::new();
    lines.push(panel_line_spans(vec![
        panel_span("School: ", PanelSpanStyle::Normal),
        panel_span(entry.school.clone(), PanelSpanStyle::Accent),
        panel_span("  Tier: ", PanelSpanStyle::Normal),
        panel_span(entry.tier.to_string(), PanelSpanStyle::Accent),
    ]));
    lines.push(panel_line_spans(vec![
        panel_span("ID: ", PanelSpanStyle::Normal),
        panel_span(entry.id.clone(), PanelSpanStyle::Accent),
    ]));
    match runtime.content.rules.game.magic_system {
        MagicSystem::Mp => {
            let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
            lines.push(panel_line_spans(vec![
                panel_span("Cost: ", PanelSpanStyle::Normal),
                panel_span(format!("MP {}", entry.cost_value), PanelSpanStyle::Accent),
                panel_span("  MP: ", PanelSpanStyle::Normal),
                panel_span(
                    format!("{}/{}", actor.current_mp, max_mp),
                    PanelSpanStyle::Accent,
                ),
            ]));
        }
        MagicSystem::TierCharges => {
            let current = actor
                .magic_tier_charges
                .get(&entry.tier)
                .copied()
                .unwrap_or(0);
            let max = runtime
                .content
                .rules
                .magic_tiers
                .iter()
                .find(|tier| tier.tier == entry.tier)
                .map(|tier| tier.max_charges)
                .unwrap_or(0);
            lines.push(panel_line_spans(vec![
                panel_span("Cost: ", PanelSpanStyle::Normal),
                panel_span(
                    format!("T{} {}", entry.tier, entry.cost_value),
                    PanelSpanStyle::Accent,
                ),
                panel_span("  Charges: ", PanelSpanStyle::Normal),
                panel_span(format!("{}/{}", current, max), PanelSpanStyle::Accent),
            ]));
        }
    }
    let allowed_targets = if entry.allowed_targets.is_empty() {
        entry.default_target.clone()
    } else {
        entry.allowed_targets.join(", ")
    };
    lines.push(panel_line_spans(vec![
        panel_span("Target: ", PanelSpanStyle::Normal),
        panel_span(entry.default_target.clone(), PanelSpanStyle::Accent),
    ]));
    lines.push(panel_line_spans(vec![
        panel_span("Allowed: ", PanelSpanStyle::Normal),
        panel_span(allowed_targets, PanelSpanStyle::Accent),
    ]));
    lines.push(panel_line_spans(vec![
        panel_span("Effect: ", PanelSpanStyle::Normal),
        panel_span(entry.effect_type.clone(), PanelSpanStyle::Accent),
        panel_span("  Power: ", PanelSpanStyle::Normal),
        panel_span(entry.effect_power.to_string(), PanelSpanStyle::Accent),
    ]));
    if let Some(reason) = &entry.reason {
        lines.push(panel_line_spans(vec![
            panel_span("Unavailable: ", PanelSpanStyle::Muted),
            panel_span(reason.clone(), PanelSpanStyle::Muted),
        ]));
    }
    lines
}

fn build_ability_description(
    runtime: &GameRuntime,
    entry: Option<&AbilityEntry>,
    actor: &engine::party::Actor,
) -> Vec<MenuPanelLine> {
    let Some(entry) = entry else {
        return vec![panel_line("No selection.")];
    };
    let mut lines = Vec::new();
    lines.push(panel_line_spans(vec![
        panel_span("Actor: ", PanelSpanStyle::Normal),
        panel_span(actor.name.clone(), PanelSpanStyle::Highlight),
    ]));
    lines.push(panel_line_spans(vec![
        panel_span("ID: ", PanelSpanStyle::Normal),
        panel_span(entry.id.clone(), PanelSpanStyle::Accent),
    ]));
    let allowed_targets = if entry.allowed_targets.is_empty() {
        entry.default_target.clone()
    } else {
        entry.allowed_targets.join(", ")
    };
    lines.push(panel_line_spans(vec![
        panel_span("Target: ", PanelSpanStyle::Normal),
        panel_span(entry.default_target.clone(), PanelSpanStyle::Accent),
    ]));
    lines.push(panel_line_spans(vec![
        panel_span("Allowed: ", PanelSpanStyle::Normal),
        panel_span(allowed_targets, PanelSpanStyle::Accent),
    ]));
    lines.push(panel_line_spans(vec![
        panel_span("Effect: ", PanelSpanStyle::Normal),
        panel_span(entry.effect_type.clone(), PanelSpanStyle::Accent),
        panel_span("  Power: ", PanelSpanStyle::Normal),
        panel_span(entry.effect_power.to_string(), PanelSpanStyle::Accent),
    ]));
    let description = runtime
        .content
        .abilities
        .abilities
        .iter()
        .find(|ability| ability.id == entry.id)
        .and_then(|ability| ability.description.clone())
        .unwrap_or_else(|| "Battle-only ability.".to_string());
    lines.push(panel_line_spans(vec![
        panel_span("Description: ", PanelSpanStyle::Accent),
        panel_span(description, PanelSpanStyle::Normal),
    ]));
    lines
}

fn menu_default_panel(menu_ui: &MenuUiFile, runtime: &GameRuntime) -> MenuPanelView {
    let panel = menu_ui
        .panels
        .iter()
        .find(|panel| panel.id == menu_ui.default_panel);
    let (title, panel_type) = match panel {
        Some(panel) => (panel.title.clone(), panel.panel_type.as_str()),
        None => ("Status".to_string(), "unknown"),
    };
    match panel_type {
        "party_summary" => MenuPanelView {
            title,
            lines: build_party_summary(runtime),
        },
        "progress" => MenuPanelView {
            title,
            lines: vec![
                panel_line("Progress panel (stub)."),
                panel_line("TODO: render ui/progress.json."),
            ],
        },
        _ => MenuPanelView {
            title,
            lines: vec![panel_line("Menu panel not configured.")],
        },
    }
}

fn build_menu_stats_view(runtime: &GameRuntime) -> MenuPanelView {
    let current_session = runtime.start_time.elapsed().as_secs();
    let total_seconds = runtime.playtime + current_session;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    let currency_id = &runtime.content.rules.game.currency.id;
    let currency_symbol = &runtime.content.rules.game.currency.symbol;
    let currency_amount = runtime.inventory.currency_amount(currency_id);

    MenuPanelView {
        title: String::new(),
        lines: vec![
            panel_line(format!("Time: {:02}:{:02}:{:02}", hours, minutes, seconds)),
            panel_line(format!("{}: {}", currency_symbol, currency_amount)),
        ],
    }
}

fn build_party_summary(runtime: &GameRuntime) -> Vec<MenuPanelLine> {
    if runtime.party.active.is_empty() {
        return vec![panel_line("No party members.")];
    }
    let mut lines = Vec::new();
    for member_id in &runtime.party.active {
        if let Some(actor) = runtime.party.roster.get(member_id) {
            let max_hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
            let max_mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
            let job_name = runtime
                .content
                .jobs
                .jobs
                .iter()
                .find(|job| job.id == actor.job_id)
                .map(|job| job.name.as_str())
                .unwrap_or(actor.job_id.as_str());
            lines.push(panel_line(format!(
                "{}  Lv{}  HP {}/{}  MP {}/{}",
                actor.name, actor.level, actor.current_hp, max_hp, actor.current_mp, max_mp
            )));
            lines.push(panel_line(format!("Job: {}", job_name)));
            lines.push(panel_line(""));
        }
    }
    lines
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

fn map_save_allowed(runtime: &GameRuntime, map_id: &str, player_pos: (i32, i32)) -> bool {
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    map.allow_save
        || map
            .save_points
            .iter()
            .any(|pos| (pos[0], pos[1]) == player_pos)
}

fn system_enabled(runtime: &GameRuntime, system: Option<&str>) -> bool {
    match system {
        Some(key) if !key.trim().is_empty() => runtime
            .content
            .rules
            .systems
            .get(key)
            .copied()
            .unwrap_or(false),
        _ => true,
    }
}

fn unlock_flag_enabled(runtime: &GameRuntime, unlock_flag: Option<&str>) -> bool {
    match unlock_flag {
        Some(flag) if !flag.trim().is_empty() => runtime.has_flag(flag),
        _ => true,
    }
}

fn read_action(bindings: &tui::input::InputBindings) -> Option<Action> {
    if let crossterm::event::Event::Key(key) = crossterm::event::read().ok()? {
        return bindings.action_for(key.code);
    }
    None
}

fn is_returning_from_child(
    return_positions: &HashMap<String, (String, (i32, i32))>,
    current_map_id: &str,
    target_map_id: &str,
) -> bool {
    return_positions
        .get(current_map_id)
        .map(|(return_map, _)| return_map == target_map_id)
        .unwrap_or(false)
}

fn build_map_view(runtime: &GameRuntime, map_id: &str) -> Option<MapView> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let npcs = map
        .npcs
        .iter()
        .map(|npc| NpcView {
            id: npc.id.clone(),
            pos: (npc.pos[0], npc.pos[1]),
            glyph: npc_glyph(runtime, &npc.id),
            palette: runtime
                .content
                .npcs
                .npcs
                .iter()
                .find(|entry| entry.id == npc.id)
                .and_then(|entry| entry.palette.clone()),
        })
        .collect();
    let signs = map
        .signs
        .iter()
        .map(|sign| tui::app::SignView {
            id: sign.id.clone(),
            pos: (sign.pos[0], sign.pos[1]),
            glyph: sign
                .glyph
                .as_ref()
                .and_then(|glyph| glyph.chars().next())
                .unwrap_or('⚑'),
            palette: sign.palette.clone(),
            text: sign.text.clone(),
        })
        .collect();
    let chests = map
        .chests
        .iter()
        .map(|chest| tui::app::ChestView {
            id: chest.id.clone(),
            pos: (chest.pos[0], chest.pos[1]),
            glyph_closed: chest
                .glyph_closed
                .as_ref()
                .and_then(|glyph| glyph.chars().next())
                .unwrap_or('▣'),
            glyph_open: chest
                .glyph_open
                .as_ref()
                .and_then(|glyph| glyph.chars().next())
                .unwrap_or('▢'),
            palette: chest.palette.clone(),
            opened: runtime.has_flag(&chest.opened_flag),
        })
        .collect();
    let save_points = map.save_points.iter().map(|pos| (pos[0], pos[1])).collect();
    let legend = map
        .legend
        .iter()
        .filter_map(|(glyph, entry)| {
            let key = glyph.chars().next()?;
            Some((
                key,
                TileRender {
                    palette: entry.palette.clone(),
                },
            ))
        })
        .collect();
    let transitions = map
        .transitions
        .iter()
        .map(|transition| TransitionView {
            pos: (transition.pos[0], transition.pos[1]),
            glyph: transition
                .glyph
                .as_ref()
                .and_then(|glyph| glyph.chars().next()),
            palette: transition.palette.clone(),
        })
        .collect();
    let use_color = runtime
        .content
        .rules
        .render
        .palette
        .eq_ignore_ascii_case("terminal");

    Some(MapView {
        name: map.name.clone(),
        hide_name: map.hide_name,
        width: map.width as u16,
        height: map.height as u16,
        tiles: map.tiles.clone(),
        legend,
        transitions,
        npcs,
        signs,
        chests,
        save_points,
        use_color,
    })
}

fn build_shop_view(runtime: &GameRuntime, shop_id: &str) -> Option<ShopView> {
    let shop = runtime
        .content
        .shops
        .shops
        .iter()
        .find(|shop| shop.id == shop_id)?;

    let items = shop
        .inventory
        .iter()
        .map(|entry| ShopItem {
            name: lookup_item_name(runtime, &entry.item),
            price: entry.price,
        })
        .collect();

    Some(ShopView {
        name: shop.name.clone(),
        items,
    })
}

fn lookup_item_name(runtime: &GameRuntime, item_id: &str) -> String {
    if let Some(item) = runtime
        .content
        .items
        .items
        .iter()
        .find(|item| item.id == item_id)
    {
        return item.name.clone();
    }
    if let Some(item) = runtime
        .content
        .equipment
        .equipment
        .iter()
        .find(|item| item.id == item_id)
    {
        return item.name.clone();
    }
    item_id.to_string()
}

fn is_passable(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    if pos.0 < 0 || pos.1 < 0 || pos.0 >= map.width as i32 || pos.1 >= map.height as i32 {
        return false;
    }
    let tile = map
        .tiles
        .get(pos.1 as usize)
        .and_then(|row| row.chars().nth(pos.0 as usize))
        .unwrap_or(' ');
    let key = tile.to_string();
    map.legend
        .get(&key)
        .map(|entry| entry.passable)
        .unwrap_or(false)
}

fn find_transition(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<engine::maps::MapTransition> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    map.transitions
        .iter()
        .find(|transition| transition.pos == [pos.0, pos.1])
        .cloned()
}

fn npc_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    map.npcs.iter().any(|npc| (npc.pos[0], npc.pos[1]) == pos)
}

fn chest_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    map.chests
        .iter()
        .any(|chest| (chest.pos[0], chest.pos[1]) == pos)
}

fn sign_at(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> bool {
    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return false,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return false,
    };
    map.signs
        .iter()
        .any(|sign| (sign.pos[0], sign.pos[1]) == pos)
}

fn find_npc_dialog(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> Option<String> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let target = map.npcs.iter().find(|npc| {
        let dx = (npc.pos[0] - pos.0).abs();
        let dy = (npc.pos[1] - pos.1).abs();
        (dx == 1 && dy == 0) || (dx == 0 && dy == 1)
    })?;

    runtime
        .content
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.id == target.id)
        .map(|npc| npc.dialog.clone())
}

fn find_chest(
    runtime: &GameRuntime,
    map_id: &str,
    pos: (i32, i32),
) -> Option<engine::maps::MapChest> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let chest = map.chests.iter().find(|chest| {
        let dx = (chest.pos[0] - pos.0).abs();
        let dy = (chest.pos[1] - pos.1).abs();
        (dx == 1 && dy == 0) || (dx == 0 && dy == 1)
    })?;
    Some(chest.clone())
}

fn find_sign_text(runtime: &GameRuntime, map_id: &str, pos: (i32, i32)) -> Option<String> {
    let index = runtime.content.map_index.get(map_id)?;
    let map = runtime.content.maps.get(*index)?;
    let sign = map.signs.iter().find(|sign| {
        let dx = (sign.pos[0] - pos.0).abs();
        let dy = (sign.pos[1] - pos.1).abs();
        (dx == 1 && dy == 0) || (dx == 0 && dy == 1)
    })?;
    Some(sign.text.clone())
}

fn open_chest(runtime: &mut GameRuntime, chest: &engine::maps::MapChest) -> String {
    if runtime.has_flag(&chest.opened_flag) {
        return "The chest is empty.".to_string();
    }

    let max_stack = runtime.content.rules.inventory.max_stack;
    let mut found = Vec::new();

    for item in &chest.loot.items {
        if item.qty <= 0 {
            continue;
        }
        runtime.inventory.add_item(&item.id, item.qty, max_stack);
        found.push(format!(
            "{} x{}",
            lookup_item_name(runtime, &item.id),
            item.qty
        ));
    }

    for item in &chest.loot.equipment {
        if item.qty <= 0 {
            continue;
        }
        runtime
            .inventory
            .add_equipment(&item.id, item.qty, max_stack);
        found.push(format!(
            "{} x{}",
            lookup_item_name(runtime, &item.id),
            item.qty
        ));
    }

    for currency in &chest.loot.currency {
        if currency.amount <= 0 {
            continue;
        }
        runtime
            .inventory
            .add_currency(&currency.id, currency.amount);
        found.push(format_currency_stack(&runtime.content.rules, currency));
    }

    runtime.set_flag(&chest.opened_flag);

    if found.is_empty() {
        "The chest is empty.".to_string()
    } else {
        format!("Found: {}.", found.join(", "))
    }
}

fn format_currency_stack(
    rules: &engine::rules::RulesFile,
    currency: &engine::maps::MapCurrencyStack,
) -> String {
    if currency.id == rules.game.currency.id {
        format!("{}{}", rules.game.currency.symbol, currency.amount)
    } else {
        format!("{} {}", currency.amount, currency.id)
    }
}

fn npc_glyph(runtime: &GameRuntime, npc_id: &str) -> char {
    runtime
        .content
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.id == npc_id)
        .and_then(|npc| npc.name.chars().next())
        .unwrap_or('N')
        .to_ascii_uppercase()
}

fn find_spawn(runtime: &GameRuntime, map_id: &str, fallback: (i32, i32)) -> (i32, i32) {
    if is_passable(runtime, map_id, fallback) {
        return fallback;
    }

    let index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return fallback,
    };
    let map = match runtime.content.maps.get(index) {
        Some(map) => map,
        None => return fallback,
    };

    for y in 0..map.height as i32 {
        for x in 0..map.width as i32 {
            if is_passable(runtime, map_id, (x, y)) {
                return (x, y);
            }
        }
    }

    fallback
}

fn default_dialog_ui() -> DialogUiFile {
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
    bindings: &tui::input::InputBindings,
    action: &engine::dialog::DialogAction,
) -> std::io::Result<bool> {
    match action.r#type.as_str() {
        "start_event" => {
            if let Some(event_id) = &action.event {
                runtime.queue_event(event_id);
            }
        }
        "open_shop" => {
            if let Some(shop) = &action.shop {
                open_shop(runtime, session, bindings, shop)?;
                return Ok(true);
            }
        }
        "set_flag" => {
            let _ = &action.flag;
        }
        "give_item" => {
            let _ = &action.item;
        }
        _ => {
            println!("Dialog action: {}", action.r#type);
        }
    }
    Ok(false)
}

fn handle_dialog_action_console(runtime: &mut GameRuntime, action: &engine::dialog::DialogAction) {
    match action.r#type.as_str() {
        "start_event" => {
            if let Some(event_id) = &action.event {
                runtime.queue_event(event_id);
            }
        }
        "open_shop" => {
            if let Some(shop) = &action.shop {
                println!("Open shop: {}", shop);
            }
        }
        "set_flag" => {
            if let Some(flag) = &action.flag {
                println!("Set flag: {}", flag);
            }
        }
        "give_item" => {
            if let Some(item) = &action.item {
                let qty = action.qty.unwrap_or(1);
                println!("Give item: {} x{}", item, qty);
            }
        }
        _ => {
            println!("Dialog action: {}", action.r#type);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum BattlePhase {
    Command,
    Magic,
    Abilities,
    Items,
    TargetEnemy,
    TargetParty,
    Victory,
    Defeat,
}

#[derive(Clone, Debug)]
enum PendingBattleAction {
    Attack,
    Magic(SpellEntry),
    Ability(AbilityEntry),
    Item(String),
}

struct BattleMenuState {
    phase: BattlePhase,
    command_index: usize,
    enemy_index: usize,
    party_index: usize,
    magic_index: usize,
    ability_index: usize,
    item_index: usize,
    pending_action: Option<PendingBattleAction>,
}

impl BattleMenuState {
    fn new() -> Self {
        Self {
            phase: BattlePhase::Command,
            command_index: 0,
            enemy_index: 0,
            party_index: 0,
            magic_index: 0,
            ability_index: 0,
            item_index: 0,
            pending_action: None,
        }
    }

    fn reset_for_actor(&mut self) {
        self.phase = BattlePhase::Command;
        self.pending_action = None;
    }
}

#[derive(Clone, Debug)]
enum BattleTurnActor {
    Party(usize),
    Enemy(usize),
}

struct BattleTurnState {
    order: Vec<BattleTurnActor>,
    index: usize,
}

impl BattleTurnState {
    fn new(order: Vec<BattleTurnActor>) -> Self {
        Self { order, index: 0 }
    }

    fn reset(&mut self, order: Vec<BattleTurnActor>) {
        self.order = order;
        self.index = 0;
    }
}

#[derive(Clone, Copy)]
enum TargetRule {
    Alive,
    KnockedOut,
}

fn enemy_target_indices(battle_state: &BattleState) -> Vec<usize> {
    battle_state
        .enemies
        .iter()
        .enumerate()
        .filter_map(|(index, enemy)| if enemy.is_alive() { Some(index) } else { None })
        .collect()
}

fn party_target_rule(action: &PendingBattleAction, runtime: &GameRuntime) -> TargetRule {
    match action {
        PendingBattleAction::Magic(entry) => target_rule_for_effect(entry.effect_type.as_str()),
        PendingBattleAction::Ability(entry) => target_rule_for_effect(entry.effect_type.as_str()),
        PendingBattleAction::Item(item_id) => runtime
            .content
            .items
            .items
            .iter()
            .find(|item| item.id == *item_id)
            .map(|item| target_rule_for_effect(item.effect.r#type.as_str()))
            .unwrap_or(TargetRule::Alive),
        PendingBattleAction::Attack => TargetRule::Alive,
    }
}

fn target_rule_for_effect(effect_type: &str) -> TargetRule {
    match effect_type {
        "revive" => TargetRule::KnockedOut,
        _ => TargetRule::Alive,
    }
}

fn party_target_indices(
    runtime: &GameRuntime,
    battle_state: &BattleState,
    rule: TargetRule,
) -> Vec<usize> {
    battle_state
        .party_order
        .iter()
        .enumerate()
        .filter_map(|(index, id)| {
            let alive = runtime
                .party
                .roster
                .get(id)
                .map(|actor| actor.current_hp > 0)
                .unwrap_or(false);
            match rule {
                TargetRule::Alive if alive => Some(index),
                TargetRule::KnockedOut if !alive => Some(index),
                _ => None,
            }
        })
        .collect()
}

fn step_target_index(current: usize, valid: &[usize], direction: i32) -> usize {
    if valid.is_empty() {
        return current;
    }
    let position = valid
        .iter()
        .position(|index| *index == current)
        .unwrap_or(0);
    let len = valid.len();
    let next = if direction >= 0 {
        (position + 1) % len
    } else {
        (position + len - 1) % len
    };
    valid[next]
}

fn ensure_valid_index(current: usize, valid: &[usize]) -> Option<usize> {
    if valid.is_empty() {
        return None;
    }
    if valid.contains(&current) {
        Some(current)
    } else {
        valid.first().copied()
    }
}

fn set_initial_enemy_target(menu_state: &mut BattleMenuState, battle_state: &BattleState) -> bool {
    let valid = enemy_target_indices(battle_state);
    if let Some(index) = ensure_valid_index(menu_state.enemy_index, &valid) {
        menu_state.enemy_index = index;
        true
    } else {
        false
    }
}

fn set_initial_party_target(
    menu_state: &mut BattleMenuState,
    battle_state: &BattleState,
    runtime: &GameRuntime,
    action: &PendingBattleAction,
) -> bool {
    let rule = party_target_rule(action, runtime);
    let valid = party_target_indices(runtime, battle_state, rule);
    if let Some(index) = ensure_valid_index(menu_state.party_index, &valid) {
        menu_state.party_index = index;
        true
    } else {
        false
    }
}

fn build_turn_order(runtime: &GameRuntime, battle_state: &BattleState) -> Vec<BattleTurnActor> {
    let mut entries: Vec<(i32, u8, usize, BattleTurnActor)> = Vec::new();
    for (index, id) in battle_state.party_order.iter().enumerate() {
        if let Some(actor) = runtime.party.roster.get(id) {
            if actor.current_hp <= 0 {
                continue;
            }
            let speed = actor.base_stats.get("agi").copied().unwrap_or(1).max(1);
            entries.push((speed, 0, index, BattleTurnActor::Party(index)));
        }
    }
    for (index, enemy) in battle_state.enemies.iter().enumerate() {
        if !enemy.is_alive() {
            continue;
        }
        let speed = enemy.stats.get("agi").copied().unwrap_or(1).max(1);
        entries.push((speed, 1, index, BattleTurnActor::Enemy(index)));
    }
    entries.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then(left.1.cmp(&right.1))
            .then(left.2.cmp(&right.2))
    });
    entries.into_iter().map(|entry| entry.3).collect()
}

fn advance_turn(menu_state: &mut BattleMenuState, turn_state: &mut BattleTurnState) {
    turn_state.index = turn_state.index.saturating_add(1);
    menu_state.reset_for_actor();
}

fn enemy_take_turn(
    runtime: &mut GameRuntime,
    battle_state: &mut BattleState,
    enemy_index: usize,
    rng: &mut impl Rng,
) -> Option<usize> {
    let Some(enemy) = battle_state.enemies.get_mut(enemy_index) else {
        return None;
    };
    if !enemy.is_alive() {
        return None;
    }
    let living_party = battle_state
        .party_order
        .iter()
        .filter(|id| {
            runtime
                .party
                .roster
                .get(*id)
                .map(|actor| actor.current_hp > 0)
                .unwrap_or(false)
        })
        .cloned()
        .collect::<Vec<_>>();
    if living_party.is_empty() {
        return None;
    }
    let Some(target_id) = living_party.choose(rng).cloned() else {
        return None;
    };
    let Some(target) = runtime.party.roster.get_mut(&target_id) else {
        return None;
    };
    let def = target.derived_stats.get("def").copied().unwrap_or(0);
    let damage = physical_damage(enemy.atk(), def, rng);
    apply_damage_to_actor(target, damage);
    push_battle_log(
        &mut battle_state.log,
        format!("{} attacks {} for {} HP.", enemy.name, target.name, damage),
    );
    if target.current_hp <= 0 {
        push_battle_log(&mut battle_state.log, format!("{} falls!", target.name));
    }
    battle_state
        .party_order
        .iter()
        .position(|id| id == &target_id)
}

fn try_start_random_battle(
    runtime: &mut GameRuntime,
    battle_ui: &BattleUiFile,
    bindings: &tui::input::InputBindings,
    session: &mut TuiSession,
    map_id: &str,
    player_pos: (i32, i32),
    rng: &mut impl Rng,
) -> std::io::Result<Option<BattleOutcome>> {
    let map_index = match runtime.content.map_index.get(map_id) {
        Some(index) => *index,
        None => return Ok(None),
    };
    let map = match runtime.content.maps.get(map_index) {
        Some(map) => map,
        None => return Ok(None),
    };
    if map.encounter_rate <= 0.0 || map.encounters.is_empty() {
        return Ok(None);
    }
    let Some(zone) = encounter_zone_for_pos(map, player_pos) else {
        return Ok(None);
    };
    if rng.r#gen::<f32>() > map.encounter_rate {
        return Ok(None);
    }
    let entry = match select_encounter_entry(&runtime.content.encounters, &zone.table, rng) {
        Some(entry) => entry,
        None => return Ok(None),
    };
    let outcome = run_battle(runtime, battle_ui, bindings, session, &entry.formation, rng)?;
    Ok(Some(outcome))
}

fn run_event_battle(
    runtime: &mut GameRuntime,
    battle_ui: &BattleUiFile,
    bindings: &tui::input::InputBindings,
    session: &mut TuiSession,
    step: &engine::events::EventStep,
) -> std::io::Result<BattleOutcome> {
    let mut rng = rand::thread_rng();
    let formation = if let Some(formation) = &step.formation {
        formation
            .iter()
            .map(|member| engine::encounters::EncounterMember {
                enemy: member.enemy.clone(),
                pos: member.pos,
            })
            .collect()
    } else if let Some(encounter) = &step.encounter {
        match select_encounter_entry(&runtime.content.encounters, encounter, &mut rng) {
            Some(entry) => entry.formation,
            None => Vec::new(),
        }
    } else {
        Vec::new()
    };
    if formation.is_empty() {
        return Ok(BattleOutcome::Victory);
    }
    run_battle(runtime, battle_ui, bindings, session, &formation, &mut rng)
}

fn run_event_battle_with_result(
    runtime: &mut GameRuntime,
    battle_ui: &BattleUiFile,
    bindings: &tui::input::InputBindings,
    session: &mut TuiSession,
    encounter_id: &str,
    formation: &[engine::events::FormationMember],
) -> std::io::Result<BattleOutcome> {
    let mut rng = rand::thread_rng();
    let formation = if formation.is_empty() {
        if encounter_id.is_empty() {
            Vec::new()
        } else {
            match select_encounter_entry(&runtime.content.encounters, encounter_id, &mut rng) {
                Some(entry) => entry.formation,
                None => Vec::new(),
            }
        }
    } else {
        formation
            .iter()
            .map(|member| engine::encounters::EncounterMember {
                enemy: member.enemy.clone(),
                pos: member.pos,
            })
            .collect()
    };
    if formation.is_empty() {
        return Ok(BattleOutcome::Victory);
    }
    run_battle(runtime, battle_ui, bindings, session, &formation, &mut rng)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum VictoryState {
    Summary,
    LevelUp(usize),
}

fn run_battle(
    runtime: &mut GameRuntime,
    battle_ui: &BattleUiFile,
    bindings: &tui::input::InputBindings,
    session: &mut TuiSession,
    formation: &[engine::encounters::EncounterMember],
    rng: &mut impl Rng,
) -> std::io::Result<BattleOutcome> {
    let mut battle_state = build_battle_state(&runtime.content, &runtime.party, formation);
    let Some(start_index) = next_living_party_index(
        &runtime.party,
        &battle_state.party_order,
        battle_state.active_index,
    ) else {
        return Ok(BattleOutcome::Defeat);
    };
    battle_state.active_index = start_index;
    push_battle_log(&mut battle_state.log, "A battle begins!");

    let mut menu_state = BattleMenuState::new();
    let mut turn_state = BattleTurnState::new(build_turn_order(runtime, &battle_state));
    let mut last_actor_id: Option<String> = None;
    let mut battle_result: Option<BattleResult> = None;
    let mut victory_state: Option<VictoryState> = None;

    loop {
        if is_enemies_defeated(&battle_state.enemies) {
            if menu_state.phase != BattlePhase::Victory {
                let empty_spells: Vec<SpellEntry> = Vec::new();
                let empty_abilities: Vec<AbilityEntry> = Vec::new();
                let empty_items: Vec<InventoryEntry> = Vec::new();
                let render_state = build_battle_render_state(
                    runtime,
                    &battle_state,
                    &menu_state,
                    battle_ui,
                    &empty_spells,
                    &empty_abilities,
                    &empty_items,
                );
                pause_after_action(
                    session,
                    battle_ui,
                    bindings,
                    &render_state,
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                )?;
                menu_state.phase = BattlePhase::Victory;
                battle_result = Some(apply_battle_rewards(runtime, &mut battle_state, rng));
                victory_state = if battle_result
                    .as_ref()
                    .map(|r| !r.level_ups.is_empty())
                    .unwrap_or(false)
                {
                    Some(VictoryState::LevelUp(0))
                } else {
                    Some(VictoryState::Summary)
                };
                push_battle_log(&mut battle_state.log, "Victory!");
            }
        }
        if is_party_defeated(&runtime.party, &battle_state.party_order) {
            if menu_state.phase != BattlePhase::Defeat {
                menu_state.phase = BattlePhase::Defeat;
                push_battle_log(&mut battle_state.log, "Defeat...");
            }
        }

        if turn_state.order.is_empty() || turn_state.index >= turn_state.order.len() {
            turn_state.reset(build_turn_order(runtime, &battle_state));
        }
        if turn_state.order.is_empty() {
            return Ok(BattleOutcome::Defeat);
        }

        let current_turn = turn_state.order.get(turn_state.index).cloned();
        let Some(current_turn) = current_turn else {
            return Ok(BattleOutcome::Defeat);
        };

        let mut actor_id = battle_state
            .party_order
            .get(battle_state.active_index)
            .cloned()
            .unwrap_or_default();

        if !matches!(menu_state.phase, BattlePhase::Victory | BattlePhase::Defeat) {
            match current_turn {
                BattleTurnActor::Party(party_index) => {
                    let Some(current_id) = battle_state.party_order.get(party_index).cloned()
                    else {
                        advance_turn(&mut menu_state, &mut turn_state);
                        continue;
                    };
                    if let Some(actor) = runtime.party.roster.get(&current_id) {
                        if actor.current_hp <= 0 {
                            advance_turn(&mut menu_state, &mut turn_state);
                            continue;
                        }
                    }
                    battle_state.active_index = party_index;
                    if last_actor_id.as_deref() != Some(current_id.as_str()) {
                        menu_state.reset_for_actor();
                        last_actor_id = Some(current_id.clone());
                    }
                    actor_id = current_id;
                }
                BattleTurnActor::Enemy(enemy_index) => {
                    if let Some(target_index) =
                        enemy_take_turn(runtime, &mut battle_state, enemy_index, rng)
                    {
                        let render_state = build_battle_render_state(
                            runtime,
                            &battle_state,
                            &menu_state,
                            battle_ui,
                            &[],
                            &[],
                            &[],
                        );
                        pause_after_action(
                            session,
                            battle_ui,
                            bindings,
                            &render_state,
                            vec![enemy_index],
                            Vec::new(),
                            Vec::new(),
                            vec![target_index],
                        )?;
                    }
                    advance_turn(&mut menu_state, &mut turn_state);
                    continue;
                }
            }
        }

        let spell_entries = build_battle_spell_entries(runtime, &actor_id);
        let ability_entries = build_battle_ability_entries(runtime, &actor_id);
        let item_entries = build_battle_item_entries(runtime);
        let render_state = build_battle_render_state(
            runtime,
            &battle_state,
            &menu_state,
            battle_ui,
            &spell_entries,
            &ability_entries,
            &item_entries,
        );

        if menu_state.phase == BattlePhase::Victory {
            match victory_state {
                Some(VictoryState::Summary) => {
                    if let Some(ref result) = battle_result {
                        tui::app::draw_victory_summary(
                            session,
                            result.rewards.exp,
                            result.rewards.currency,
                            &runtime.content.rules.game.currency.symbol,
                            &result.rewards.items,
                        )?;
                    }
                }
                Some(VictoryState::LevelUp(index)) => {
                    if let Some(ref result) = battle_result {
                        if let Some(diff) = result.level_ups.get(index) {
                            tui::app::draw_level_up_modal(
                                session,
                                &diff.actor_name,
                                diff.old_level,
                                diff.new_level,
                                &diff.stat_changes,
                            )?;
                        }
                    }
                }
                None => draw_battle(session, battle_ui, &render_state)?,
            }
        } else {
            draw_battle(session, battle_ui, &render_state)?;
        }

        let Some(action) = read_action(bindings) else {
            continue;
        };

        match menu_state.phase {
            BattlePhase::Victory => {
                if matches!(action, Action::Confirm | Action::Cancel | Action::Menu) {
                    match victory_state {
                        Some(VictoryState::Summary) => {
                            if let Some(ref result) = battle_result {
                                if !result.level_ups.is_empty() {
                                    victory_state = Some(VictoryState::LevelUp(0));
                                } else {
                                    return Ok(BattleOutcome::Victory);
                                }
                            } else {
                                return Ok(BattleOutcome::Victory);
                            }
                        }
                        Some(VictoryState::LevelUp(index)) => {
                            if let Some(ref result) = battle_result {
                                if index + 1 < result.level_ups.len() {
                                    victory_state = Some(VictoryState::LevelUp(index + 1));
                                } else {
                                    return Ok(BattleOutcome::Victory);
                                }
                            } else {
                                return Ok(BattleOutcome::Victory);
                            }
                        }
                        None => return Ok(BattleOutcome::Victory),
                    }
                }
            }
            BattlePhase::Defeat => {
                if matches!(action, Action::Confirm | Action::Cancel | Action::Menu) {
                    return Ok(BattleOutcome::Defeat);
                }
            }
            BattlePhase::Command => match action {
                Action::MoveUp => {
                    if menu_state.command_index > 0 {
                        menu_state.command_index -= 1;
                    }
                }
                Action::MoveDown => {
                    let max = battle_ui.panels.commands.items.len();
                    if menu_state.command_index + 1 < max {
                        menu_state.command_index += 1;
                    }
                }
                Action::Confirm => {
                    let Some(command_label) = battle_ui
                        .panels
                        .commands
                        .items
                        .get(menu_state.command_index)
                    else {
                        continue;
                    };
                    match command_kind(command_label) {
                        Some(CommandKind::Attack) => {
                            menu_state.phase = BattlePhase::TargetEnemy;
                            menu_state.pending_action = Some(PendingBattleAction::Attack);
                            if !set_initial_enemy_target(&mut menu_state, &battle_state) {
                                push_battle_log(&mut battle_state.log, "No valid targets.");
                                menu_state.reset_for_actor();
                            }
                        }
                        Some(CommandKind::Magic) => {
                            if spell_entries.is_empty() {
                                push_battle_log(&mut battle_state.log, "No spells available.");
                            } else {
                                menu_state.phase = BattlePhase::Magic;
                                menu_state.magic_index = 0;
                            }
                        }
                        Some(CommandKind::Abilities) => {
                            if ability_entries.is_empty() {
                                push_battle_log(&mut battle_state.log, "No abilities available.");
                            } else {
                                menu_state.phase = BattlePhase::Abilities;
                                menu_state.ability_index = 0;
                            }
                        }
                        Some(CommandKind::Items) => {
                            if item_entries.is_empty() {
                                push_battle_log(&mut battle_state.log, "No items available.");
                            } else {
                                menu_state.phase = BattlePhase::Items;
                                menu_state.item_index = 0;
                            }
                        }
                        Some(CommandKind::Run) => {
                            if rng.r#gen::<f32>() < 0.5 {
                                push_battle_log(&mut battle_state.log, "Escaped!");
                                return Ok(BattleOutcome::Escaped);
                            }
                            push_battle_log(&mut battle_state.log, "Escape failed!");
                            let render_state = build_battle_render_state(
                                runtime,
                                &battle_state,
                                &menu_state,
                                battle_ui,
                                &spell_entries,
                                &ability_entries,
                                &item_entries,
                            );
                            pause_after_action(
                                session,
                                battle_ui,
                                bindings,
                                &render_state,
                                Vec::new(),
                                vec![battle_state.active_index],
                                Vec::new(),
                                Vec::new(),
                            )?;
                            advance_turn(&mut menu_state, &mut turn_state);
                        }
                        None => {}
                    }
                }
                _ => {}
            },
            BattlePhase::Magic => match action {
                Action::MoveUp => {
                    if menu_state.magic_index > 0 {
                        menu_state.magic_index -= 1;
                    }
                }
                Action::MoveDown => {
                    if menu_state.magic_index + 1 < spell_entries.len() {
                        menu_state.magic_index += 1;
                    }
                }
                Action::Cancel | Action::Menu => {
                    menu_state.reset_for_actor();
                }
                Action::Confirm => {
                    let Some(entry) = spell_entries.get(menu_state.magic_index) else {
                        continue;
                    };
                    if !entry.usable {
                        let reason = entry
                            .reason
                            .clone()
                            .unwrap_or_else(|| "Cannot cast.".to_string());
                        push_battle_log(&mut battle_state.log, reason);
                        continue;
                    }
                    match entry.default_target.as_str() {
                        "enemy" => {
                            menu_state.phase = BattlePhase::TargetEnemy;
                            menu_state.pending_action =
                                Some(PendingBattleAction::Magic(entry.clone()));
                            if !set_initial_enemy_target(&mut menu_state, &battle_state) {
                                push_battle_log(&mut battle_state.log, "No valid targets.");
                                menu_state.reset_for_actor();
                            }
                        }
                        "party" => {
                            execute_magic_action(
                                runtime,
                                &mut battle_state,
                                &actor_id,
                                entry,
                                None,
                                rng,
                            );
                            let render_state = build_battle_render_state(
                                runtime,
                                &mut battle_state,
                                &menu_state,
                                battle_ui,
                                &spell_entries,
                                &ability_entries,
                                &item_entries,
                            );
                            pause_after_action(
                                session,
                                battle_ui,
                                bindings,
                                &render_state,
                                Vec::new(),
                                vec![battle_state.active_index],
                                Vec::new(),
                                (0..battle_state.party_order.len()).collect::<Vec<_>>(),
                            )?;
                            advance_turn(&mut menu_state, &mut turn_state);
                        }
                        _ => {
                            menu_state.phase = BattlePhase::TargetParty;
                            menu_state.pending_action =
                                Some(PendingBattleAction::Magic(entry.clone()));
                            if let Some(action) = menu_state.pending_action.clone() {
                                if !set_initial_party_target(
                                    &mut menu_state,
                                    &battle_state,
                                    runtime,
                                    &action,
                                ) {
                                    push_battle_log(&mut battle_state.log, "No valid targets.");
                                    menu_state.reset_for_actor();
                                }
                            }
                        }
                    }
                }
                _ => {}
            },
            BattlePhase::Abilities => match action {
                Action::MoveUp => {
                    if menu_state.ability_index > 0 {
                        menu_state.ability_index -= 1;
                    }
                }
                Action::MoveDown => {
                    if menu_state.ability_index + 1 < ability_entries.len() {
                        menu_state.ability_index += 1;
                    }
                }
                Action::Cancel | Action::Menu => {
                    menu_state.reset_for_actor();
                }
                Action::Confirm => {
                    let Some(entry) = ability_entries.get(menu_state.ability_index) else {
                        continue;
                    };
                    match entry.default_target.as_str() {
                        "enemy" => {
                            menu_state.phase = BattlePhase::TargetEnemy;
                            menu_state.pending_action =
                                Some(PendingBattleAction::Ability(entry.clone()));
                            if !set_initial_enemy_target(&mut menu_state, &battle_state) {
                                push_battle_log(&mut battle_state.log, "No valid targets.");
                                menu_state.reset_for_actor();
                            }
                        }
                        "party" => {
                            execute_ability_action(
                                runtime,
                                &mut battle_state,
                                &actor_id,
                                entry,
                                None,
                                rng,
                            );
                            let render_state = build_battle_render_state(
                                runtime,
                                &mut battle_state,
                                &menu_state,
                                battle_ui,
                                &spell_entries,
                                &ability_entries,
                                &item_entries,
                            );
                            let party_indices =
                                (0..battle_state.party_order.len()).collect::<Vec<_>>();
                            pause_after_action(
                                session,
                                battle_ui,
                                bindings,
                                &render_state,
                                Vec::new(),
                                vec![battle_state.active_index],
                                Vec::new(),
                                party_indices,
                            )?;
                            advance_turn(&mut menu_state, &mut turn_state);
                        }
                        _ => {
                            menu_state.phase = BattlePhase::TargetParty;
                            menu_state.pending_action =
                                Some(PendingBattleAction::Ability(entry.clone()));
                            if let Some(action) = menu_state.pending_action.clone() {
                                if !set_initial_party_target(
                                    &mut menu_state,
                                    &battle_state,
                                    runtime,
                                    &action,
                                ) {
                                    push_battle_log(&mut battle_state.log, "No valid targets.");
                                    menu_state.reset_for_actor();
                                }
                            }
                        }
                    }
                }
                _ => {}
            },
            BattlePhase::Items => match action {
                Action::MoveUp => {
                    if menu_state.item_index > 0 {
                        menu_state.item_index -= 1;
                    }
                }
                Action::MoveDown => {
                    if menu_state.item_index + 1 < item_entries.len() {
                        menu_state.item_index += 1;
                    }
                }
                Action::Cancel | Action::Menu => {
                    menu_state.reset_for_actor();
                }
                Action::Confirm => {
                    let Some(entry) = item_entries.get(menu_state.item_index) else {
                        continue;
                    };
                    if !entry.usable {
                        push_battle_log(&mut battle_state.log, "Item unusable.");
                        continue;
                    }
                    let Some(item) = runtime
                        .content
                        .items
                        .items
                        .iter()
                        .find(|item| item.id == entry.id)
                        .cloned()
                    else {
                        continue;
                    };
                    match item.usage.target.as_str() {
                        "party" => {
                            execute_item_action(runtime, &mut battle_state, &actor_id, &item, None);
                            let render_state = build_battle_render_state(
                                runtime,
                                &mut battle_state,
                                &menu_state,
                                battle_ui,
                                &spell_entries,
                                &ability_entries,
                                &item_entries,
                            );
                            let party_indices =
                                (0..battle_state.party_order.len()).collect::<Vec<_>>();
                            pause_after_action(
                                session,
                                battle_ui,
                                bindings,
                                &render_state,
                                Vec::new(),
                                vec![battle_state.active_index],
                                Vec::new(),
                                party_indices,
                            )?;
                            advance_turn(&mut menu_state, &mut turn_state);
                        }
                        "enemy" => {
                            menu_state.phase = BattlePhase::TargetEnemy;
                            menu_state.pending_action =
                                Some(PendingBattleAction::Item(item.id.clone()));
                            if !set_initial_enemy_target(&mut menu_state, &battle_state) {
                                push_battle_log(&mut battle_state.log, "No valid targets.");
                                menu_state.reset_for_actor();
                            }
                        }
                        _ => {
                            menu_state.phase = BattlePhase::TargetParty;
                            menu_state.pending_action =
                                Some(PendingBattleAction::Item(item.id.clone()));
                            if let Some(action) = menu_state.pending_action.clone() {
                                if !set_initial_party_target(
                                    &mut menu_state,
                                    &battle_state,
                                    runtime,
                                    &action,
                                ) {
                                    push_battle_log(&mut battle_state.log, "No valid targets.");
                                    menu_state.reset_for_actor();
                                }
                            }
                        }
                    }
                }
                _ => {}
            },
            BattlePhase::TargetEnemy => match action {
                Action::MoveUp => {
                    let valid = enemy_target_indices(&battle_state);
                    menu_state.enemy_index = step_target_index(menu_state.enemy_index, &valid, -1);
                }
                Action::MoveDown => {
                    let valid = enemy_target_indices(&battle_state);
                    menu_state.enemy_index = step_target_index(menu_state.enemy_index, &valid, 1);
                }
                Action::Cancel | Action::Menu => {
                    menu_state.reset_for_actor();
                }
                Action::Confirm => {
                    let valid = enemy_target_indices(&battle_state);
                    let Some(target_index) = ensure_valid_index(menu_state.enemy_index, &valid)
                    else {
                        push_battle_log(&mut battle_state.log, "No valid targets.");
                        menu_state.reset_for_actor();
                        continue;
                    };
                    menu_state.enemy_index = target_index;
                    let was_alive = battle_state
                        .enemies
                        .get(menu_state.enemy_index)
                        .map(|enemy| enemy.is_alive())
                        .unwrap_or(false);
                    if let Some(action) = menu_state.pending_action.take() {
                        match action {
                            PendingBattleAction::Attack => {
                                execute_attack_action(
                                    runtime,
                                    &mut battle_state,
                                    &actor_id,
                                    menu_state.enemy_index,
                                    rng,
                                );
                                let render_state = build_battle_render_state(
                                    runtime,
                                    &battle_state,
                                    &menu_state,
                                    battle_ui,
                                    &spell_entries,
                                    &ability_entries,
                                    &item_entries,
                                );
                                pause_after_action(
                                    session,
                                    battle_ui,
                                    bindings,
                                    &render_state,
                                    Vec::new(),
                                    vec![battle_state.active_index],
                                    vec![menu_state.enemy_index],
                                    Vec::new(),
                                )?;
                            }
                            PendingBattleAction::Magic(entry) => {
                                execute_magic_action(
                                    runtime,
                                    &mut battle_state,
                                    &actor_id,
                                    &entry,
                                    Some(menu_state.enemy_index),
                                    rng,
                                );
                                let render_state = build_battle_render_state(
                                    runtime,
                                    &battle_state,
                                    &menu_state,
                                    battle_ui,
                                    &spell_entries,
                                    &ability_entries,
                                    &item_entries,
                                );
                                pause_after_action(
                                    session,
                                    battle_ui,
                                    bindings,
                                    &render_state,
                                    Vec::new(),
                                    vec![battle_state.active_index],
                                    vec![menu_state.enemy_index],
                                    Vec::new(),
                                )?;
                            }
                            PendingBattleAction::Ability(entry) => {
                                execute_ability_action(
                                    runtime,
                                    &mut battle_state,
                                    &actor_id,
                                    &entry,
                                    Some(menu_state.enemy_index),
                                    rng,
                                );
                                let render_state = build_battle_render_state(
                                    runtime,
                                    &battle_state,
                                    &menu_state,
                                    battle_ui,
                                    &spell_entries,
                                    &ability_entries,
                                    &item_entries,
                                );
                                pause_after_action(
                                    session,
                                    battle_ui,
                                    bindings,
                                    &render_state,
                                    Vec::new(),
                                    vec![battle_state.active_index],
                                    vec![menu_state.enemy_index],
                                    Vec::new(),
                                )?;
                            }
                            PendingBattleAction::Item(item_id) => {
                                if let Some(item) = runtime
                                    .content
                                    .items
                                    .items
                                    .iter()
                                    .find(|item| item.id == item_id)
                                    .cloned()
                                {
                                    execute_item_action(
                                        runtime,
                                        &mut battle_state,
                                        &actor_id,
                                        &item,
                                        Some(menu_state.enemy_index),
                                    );
                                    let render_state = build_battle_render_state(
                                        runtime,
                                        &battle_state,
                                        &menu_state,
                                        battle_ui,
                                        &spell_entries,
                                        &ability_entries,
                                        &item_entries,
                                    );
                                    pause_after_action(
                                        session,
                                        battle_ui,
                                        bindings,
                                        &render_state,
                                        Vec::new(),
                                        vec![battle_state.active_index],
                                        vec![menu_state.enemy_index],
                                        Vec::new(),
                                    )?;
                                }
                            }
                        }
                        pause_on_enemy_defeat(
                            session,
                            battle_ui,
                            bindings,
                            runtime,
                            &mut battle_state,
                            &menu_state,
                            &spell_entries,
                            &ability_entries,
                            &item_entries,
                            was_alive,
                            menu_state.enemy_index,
                        )?;
                        advance_turn(&mut menu_state, &mut turn_state);
                    }
                }
                _ => {}
            },
            BattlePhase::TargetParty => match action {
                Action::MoveUp => {
                    if let Some(action) = menu_state.pending_action.as_ref() {
                        let rule = party_target_rule(action, runtime);
                        let valid = party_target_indices(runtime, &battle_state, rule);
                        menu_state.party_index =
                            step_target_index(menu_state.party_index, &valid, -1);
                    }
                }
                Action::MoveDown => {
                    if let Some(action) = menu_state.pending_action.as_ref() {
                        let rule = party_target_rule(action, runtime);
                        let valid = party_target_indices(runtime, &battle_state, rule);
                        menu_state.party_index =
                            step_target_index(menu_state.party_index, &valid, 1);
                    }
                }
                Action::Cancel | Action::Menu => {
                    menu_state.reset_for_actor();
                }
                Action::Confirm => {
                    let Some(action) = menu_state.pending_action.take() else {
                        continue;
                    };
                    let rule = party_target_rule(&action, runtime);
                    let valid = party_target_indices(runtime, &battle_state, rule);
                    let Some(target_index) = ensure_valid_index(menu_state.party_index, &valid)
                    else {
                        let message = match rule {
                            TargetRule::KnockedOut => "No fallen allies.",
                            TargetRule::Alive => "No valid targets.",
                        };
                        push_battle_log(&mut battle_state.log, message);
                        menu_state.reset_for_actor();
                        continue;
                    };
                    menu_state.party_index = target_index;
                    match action {
                        PendingBattleAction::Magic(entry) => {
                            execute_magic_action(
                                runtime,
                                &mut battle_state,
                                &actor_id,
                                &entry,
                                Some(menu_state.party_index),
                                rng,
                            );
                            let render_state = build_battle_render_state(
                                runtime,
                                &battle_state,
                                &menu_state,
                                battle_ui,
                                &spell_entries,
                                &ability_entries,
                                &item_entries,
                            );
                            pause_after_action(
                                session,
                                battle_ui,
                                bindings,
                                &render_state,
                                Vec::new(),
                                vec![battle_state.active_index],
                                Vec::new(),
                                vec![menu_state.party_index],
                            )?;
                        }
                        PendingBattleAction::Ability(entry) => {
                            execute_ability_action(
                                runtime,
                                &mut battle_state,
                                &actor_id,
                                &entry,
                                Some(menu_state.party_index),
                                rng,
                            );
                            let render_state = build_battle_render_state(
                                runtime,
                                &battle_state,
                                &menu_state,
                                battle_ui,
                                &spell_entries,
                                &ability_entries,
                                &item_entries,
                            );
                            pause_after_action(
                                session,
                                battle_ui,
                                bindings,
                                &render_state,
                                Vec::new(),
                                vec![battle_state.active_index],
                                Vec::new(),
                                vec![menu_state.party_index],
                            )?;
                        }
                        PendingBattleAction::Item(item_id) => {
                            if let Some(item) = runtime
                                .content
                                .items
                                .items
                                .iter()
                                .find(|item| item.id == item_id)
                                .cloned()
                            {
                                execute_item_action(
                                    runtime,
                                    &mut battle_state,
                                    &actor_id,
                                    &item,
                                    Some(menu_state.party_index),
                                );
                                let render_state = build_battle_render_state(
                                    runtime,
                                    &battle_state,
                                    &menu_state,
                                    battle_ui,
                                    &spell_entries,
                                    &ability_entries,
                                    &item_entries,
                                );
                                pause_after_action(
                                    session,
                                    battle_ui,
                                    bindings,
                                    &render_state,
                                    Vec::new(),
                                    vec![battle_state.active_index],
                                    vec![menu_state.enemy_index],
                                    Vec::new(),
                                )?;
                            }
                        }
                        PendingBattleAction::Attack => {}
                    }
                    advance_turn(&mut menu_state, &mut turn_state);
                }
                _ => {}
            },
        }
    }
}

fn build_battle_render_state(
    runtime: &GameRuntime,
    battle_state: &BattleState,
    menu_state: &BattleMenuState,
    battle_ui: &BattleUiFile,
    spell_entries: &[SpellEntry],
    ability_entries: &[AbilityEntry],
    item_entries: &[InventoryEntry],
) -> BattleRenderState {
    let enemies = battle_state
        .enemies
        .iter()
        .map(|enemy| BattleEnemyView {
            name: enemy.name.clone(),
            hp: enemy.current_hp,
            max_hp: enemy.max_hp(),
            glyph: enemy.sprite.glyph.chars().next().unwrap_or('!'),
            palette: Some(enemy.sprite.palette.clone()),
            art: enemy.art.as_ref().map(|art| art.lines.clone()),
            art_palette: enemy.art.as_ref().map(|art| art.palette.clone()),
            pos: enemy.pos,
            alive: enemy.is_alive(),
            show_hp: enemy.scanned,
        })
        .collect();
    let party_positions = party_sprite_positions(
        battle_state.party_order.len(),
        battle_ui.layout.party_grid.columns,
    );
    let party = battle_state
        .party_order
        .iter()
        .enumerate()
        .filter_map(|(index, id)| runtime.party.roster.get(id).map(|actor| (index, actor)))
        .map(|(index, actor)| {
            let job = runtime
                .content
                .jobs
                .jobs
                .iter()
                .find(|job| job.id == actor.job_id);
            let (glyph, palette, art, art_palette) = job
                .map(|job| {
                    (
                        job.sprite.glyph.chars().next().unwrap_or('@'),
                        Some(job.sprite.palette.clone()),
                        job.art.as_ref().map(|art| art.lines.clone()),
                        job.art.as_ref().map(|art| art.palette.clone()),
                    )
                })
                .unwrap_or((
                    actor.name.chars().next().unwrap_or('@'),
                    Some("player".to_string()),
                    None,
                    None,
                ));
            BattlePartyView {
                name: actor.name.clone(),
                hp: actor.current_hp,
                max_hp: actor.derived_stats.get("hp").copied().unwrap_or(0),
                mp: actor.current_mp,
                max_mp: actor.derived_stats.get("mp").copied().unwrap_or(0),
                status: Vec::new(),
                alive: actor.current_hp > 0,
                active: index == battle_state.active_index,
                glyph,
                palette,
                art,
                art_palette,
                pos: *party_positions.get(index).unwrap_or(&(8, 4)),
            }
        })
        .collect();
    let command_panel = build_battle_command_panel(
        runtime,
        menu_state,
        battle_ui,
        spell_entries,
        ability_entries,
        item_entries,
    );
    BattleRenderState {
        enemies,
        party,
        command_panel,
        selected_enemy: menu_state.enemy_index,
        selected_party: menu_state.party_index,
        focus: battle_focus(menu_state),
        log: battle_state.log.clone(),
        use_color: runtime
            .content
            .rules
            .render
            .palette
            .eq_ignore_ascii_case("terminal"),
        flash_enemies: Vec::new(),
        flash_party: Vec::new(),
        acting_enemies: Vec::new(),
        acting_party: Vec::new(),
    }
}

fn party_sprite_positions(count: usize, columns: u16) -> Vec<(i32, i32)> {
    if count == 0 {
        return Vec::new();
    }
    let columns = columns.max(1).min(10) as usize;
    let rows = (count + columns - 1) / columns;
    let rows = rows.min(6).max(1);
    let start_col = (10 - columns) as i32;
    let start_row = ((6 - rows) / 2) as i32;
    (0..count)
        .map(|index| {
            let col = (index % columns) as i32;
            let row = (index / columns) as i32;
            (start_col + col, start_row + row)
        })
        .collect()
}

fn build_battle_command_panel(
    runtime: &GameRuntime,
    menu_state: &BattleMenuState,
    battle_ui: &BattleUiFile,
    spell_entries: &[SpellEntry],
    ability_entries: &[AbilityEntry],
    item_entries: &[InventoryEntry],
) -> BattleCommandPanelView {
    match menu_state.phase {
        BattlePhase::Magic => BattleCommandPanelView {
            mode: BattleCommandPanelMode::Magic,
            title: battle_ui.panels.commands.title.clone(),
            items: Vec::new(),
            columns: battle_ui
                .menus
                .magic
                .columns
                .iter()
                .map(|column| column.label.clone())
                .collect(),
            rows: spell_entries
                .iter()
                .map(|entry| {
                    vec![
                        if entry.usable {
                            entry.name.clone()
                        } else {
                            format!("{} *", entry.name)
                        },
                        spell_cost_label(entry),
                    ]
                })
                .collect(),
            selected: menu_state
                .magic_index
                .min(spell_entries.len().saturating_sub(1)),
        },
        BattlePhase::Abilities => BattleCommandPanelView {
            mode: BattleCommandPanelMode::Abilities,
            title: battle_ui.panels.commands.title.clone(),
            items: Vec::new(),
            columns: battle_ui
                .menus
                .abilities
                .columns
                .iter()
                .map(|column| column.label.clone())
                .collect(),
            rows: ability_entries
                .iter()
                .map(|entry| vec![entry.name.clone()])
                .collect(),
            selected: menu_state
                .ability_index
                .min(ability_entries.len().saturating_sub(1)),
        },
        BattlePhase::Items => BattleCommandPanelView {
            mode: BattleCommandPanelMode::Items,
            title: battle_ui.panels.commands.title.clone(),
            items: Vec::new(),
            columns: battle_ui
                .menus
                .items
                .columns
                .iter()
                .map(|column| column.label.clone())
                .collect(),
            rows: item_entries
                .iter()
                .map(|entry| vec![entry.label.clone(), entry.available_qty.to_string()])
                .collect(),
            selected: menu_state
                .item_index
                .min(item_entries.len().saturating_sub(1)),
        },
        _ => BattleCommandPanelView {
            mode: BattleCommandPanelMode::Commands,
            title: battle_ui.panels.commands.title.clone(),
            items: battle_ui
                .panels
                .commands
                .items
                .iter()
                .map(|label| BattleCommandItem {
                    label: label.clone(),
                    enabled: command_enabled(
                        runtime,
                        label,
                        spell_entries,
                        ability_entries,
                        item_entries,
                    ),
                })
                .collect(),
            columns: Vec::new(),
            rows: Vec::new(),
            selected: menu_state
                .command_index
                .min(battle_ui.panels.commands.items.len().saturating_sub(1)),
        },
    }
}

fn pause_after_action(
    session: &mut TuiSession,
    battle_ui: &BattleUiFile,
    bindings: &tui::input::InputBindings,
    render_state: &BattleRenderState,
    acting_enemies: Vec<usize>,
    acting_party: Vec<usize>,
    flash_enemies: Vec<usize>,
    flash_party: Vec<usize>,
) -> std::io::Result<()> {
    if let Some(animation) = &battle_ui.animation {
        if !flash_enemies.is_empty()
            || !flash_party.is_empty()
            || !acting_enemies.is_empty()
            || !acting_party.is_empty()
        {
            let base_acting_state = BattleRenderState {
                acting_enemies: acting_enemies.clone(),
                acting_party: acting_party.clone(),
                ..render_state.clone()
            };
            let cycles = animation.flash_cycles.max(1);
            let delay = Duration::from_millis(animation.flash_ms.max(1));
            for _ in 0..cycles {
                let mut flash_state = base_acting_state.clone();
                flash_state.flash_enemies = flash_enemies.clone();
                flash_state.flash_party = flash_party.clone();
                draw_battle(session, battle_ui, &flash_state)?;
                sleep(delay);
                draw_battle(session, battle_ui, &base_acting_state)?;
                sleep(delay);
            }
        }
    }
    draw_battle(session, battle_ui, render_state)?;
    wait_for_battle_dialog(bindings, battle_ui)
}

fn pause_on_enemy_defeat(
    session: &mut TuiSession,
    battle_ui: &BattleUiFile,
    bindings: &tui::input::InputBindings,
    runtime: &GameRuntime,
    battle_state: &mut BattleState,
    menu_state: &BattleMenuState,
    spell_entries: &[SpellEntry],
    ability_entries: &[AbilityEntry],
    item_entries: &[InventoryEntry],
    was_alive: bool,
    target_index: usize,
) -> std::io::Result<()> {
    let defeated = was_alive
        && battle_state
            .enemies
            .get(target_index)
            .map(|enemy| !enemy.is_alive())
            .unwrap_or(false);
    if !defeated {
        return Ok(());
    }
    if let Some(enemy) = battle_state.enemies.get(target_index) {
        battle_state.log.push(format!("{} defeated.", enemy.name));
        let render_state = build_battle_render_state(
            runtime,
            battle_state,
            menu_state,
            battle_ui,
            spell_entries,
            ability_entries,
            item_entries,
        );
        pause_after_action(
            session,
            battle_ui,
            bindings,
            &render_state,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )?;
    }
    Ok(())
}

fn wait_for_battle_dialog(
    bindings: &tui::input::InputBindings,
    battle_ui: &BattleUiFile,
) -> std::io::Result<()> {
    let Some(log) = &battle_ui.log else {
        return Ok(());
    };
    if log.auto_advance_ms == 0 && !log.allow_skip {
        return Ok(());
    }
    let timeout = Duration::from_millis(log.auto_advance_ms);
    let start = Instant::now();
    loop {
        let elapsed = start.elapsed();
        if log.auto_advance_ms > 0 && elapsed >= timeout {
            break;
        }
        if log.allow_skip {
            let wait = if log.auto_advance_ms == 0 {
                Duration::from_millis(50)
            } else {
                timeout.saturating_sub(elapsed)
            };
            if crossterm::event::poll(wait)? {
                if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                    if let Some(action) = bindings.action_for(key.code) {
                        if matches!(action, Action::Confirm | Action::Cancel | Action::Menu) {
                            break;
                        }
                    }
                }
            }
        } else if log.auto_advance_ms > 0 {
            sleep(timeout);
            break;
        }
    }
    Ok(())
}

fn execute_attack_action(
    runtime: &GameRuntime,
    battle_state: &mut BattleState,
    actor_id: &str,
    enemy_index: usize,
    rng: &mut impl Rng,
) {
    let Some(actor) = runtime.party.roster.get(actor_id) else {
        return;
    };
    let Some(enemy) = battle_state.enemies.get_mut(enemy_index) else {
        return;
    };
    if !enemy.is_alive() {
        push_battle_log(&mut battle_state.log, "No target.");
        return;
    }
    let atk = actor.derived_stats.get("atk").copied().unwrap_or(0);
    let damage = physical_damage(atk, enemy.def(), rng);
    apply_damage_to_enemy(enemy, damage);
    push_battle_log(
        &mut battle_state.log,
        format!("{} attacks {} for {} HP.", actor.name, enemy.name, damage),
    );
}

fn execute_magic_action(
    runtime: &mut GameRuntime,
    battle_state: &mut BattleState,
    actor_id: &str,
    entry: &SpellEntry,
    target_index: Option<usize>,
    rng: &mut impl Rng,
) {
    let magic_system = runtime.content.rules.game.magic_system.clone();
    let (actor_name, matk) = {
        let Some(actor) = runtime.party.roster.get_mut(actor_id) else {
            return;
        };
        if !spell_cost_available(
            magic_system.clone(),
            actor,
            entry.cost_type.as_str(),
            entry.tier,
            entry.cost_value,
        ) {
            push_battle_log(&mut battle_state.log, "Not enough MP.");
            return;
        }
        if !consume_spell_cost(magic_system, actor, entry) {
            push_battle_log(&mut battle_state.log, "Not enough MP.");
            return;
        }
        (
            actor.name.clone(),
            actor.derived_stats.get("matk").copied().unwrap_or(0),
        )
    };

    match entry.default_target.as_str() {
        "enemy" => {
            if let Some(index) = target_index {
                if let Some(enemy) = battle_state.enemies.get_mut(index) {
                    if !enemy.is_alive() {
                        push_battle_log(&mut battle_state.log, "No target.");
                        return;
                    }
                    match entry.effect_type.as_str() {
                        "damage" => {
                            let base = (entry.effect_power + matk / 2).max(1);
                            let damage = roll_damage(base, rng);
                            apply_damage_to_enemy(enemy, damage);
                            push_battle_log(
                                &mut battle_state.log,
                                format!(
                                    "{} casts {} on {} for {} HP.",
                                    actor_name, entry.name, enemy.name, damage
                                ),
                            );
                        }
                        "scan" => {
                            enemy.scanned = true;
                            push_battle_log(
                                &mut battle_state.log,
                                format!(
                                    "{} scans {}: {}/{} HP.",
                                    actor_name,
                                    enemy.name,
                                    enemy.current_hp.max(0),
                                    enemy.max_hp().max(1)
                                ),
                            );
                        }
                        _ => {
                            push_battle_log(&mut battle_state.log, "Nothing happens.");
                        }
                    }
                }
            }
        }
        "party" => {
            for id in battle_state.party_order.clone() {
                apply_spell_to_actor(runtime, entry, &id);
            }
            push_battle_log(
                &mut battle_state.log,
                format!("{} casts {} on the party.", actor_name, entry.name),
            );
        }
        _ => {
            let target_id = target_index
                .and_then(|index| battle_state.party_order.get(index))
                .cloned()
                .unwrap_or_else(|| actor_id.to_string());
            apply_spell_to_actor(runtime, entry, &target_id);
            let target_name = runtime
                .party
                .roster
                .get(&target_id)
                .map(|actor| actor.name.clone())
                .unwrap_or_else(|| target_id.clone());
            push_battle_log(
                &mut battle_state.log,
                format!("{} casts {} on {}.", actor_name, entry.name, target_name),
            );
        }
    }
}

fn execute_ability_action(
    runtime: &mut GameRuntime,
    battle_state: &mut BattleState,
    actor_id: &str,
    entry: &AbilityEntry,
    target_index: Option<usize>,
    rng: &mut impl Rng,
) {
    let (actor_name, atk) = {
        let Some(actor) = runtime.party.roster.get(actor_id) else {
            return;
        };
        (
            actor.name.clone(),
            actor.derived_stats.get("atk").copied().unwrap_or(0),
        )
    };

    match entry.default_target.as_str() {
        "enemy" => {
            if let Some(index) = target_index {
                if let Some(enemy) = battle_state.enemies.get_mut(index) {
                    if !enemy.is_alive() {
                        push_battle_log(&mut battle_state.log, "No target.");
                        return;
                    }
                    match entry.effect_type.as_str() {
                        "damage" => {
                            let base = (entry.effect_power + atk / 2).max(1);
                            let damage = roll_damage(base, rng);
                            apply_damage_to_enemy(enemy, damage);
                            push_battle_log(
                                &mut battle_state.log,
                                format!(
                                    "{} uses {} on {} for {} HP.",
                                    actor_name, entry.name, enemy.name, damage
                                ),
                            );
                        }
                        "scan" => {
                            enemy.scanned = true;
                            push_battle_log(
                                &mut battle_state.log,
                                format!(
                                    "{} scans {}: {}/{} HP.",
                                    actor_name,
                                    enemy.name,
                                    enemy.current_hp.max(0),
                                    enemy.max_hp().max(1)
                                ),
                            );
                        }
                        _ => {
                            push_battle_log(&mut battle_state.log, "Nothing happens.");
                        }
                    }
                }
            }
        }
        "party" => {
            for id in battle_state.party_order.clone() {
                apply_ability_to_actor(runtime, entry, &id);
            }
            push_battle_log(
                &mut battle_state.log,
                format!("{} uses {} on the party.", actor_name, entry.name),
            );
        }
        _ => {
            let target_id = target_index
                .and_then(|index| battle_state.party_order.get(index))
                .cloned()
                .unwrap_or_else(|| actor_id.to_string());
            apply_ability_to_actor(runtime, entry, &target_id);
            let target_name = runtime
                .party
                .roster
                .get(&target_id)
                .map(|actor| actor.name.clone())
                .unwrap_or_else(|| target_id.clone());
            push_battle_log(
                &mut battle_state.log,
                format!("{} uses {} on {}.", actor_name, entry.name, target_name),
            );
        }
    }
}

fn execute_item_action(
    runtime: &mut GameRuntime,
    battle_state: &mut BattleState,
    actor_id: &str,
    item: &engine::entities::ItemDefinition,
    target_index: Option<usize>,
) {
    let target_ids = match item.usage.target.as_str() {
        "party" => battle_state.party_order.clone(),
        "enemy" => Vec::new(),
        _ => target_index
            .and_then(|index| battle_state.party_order.get(index))
            .map(|id| vec![id.clone()])
            .unwrap_or_else(|| vec![actor_id.to_string()]),
    };

    if !item_usage_allows_battle(&item.usage.context) {
        push_battle_log(&mut battle_state.log, "Item unusable.");
        return;
    }
    if !runtime.inventory.remove_item(&item.id, 1) {
        push_battle_log(&mut battle_state.log, "No items left.");
        return;
    }
    for target_id in target_ids {
        apply_item_to_actor(runtime, item, &target_id);
    }
    let actor_name = runtime
        .party
        .roster
        .get(actor_id)
        .map(|actor| actor.name.clone())
        .unwrap_or_else(|| actor_id.to_string());
    push_battle_log(
        &mut battle_state.log,
        format!("{} uses {}.", actor_name, item.name),
    );
}

fn command_enabled(
    runtime: &GameRuntime,
    label: &str,
    spell_entries: &[SpellEntry],
    ability_entries: &[AbilityEntry],
    item_entries: &[InventoryEntry],
) -> bool {
    match command_kind(label) {
        Some(CommandKind::Magic) => {
            system_enabled(runtime, Some("magic")) && !spell_entries.is_empty()
        }
        Some(CommandKind::Abilities) => !ability_entries.is_empty(),
        Some(CommandKind::Items) => {
            system_enabled(runtime, Some("items")) && !item_entries.is_empty()
        }
        _ => true,
    }
}

fn battle_focus(menu_state: &BattleMenuState) -> BattleFocus {
    match menu_state.phase {
        BattlePhase::TargetEnemy => BattleFocus::Enemies,
        BattlePhase::TargetParty => BattleFocus::Party,
        _ => BattleFocus::Commands,
    }
}

fn build_battle_spell_entries(runtime: &GameRuntime, actor_id: &str) -> Vec<SpellEntry> {
    let Some(actor) = runtime.party.roster.get(actor_id) else {
        return Vec::new();
    };
    let mut school_lookup = HashMap::new();
    for school in &runtime.content.spells.schools {
        school_lookup.insert(school.id.as_str(), school.name.as_str());
    }
    let mut spell_ids = collect_spell_ids(runtime, actor);
    spell_ids.sort();
    spell_ids.dedup();
    let mut entries = Vec::new();
    for spell_id in spell_ids {
        let Some(spell) = runtime
            .content
            .spells
            .spells
            .iter()
            .find(|spell| spell.id == spell_id)
        else {
            continue;
        };
        let school = school_lookup
            .get(spell.school.as_str())
            .copied()
            .unwrap_or(spell.school.as_str())
            .to_string();
        let (usable, reason) = spell_cast_status_battle(runtime, actor, spell);
        entries.push(SpellEntry {
            id: spell.id.clone(),
            name: spell.name.clone(),
            school,
            tier: spell.tier,
            cost_type: spell.cost.r#type.clone(),
            cost_value: spell.cost.value,
            default_target: spell.default_target.clone(),
            allowed_targets: spell.allowed_targets.clone(),
            effect_type: spell.effect.r#type.clone(),
            effect_power: spell.effect.power,
            usable,
            reason,
        });
    }
    entries.sort_by(|left, right| {
        left.school
            .cmp(&right.school)
            .then(left.tier.cmp(&right.tier))
            .then(left.name.cmp(&right.name))
    });
    entries
}

fn build_battle_ability_entries(runtime: &GameRuntime, actor_id: &str) -> Vec<AbilityEntry> {
    let Some(actor) = runtime.party.roster.get(actor_id) else {
        return Vec::new();
    };
    let mut ability_ids = collect_ability_ids(runtime, actor);
    ability_ids.sort();
    ability_ids.dedup();
    let mut entries = Vec::new();
    for ability_id in ability_ids {
        let Some(ability) = runtime
            .content
            .abilities
            .abilities
            .iter()
            .find(|ability| ability.id == ability_id)
        else {
            continue;
        };
        entries.push(AbilityEntry {
            id: ability.id.clone(),
            name: ability.name.clone(),
            default_target: ability.default_target.clone(),
            allowed_targets: ability.allowed_targets.clone(),
            effect_type: ability.effect.r#type.clone(),
            effect_power: ability.effect.power,
        });
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}

fn build_battle_item_entries(runtime: &GameRuntime) -> Vec<InventoryEntry> {
    let mut entries = Vec::new();
    for item in &runtime.content.items.items {
        let qty = runtime.inventory.item_qty(&item.id);
        if qty <= 0 {
            continue;
        }
        if !item_usage_allows_battle(&item.usage.context) {
            continue;
        }
        entries.push(InventoryEntry {
            id: item.id.clone(),
            label: item.name.clone(),
            available_qty: qty,
            total_qty: qty,
            kind: InventoryKind::Item,
            slot: None,
            category: None,
            usable: true,
            equipped_by: Vec::new(),
            usage_target: item.usage.target.clone(),
        });
    }
    entries.sort_by(|left, right| left.label.cmp(&right.label));
    entries
}

fn spell_cast_status_battle(
    runtime: &GameRuntime,
    actor: &engine::party::Actor,
    spell: &engine::entities::SpellDefinition,
) -> (bool, Option<String>) {
    if !spell_effect_allows_battle(spell.effect.r#type.as_str()) {
        return (false, Some("Unsupported effect.".to_string()));
    }
    let magic_system = runtime.content.rules.game.magic_system.clone();
    if !spell_system_matches(magic_system.clone(), spell.cost.r#type.as_str()) {
        return (false, Some("Cost system mismatch.".to_string()));
    }
    if !spell_cost_available(
        magic_system.clone(),
        actor,
        spell.cost.r#type.as_str(),
        spell.tier,
        spell.cost.value,
    ) {
        let reason = match magic_system {
            MagicSystem::Mp => "Not enough MP.",
            MagicSystem::TierCharges => "No tier charges.",
        };
        return (false, Some(reason.to_string()));
    }
    (true, None)
}

fn spell_effect_allows_battle(effect: &str) -> bool {
    matches!(effect, "heal" | "revive" | "damage" | "scan")
}

fn item_usage_allows_battle(context: &str) -> bool {
    matches!(context, "battle" | "both")
}

fn apply_battle_rewards(
    runtime: &mut GameRuntime,
    battle_state: &mut BattleState,
    rng: &mut impl Rng,
) -> BattleResult {
    let base_rewards = collect_rewards(&battle_state.enemies, rng);
    let mut result = BattleResult {
        rewards: base_rewards.clone(),
        level_ups: Vec::new(),
    };

    if result.rewards.exp > 0 {
        let rules = Ruleset::from_file(runtime.content.rules.clone());
        for actor_id in runtime.party.active.clone() {
            if let Some(actor) = runtime.party.roster.get_mut(&actor_id) {
                let old_level = actor.level;
                let old_stats = actor.derived_stats.clone();

                let levels_gained = gain_exp(&runtime.content, &rules, actor, result.rewards.exp);

                if levels_gained > 0 {
                    let new_stats = actor.derived_stats.clone();
                    let mut stat_changes = HashMap::new();

                    for (stat, new_value) in new_stats {
                        let old_value = old_stats.get(&stat).copied().unwrap_or(0);
                        let diff = new_value - old_value;
                        if diff != 0 {
                            stat_changes.insert(stat, (new_value, diff));
                        }
                    }

                    result.level_ups.push(LevelUpDiff {
                        actor_name: actor.name.clone(),
                        old_level,
                        new_level: actor.level,
                        stat_changes,
                    });
                }
            }
        }
    }

    if result.rewards.currency > 0 {
        let currency = &runtime.content.rules.game.currency;
        runtime
            .inventory
            .add_currency(currency.id.as_str(), result.rewards.currency);
    }

    if !result.rewards.items.is_empty() {
        let max_stack = runtime.content.rules.inventory.max_stack;
        for (item_id, qty) in &result.rewards.items {
            if runtime
                .content
                .items
                .items
                .iter()
                .any(|item| item.id == *item_id)
            {
                runtime.inventory.add_item(item_id, *qty, max_stack);
            } else if runtime
                .content
                .equipment
                .equipment
                .iter()
                .any(|item| item.id == *item_id)
            {
                runtime.inventory.add_equipment(item_id, *qty, max_stack);
            }
        }
    }

    result
}

fn push_battle_log(log: &mut Vec<String>, message: impl Into<String>) {
    log.push(message.into());
    let max_entries = 6;
    if log.len() > max_entries {
        let drain = log.len() - max_entries;
        log.drain(0..drain);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum CommandKind {
    Attack,
    Magic,
    Abilities,
    Items,
    Run,
}

fn command_kind(label: &str) -> Option<CommandKind> {
    match label.to_ascii_lowercase().as_str() {
        "attack" => Some(CommandKind::Attack),
        "magic" => Some(CommandKind::Magic),
        "abilities" => Some(CommandKind::Abilities),
        "items" => Some(CommandKind::Items),
        "run" => Some(CommandKind::Run),
        _ => None,
    }
}

fn encounter_zone_for_pos(
    map: &engine::maps::MapFile,
    pos: (i32, i32),
) -> Option<&engine::maps::EncounterZone> {
    map.encounters
        .iter()
        .find(|zone| pos_in_rect(pos, zone.rect))
}

fn pos_in_rect(pos: (i32, i32), rect: [i32; 4]) -> bool {
    let (x, y) = pos;
    x >= rect[0] && y >= rect[1] && x < rect[0] + rect[2] && y < rect[1] + rect[3]
}

fn select_encounter_entry(
    encounters: &engine::encounters::EncountersFile,
    table_id: &str,
    rng: &mut impl Rng,
) -> Option<engine::encounters::EncounterEntry> {
    let table = encounters
        .tables
        .iter()
        .find(|table| table.id == table_id)?;
    let total_weight: i32 = table.entries.iter().map(|entry| entry.weight.max(0)).sum();
    if total_weight <= 0 {
        return table.entries.first().cloned();
    }
    let roll = rng.gen_range(0..total_weight);
    let mut cursor = 0;
    for entry in &table.entries {
        cursor += entry.weight.max(0);
        if roll < cursor {
            return Some(entry.clone());
        }
    }
    table.entries.first().cloned()
}

fn open_shop(
    runtime: &GameRuntime,
    session: &mut TuiSession,
    bindings: &tui::input::InputBindings,
    shop_id: &str,
) -> std::io::Result<()> {
    let shop = match build_shop_view(runtime, shop_id) {
        Some(shop) => shop,
        None => {
            println!("Shop not found: {}", shop_id);
            return Ok(());
        }
    };
    let _ = show_shop(session, &shop, bindings)?;
    Ok(())
}

fn run_validate() {
    let args: Vec<String> = env::args().skip(2).collect();
    let content_dir = parse_content_dir(&args).unwrap_or_else(|| PathBuf::from("content/demo"));
    let mut errors = engine::validate::validate_content(&content_dir);

    let input_path = content_dir.join("input.json");
    let title_ui_path = content_dir.join("ui").join("title.json");
    let menu_ui_path = content_dir.join("ui").join("menu.json");
    let battle_ui_path = content_dir.join("ui").join("battle.json");
    let dialog_ui_path = content_dir.join("ui").join("dialog.json");
    let progress_ui_path = content_dir.join("ui").join("progress.json");

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
        errors.push(format!("ui/progress.json: {}", err));
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

fn run_new_project() {
    println!("Creating new OpenCrystal project...");
}

fn run_build() {
    println!("Building OpenCrystal content...");
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

fn print_usage() {
    println!(
        "OpenCrystal\n\nUsage:\n  cryst play [--render=auto|wide|modern] [--content path]\n  cryst validate\n  cryst new-project\n  cryst build"
    );
}
