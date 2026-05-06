use std::io::Write;
use std::path::{Component, Path, PathBuf};

use crate::ci;
use crate::git::{self, ChangeStatus, GitError};
use crate::langs::{get_from_ext, get_function_spaces};
use crate::metric_selector::{MetricSelector, Polarity, parse_metric_selectors};
use crate::mk_globset;

// ── Types ──────────────────────────────────────────────────────────────

const LINGUIST_GENERATED_ATTR: &str = "linguist-generated";

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum DiffFormat {
    Markdown,
    Json,
}

// ── Per-file diff data ─────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
struct MetricDiff {
    name: &'static str,
    label: &'static str,
    current: f64,
    baseline: f64,
    delta: f64,
    polarity: Polarity,
    is_new: bool,
    is_deleted: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
struct FileDiff {
    path: PathBuf,
    metrics: Vec<MetricDiff>,
    is_new: bool,
    is_deleted: bool,
}

impl FileDiff {
    fn all_unchanged(&self) -> bool {
        self.metrics.iter().all(|m| m.delta == 0.0)
    }

    /// Sort key: total function count descending, then path ascending.
    fn sort_key(&self) -> (std::cmp::Reverse<i64>, PathBuf) {
        let functions = self
            .metrics
            .iter()
            .find(|m| m.name == "nom.functions")
            .map(|m| m.current as i64)
            .unwrap_or(0);
        (std::cmp::Reverse(functions), self.path.clone())
    }
}

// ── CLI args ───────────────────────────────────────────────────────────

#[derive(clap::Args, Debug)]
pub(crate) struct DiffOpts {
    /// Base revision to compare from.
    #[clap(long)]
    from: Option<String>,
    /// Head revision to compare to.
    #[clap(long)]
    to: Option<String>,
    /// Comma-separated metrics to compare
    /// (default: cyclomatic,cognitive,nom.functions,loc.lloc,mi.visual_studio).
    /// Prefix with + for higher-is-better, - for lower-is-better.
    #[clap(long, short = 'M', value_delimiter = ',')]
    metrics: Vec<String>,
    /// Repository-relative files or directories to compare.
    #[clap(long, short, value_parser, num_args(0..))]
    paths: Vec<PathBuf>,
    /// Glob to include files.
    #[clap(long, short = 'I', num_args(0..))]
    include: Vec<String>,
    /// Glob to exclude files.
    #[clap(long, short = 'X', num_args(0..))]
    exclude: Vec<String>,
    /// Output format.
    #[clap(long, short = 'O', value_enum)]
    output_format: Option<DiffFormat>,
    /// Show files where all metrics are unchanged.
    #[clap(long)]
    show_unchanged: bool,
    /// Skip files marked as generated via `linguist-generated` git attributes.
    #[clap(
        long,
        default_value_t = true,
        action = clap::ArgAction::Set,
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "true"
    )]
    ignore_generated: bool,
}

// ── Orchestration ──────────────────────────────────────────────────────

pub(crate) fn run_diff(opts: DiffOpts) {
    if let Err(e) = run_diff_inner(opts) {
        log::error!("{e}");
        std::process::exit(1);
    }
}

