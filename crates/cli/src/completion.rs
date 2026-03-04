use std::io;

use clap::CommandFactory;
use clap_complete::{generate, Generator, Shell};

use crate::{Cli, CompletionShell};

pub(crate) fn run_completion(shell: CompletionShell) {
    let mut command = Cli::command();
    match shell {
        CompletionShell::Bash => print_completion(Shell::Bash, &mut command),
        CompletionShell::Zsh => print_completion(Shell::Zsh, &mut command),
    }
}

fn print_completion<G: Generator>(generator: G, command: &mut clap::Command) {
    generate(
        generator,
        command,
        command.get_name().to_string(),
        &mut io::stdout(),
    );
}
