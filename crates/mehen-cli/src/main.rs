//! `mehen` — 1.0 CLI binary.
//!
//! Phase 1-5 scope: define the new command surface (`metrics`, `diff`,
//! `top-offenders`) and exit code contract per the rewrite plan §2 / §4.1,
//! and wire `mehen metrics` end-to-end through the engine. `diff` and
//! `top-offenders` remain stubs until the orchestrators land in follow-up
//! phases.

mod args;
mod commands;
mod exit;

use clap::Parser;

use args::{Cli, Command};
use exit::ExitCode;

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    let code = run(cli);
    std::process::exit(code.into());
}

fn run(cli: Cli) -> ExitCode {
    match cli.command {
        Command::Metrics(args) => commands::metrics(args),
        Command::Diff(_args) => not_yet_implemented("diff"),
        Command::TopOffenders(_args) => not_yet_implemented("top-offenders"),
    }
}

fn not_yet_implemented(name: &str) -> ExitCode {
    log::error!(
        "`mehen {name}` is not yet wired up in the 1.0 binary; \
         use the pre-1.0 binary at the workspace root until Phase 5 ports \
         the orchestrators."
    );
    ExitCode::SetupError
}
