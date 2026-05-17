//! `mehen` — 1.0 CLI binary.
//!
//! Phase 1 scope: define the new command surface (`metrics`, `diff`,
//! `top-offenders`) and exit code contract per the rewrite plan §2 / §4.1.
//!
//! The 1.0 binary lives in `crates/mehen-cli/`. Until Phase 5 finishes the
//! orchestration, the existing pre-1.0 binary at the workspace root keeps
//! providing user-facing behavior; this binary is a stub that exits with a
//! "not yet implemented in 1.0" message for any real work. The shape of
//! commands and flags is what gets locked down in Phase 1.

mod args;
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
        Command::Metrics(_args) => not_yet_implemented("metrics"),
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
