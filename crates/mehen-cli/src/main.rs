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

use std::io::{self, Write};

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

    if cli.version {
        print_version(cli.json);
        return;
    }

    let Some(command) = cli.command else {
        // Match clap's default "no subcommand and no global action"
        // behaviour: print help to stderr and exit non-zero.
        let _ = <Cli as clap::CommandFactory>::command().print_help();
        std::process::exit(ExitCode::SetupError.into());
    };

    let code = run(command);
    std::process::exit(code.into());
}

fn run(command: Command) -> ExitCode {
    match command {
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

/// Print the CLI version. With `as_json = true`, emits a
/// `{"name":"mehen","version":"X.Y.Z"}` payload that the GitHub
/// Action consumes via `mehen --version --json` to stamp its sticky
/// PR-comment footer. The plain form (`as_json = false`) prints
/// `mehen X.Y.Z` — identical to clap's auto-generated output.
fn print_version(as_json: bool) {
    let mut stdout = io::stdout().lock();
    if as_json {
        // Hand-rolled JSON to avoid pulling `serde_json` into the CLI
        // crate just for this one payload — the env-var values never
        // contain characters that need escaping.
        writeln!(
            stdout,
            "{{\"name\":\"{}\",\"version\":\"{}\"}}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        )
        .expect("failed to write version payload");
    } else {
        writeln!(
            stdout,
            "{} {}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        )
        .expect("failed to write version");
    }
}
