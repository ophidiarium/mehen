use std::io::Write;
use std::path::{Component, Path, PathBuf};

use crate::ci;
#[cfg(feature = "markdown")]
use crate::diff_markdown::{DocDiffFile, DocRenderCtx, render_doc_section};
use crate::git::{self, ChangeStatus, GitError};
#[cfg(feature = "markdown")]
use crate::langs::LANG;
use crate::langs::{get_from_ext, get_function_spaces};
#[cfg(feature = "markdown")]
use crate::markdown;
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
    /// Exit non-zero when the named thresholds are crossed
    /// (comma-separated: `dmi-drop`, `new-broken-link`, `filler-high`, `all`).
    #[clap(
        long,
        value_delimiter = ',',
        value_parser = parse_fail_on_flag,
    )]
    fail_on: Vec<FailOn>,
}

/// Identifies one of the documented doc-metric CI gates. Any other value is
/// rejected by clap at parse time rather than being silently ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FailOn {
    DmiDrop,
    NewBrokenLink,
    FillerHigh,
    All,
}

impl FailOn {
    #[cfg(feature = "markdown")]
    fn as_str(self) -> &'static str {
        match self {
            Self::DmiDrop => "dmi-drop",
            Self::NewBrokenLink => "new-broken-link",
            Self::FillerHigh => "filler-high",
            Self::All => "all",
        }
    }
}

/// Custom clap value parser so misspelled flags (e.g. `new-borken-link`)
/// produce an `InvalidValue` error at CLI-parse time instead of being
/// silently dropped downstream.
fn parse_fail_on_flag(raw: &str) -> Result<FailOn, clap::Error> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "dmi-drop" => Ok(FailOn::DmiDrop),
        "new-broken-link" => Ok(FailOn::NewBrokenLink),
        "filler-high" => Ok(FailOn::FillerHigh),
        "all" => Ok(FailOn::All),
        other => Err(clap::Error::raw(
            clap::error::ErrorKind::InvalidValue,
            format!(
                "unknown --fail-on value `{other}`; expected one of: dmi-drop, new-broken-link, filler-high, all\n"
            ),
        )),
    }
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
    #[cfg(feature = "markdown")]
    let mut markdown_files: Vec<git::ChangedFile> = Vec::new();
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

        let ext_lang = p
            .extension()
            .and_then(|e| e.to_str())
            .and_then(get_from_ext);
        if ext_lang.is_none() {
            continue;
        }

        #[cfg(feature = "markdown")]
        if matches!(ext_lang, Some(LANG::Markdown)) {
            markdown_files.push(cf.clone());
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

    // Markdown doc section — parallel pipeline for `.md`-like files.
    #[cfg(feature = "markdown")]
    let doc_files: Vec<DocDiffFile> = {
        let mut out: Vec<DocDiffFile> = Vec::new();
        for cf in &markdown_files {
            let is_deleted = cf.status == ChangeStatus::Deleted;
            let is_candidate_new = cf.status == ChangeStatus::Added;
            let base_metrics = if is_candidate_new {
                None
            } else {
                match git::read_blob(&repo, &from_ref, &cf.path) {
                    Ok(Some(bytes)) => Some(markdown::analyze_markdown(
                        &String::from_utf8_lossy(&bytes),
                        &cf.path,
                    )),
                    Ok(None) => None,
                    Err(e) => {
                        log::warn!("Skipping baseline for {}: {e}", cf.path.display());
                        None
                    }
                }
            };
            let head_metrics = if is_deleted {
                None
            } else {
                match git::read_blob(&repo, &to_ref, &cf.path) {
                    Ok(Some(bytes)) => Some(markdown::analyze_markdown(
                        &String::from_utf8_lossy(&bytes),
                        &cf.path,
                    )),
                    Ok(None) => None,
                    Err(e) => {
                        log::warn!("Skipping current for {}: {e}", cf.path.display());
                        None
                    }
                }
            };
            let is_new = is_candidate_new && base_metrics.is_none();
            out.push(DocDiffFile {
                path: cf.path.clone(),
                head: head_metrics,
                base: base_metrics,
                is_new,
                is_deleted,
            });
        }
        out
    };

    // 7. Output
    let format = opts.output_format.unwrap_or(DiffFormat::Markdown);
    match format {
        DiffFormat::Markdown => {
            print_markdown(&diffs, &selectors, &from_label, &from_ref, &to_ref);
            #[cfg(feature = "markdown")]
            if !doc_files.is_empty() {
                let mut ctx = DocRenderCtx::new(&from_label);
                let repo_url = ci_ctx
                    .as_ref()
                    .and_then(|c| c.repository.as_ref())
                    .map(|r| format!("https://github.com/{r}"));
                ctx.repo_url = repo_url.as_deref();
                ctx.head_sha = Some(&to_ref);
                if let Some(doc_md) = render_doc_section(&doc_files, &ctx) {
                    let mut stdout = std::io::stdout().lock();
                    writeln!(stdout).ok();
                    write!(stdout, "{doc_md}").ok();
                }
            }
        }
        DiffFormat::Json => {
            #[cfg(feature = "markdown")]
            let doc_ref: Option<&[DocDiffFile]> = if doc_files.is_empty() {
                None
            } else {
                Some(&doc_files)
            };
            #[cfg(not(feature = "markdown"))]
            let doc_ref: Option<&[()]> = None;
            if let Err(e) = print_json(&diffs, doc_ref) {
                // Surface the error loudly — exit code 2 mirrors the
                // --fail-on gate and is distinct from the generic exit 1
                // that covers setup/IO errors in run_diff_inner.
                log::error!("diff: failed to emit JSON output: {e}");
                std::process::exit(2);
            }
        }
    }

    // --fail-on check.
    #[cfg(feature = "markdown")]
    {
        let failures = evaluate_fail_on(&opts.fail_on, &doc_files);
        if !failures.is_empty() {
            log::error!("--fail-on threshold crossed: {}", failures.join(", "));
            std::process::exit(2);
        }
    }
    #[cfg(not(feature = "markdown"))]
    {
        if !opts.fail_on.is_empty() {
            log::warn!(
                "--fail-on was set but the `markdown` feature is disabled; no doc-metric thresholds are evaluated"
            );
        }
    }

    Ok(())
}

