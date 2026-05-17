//! `mehen` — 1.0 CLI binary.
//!
//! `metrics` runs through the new architecture (mehen-engine + per-language
//! analyzer crates). `diff` and `top-offenders` delegate to the
//! pre-1.0 orchestrators that now live in `mehen_engine::legacy::*`
//! until phase 5 follow-up commits split them across `mehen-engine`,
//! the per-language crates and `mehen-report` while preserving every
//! existing test fixture and snapshot.

mod args;
mod commands;
mod exit;

use clap::Parser;

use args::{Cli, Command};
use exit::ExitCode;

fn main() {
    env_logger::init();
    // Register the legacy embedded-code dispatch so the moved
    // `mehen-markdown` analyzer can fold fenced-code metrics into its
    // output. Idempotent — safe to call multiple times.
    mehen_engine::init_markdown();
    let cli = Cli::parse();

    let code = run(cli);
    std::process::exit(code.into());
}

fn run(cli: Cli) -> ExitCode {
    match cli.command {
        Command::Metrics(args) => commands::metrics(args),
        Command::Diff(opts) => {
            mehen_engine::run_diff(opts);
            ExitCode::Success
        }
        Command::TopOffenders(opts) => {
            mehen_engine::run_top_offenders(opts);
            ExitCode::Success
        }
    }
}
