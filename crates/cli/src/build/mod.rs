mod args;
mod common;
mod docs;
mod new;
mod new_project;
mod strings;
mod upgrade;

use args::BuildMapOptions;
use common::resolve_content_dir;

pub fn run_build(args: Vec<String>) {
    if args.is_empty() {
        print_build_usage();
        return;
    }
    match args[0].as_str() {
        "new" => new::run_build_new(&args[1..]),
        "map" => run_build_map(&args[1..]),
        "upgrade" => upgrade::run_build_upgrade(&args[1..]),
        "strings" => strings::run_build_strings(&args[1..]),
        "new-project" => new_project::run_build_new_project(&args[1..]),
        "docs" => docs::run_build_docs(&args[1..]),
        other => {
            eprintln!("Unknown build command: {}", other);
            print_build_usage();
        }
    }
}

fn run_build_map(args: &[String]) {
    let options = BuildMapOptions::from_args(args);
    let Some(id) = options.id else {
        eprintln!("Missing map id. Example: cryst build map castle_hall");
        return;
    };
    if options.used_content_dir {
        eprintln!("--content-dir is not supported for build map. Use --content <pack_path>.");
        return;
    }
    let content_dir = match resolve_content_dir(options.content_dir.as_deref()) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    };

    if let Err(err) = crate::map_editor::run_map_editor(&content_dir, &id) {
        eprintln!("{}", err);
    }
}

fn print_build_usage() {
    println!(
        "Build usage:\n  cryst build new <kind> <id> [--content path] [--content-dir path] [--name label] [--force]\n  cryst build map <id> [--content path]\n  cryst build upgrade [--content path] [--content-dir path] [--dry-run]\n  cryst build strings [--content path] [--content-dir path] [--force]\n  cryst build new-project <name> [--path path]\n  cryst build docs [-s|--schemas] [-a|--architecture] [-c|--content-authoring] [-j|--jobs]"
    );
}
