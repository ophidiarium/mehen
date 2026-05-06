//! `mehen top-offenders` — rank files by one or more metrics.
//!
//! Usage:
//! ```text
//! mehen top-offenders \
//!     --metric loc.lloc --metric cognitive \
//!     --max-results 5 --output-format json \
//!     src/module/rocket
//! ```
//!
//! `--metric` is repeatable; order matters — the final list is sorted
//! lexicographically by the tuple of metric values (first metric is the
//! primary key, second breaks ties, etc). Each metric's sort direction comes
//! from its [`Polarity`]: `LowerIsBetter` metrics sort descending (highest
//! first — those are the worst offenders), `HigherIsBetter` metrics sort
//! ascending (lowest first).

use std::cmp::Ordering;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread::available_parallelism;

use crate::concurrent_files::{ConcurrentRunner, FilesData};
use crate::langs::{LANG, get_from_ext, get_function_spaces};
use crate::metric_selector::{MetricSelector, Polarity, parse_metric_selectors};
use crate::mk_globset;
use crate::tools::{guess_language, read_file_with_eol};

// ── CLI args ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum TopOffendersFormat {
    Markdown,
    Json,
}

#[derive(clap::Args, Debug)]
pub(crate) struct TopOffendersOpts {
    /// Metric to rank by. Repeatable; order matters — the first `--metric` is
    /// the primary sort key, the next breaks ties, etc.
    ///
    /// Prefix with `+` to flip a metric to higher-is-better (best at top) or
    /// `-` for lower-is-better. Without a prefix the metric's default polarity
    /// is used. Known names: `cyclomatic`, `cognitive`, `nom.functions`,
    /// `loc.lloc`, `mi.original`, `mi.sei`, `mi.visual_studio`,
    /// `halstead.volume`, `abc`.
    #[clap(
        long = "metric",
        short = 'M',
        required = true,
        num_args = 1,
        allow_hyphen_values = true
    )]
    metrics: Vec<String>,

    /// Maximum number of offenders to return.
    #[clap(long, default_value_t = 10)]
    max_results: usize,

    /// Output format.
    #[clap(long, short = 'O', value_enum, default_value_t = TopOffendersFormat::Markdown)]
    output_format: TopOffendersFormat,

    /// Glob to include files. Repeat the flag for multiple patterns.
    #[clap(long, short = 'I', num_args = 1)]
    include: Vec<String>,

    /// Glob to exclude files. Repeat the flag for multiple patterns.
    #[clap(long, short = 'X', num_args = 1)]
    exclude: Vec<String>,

    /// Number of parser jobs.
    #[clap(long, short = 'j')]
    num_jobs: Option<usize>,

    /// Language type override (skip auto-detection).
    #[clap(long, short)]
    language_type: Option<String>,

    /// One or more files or directories to analyze.
    #[clap(required = true, num_args = 1..)]
    paths: Vec<PathBuf>,
}

