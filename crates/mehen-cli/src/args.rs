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

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Analyze exactly one file and emit a metrics report.
    Metrics(MetricsArgs),
    /// Compare metrics between two git revisions.
    Diff(DiffArgs),
    /// Rank files by one or more metrics (worst offenders first).
    TopOffenders(TopOffendersArgs),
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

#[derive(Debug, Args)]
pub struct DiffArgs {
    /// Base revision.
    #[arg(long, default_value = "origin/main")]
    pub from: String,

    /// Head revision.
    #[arg(long, default_value = "HEAD")]
    pub to: String,

    /// Restrict diff to these path prefixes.
    #[arg(long, num_args = 0..)]
    pub paths: Vec<PathBuf>,

    /// Output format. `github-markdown` is the action-friendly comment body;
    /// `json` is the machine output the action consumes for thresholds.
    #[arg(long, default_value = "github-markdown")]
    pub format: DiffFormat,

    /// Threshold rules. Repeatable; format `selector=value` (e.g.
    /// `cognitive=5`, `loc.lloc=120`).
    #[arg(long = "threshold", num_args = 0..)]
    pub thresholds: Vec<String>,

    /// Built-in profile preset.
    #[arg(long, default_value = "default")]
    pub profile: Profile,
}

#[derive(Debug, Args)]
pub struct TopOffendersArgs {
    /// Roots to walk. At least one path is required.
    #[arg(required = true, num_args = 1..)]
    pub paths: Vec<PathBuf>,

    /// Metric selectors. Repeatable; ranks by the first selector and breaks
    /// ties with subsequent selectors.
    #[arg(long = "metric", num_args = 0..)]
    pub metrics: Vec<String>,

    /// Glob to include files.
    #[arg(long, num_args = 0..)]
    pub include: Vec<String>,

    /// Glob to exclude files.
    #[arg(long, num_args = 0..)]
    pub exclude: Vec<String>,

    /// Maximum number of results to print.
    #[arg(long, default_value_t = 20)]
    pub max_results: usize,

    /// Output format.
    #[arg(long, default_value = "markdown")]
    pub format: TopOffendersFormat,

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
pub enum DiffFormat {
    #[value(name = "github-markdown")]
    GithubMarkdown,
    Json,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum TopOffendersFormat {
    Markdown,
    Json,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum Profile {
    Default,
    Ci,
    Strict,
}
