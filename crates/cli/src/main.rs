use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use engine::{
    Engine,
    content::Content,
    rules::Ruleset,
    runtime::{GameRuntime, GameState, MenuFocus},
    world::WorldState,
};
use tui::app::{
    ChoiceView, MapView, MenuEntryView, MenuPane, MenuPanelView, NpcView, ShopItem, ShopView,
    TileRender, TitleAction, TransitionView, TuiSession, draw_menu, draw_menu_frame,
    draw_overworld, draw_overworld_with_tooltip, run_title, show_centered_dialog_on_map,
    show_dialog, show_dialog_on_map, show_dialog_with_choices, show_dialog_with_choices_on_map,
    show_shop,
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
    runtime.start_new_game(&rules);

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
        let right_panel = if matches!(focus, MenuPane::Detail) {
            let selected = entries.get(runtime.menu_state.selected);
            let label = selected
                .map(|entry| entry.view.label.as_str())
                .unwrap_or("Menu");
            let action = runtime
                .menu_state
                .active_submenu
                .as_deref()
                .or_else(|| selected.map(|entry| entry.action.as_str()))
                .unwrap_or("menu");
            menu_detail_panel(label, action)
        } else {
            menu_default_panel(menu_ui, runtime)
        };

        draw_menu(
            session,
            menu_ui,
            &entry_views,
            runtime.menu_state.selected,
            focus,
            &right_panel,
        )?;

        if let Some(action) = read_action(bindings) {
            match action {
                Action::MoveUp => {
                    if matches!(focus, MenuPane::List) && runtime.menu_state.selected > 0 {
                        runtime.menu_state.selected -= 1;
                    }
                }
                Action::MoveDown => {
                    if matches!(focus, MenuPane::List)
                        && runtime.menu_state.selected + 1 < entry_views.len()
                    {
                        runtime.menu_state.selected += 1;
                    }
                }
                Action::Confirm => {
                    if matches!(focus, MenuPane::List) {
                        if let Some(entry) = entries.get(runtime.menu_state.selected) {
                            if entry.selectable {
                                runtime.menu_state.focus = MenuFocus::Detail;
                                runtime.menu_state.active_submenu = Some(entry.action.clone());
                            }
                        }
                    }
                }
                Action::Cancel | Action::Menu => {
                    if matches!(focus, MenuPane::Detail) {
                        runtime.menu_state.focus = MenuFocus::List;
                        runtime.menu_state.active_submenu = None;
                    } else {
                        runtime.close_menu();
                        return Ok(());
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
                        );
                    })? {
                        return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "quit"));
                    }
                }
                _ => {}
            }
        }
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
                "Progress panel (stub).".to_string(),
                "TODO: render ui/progress.json.".to_string(),
            ],
        },
        _ => MenuPanelView {
            title,
            lines: vec!["Menu panel not configured.".to_string()],
        },
    }
}

fn build_party_summary(runtime: &GameRuntime) -> Vec<String> {
    if runtime.party.active.is_empty() {
        return vec!["No party members.".to_string()];
    }
    let mut lines = Vec::new();
    for member_id in &runtime.party.active {
        if let Some(actor) = runtime.party.roster.get(member_id) {
            let hp = actor.derived_stats.get("hp").copied().unwrap_or(0);
            let mp = actor.derived_stats.get("mp").copied().unwrap_or(0);
            let line = format!("{}  Lv{}  HP {}  MP {}", actor.name, actor.level, hp, mp);
            lines.push(line);
        }
    }
    if lines.is_empty() {
        lines.push("No party members.".to_string());
    }
    lines
}

fn menu_detail_panel(label: &str, action: &str) -> MenuPanelView {
    MenuPanelView {
        title: label.to_string(),
        lines: vec![
            format!("{} menu not implemented.", label),
            format!("TODO: implement '{}' submenu.", action),
        ],
    }
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
