use std::env;
use std::path::PathBuf;

use engine::{rules::RulesFile, rules::Ruleset, world::WorldState, world::WorldsFile, Engine};
use tui::input::InputFile;
use tui::renderer::RenderMode;

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
    let content_dir = parse_content_dir(&args).unwrap_or_else(|| PathBuf::from("content"));
    let rules_path = content_dir.join("rules.json");
    let worlds_path = content_dir.join("worlds.json");
    let input_path = content_dir.join("input.json");

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

    if let Err(err) = InputFile::load(&input_path) {
        eprintln!("Failed to load input bindings: {}", err);
    }

    let _engine = Engine::new(rules, world);

    match render_mode {
        RenderMode::Auto => println!("Starting OpenCrystal (render: auto)..."),
        RenderMode::Wide => println!("Starting OpenCrystal (render: wide)..."),
        RenderMode::Modern => println!("Starting OpenCrystal (render: modern)..."),
    }
}

fn run_validate() {
    println!("Validating content...");
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
