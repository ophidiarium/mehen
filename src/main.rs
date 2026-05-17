#![allow(clippy::upper_case_acronyms)]

mod alterator;
mod checker;
mod ci;
mod concurrent_files;
mod diff;
#[cfg(feature = "markdown")]
mod diff_markdown;
mod formats;
mod getter;
mod git;
mod langs;
mod languages;
mod macros;
#[cfg(feature = "markdown")]
mod markdown;
mod metric_selector;
mod metrics;
mod node;
mod parser;
mod preproc;
mod rust_metric_helpers;
mod spaces;
mod tools;
mod top_offenders;
mod traits;

use std::io::{self, Write};
use std::process;

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::diff::DiffOpts;
use crate::top_offenders::TopOffendersOpts;

pub(crate) fn mk_globset(elems: Vec<String>) -> GlobSet {
    if elems.is_empty() {
        return GlobSet::empty();
    }

    let mut globset = GlobSetBuilder::new();
    elems.iter().filter(|e| !e.is_empty()).for_each(|e| {
        if let Ok(glob) = Glob::new(e) {
            globset.add(glob);
        }
    });
    globset.build().map_or(GlobSet::empty(), |globset| globset)
}

#[derive(clap::Parser, Debug)]
#[clap(
    name = "mehen",
    author,
    about = "Compare metrics between revisions and rank files by metric.",
    disable_version_flag = true
)]
struct Cli {
    /// Print version information.
    #[clap(long, short = 'V', global = true)]
    version: bool,

    /// Emit output as JSON (currently only supported with --version).
    #[clap(long, global = true, requires = "version")]
    json: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Compare metrics between two git revisions.
    Diff(DiffOpts),
    /// Rank files by one or more metrics (worst offenders first).
    TopOffenders(TopOffendersOpts),
}

fn main() {
    env_logger::init();
    let cli = <Cli as clap::Parser>::parse();

    if cli.version {
        print_version(cli.json);
        return;
    }

    match cli.command {
        Some(Command::Diff(opts)) => diff::run_diff(opts),
        Some(Command::TopOffenders(opts)) => top_offenders::run_top_offenders(opts),
        None => {
            log::error!(
                "no subcommand provided; run `mehen --help` to see `diff` and `top-offenders`."
            );
            process::exit(1);
        }
    }
}

fn print_version(as_json: bool) {
    let mut stdout = io::stdout().lock();
    if as_json {
        let payload = serde_json::json!({
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
        });
        writeln!(
            stdout,
            "{}",
            serde_json::to_string(&payload).expect("serialize version payload")
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
