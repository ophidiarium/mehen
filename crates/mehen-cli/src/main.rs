//! `mehen` — 1.0 CLI binary.
//!
//! `metrics` runs through the new architecture (mehen-engine + per-language
//! analyzer crates). `diff` and `top-offenders` delegate to the legacy
//! implementations in the root `mehen` library; phase 5 follow-up ports
//! those orchestrators into `mehen-engine` and removes the `mehen` lib
//! dependency.

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
        Command::Diff(opts) => {
            // `run_diff` calls `process::exit` itself on failure, so this
            // path never returns ExitCode unless the diff succeeded.
            mehen::diff::run_diff(opts);
            ExitCode::Success
        }
        Command::TopOffenders(opts) => {
            mehen::top_offenders::run_top_offenders(opts);
            ExitCode::Success
        }
    }
}