#[cfg(feature = "markdown")]
fn doc_json_payload(files: &[DocDiffFile]) -> Vec<serde_json::Value> {
    files
        .iter()
        .map(|f| {
            serde_json::json!({
                "path": f.path.to_string_lossy(),
                "is_new": f.is_new,
                "is_deleted": f.is_deleted,
                "base": f.base,
                "head": f.head,
            })
        })
        .collect()
}

#[cfg(feature = "markdown")]
fn evaluate_fail_on(flags: &[FailOn], docs: &[DocDiffFile]) -> Vec<String> {
    let mut enabled: std::collections::BTreeSet<FailOn> = std::collections::BTreeSet::new();
    for f in flags {
        match f {
            FailOn::All => {
                enabled.insert(FailOn::DmiDrop);
                enabled.insert(FailOn::NewBrokenLink);
                enabled.insert(FailOn::FillerHigh);
            }
            other => {
                enabled.insert(*other);
            }
        }
    }
    if enabled.is_empty() {
        return Vec::new();
    }
    // If the caller asked to gate on doc metrics but no markdown files are
    // in the diff, log a warning so users notice the flag silently matched
    // nothing. The gate itself still returns success (no docs → no metric
    // breach possible) so existing CI doesn't break.
    if docs.iter().all(|f| f.head.is_none()) {
        let flags: Vec<&str> = enabled.iter().copied().map(FailOn::as_str).collect();
        log::warn!(
            "--fail-on {flags:?} has no Markdown files in the diff; no doc-metric thresholds were evaluated"
        );
    }
    let mut failures: Vec<String> = Vec::new();
    for f in docs {
        let Some(head) = &f.head else { continue };
        let base = f.base.as_ref();
        if enabled.contains(&FailOn::DmiDrop)
            && let Some(b) = base
        {
            let hd = head.maintainability.documentation_maintainability_index;
            let bd = b.maintainability.documentation_maintainability_index;
            if bd - hd >= 3.0 {
                failures.push(format!("dmi-drop:{}", f.path.display()));
            }
        }
        if enabled.contains(&FailOn::NewBrokenLink) {
            // Identity-based diff keyed on (class, destination) — line
            // numbers MAY change without a new broken link (e.g. a doc
            // prepends content, shifting every link down one line). The CI
            // gate fires only when a key appears more often in head than in
            // base. Line numbers still flow through to the callout layer for
            // the PR comment; they just don't drive the fail-on decision.
            // See §39.4.
            let mut head_counts: std::collections::BTreeMap<
                (crate::markdown::types::LinkClass, &str),
                usize,
            > = std::collections::BTreeMap::new();
            for l in &head.link_records {
                if matches!(l.resolved, Some(false)) {
                    *head_counts
                        .entry((l.class, l.destination.as_str()))
                        .or_insert(0) += 1;
                }
            }
            let mut base_counts: std::collections::BTreeMap<
                (crate::markdown::types::LinkClass, &str),
                usize,
            > = std::collections::BTreeMap::new();
            if let Some(b) = base {
                for l in &b.link_records {
                    if matches!(l.resolved, Some(false)) {
                        *base_counts
                            .entry((l.class, l.destination.as_str()))
                            .or_insert(0) += 1;
                    }
                }
            }
            let has_new_broken = head_counts.iter().any(|(key, head_n)| {
                let base_n = base_counts.get(key).copied().unwrap_or(0);
                *head_n > base_n
            });
            if has_new_broken {
                failures.push(format!("new-broken-link:{}", f.path.display()));
            }
        }
        if enabled.contains(&FailOn::FillerHigh) && head.ai_era.filler_lazy_structure_risk >= 0.60 {
            failures.push(format!("filler-high:{}", f.path.display()));
        }
    }
    failures
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

    // Source-code anchor (§39.1: sibling of the docs anchor).
    out.push_str("<!-- mehen-metrics -->\n");
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

/// Emit a single JSON document with a `source_code` key and an optional
/// `markdown` key. Downstream consumers (`jq`, `serde_json`) see one top-level
/// object, not two concatenated arrays.
///
/// Serialization errors bubble up as `Err` so `run_diff_inner` exits
/// non-zero instead of silently writing an empty `""` to stdout.
#[cfg(feature = "markdown")]
fn print_json(
    diffs: &[FileDiff],
    docs: Option<&[DocDiffFile]>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut payload = serde_json::Map::new();
    payload.insert("source_code".to_string(), serde_json::to_value(diffs)?);
    if let Some(docs) = docs {
        payload.insert(
            "markdown".to_string(),
            serde_json::Value::Array(doc_json_payload(docs)),
        );
    }
    let json = serde_json::to_string_pretty(&serde_json::Value::Object(payload))?;
    writeln!(std::io::stdout().lock(), "{json}")?;
    Ok(())
}

#[cfg(not(feature = "markdown"))]
fn print_json(diffs: &[FileDiff], _docs: Option<&[()]>) -> Result<(), Box<dyn std::error::Error>> {
    let mut payload = serde_json::Map::new();
    payload.insert("source_code".to_string(), serde_json::to_value(diffs)?);
    let json = serde_json::to_string_pretty(&serde_json::Value::Object(payload))?;
    writeln!(std::io::stdout().lock(), "{json}")?;
    Ok(())
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
            fail_on: vec![],
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
            fail_on: vec![],
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
            fail_on: vec![],
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
            fail_on: vec![],
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

    // ── `--fail-on new-broken-link` gating tests ───────────────────────
    //
    // Ensure the CI gate keys on `(class, destination)` identity — a link
    // that merely shifts to a different line number MUST NOT trip the gate,
    // but a duplicate broken destination MUST.

    #[cfg(feature = "markdown")]
    fn broken_link_for_fail_on(
        line: u64,
        class: crate::markdown::types::LinkClass,
        destination: &str,
    ) -> crate::markdown::types::LinkRecord {
        crate::markdown::types::LinkRecord {
            line,
            class,
            destination: destination.to_string(),
            text: String::new(),
            is_image: false,
            is_bare_url: false,
            resolved: Some(false),
        }
    }

    #[cfg(feature = "markdown")]
    fn minimal_md_metrics(path: &str) -> crate::markdown::types::MarkdownMetrics {
        crate::markdown::types::MarkdownMetrics {
            path: path.to_string(),
            loc: Default::default(),
            loc_ratios: Default::default(),
            size: Default::default(),
            ecu_inputs: Default::default(),
            sections: vec![],
            complexity: Default::default(),
            links: Default::default(),
            link_records: vec![],
            visuals: Default::default(),
            tables: Default::default(),
            maintainability: Default::default(),
            grounding: Default::default(),
            ai_era: Default::default(),
            review: Default::default(),
            artifacts: vec![],
            prose: Default::default(),
        }
    }

    #[cfg(feature = "markdown")]
    #[test]
    fn fail_on_new_broken_link_ignores_line_only_shift() {
        let mut head = minimal_md_metrics("docs/a.md");
        head.link_records = vec![broken_link_for_fail_on(
            42,
            crate::markdown::types::LinkClass::Relative,
            "./guide.md",
        )];
        let mut base = minimal_md_metrics("docs/a.md");
        base.link_records = vec![broken_link_for_fail_on(
            10,
            crate::markdown::types::LinkClass::Relative,
            "./guide.md",
        )];

        let doc = DocDiffFile {
            path: PathBuf::from("docs/a.md"),
            head: Some(head),
            base: Some(base),
            is_new: false,
            is_deleted: false,
        };

        let flags = vec![FailOn::NewBrokenLink];
        let failures = evaluate_fail_on(&flags, std::slice::from_ref(&doc));
        assert!(
            failures.is_empty(),
            "line-only shift must not trip new-broken-link; got: {failures:?}",
        );
    }

    #[cfg(feature = "markdown")]
    #[test]
    fn fail_on_new_broken_link_trips_on_new_occurrence() {
        // Head has 2 broken refs to the same destination; base has 1. The
        // second occurrence is net-new so the gate must fire.
        let mut head = minimal_md_metrics("docs/a.md");
        head.link_records = vec![
            broken_link_for_fail_on(10, crate::markdown::types::LinkClass::Relative, "./g.md"),
            broken_link_for_fail_on(20, crate::markdown::types::LinkClass::Relative, "./g.md"),
        ];
        let mut base = minimal_md_metrics("docs/a.md");
        base.link_records = vec![broken_link_for_fail_on(
            10,
            crate::markdown::types::LinkClass::Relative,
            "./g.md",
        )];

        let doc = DocDiffFile {
            path: PathBuf::from("docs/a.md"),
            head: Some(head),
            base: Some(base),
            is_new: false,
            is_deleted: false,
        };

        let flags = vec![FailOn::NewBrokenLink];
        let failures = evaluate_fail_on(&flags, std::slice::from_ref(&doc));
        assert_eq!(failures.len(), 1);
        assert!(failures[0].starts_with("new-broken-link:"));
    }

    #[cfg(feature = "markdown")]
    #[test]
    fn fail_on_new_broken_link_trips_on_brand_new_destination() {
        let mut head = minimal_md_metrics("docs/a.md");
        head.link_records = vec![broken_link_for_fail_on(
            5,
            crate::markdown::types::LinkClass::Relative,
            "./added.md",
        )];
        let base = minimal_md_metrics("docs/a.md");

        let doc = DocDiffFile {
            path: PathBuf::from("docs/a.md"),
            head: Some(head),
            base: Some(base),
            is_new: false,
            is_deleted: false,
        };

        let flags = vec![FailOn::NewBrokenLink];
        let failures = evaluate_fail_on(&flags, std::slice::from_ref(&doc));
        assert_eq!(failures.len(), 1);
    }

    // ── print_json error-propagation ────────────────────────────────────

    #[test]
    fn print_json_happy_path_is_ok() {
        let diffs: Vec<FileDiff> = vec![FileDiff {
            path: PathBuf::from("a.rs"),
            metrics: vec![],
            is_new: false,
            is_deleted: false,
        }];
        #[cfg(feature = "markdown")]
        let res = print_json(&diffs, None);
        #[cfg(not(feature = "markdown"))]
        let res = print_json(&diffs, None);
        assert!(res.is_ok(), "valid input must serialize cleanly");
    }

    #[test]
    fn print_json_returns_result_type() {
        // §39 regression guard: print_json must return `Result<_, _>` so
        // callers can exit non-zero on serialization failure. Before, the
        // emitter used `unwrap_or_default` and silently wrote an empty
        // JSON document to stdout when serde_json failed.
        //
        // We can only exercise the happy path deterministically here —
        // serde_json's non-finite-float policy varies by version — but a
        // type-level assertion that the function signature returns
        // `Result` is enough to lock in the fix: the caller at the
        // DiffFormat::Json branch uses `if let Err(e) = print_json(..)`
        // so any future regression to an infallible signature would
        // break compilation.
        let diffs: Vec<FileDiff> = vec![];
        #[cfg(feature = "markdown")]
        let res: Result<(), Box<dyn std::error::Error>> = print_json(&diffs, None);
        #[cfg(not(feature = "markdown"))]
        let res: Result<(), Box<dyn std::error::Error>> = print_json(&diffs, None);
        assert!(res.is_ok());
    }

    // ── `--fail-on` CLI-parse validation ────────────────────────────────

    #[test]
    fn fail_on_parser_accepts_every_documented_value() {
        let cli = TestDiffCli::try_parse_from([
            "mehen",
            "--fail-on",
            "dmi-drop,new-broken-link,filler-high,all",
        ])
        .expect("every documented value must parse");
        assert_eq!(
            cli.opts.fail_on,
            vec![
                FailOn::DmiDrop,
                FailOn::NewBrokenLink,
                FailOn::FillerHigh,
                FailOn::All,
            ]
        );
    }

    #[test]
    fn fail_on_parser_trims_and_lowercases() {
        let cli = TestDiffCli::try_parse_from(["mehen", "--fail-on", "  Dmi-Drop , ALL "])
            .expect("case and whitespace must be normalized");
        assert_eq!(cli.opts.fail_on, vec![FailOn::DmiDrop, FailOn::All]);
    }

    #[test]
    fn fail_on_parser_rejects_unknown_value() {
        // Regression: before, typos like `new-borken-link` were silently
        // dropped into an empty filter set. Now clap must error at parse
        // time so the mistake is loud.
        let err = TestDiffCli::try_parse_from(["mehen", "--fail-on", "new-borken-link"])
            .expect_err("unknown value must be rejected");
        // Clap wraps custom value-parser errors under ValueValidation; the
        // raw ErrorKind::InvalidValue we produce survives either way. Accept
        // both so the test is robust across clap internals.
        assert!(
            matches!(
                err.kind(),
                clap::error::ErrorKind::InvalidValue | clap::error::ErrorKind::ValueValidation,
            ),
            "expected InvalidValue or ValueValidation, got: {:?}",
            err.kind(),
        );
        let rendered = err.to_string();
        assert!(
            rendered.contains("new-borken-link"),
            "error must mention the offending value, got: {rendered}"
        );
    }

    #[test]
    fn fail_on_parser_rejects_partial_match_in_list() {
        // Mix of a valid and a misspelled flag must still fail the whole
        // parse so no value is silently dropped.
        let err = TestDiffCli::try_parse_from(["mehen", "--fail-on", "dmi-drop,filler-hihg"])
            .expect_err("list with an invalid entry must be rejected");
        assert!(matches!(
            err.kind(),
            clap::error::ErrorKind::InvalidValue | clap::error::ErrorKind::ValueValidation,
        ));
        assert!(err.to_string().contains("filler-hihg"));
    }
}
