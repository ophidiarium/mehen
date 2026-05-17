//! Transitional pre-1.0 binary kept around so the legacy
//! `tests/pr_comment_golden.rs` snapshot tests can still drive the
//! Markdown / diff pipeline through a subprocess until phase 5 ports
//! `mehen diff` into `mehen-cli`.
//!
//! Removed entirely once the diff/Markdown machinery has been physically
//! relocated into `mehen-engine` and `mehen-report`.

use std::io::{self, Write};
use std::process;

use mehen::diff::DiffOpts;
use mehen::top_offenders::TopOffendersOpts;
use mehen::{diff, top_offenders};

#[derive(clap::Parser, Debug)]
#[clap(
    name = "mehen-pre1",
    author,
    about = "Pre-1.0 transitional binary (legacy diff + top-offenders).",
    disable_version_flag = true
)]
struct Cli {
    #[clap(long, short = 'V', global = true)]
    version: bool,

    #[clap(long, global = true, requires = "version")]
    json: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    Diff(DiffOpts),
    TopOffenders(TopOffendersOpts),
}

fn main() {
    env_logger::init();
    let cli = <Cli as clap::Parser>::parse();

    if cli.version {
        let mut stdout = io::stdout().lock();
        if cli.json {
            let payload = serde_json::json!({
                "name": env!("CARGO_PKG_NAME"),
                "version": env!("CARGO_PKG_VERSION"),
            });
            writeln!(stdout, "{}", serde_json::to_string(&payload).unwrap()).unwrap();
        } else {
            writeln!(
                stdout,
                "{} {}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            )
            .unwrap();
        }
        return;
    }

    match cli.command {
        Some(Command::Diff(opts)) => diff::run_diff(opts),
        Some(Command::TopOffenders(opts)) => top_offenders::run_top_offenders(opts),
        None => {
            log::error!(
                "no subcommand provided; run `mehen-pre1 --help` to see `diff` and `top-offenders`."
            );
            process::exit(1);
        }
    }
}
