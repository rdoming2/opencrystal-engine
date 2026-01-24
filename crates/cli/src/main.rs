use std::env;
use std::path::PathBuf;

use engine::{
    content::Content,
    rules::Ruleset,
    runtime::{GameRuntime, GameState},
    world::WorldState,
    Engine,
};
use tui::input::InputFile;
use tui::renderer::RenderMode;
use tui::ui::{BattleUiFile, ProgressUiFile, TitleUiFile};

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
    let battle_ui_path = content_dir.join("ui").join("battle.json");
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

    if let Err(err) = InputFile::load(&input_path) {
        eprintln!("Failed to load input bindings: {}", err);
    }

    if let Err(err) = TitleUiFile::load(&title_ui_path) {
        eprintln!("Failed to load title UI: {}", err);
    }

    if let Err(err) = BattleUiFile::load(&battle_ui_path) {
        eprintln!("Failed to load battle UI: {}", err);
    }

    if let Err(err) = ProgressUiFile::load(&progress_ui_path) {
        eprintln!("Failed to load progress UI: {}", err);
    }

    let _engine = Engine::new(rules.clone(), world);
    let mut runtime = GameRuntime::new(content);
    runtime.start_new_game(&rules);

    match render_mode {
        RenderMode::Auto => println!("Starting OpenCrystal (render: auto)..."),
        RenderMode::Wide => println!("Starting OpenCrystal (render: wide)..."),
        RenderMode::Modern => println!("Starting OpenCrystal (render: modern)..."),
    }

    if let Some(event_id) = runtime.active_event.as_deref() {
        println!("Queued start event: {}", event_id);
    } else {
        println!("Starting in overworld.");
    }

    while let Some(step) = runtime.next_event_step() {
        print_event_step(&step);
    }

    if runtime.is_event_complete() && runtime.state == GameState::Overworld {
        println!("Event queue completed.");
    }
}

fn print_event_step(step: &engine::events::EventStep) {
    match step.r#type.as_str() {
        "dialog" => {
            let speaker = step.speaker.as_deref().unwrap_or("Narrator");
            let text = step.text.as_deref().unwrap_or("");
            println!("{}: {}", speaker, text);
        }
        "narration" => {
            let text = step.text.as_deref().unwrap_or("");
            println!("Narration: {}", text);
        }
        "set_flag" => {
            if let Some(flag) = &step.flag {
                println!("Set flag: {}", flag);
            }
        }
        "require_flags" => {
            if let Some(flags) = &step.flags {
                println!("Require flags: {}", flags.join(", "));
            }
        }
        "start_dialog" => {
            if let Some(dialog) = &step.dialog {
                println!("Start dialog: {}", dialog);
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
                println!("Open shop: {}", shop);
            }
        }
        "npc_show" | "npc_hide" | "npc_move" | "npc_set_sprite" => {
            println!("NPC action: {}", step.r#type);
        }
        other => {
            println!("Event step: {}", other);
        }
    }
}

fn run_validate() {
    let args: Vec<String> = env::args().skip(2).collect();
    let content_dir = parse_content_dir(&args).unwrap_or_else(|| PathBuf::from("content/demo"));
    let mut errors = engine::validate::validate_content(&content_dir);

    let input_path = content_dir.join("input.json");
    let title_ui_path = content_dir.join("ui").join("title.json");
    let battle_ui_path = content_dir.join("ui").join("battle.json");
    let progress_ui_path = content_dir.join("ui").join("progress.json");

    if let Err(err) = InputFile::load(&input_path) {
        errors.push(format!("input.json: {}", err));
    }
    if let Err(err) = TitleUiFile::load(&title_ui_path) {
        errors.push(format!("ui/title.json: {}", err));
    }
    if let Err(err) = BattleUiFile::load(&battle_ui_path) {
        errors.push(format!("ui/battle.json: {}", err));
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
