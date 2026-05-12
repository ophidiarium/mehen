#![allow(clippy::upper_case_acronyms)]

mod getter;
mod macros;

mod alterator;

mod node;

mod metrics;

mod languages;

mod checker;

mod output;

mod spaces;

mod find;

mod function;

mod count;

mod preproc;

mod langs;

mod tools;

mod concurrent_files;

mod traits;

mod parser;

mod formats;

mod ci;
mod diff;
mod git;
#[cfg(feature = "markdown")]
mod markdown;
mod metric_selector;
mod rust_metric_helpers;
mod top_offenders;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread::available_parallelism;

use clap::builder::{PossibleValuesParser, TypedValueParser};
use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::concurrent_files::{ConcurrentRunner, FilesData};
use crate::count::{Count, CountCfg};
use crate::diff::DiffOpts;
use crate::find::{Find, FindCfg};
use crate::formats::Format;
use crate::function::{Function, FunctionCfg};
use crate::langs::{LANG, action, get_from_ext, get_function_spaces};
use crate::output::{Dump, DumpCfg};
use crate::spaces::{Metrics, MetricsCfg};
use crate::tools::{guess_language, read_file_with_eol};
use crate::top_offenders::TopOffendersOpts;

#[derive(Debug)]
struct Config {
    dump: bool,
    find_filter: Vec<String>,
    count_filter: Vec<String>,
    language: Option<LANG>,
    function: bool,
    metrics: bool,
    output_format: Option<Format>,
    output: Option<PathBuf>,
    pretty: bool,
    line_start: Option<usize>,
    line_end: Option<usize>,
    count_lock: Option<Arc<Mutex<Count>>>,
}

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

fn act_on_file(path: PathBuf, cfg: &Config) -> std::io::Result<()> {
    let source = if let Some(source) = read_file_with_eol(&path)? {
        source
    } else {
        return Ok(());
    };

    let language = if let Some(language) = cfg.language {
        language
    } else if let Some(language) = guess_language(&source, &path).0 {
        language
    } else {
        return Ok(());
    };

    if cfg.dump {
        let cfg = DumpCfg {
            line_start: cfg.line_start,
            line_end: cfg.line_end,
        };
        action::<Dump>(&language, source, &path, None, cfg)
    } else if cfg.metrics {
        // Markdown metrics run through a dedicated documentation pipeline,
        // not the source-code `FuncSpace` path. Intercept here so the output
        // schema stays shaped per §23 instead of masquerading as code spaces.
        #[cfg(feature = "markdown")]
        if matches!(language, LANG::Markdown) {
            let source_str = String::from_utf8_lossy(&source).into_owned();
            let metrics = markdown::analyze_markdown(&source_str, &path);
            if let Some(output_format) = &cfg.output_format {
                output_format.dump_formats(metrics, path, cfg.output.as_ref(), cfg.pretty);
            } else {
                // Default stdout rendering is a pretty-printed JSON blob —
                // the same as the rest of the metrics pipeline's dev path.
                match serde_json::to_string_pretty(&metrics) {
                    Ok(rendered) => {
                        writeln!(io::stdout().lock(), "{rendered}")
                            .expect("failed to write markdown metrics");
                    }
                    Err(e) => {
                        log::error!("Failed to serialize markdown metrics: {e}");
                    }
                }
            }
            return Ok(());
        }
        if let Some(output_format) = &cfg.output_format {
            if let Some(space) = get_function_spaces(&language, source, &path, None) {
                output_format.dump_formats(space, path, cfg.output.as_ref(), cfg.pretty);
            }
            Ok(())
        } else {
            let cfg = MetricsCfg { path };
            let path = cfg.path.clone();
            action::<Metrics>(&language, source, &path, None, cfg)
        }
    } else if cfg.function {
        let cfg = FunctionCfg { path: path.clone() };
        action::<Function>(&language, source, &path, None, cfg)
    } else if !cfg.find_filter.is_empty() {
        let cfg = FindCfg {
            path: path.clone(),
            filters: cfg.find_filter.clone(),
            line_start: cfg.line_start,
            line_end: cfg.line_end,
        };
        action::<Find>(&language, source, &path, None, cfg)
    } else if let Some(count_lock) = &cfg.count_lock {
        let cfg = CountCfg {
            filters: cfg.count_filter.clone(),
            stats: count_lock.clone(),
        };
        action::<Count>(&language, source, &path, None, cfg)
    } else {
        Ok(())
    }
}

