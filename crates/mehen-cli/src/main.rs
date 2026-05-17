//! `mehen` — 1.0 CLI binary.
//!
//! Phase 5 surface: `metrics`, `diff`, `top-offenders` all run through
//! the new architecture (mehen-engine + per-language analyzer crates).

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
        Command::Diff(args) => commands::diff(args),
        Command::TopOffenders(args) => commands::top_offenders(args),
    }
}
