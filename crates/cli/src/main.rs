use std::env;

use engine::{rules::Ruleset, world::WorldState, Engine};
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
    let rules = Ruleset::demo();
    let world = WorldState::new("gaia", "overworld_gaia", (20, 14));
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

fn print_usage() {
    println!(
        "OpenCrystal\n\nUsage:\n  cryst play [--render=auto|wide|modern]\n  cryst validate\n  cryst new-project\n  cryst build"
    );
}
