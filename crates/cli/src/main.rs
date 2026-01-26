use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use engine::{
    Engine,
    content::Content,
    party::{PartyState, actor_slots, exp_for_level, recompute_derived_stats},
    rules::{PartyMode, Ruleset},
    runtime::{GameRuntime, GameState, MenuFocus},
    world::WorldState,
};
use tui::app::{
    ChoiceView, MapView, MenuEntryView, MenuPane, MenuPanelLine, MenuPanelSpan, MenuPanelView,
    NpcView, PanelSpanStyle, ShopItem, ShopView, TileRender, TitleAction, TransitionView,
    TuiSession, draw_menu, draw_menu_frame, draw_overworld, draw_overworld_with_tooltip,
    prompt_choice, prompt_text, run_title, show_centered_dialog_on_map, show_dialog,
    show_dialog_on_map, show_dialog_with_choices, show_dialog_with_choices_on_map, show_shop,
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

    if let Err(err) = BattleUiFile::load(&battle_ui_path) {
        eprintln!("Failed to load battle UI: {}", err);
    }

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
                if let Err(err) = run_event_loop(&mut runtime, &dialog_ui, &input_bindings, session)
                {
                    if err.kind() == std::io::ErrorKind::Interrupted {
                        return;
                    }
                }
                let spawn = find_spawn(&runtime, &world.map_id, world.position);
                if let Err(err) = run_overworld_loop(
                    session,
                    &mut runtime,
                    &dialog_ui,
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
    bindings: &tui::input::InputBindings,
    session: &mut TuiSession,
) -> std::io::Result<()> {
    while runtime.state == GameState::Event {
        match runtime.next_event_step() {
            Some(step) => handle_event_step(runtime, dialog_ui, bindings, session, &step)?,
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

fn handle_event_step(
    runtime: &mut GameRuntime,
    dialog_ui: &DialogUiFile,
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
            println!("Start battle.");
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
                    if let Some(text) = find_sign_text(runtime, &current_map_id, player_pos) {
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
        }

        if !is_passable(runtime, &current_map_id, player_pos)
            || npc_at(runtime, &current_map_id, player_pos)
            || sign_at(runtime, &current_map_id, player_pos)
        {
            player_pos = previous_pos;
        }
    }
    Ok(())
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
    qty: i32,
    kind: InventoryKind,
    slot: Option<String>,
    category: Option<String>,
    usable: bool,
    equipped_by: Vec<String>,
    usage_target: String,
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
        draw_menu(
            session,
            menu_ui,
            &entry_views,
            runtime.menu_state.selected,
            focus,
            &right_panel,
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

                                    "equipment" => {
                                        runtime.menu_state.focus = MenuFocus::Detail;
                                        runtime.menu_state.active_submenu =
                                            Some(entry.action.clone());
                                        runtime.menu_state.detail_page = 0;
                                        runtime.menu_state.detail_selection = 0;
                                        runtime.menu_state.detail_actor = 0;
                                        runtime.menu_state.detail_slot = 0;
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
                    if tui::app::confirm_quit(session, |frame| {
                        draw_menu_frame(
                            frame,
                            menu_ui,
                            &entry_views,
                            runtime.menu_state.selected,
                            focus,
                            &right_panel,
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
    if index == 0 { 4 } else { index - 1 }
}

fn toggle_sort_index(index: usize) -> usize {
    if index == 0 { 1 } else { 0 }
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
                qty,
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
            entries.push(InventoryEntry {
                id: equipment.id.clone(),
                label: equipment.name.clone(),
                qty,
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
        qty: 0,
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
        let total_qty = runtime.inventory.equipment_qty(&equipment.id);
        let equipped_count = equipped_counts.get(&equipment.id).copied().unwrap_or(0);
        let mut available = total_qty - equipped_count;
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
        let usable = available > 0 || !equipped_by.is_empty();
        entries.push(InventoryEntry {
            id: equipment.id.clone(),
            label: equipment.name.clone(),
            qty: available.max(0),
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
    lines.push(header);
    for (index, entry) in entries.iter().enumerate() {
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
            format!("{} x{}", entry.label, entry.qty),
            if is_selected {
                PanelSpanStyle::Highlight
            } else if entry.usable {
                PanelSpanStyle::Normal
            } else {
                PanelSpanStyle::Muted
            },
        ));
        if let Some(owner) = equipped_label(entry) {
            spans.push(panel_span(format!(" {}", owner), PanelSpanStyle::Accent));
        }
        lines.push(panel_line_spans(spans));
    }
    lines.push(panel_line("------------------------------"));
    if runtime.menu_state.detail_page == 1 {
        lines.extend(build_item_target_panel(runtime, entries.get(selection)));
        lines.push(panel_line("------------------------------"));
    }
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
    lines.push(panel_line("Target:"));
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
        for (index, entry) in entries.iter().enumerate() {
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
                format!("{} x{}", entry.label, entry.qty.max(0)),
                if is_selected {
                    PanelSpanStyle::Highlight
                } else if entry.usable {
                    PanelSpanStyle::Normal
                } else {
                    PanelSpanStyle::Muted
                },
            ));
            if let Some(owner) = equipped_label(entry) {
                spans.push(panel_span(format!(" {}", owner), PanelSpanStyle::Accent));
            }
            lines.push(panel_line_spans(spans));
        }
        lines.push(panel_line("------------------------------"));
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
