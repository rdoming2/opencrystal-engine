mod args;
mod common;
mod docs;
mod new;
mod new_project;
mod strings;
mod upgrade;

use args::BuildMapArgs;
pub(crate) use args::{BuildCommand, BuildNewProjectArgs};
use common::resolve_content_dir;

pub fn run_build(command: BuildCommand) {
    match command {
        BuildCommand::New(args) => new::run_build_new(&args.to_argv()),
        BuildCommand::Map(args) => run_build_map(args),
        BuildCommand::Upgrade(args) => upgrade::run_build_upgrade(&args.to_argv()),
        BuildCommand::Strings(args) => strings::run_build_strings(&args.to_argv()),
        BuildCommand::NewProject(args) => new_project::run_build_new_project(&args.to_argv()),
        BuildCommand::Docs(args) => docs::run_build_docs(&args.to_argv()),
    }
}

fn run_build_map(options: BuildMapArgs) {
    if options.content_dir.is_some() {
        eprintln!("--content-dir is not supported for build map. Use --content <pack_path>.");
        return;
    }
    let content_dir = match resolve_content_dir(options.content.as_deref()) {
        Ok(dir) => dir,
        Err(err) => {
            eprintln!("{}", err);
            return;
        }
    };

    if let Err(err) = crate::map_editor::run_map_editor(&content_dir, &options.id) {
        eprintln!("{}", err);
    }
}

fn print_build_usage() {
    println!(
        "Build usage:\n  cryst build new <kind> <id> [--content path] [--content-dir path] [--name label] [--force]\n  cryst build map <id> [--content path]\n  cryst build upgrade [--content path] [--content-dir path] [--dry-run]\n  cryst build strings [--content path] [--content-dir path] [--force]\n  cryst build new-project <name> [--path path]\n  cryst build docs [-s|--schemas] [-a|--architecture] [-c|--content-authoring] [-j|--jobs]"
    );
}