fn run_diff_inner(opts: DiffOpts) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Resolve refs
    let ci_ctx = ci::detect();
    let (from_ref, to_ref) = resolve_refs(&opts, &ci_ctx);

    // 2. Get changed file list
    let repo = git::open_repo()?;
    let from_label = git::friendly_ref_label(&repo, &from_ref);
    let changed = get_changed_files(&repo, &from_ref, &to_ref, &ci_ctx)?;

    // 3. Filter files
    let include = mk_globset(opts.include);
    let exclude = mk_globset(opts.exclude);
    let paths = normalize_path_filters(&opts.paths);
    let selectors = parse_metric_selectors(&opts.metrics);
    let mut generated_filter = opts
        .ignore_generated
        .then(|| GeneratedFilter::new(&repo))
        .transpose()?;

    let mut filtered = Vec::new();
    for cf in changed {
        let p = &cf.path;
        if !path_is_selected(p, &paths)
            || (!include.is_empty() && !include.is_match(p))
            || (!exclude.is_empty() && exclude.is_match(p))
        {
            continue;
        }

        if let Some(filter) = generated_filter.as_mut()
            && filter.is_generated(p)?
        {
            continue;
        }

        if p.extension()
            .and_then(|e| e.to_str())
            .and_then(get_from_ext)
            .is_none()
        {
            continue;
        }

        filtered.push(cf);
    }

    // 4. Compute metrics for each file
    let mut diffs = Vec::new();
    for cf in &filtered {
        let ext = cf.path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let lang = match get_from_ext(ext) {
            Some(l) => l,
            None => continue,
        };

        let is_deleted = cf.status == ChangeStatus::Deleted;
        let is_new = cf.status == ChangeStatus::Added;

        let baseline_space = if is_new {
            None
        } else {
            match git::read_blob(&repo, &from_ref, &cf.path) {
                Ok(Some(bytes)) => get_function_spaces(&lang, bytes, &cf.path, None),
                Ok(None) => None,
                Err(e) => {
                    log::warn!("Skipping baseline for {}: {e}", cf.path.display());
                    None
                }
            }
        };

        let current_space = if is_deleted {
            None
        } else {
            match git::read_blob(&repo, &to_ref, &cf.path) {
                Ok(Some(bytes)) => get_function_spaces(&lang, bytes, &cf.path, None),
                Ok(None) => None,
                Err(e) => {
                    log::warn!("Skipping current for {}: {e}", cf.path.display());
                    None
                }
            }
        };

        let metric_diffs: Vec<MetricDiff> = selectors
            .iter()
            .map(|sel| {
                let baseline = baseline_space
                    .as_ref()
                    .map(|s| (sel.extract)(s))
                    .unwrap_or(0.0);
                let current = current_space
                    .as_ref()
                    .map(|s| (sel.extract)(s))
                    .unwrap_or(0.0);
                MetricDiff {
                    name: sel.name,
                    label: sel.label,
                    current,
                    baseline,
                    delta: current - baseline,
                    polarity: sel.polarity,
                    is_new: is_new && baseline_space.is_none(),
                    is_deleted,
                }
            })
            .collect();

        diffs.push(FileDiff {
            path: cf.path.clone(),
            metrics: metric_diffs,
            is_new: is_new && baseline_space.is_none(),
            is_deleted,
        });
    }

    // 5. Filter unchanged
    if !opts.show_unchanged {
        diffs.retain(|d| !d.all_unchanged());
    }

    // 6. Sort
    diffs.sort_by_key(|a| a.sort_key());

    // 7. Output
    let format = opts.output_format.unwrap_or(DiffFormat::Markdown);
    match format {
        DiffFormat::Markdown => print_markdown(&diffs, &selectors, &from_label, &from_ref, &to_ref),
        DiffFormat::Json => print_json(&diffs),
    }

    Ok(())
}

// ── Generated-file filtering ───────────────────────────────────────────

struct GeneratedFilter<'repo> {
    attrs: gix::AttributeStack<'repo>,
    outcome: gix::attrs::search::Outcome,
}

impl<'repo> GeneratedFilter<'repo> {
    fn new(repo: &'repo gix::Repository) -> Result<Self, Box<dyn std::error::Error>> {
        let index = repo.index_or_empty()?;
        let source = gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping
            .adjust_for_bare(repo.is_bare());
        let attrs = repo.attributes_only(&index, source)?;
        let outcome = attrs.selected_attribute_matches([LINGUIST_GENERATED_ATTR]);
        Ok(Self { attrs, outcome })
    }

    fn is_generated(&mut self, path: &Path) -> std::io::Result<bool> {
        self.attrs
            .at_path(path, None)?
            .matching_attributes(&mut self.outcome);
        Ok(self
            .outcome
            .iter_selected()
            .next()
            .is_some_and(|matched| is_linguist_generated_state(matched.assignment.state)))
    }
}

fn is_linguist_generated_state(state: gix::attrs::StateRef<'_>) -> bool {
    match state {
        gix::attrs::StateRef::Set => true,
        gix::attrs::StateRef::Value(value) => {
            let value: &[u8] = value.as_bstr().as_ref();
            value.eq_ignore_ascii_case(b"true")
        }
        gix::attrs::StateRef::Unset | gix::attrs::StateRef::Unspecified => false,
    }
}

