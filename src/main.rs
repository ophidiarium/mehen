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

mod ops;

mod find;

mod function;

mod count;

mod preproc;

mod langs;

mod tools;

mod concurrent_files;

mod traits;

mod parser;

mod comment_rm;

mod formats;

mod ci;
mod diff;
mod git;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread::available_parallelism;

use clap::builder::{PossibleValuesParser, TypedValueParser};
use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::comment_rm::{CommentRm, CommentRmCfg};
use crate::concurrent_files::{ConcurrentRunner, FilesData};
use crate::count::{Count, CountCfg};
use crate::diff::DiffOpts;
use crate::find::{Find, FindCfg};
use crate::formats::Format;
use crate::function::{Function, FunctionCfg};
use crate::langs::{LANG, action, get_from_ext, get_function_spaces, get_ops};
use crate::ops::{OpsCfg, OpsCode};
use crate::output::{Dump, DumpCfg};
use crate::spaces::{Metrics, MetricsCfg};
use crate::tools::{guess_language, read_file_with_eol};

#[derive(Debug)]
struct Config {
    dump: bool,
    in_place: bool,
    comments: bool,
    find_filter: Vec<String>,
    count_filter: Vec<String>,
    language: Option<LANG>,
    function: bool,
    metrics: bool,
    ops: bool,
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
    } else if cfg.ops {
        if let Some(output_format) = &cfg.output_format {
            let ops = get_ops(&language, source, &path, None).unwrap();
            output_format.dump_formats(ops, path, cfg.output.as_ref(), cfg.pretty);
            Ok(())
        } else {
            let cfg = OpsCfg { path };
            let path = cfg.path.clone();
            action::<OpsCode>(&language, source, &path, None, cfg)
        }
    } else if cfg.comments {
        let cfg = CommentRmCfg {
            in_place: cfg.in_place,
            path,
        };
        let path = cfg.path.clone();
        action::<CommentRm>(&language, source, &path, None, cfg)
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
#[clap(name = "mehen", version, author, about = "Analyze source code.")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    analyze: AnalyzeOpts,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Compare metrics between two git revisions.
    Diff(DiffOpts),
}

#[derive(clap::Args, Debug)]
struct AnalyzeOpts {
    /// Input files to analyze.
    #[clap(long, short, value_parser)]
    paths: Vec<PathBuf>,
    /// Output AST to stdout.
    #[clap(long, short)]
    dump: bool,
    /// Remove comments in the specified files.
    #[clap(long, short)]
    comments: bool,
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
    /// Retrieve all operands and operators in a code.
    #[clap(long, conflicts_with = "metrics")]
    ops: bool,
    /// Do action in place.
    #[clap(long, short)]
    in_place: bool,
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
    if (opts.metrics || opts.ops) && opts.output.is_some() && !output_is_dir {
        log::error!("The output parameter must be a directory");
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
        in_place: opts.in_place,
        comments: opts.comments,
        find_filter: opts.find,
        count_filter: opts.count,
        language,
        function: opts.function,
        metrics: opts.metrics,
        ops: opts.ops,
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

    match cli.command {
        Some(Command::Diff(opts)) => diff::run_diff(opts),
        None => run_analyze(cli.analyze),
    }
}
