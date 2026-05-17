use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

/// `mehen` — code metrics CLI.
#[derive(Debug, Parser)]
#[command(
    name = "mehen",
    bin_name = "mehen",
    about = "Compute and report code metrics.",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// During the v1 rewrite the `Diff` and `TopOffenders` subcommands flatten
/// the legacy `DiffOpts` / `TopOffendersOpts` argument shapes from the
/// pre-1.0 root crate so the existing tests against those flag surfaces
/// keep passing through the new binary. Phase 5 follow-up replaces the
/// flattened legacy types with the rewrite-plan-defined arg shape and
/// drops the dependency on the root `mehen` library.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Analyze exactly one file and emit a metrics report.
    Metrics(MetricsArgs),
    /// Compare metrics between two git revisions.
    Diff(mehen::diff::DiffOpts),
    /// Rank files by one or more metrics (worst offenders first).
    TopOffenders(mehen::top_offenders::TopOffendersOpts),
}

#[derive(Debug, Args)]
pub struct MetricsArgs {
    /// Path to the file to analyze. `mehen metrics` never walks directories.
    pub path: PathBuf,

    /// Override language detection.
    #[arg(long)]
    pub language: Option<String>,

    /// Output format.
    #[arg(long, default_value = "json")]
    pub format: OutputFormat,

    /// Pretty-print JSON output.
    #[arg(long)]
    pub pretty: bool,

    /// Built-in profile preset.
    #[arg(long, default_value = "default")]
    pub profile: Profile,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Markdown,
    Yaml,
    Toml,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Profile {
    Default,
    Ci,
    Strict,
}
