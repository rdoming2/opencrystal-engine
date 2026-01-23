use std::env;
use std::path::PathBuf;

use engine::{
    dialog::DialogFile,
    encounters::EncountersFile,
    entities::{
        EnemiesFile, EquipmentFile, ItemsFile, JobsFile, NpcsFile, ShopsFile, SpellsFile,
        VehiclesFile,
    },
    events::EventFile,
    maps::MapFile,
    rules::RulesFile,
    rules::Ruleset,
    stats::StatsFile,
    world::WorldState,
    world::WorldsFile,
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
    let rules_path = content_dir.join("rules.json");
    let worlds_path = content_dir.join("worlds.json");
    let input_path = content_dir.join("input.json");
    let title_ui_path = content_dir.join("ui").join("title.json");
    let battle_ui_path = content_dir.join("ui").join("battle.json");
    let stats_path = content_dir.join("stats.json");
    let entities_dir = content_dir.join("entities");
    let maps_dir = content_dir.join("maps");
    let events_dir = content_dir.join("events");
    let dialog_dir = content_dir.join("dialog");
    let progress_ui_path = content_dir.join("ui").join("progress.json");

    let rules = match RulesFile::load(&rules_path) {
        Ok(file) => Ruleset::from_file(file),
        Err(err) => {
            eprintln!("Failed to load rules: {}", err);
            Ruleset::demo()
        }
    };

    let world = match WorldsFile::load(&worlds_path) {
        Ok(file) => {
            if let Some(world) = file.worlds.first() {
                WorldState::new(&world.id, &world.starting_map, (0, 0))
            } else {
                WorldState::new("gaia", "overworld_gaia", (20, 14))
            }
        }
        Err(err) => {
            eprintln!("Failed to load worlds: {}", err);
            WorldState::new("gaia", "overworld_gaia", (20, 14))
        }
    };

    load_optional(
        &stats_path,
        |path| StatsFile::load(path),
        "stats",
        &mut Vec::new(),
    );
    load_entities(&entities_dir);
    load_directory(&maps_dir, |path| MapFile::load(path), "map");
    load_directory(&events_dir, |path| EventFile::load(path), "event");
    load_directory(&dialog_dir, |path| DialogFile::load(path), "dialog");
    load_optional(
        &entities_dir.join("encounters.json"),
        |path| EncountersFile::load(path),
        "encounters",
        &mut Vec::new(),
    );

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

    let _engine = Engine::new(rules, world);

    match render_mode {
        RenderMode::Auto => println!("Starting OpenCrystal (render: auto)..."),
        RenderMode::Wide => println!("Starting OpenCrystal (render: wide)..."),
        RenderMode::Modern => println!("Starting OpenCrystal (render: modern)..."),
    }
}

fn load_entities(entities_dir: &PathBuf) {
    let mut errors = Vec::new();
    load_optional(
        &entities_dir.join("jobs.json"),
        |path| JobsFile::load(path),
        "jobs",
        &mut errors,
    );
    load_optional(
        &entities_dir.join("spells.json"),
        |path| SpellsFile::load(path),
        "spells",
        &mut errors,
    );
    load_optional(
        &entities_dir.join("items.json"),
        |path| ItemsFile::load(path),
        "items",
        &mut errors,
    );
    load_optional(
        &entities_dir.join("equipment.json"),
        |path| EquipmentFile::load(path),
        "equipment",
        &mut errors,
    );
    load_optional(
        &entities_dir.join("enemies.json"),
        |path| EnemiesFile::load(path),
        "enemies",
        &mut errors,
    );
    load_optional(
        &entities_dir.join("vehicles.json"),
        |path| VehiclesFile::load(path),
        "vehicles",
        &mut errors,
    );
    load_optional(
        &entities_dir.join("shops.json"),
        |path| ShopsFile::load(path),
        "shops",
        &mut errors,
    );
    load_optional(
        &entities_dir.join("npcs.json"),
        |path| NpcsFile::load(path),
        "npcs",
        &mut errors,
    );

    for error in errors {
        eprintln!("Failed to load {}", error);
    }
}

fn load_optional<T, F>(path: &PathBuf, loader: F, label: &str, errors: &mut Vec<String>)
where
    F: FnOnce(&std::path::Path) -> Result<T, String>,
{
    if !path.exists() {
        return;
    }
    if let Err(err) = loader(path.as_path()) {
        errors.push(format!("{}: {}", label, err));
    }
}

fn load_directory<T, F>(dir: &PathBuf, loader: F, label: &str)
where
    F: Fn(&std::path::Path) -> Result<T, String>,
{
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("Failed to read {} directory: {}", label, err);
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        if let Err(err) = loader(path.as_path()) {
            eprintln!("Failed to load {}: {}", label, err);
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