// ── Ref resolution ─────────────────────────────────────────────────────

fn resolve_refs(opts: &DiffOpts, ci_ctx: &Option<ci::CiContext>) -> (String, String) {
    if let (Some(from), Some(to)) = (&opts.from, &opts.to) {
        return (from.clone(), to.clone());
    }

    if let Some(ctx) = ci_ctx {
        let to = opts
            .to
            .clone()
            .or_else(|| ctx.head_sha.clone())
            .unwrap_or_else(|| "HEAD".to_string());

        let from = opts
            .from
            .clone()
            .unwrap_or_else(|| match ctx.event_name.as_str() {
                "push" => "HEAD~1".to_string(),
                "pull_request" | "merge_group" => ctx
                    .base_ref
                    .as_ref()
                    .map(|b| format!("origin/{b}"))
                    .unwrap_or_else(|| "origin/main".to_string()),
                _ => "main".to_string(),
            });

        return (from, to);
    }

    let from = opts.from.clone().unwrap_or_else(|| "main".to_string());
    let to = opts.to.clone().unwrap_or_else(|| "HEAD".to_string());
    (from, to)
}

fn get_changed_files(
    repo: &gix::Repository,
    from: &str,
    to: &str,
    ci_ctx: &Option<ci::CiContext>,
) -> Result<Vec<git::ChangedFile>, GitError> {
    // For push events with changed_files from payload, use those directly
    if let Some(ctx) = ci_ctx
        && ctx.event_name == "push"
        && let Some(ref files) = ctx.changed_files
    {
        return Ok(files
            .iter()
            .map(|p| git::ChangedFile {
                path: p.clone(),
                // We don't know the exact status from the payload after dedup,
                // treat as Modified (will check both revs anyway)
                status: ChangeStatus::Modified,
            })
            .collect());
    }

    git::changed_files(repo, from, to)
}

fn normalize_path_filters(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .map(|path| normalize_path_filter(path))
        .collect()
}

fn normalize_path_filter(path: &Path) -> PathBuf {
    let mut cleaned = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => cleaned.push(part),
            other => cleaned.push(other.as_os_str()),
        }
    }

    cleaned
}

fn path_is_selected(path: &Path, paths: &[PathBuf]) -> bool {
    paths.is_empty()
        || paths.iter().any(|selected| {
            selected.as_os_str().is_empty() || path == selected || path.starts_with(selected)
        })
}

// ── Markdown output ────────────────────────────────────────────────────

fn print_markdown(
    diffs: &[FileDiff],
    selectors: &[MetricSelector],
    from_label: &str,
    from: &str,
    to: &str,
) {
    let mut out = String::new();

    out.push_str(&format!(
        "## [Mehen](https://github.com/ophidiarium/mehen) Summary (`{from}`..`{to}`)\n\n"
    ));

    if diffs.is_empty() {
        out.push_str("No metric changes detected.\n");
        write!(std::io::stdout().lock(), "{out}").unwrap();
        return;
    }

    // Header
    out.push_str("| File |");
    for sel in selectors {
        out.push_str(&format!(" {} |", sel.label));
    }
    out.push('\n');

    // Separator
    out.push_str("|---|");
    for _ in selectors {
        out.push_str("---:|");
    }
    out.push('\n');

    // Rows
    for diff in diffs {
        out.push_str(&format!("| {} |", diff.path.display()));
        for md in &diff.metrics {
            out.push(' ');
            out.push_str(&format_metric_cell(md, from_label));
            out.push_str(" |");
        }
        out.push('\n');
    }

    write!(std::io::stdout().lock(), "{out}").unwrap();
}

fn format_metric_cell(md: &MetricDiff, from: &str) -> String {
    let current = format_f64(md.current);

    if md.is_new {
        return format!("{current} \u{1F195}"); // 🆕
    }

    if md.is_deleted {
        let baseline = format_f64(md.baseline);
        let emoji = trend_emoji(md.delta, md.polarity);
        return format!("0 (was: {baseline}) {emoji}");
    }

    if md.delta == 0.0 {
        return format!("{current} \u{26AA}"); // ⚪
    }

    let baseline = format_f64(md.baseline);
    let emoji = trend_emoji(md.delta, md.polarity);
    format!("{current} ({from}: {baseline}) {emoji}")
}