// ── Per-file offender data ─────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
struct MetricValue {
    name: &'static str,
    label: &'static str,
    value: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
struct FileOffender {
    path: PathBuf,
    metrics: Vec<MetricValue>,
}

// ── Concurrent runner glue ─────────────────────────────────────────────

struct TopOffendersCfg {
    selectors: Vec<MetricSelector>,
    language: Option<LANG>,
    results: Arc<Mutex<Vec<FileOffender>>>,
}

fn act_on_file(path: PathBuf, cfg: &TopOffendersCfg) -> std::io::Result<()> {
    let source = match read_file_with_eol(&path)? {
        Some(s) => s,
        None => return Ok(()),
    };

    let language = if let Some(language) = cfg.language {
        language
    } else if let Some(language) = guess_language(&source, &path).0 {
        language
    } else {
        return Ok(());
    };

    let space = match get_function_spaces(&language, source, &path, None) {
        Some(space) => space,
        None => return Ok(()),
    };

    let metrics: Vec<MetricValue> = cfg
        .selectors
        .iter()
        .map(|sel| MetricValue {
            name: sel.name,
            label: sel.label,
            value: (sel.extract)(&space),
        })
        .collect();

    cfg.results
        .lock()
        .expect("top-offenders results mutex poisoned")
        .push(FileOffender { path, metrics });

    Ok(())
}

// ── Sorting ─────────────────────────────────────────────────────────────

/// Compare two offenders by the tuple of metric values, respecting polarity
/// on each axis.
///
/// For `LowerIsBetter` metrics, the *larger* value is "worse" and should sort
/// to the top, so we reverse the natural ordering. For `HigherIsBetter`, the
/// *smaller* value is worse — keep the natural ordering. Ties on the first
/// metric fall through to the next.
fn cmp_offenders(a: &FileOffender, b: &FileOffender, selectors: &[MetricSelector]) -> Ordering {
    for (i, sel) in selectors.iter().enumerate() {
        // Lengths are guaranteed equal: `act_on_file` builds `metrics` by
        // iterating `selectors` 1:1. Defensive `get` avoids a panic on any
        // accidental skew and treats missing as 0.0.
        let av = a.metrics.get(i).map(|m| m.value).unwrap_or(0.0);
        let bv = b.metrics.get(i).map(|m| m.value).unwrap_or(0.0);
        let base = av.total_cmp(&bv);
        let ord = match sel.polarity {
            Polarity::LowerIsBetter => base.reverse(), // worst (biggest) first
            Polarity::HigherIsBetter => base,          // worst (smallest) first
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }
    // Final tie-breaker: stable by path so output is deterministic.
    a.path.cmp(&b.path)
}

// ── Output ──────────────────────────────────────────────────────────────

fn print_json(offenders: &[FileOffender]) {
    let json =
        serde_json::to_string_pretty(offenders).expect("offender list is always serializable");
    writeln!(std::io::stdout().lock(), "{json}").expect("failed to write to stdout");
}

fn print_markdown(offenders: &[FileOffender], selectors: &[MetricSelector]) {
    let mut out = String::new();

    if offenders.is_empty() {
        out.push_str("## Top Offenders\n\nNo matching files found.\n");
        write!(std::io::stdout().lock(), "{out}").expect("failed to write to stdout");
        return;
    }

    // Title lists the ranking metrics in order.
    let metric_list = selectors
        .iter()
        .map(|s| s.name)
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!("## Top Offenders (by {metric_list})\n\n"));

    // Header
    out.push_str("| File |");
    for sel in selectors {
        out.push_str(&format!(" {} |", sel.label));
    }
    out.push('\n');

    // Separator: left-align path, right-align numeric columns.
    out.push_str("|---|");
    for _ in selectors {
        out.push_str("---:|");
    }
    out.push('\n');

    // Rows
    for o in offenders {
        out.push_str(&format!("| {} |", o.path.display()));
        for mv in &o.metrics {
            out.push_str(&format!(" {} |", format_value(mv.value)));
        }
        out.push('\n');
    }

    write!(std::io::stdout().lock(), "{out}").expect("failed to write to stdout");
}

fn format_value(v: f64) -> String {
    if v.is_nan() {
        "NaN".to_string()
    } else if v == v.trunc() && v.abs() < 1e18 {
        format!("{}", v as i64)
    } else {
        format!("{:.2}", v)
    }
}

fn resolve_num_jobs(requested: Option<usize>, available: Option<usize>) -> usize {
    requested.unwrap_or_else(|| available.unwrap_or(2))
}

// ── Orchestration ──────────────────────────────────────────────────────

pub(crate) fn run_top_offenders(opts: TopOffendersOpts) {
    let selectors = parse_metric_selectors(&opts.metrics);
    if selectors.is_empty() {
        // `parse_metric_selectors` logs each unknown name via `log::warn!`;
        // if nothing resolved, the whole command has nothing to rank.
        log::error!("No valid metrics selected. See `mehen top-offenders --help`.");
        process::exit(1);
    }

    let language = match opts.language_type.as_deref().filter(|s| !s.is_empty()) {
        Some(raw) => match get_from_ext(raw) {
            Some(language) => Some(language),
            None => {
                log::error!("Unknown language type '{raw}'.");
                process::exit(1);
            }
        },
        None => None,
    };

    let num_jobs = resolve_num_jobs(
        opts.num_jobs,
        available_parallelism().ok().map(|threads| threads.get()),
    );

    let include = mk_globset(opts.include);
    let exclude = mk_globset(opts.exclude);

    let results: Arc<Mutex<Vec<FileOffender>>> = Arc::new(Mutex::new(Vec::new()));

    let cfg = TopOffendersCfg {
        selectors: selectors.clone(),
        language,
        results: results.clone(),
    };

    let files_data = FilesData {
        include,
        exclude,
        paths: opts.paths,
    };

    if let Err(e) = ConcurrentRunner::new(num_jobs, act_on_file).run(cfg, files_data) {
        log::error!("{e}");
        process::exit(1);
    }

    let mut offenders = Arc::try_unwrap(results)
        .expect("results Arc still has outstanding references")
        .into_inner()
        .expect("results mutex poisoned");

    offenders.sort_by(|a, b| cmp_offenders(a, b, &selectors));
    offenders.truncate(opts.max_results);

    match opts.output_format {
        TopOffendersFormat::Json => print_json(&offenders),
        TopOffendersFormat::Markdown => print_markdown(&offenders, &selectors),
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn selector(name: &'static str, polarity: Polarity) -> MetricSelector {
        MetricSelector {
            name,
            label: name,
            polarity,
            // Unused in cmp tests — comparison reads pre-computed values.
            extract: |_| 0.0,
        }
    }

    fn offender(path: &str, values: &[(&'static str, f64)]) -> FileOffender {
        FileOffender {
            path: PathBuf::from(path),
            metrics: values
                .iter()
                .map(|(n, v)| MetricValue {
                    name: n,
                    label: n,
                    value: *v,
                })
                .collect(),
        }
    }

    #[test]
    fn lower_is_better_puts_largest_value_first() {
        let selectors = [selector("loc.lloc", Polarity::LowerIsBetter)];
        let mut xs = [
            offender("small.rs", &[("loc.lloc", 10.0)]),
            offender("huge.rs", &[("loc.lloc", 1000.0)]),
            offender("medium.rs", &[("loc.lloc", 100.0)]),
        ];
        xs.sort_by(|a, b| cmp_offenders(a, b, &selectors));
        assert_eq!(xs[0].path, PathBuf::from("huge.rs"));
        assert_eq!(xs[1].path, PathBuf::from("medium.rs"));
        assert_eq!(xs[2].path, PathBuf::from("small.rs"));
    }

    #[test]
    fn higher_is_better_puts_smallest_value_first() {
        // e.g. maintainability index: the worst maintainability is the lowest MI.
        let selectors = [selector("mi.visual_studio", Polarity::HigherIsBetter)];
        let mut xs = [
            offender("good.rs", &[("mi", 120.0)]),
            offender("bad.rs", &[("mi", 10.0)]),
            offender("mid.rs", &[("mi", 60.0)]),
        ];
        xs.sort_by(|a, b| cmp_offenders(a, b, &selectors));
        assert_eq!(xs[0].path, PathBuf::from("bad.rs"));
        assert_eq!(xs[1].path, PathBuf::from("mid.rs"));
        assert_eq!(xs[2].path, PathBuf::from("good.rs"));
    }

    #[test]
    fn ties_on_primary_metric_fall_through_to_secondary() {
        // Two files tie on loc.lloc; the secondary key (cognitive) decides.
        let selectors = [
            selector("loc.lloc", Polarity::LowerIsBetter),
            selector("cognitive", Polarity::LowerIsBetter),
        ];
        let mut xs = [
            offender("a.rs", &[("loc.lloc", 100.0), ("cognitive", 5.0)]),
            offender("b.rs", &[("loc.lloc", 100.0), ("cognitive", 30.0)]),
            offender("c.rs", &[("loc.lloc", 50.0), ("cognitive", 999.0)]),
        ];
        xs.sort_by(|a, b| cmp_offenders(a, b, &selectors));
        // Both 100-loc files come before 50-loc file; between them, cognitive
        // 30 (worse) beats cognitive 5.
        assert_eq!(xs[0].path, PathBuf::from("b.rs"));
        assert_eq!(xs[1].path, PathBuf::from("a.rs"));
        assert_eq!(xs[2].path, PathBuf::from("c.rs"));
    }

    #[test]
    fn all_tied_breaks_by_path_for_determinism() {
        let selectors = [selector("loc.lloc", Polarity::LowerIsBetter)];
        let mut xs = [
            offender("zzz.rs", &[("loc.lloc", 42.0)]),
            offender("aaa.rs", &[("loc.lloc", 42.0)]),
            offender("mmm.rs", &[("loc.lloc", 42.0)]),
        ];
        xs.sort_by(|a, b| cmp_offenders(a, b, &selectors));
        assert_eq!(xs[0].path, PathBuf::from("aaa.rs"));
        assert_eq!(xs[1].path, PathBuf::from("mmm.rs"));
        assert_eq!(xs[2].path, PathBuf::from("zzz.rs"));
    }

    #[test]
    fn mixed_polarities_sort_each_axis_independently() {
        // Primary: lower-is-better (loc); secondary: higher-is-better (mi).
        // On a loc tie, the file with lower mi is the worse offender.
        let selectors = [
            selector("loc.lloc", Polarity::LowerIsBetter),
            selector("mi.visual_studio", Polarity::HigherIsBetter),
        ];
        let mut xs = [
            offender("low_loc_high_mi.rs", &[("loc", 10.0), ("mi", 120.0)]),
            offender("high_loc_high_mi.rs", &[("loc", 200.0), ("mi", 120.0)]),
            offender("high_loc_low_mi.rs", &[("loc", 200.0), ("mi", 30.0)]),
        ];
        xs.sort_by(|a, b| cmp_offenders(a, b, &selectors));
        assert_eq!(xs[0].path, PathBuf::from("high_loc_low_mi.rs"));
        assert_eq!(xs[1].path, PathBuf::from("high_loc_high_mi.rs"));
        assert_eq!(xs[2].path, PathBuf::from("low_loc_high_mi.rs"));
    }

    #[test]
    fn format_value_renders_integers_without_decimals() {
        assert_eq!(format_value(42.0), "42");
        assert_eq!(format_value(0.0), "0");
        assert_eq!(format_value(1.5), "1.50");
        assert_eq!(format_value(100.567), "100.57");
    }

    #[test]
    fn explicit_num_jobs_is_not_predecremented() {
        assert_eq!(resolve_num_jobs(Some(8), Some(16)), 8);
    }

    #[test]
    fn num_jobs_falls_back_to_conservative_thread_count() {
        assert_eq!(resolve_num_jobs(None, None), 2);
    }
}