#[derive(clap::Parser, Debug)]
#[clap(
    name = "mehen",
    author,
    about = "Analyze source code.",
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

    #[command(flatten)]
    analyze: AnalyzeOpts,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Compare metrics between two git revisions.
    Diff(DiffOpts),
    /// Rank files by one or more metrics (worst offenders first).
    TopOffenders(TopOffendersOpts),
}

#[derive(clap::Args, Debug)]
struct AnalyzeOpts {
    /// Input files to analyze.
    #[clap(long, short, value_parser)]
    paths: Vec<PathBuf>,
    /// Output AST to stdout.
    #[clap(long, short)]
    dump: bool,
    /// Find nodes of the given type.
    #[clap(long, short, number_of_values = 1)]
    find: Vec<String>,
    /// Get functions and their spans.
    #[clap(long, short = 'F')]
    function: bool,
    /// Count nodes of the given type: comma separated list.
    #[clap(long, short = 'C', number_of_values = 1)]
    count: Vec<String>,
    /// Compute different metrics.
    #[clap(long, short)]
    metrics: bool,
    /// Glob to include files.
    #[clap(long, short = 'I', num_args(0..))]
    include: Vec<String>,
    /// Glob to exclude files.
    #[clap(long, short = 'X', num_args(0..))]
    exclude: Vec<String>,
    /// Number of jobs.
    #[clap(long, short = 'j')]
    num_jobs: Option<usize>,
    /// Language type.
    #[clap(long, short)]
    language_type: Option<String>,
    /// Output metrics as different formats.
    #[clap(long, short = 'O', value_parser = PossibleValuesParser::new(Format::all())
        .map(|s| s.parse::<Format>().unwrap()))]
    output_format: Option<Format>,
    /// Dump a pretty json file.
    #[clap(long = "pr")]
    pretty: bool,
    /// Output file/directory.
    #[clap(long, short, value_parser)]
    output: Option<PathBuf>,
    /// Line start.
    #[clap(long = "ls")]
    line_start: Option<usize>,
    /// Line end.
    #[clap(long = "le")]
    line_end: Option<usize>,
    /// Print the warnings.
    #[clap(long, short)]
    warning: bool,
}

fn run_analyze(opts: AnalyzeOpts) {
    let count_lock = if !opts.count.is_empty() {
        Some(Arc::new(Mutex::new(Count::default())))
    } else {
        None
    };

    let output_is_dir = opts.output.as_ref().map(|p| p.is_dir()).unwrap_or(false);
    if opts.metrics && opts.output_format.is_some() && opts.output.is_some() && !output_is_dir {
        log::error!("The output parameter must be a directory");
        process::exit(1);
    }

    if opts.output.is_some() && (!opts.metrics || opts.output_format.is_none()) {
        log::error!("--output is only supported together with --metrics and --output-format");
        process::exit(1);
    }

    let typ = opts.language_type.unwrap_or_default();
    let language = if typ.is_empty() {
        None
    } else {
        get_from_ext(&typ)
    };

    let num_jobs = opts
        .num_jobs
        .map(|num_jobs| std::cmp::max(2, num_jobs) - 1)
        .unwrap_or_else(|| {
            std::cmp::max(
                2,
                available_parallelism()
                    .expect("Unrecoverable: Failed to get thread count")
                    .get(),
            ) - 1
        });

    let include = mk_globset(opts.include);
    let exclude = mk_globset(opts.exclude);

    let cfg = Config {
        dump: opts.dump,
        find_filter: opts.find,
        count_filter: opts.count,
        language,
        function: opts.function,
        metrics: opts.metrics,
        output_format: opts.output_format,
        pretty: opts.pretty,
        output: opts.output.clone(),
        line_start: opts.line_start,
        line_end: opts.line_end,
        count_lock: count_lock.clone(),
    };

    let files_data = FilesData {
        include,
        exclude,
        paths: opts.paths,
    };

    let _all_files = match ConcurrentRunner::new(num_jobs, act_on_file).run(cfg, files_data) {
        Ok(all_files) => all_files,
        Err(e) => {
            log::error!("{e}");
            process::exit(1);
        }
    };

    if let Some(count) = count_lock {
        let count = Arc::try_unwrap(count).unwrap().into_inner().unwrap();
        writeln!(io::stdout(), "{count}").expect("failed to write to stdout");
    }
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
        None => run_analyze(cli.analyze),
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