fn trend_emoji(delta: f64, polarity: Polarity) -> &'static str {
    if delta == 0.0 {
        return "\u{26AA}"; // ⚪
    }
    match polarity {
        Polarity::LowerIsBetter => {
            if delta > 0.0 {
                "\u{1F534}" // 🔴
            } else {
                "\u{1F7E2}" // 🟢
            }
        }
        Polarity::HigherIsBetter => {
            if delta > 0.0 {
                "\u{1F7E2}" // 🟢
            } else {
                "\u{1F534}" // 🔴
            }
        }
    }
}

fn format_f64(v: f64) -> String {
    if v == v.trunc() {
        format!("{}", v as i64)
    } else {
        format!("{:.2}", v)
    }
}

// ── JSON output ────────────────────────────────────────────────────────

fn print_json(diffs: &[FileDiff]) {
    let json = serde_json::to_string_pretty(diffs).unwrap();
    writeln!(std::io::stdout().lock(), "{json}").unwrap();
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser as _;

    #[derive(clap::Parser, Debug)]
    struct TestDiffCli {
        #[command(flatten)]
        opts: DiffOpts,
    }

    #[test]
    fn test_parse_metric_selectors_defaults() {
        let selectors = parse_metric_selectors(&[]);
        assert_eq!(selectors.len(), 5);
        assert_eq!(selectors[0].name, "cyclomatic");
        assert_eq!(selectors[1].name, "cognitive");
        assert_eq!(selectors[2].name, "nom.functions");
        assert_eq!(selectors[3].name, "loc.lloc");
        assert_eq!(selectors[4].name, "mi.visual_studio");
    }

    #[test]
    fn test_parse_metric_selectors_custom() {
        let specs = vec!["mi.original".to_string(), "halstead.volume".to_string()];
        let selectors = parse_metric_selectors(&specs);
        assert_eq!(selectors.len(), 2);
        assert_eq!(selectors[0].name, "mi.original");
        assert_eq!(selectors[0].polarity, Polarity::HigherIsBetter);
        assert_eq!(selectors[1].name, "halstead.volume");
        assert_eq!(selectors[1].polarity, Polarity::LowerIsBetter);
    }

    #[test]
    fn test_parse_metric_selectors_all_mi_variants() {
        let specs = vec![
            "mi.original".to_string(),
            "mi.sei".to_string(),
            "mi.visual_studio".to_string(),
        ];
        let selectors = parse_metric_selectors(&specs);
        assert_eq!(selectors.len(), 3);
        assert_eq!(selectors[0].name, "mi.original");
        assert_eq!(selectors[1].name, "mi.sei");
        assert_eq!(selectors[2].name, "mi.visual_studio");
        for sel in &selectors {
            assert_eq!(sel.polarity, Polarity::HigherIsBetter);
        }
    }

    #[test]
    fn test_parse_metric_selectors_bare_mi_is_unknown() {
        let specs = vec!["mi".to_string()];
        let selectors = parse_metric_selectors(&specs);
        assert!(selectors.is_empty());
    }

    #[test]
    fn test_parse_metric_selectors_polarity_override() {
        let specs = vec![
            "+nom.functions".to_string(),
            "-mi.visual_studio".to_string(),
        ];
        let selectors = parse_metric_selectors(&specs);
        assert_eq!(selectors.len(), 2);
        assert_eq!(selectors[0].name, "nom.functions");
        assert_eq!(selectors[0].polarity, Polarity::HigherIsBetter);
        assert_eq!(selectors[1].name, "mi.visual_studio");
        assert_eq!(selectors[1].polarity, Polarity::LowerIsBetter);
    }

    #[test]
    fn test_parse_metric_selectors_unknown() {
        let specs = vec!["nonexistent".to_string()];
        let selectors = parse_metric_selectors(&specs);
        assert!(selectors.is_empty());
    }

    #[test]
    fn test_ignore_generated_defaults_to_true() {
        let cli = TestDiffCli::try_parse_from(["mehen"]).unwrap();
        assert!(cli.opts.ignore_generated);
    }

    #[test]
    fn test_ignore_generated_accepts_bare_flag() {
        let cli = TestDiffCli::try_parse_from(["mehen", "--ignore-generated"]).unwrap();
        assert!(cli.opts.ignore_generated);
    }

    #[test]
    fn test_ignore_generated_can_be_disabled() {
        let cli = TestDiffCli::try_parse_from(["mehen", "--ignore-generated=false"]).unwrap();
        assert!(!cli.opts.ignore_generated);
    }

    #[test]
    fn test_trend_emoji_lower_is_better() {
        assert_eq!(trend_emoji(1.0, Polarity::LowerIsBetter), "\u{1F534}");
        assert_eq!(trend_emoji(-1.0, Polarity::LowerIsBetter), "\u{1F7E2}");
        assert_eq!(trend_emoji(0.0, Polarity::LowerIsBetter), "\u{26AA}");
    }

    #[test]
    fn test_trend_emoji_higher_is_better() {
        assert_eq!(trend_emoji(1.0, Polarity::HigherIsBetter), "\u{1F7E2}");
        assert_eq!(trend_emoji(-1.0, Polarity::HigherIsBetter), "\u{1F534}");
        assert_eq!(trend_emoji(0.0, Polarity::HigherIsBetter), "\u{26AA}");
    }

    #[test]
    fn test_format_f64_integer() {
        assert_eq!(format_f64(42.0), "42");
        assert_eq!(format_f64(0.0), "0");
    }

    #[test]
    fn test_format_f64_decimal() {
        assert_eq!(format_f64(2.75), "2.75");
        assert_eq!(format_f64(100.567), "100.57");
    }

    #[test]
    fn test_format_metric_cell_new() {
        let md = MetricDiff {
            name: "cyclomatic",
            label: "Cyclomatic",
            current: 5.0,
            baseline: 0.0,
            delta: 5.0,
            polarity: Polarity::LowerIsBetter,
            is_new: true,
            is_deleted: false,
        };
        assert_eq!(format_metric_cell(&md, "main"), "5 \u{1F195}");
    }

    #[test]
    fn test_format_metric_cell_unchanged() {
        let md = MetricDiff {
            name: "cyclomatic",
            label: "Cyclomatic",
            current: 5.0,
            baseline: 5.0,
            delta: 0.0,
            polarity: Polarity::LowerIsBetter,
            is_new: false,
            is_deleted: false,
        };
        assert_eq!(format_metric_cell(&md, "main"), "5 \u{26AA}");
    }

    #[test]
    fn test_format_metric_cell_increase_lower_is_better() {
        let md = MetricDiff {
            name: "cyclomatic",
            label: "Cyclomatic",
            current: 12.0,
            baseline: 8.0,
            delta: 4.0,
            polarity: Polarity::LowerIsBetter,
            is_new: false,
            is_deleted: false,
        };
        assert_eq!(format_metric_cell(&md, "main"), "12 (main: 8) \u{1F534}");
    }

    #[test]
    fn test_format_metric_cell_deleted() {
        let md = MetricDiff {
            name: "cyclomatic",
            label: "Cyclomatic",
            current: 0.0,
            baseline: 10.0,
            delta: -10.0,
            polarity: Polarity::LowerIsBetter,
            is_new: false,
            is_deleted: true,
        };
        assert_eq!(format_metric_cell(&md, "main"), "0 (was: 10) \u{1F7E2}");
    }

    #[test]
    fn test_file_diff_all_unchanged() {
        let diff = FileDiff {
            path: PathBuf::from("foo.rs"),
            metrics: vec![MetricDiff {
                name: "cyclomatic",
                label: "Cyclomatic",
                current: 5.0,
                baseline: 5.0,
                delta: 0.0,
                polarity: Polarity::LowerIsBetter,
                is_new: false,
                is_deleted: false,
            }],
            is_new: false,
            is_deleted: false,
        };
        assert!(diff.all_unchanged());
    }

    #[test]
    fn test_resolve_refs_explicit() {
        let opts = DiffOpts {
            from: Some("abc".to_string()),
            to: Some("def".to_string()),
            metrics: vec![],
            paths: vec![],
            include: vec![],
            exclude: vec![],
            output_format: None,
            show_unchanged: false,
            ignore_generated: true,
        };
        let (from, to) = resolve_refs(&opts, &None);
        assert_eq!(from, "abc");
        assert_eq!(to, "def");
    }

    #[test]
    fn test_resolve_refs_no_ci() {
        let opts = DiffOpts {
            from: None,
            to: None,
            metrics: vec![],
            paths: vec![],
            include: vec![],
            exclude: vec![],
            output_format: None,
            show_unchanged: false,
            ignore_generated: true,
        };
        let (from, to) = resolve_refs(&opts, &None);
        assert_eq!(from, "main");
        assert_eq!(to, "HEAD");
    }

    #[test]
    fn test_resolve_refs_github_pr() {
        let ctx = ci::CiContext {
            provider: ci::CiProvider::GitHubActions,
            event_name: "pull_request".to_string(),
            base_ref: Some("develop".to_string()),
            head_sha: Some("abc123".to_string()),
            changed_files: None,
            pr_number: Some(42),
            repository: Some("owner/repo".to_string()),
        };
        let opts = DiffOpts {
            from: None,
            to: None,
            metrics: vec![],
            paths: vec![],
            include: vec![],
            exclude: vec![],
            output_format: None,
            show_unchanged: false,
            ignore_generated: true,
        };
        let (from, to) = resolve_refs(&opts, &Some(ctx));
        assert_eq!(from, "origin/develop");
        assert_eq!(to, "abc123");
    }

    #[test]
    fn test_resolve_refs_github_push() {
        let ctx = ci::CiContext {
            provider: ci::CiProvider::GitHubActions,
            event_name: "push".to_string(),
            base_ref: None,
            head_sha: Some("def456".to_string()),
            changed_files: None,
            pr_number: None,
            repository: Some("owner/repo".to_string()),
        };
        let opts = DiffOpts {
            from: None,
            to: None,
            metrics: vec![],
            paths: vec![],
            include: vec![],
            exclude: vec![],
            output_format: None,
            show_unchanged: false,
            ignore_generated: true,
        };
        let (from, to) = resolve_refs(&opts, &Some(ctx));
        assert_eq!(from, "HEAD~1");
        assert_eq!(to, "def456");
    }

    #[test]
    fn test_normalize_path_filters() {
        let paths = normalize_path_filters(&[
            PathBuf::from("."),
            PathBuf::from("./internal"),
            PathBuf::from("cmd/tally/"),
        ]);

        assert_eq!(
            paths,
            vec![
                PathBuf::new(),
                PathBuf::from("internal"),
                PathBuf::from("cmd/tally")
            ]
        );
    }

    #[test]
    fn test_path_is_selected() {
        let paths = vec![PathBuf::from("internal"), PathBuf::from("main.go")];

        assert!(path_is_selected(
            Path::new("internal/config/config.go"),
            &paths
        ));
        assert!(path_is_selected(Path::new("main.go"), &paths));
        assert!(!path_is_selected(Path::new("internal2/config.go"), &paths));
        assert!(!path_is_selected(Path::new("cmd/tally/main.go"), &paths));

        let paths_with_root = vec![PathBuf::from("internal"), PathBuf::new()];
        assert!(path_is_selected(
            Path::new("cmd/tally/main.go"),
            &paths_with_root
        ));
    }

    #[test]
    fn test_generated_filter_reads_linguist_generated_attributes() {
        let dir = tempfile::tempdir().unwrap();
        let repo = gix::init(dir.path()).unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join(".gitattributes"),
            "\
*.rs linguist-generated
src/manual.rs -linguist-generated
src/false.rs linguist-generated=false
src/unspecified.rs !linguist-generated
src/value.txt linguist-generated=true
",
        )
        .unwrap();

        let mut filter = GeneratedFilter::new(&repo).unwrap();

        assert!(filter.is_generated(Path::new("src/generated.rs")).unwrap());
        assert!(!filter.is_generated(Path::new("src/manual.rs")).unwrap());
        assert!(!filter.is_generated(Path::new("src/false.rs")).unwrap());
        assert!(
            !filter
                .is_generated(Path::new("src/unspecified.rs"))
                .unwrap()
        );
        assert!(filter.is_generated(Path::new("src/value.txt")).unwrap());
    }
}
