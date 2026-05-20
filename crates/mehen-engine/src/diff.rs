//! `mehen diff` orchestrator.
//!
//! Phase 5 implementation: walks `mehen-git`'s changed-file list,
//! analyzes each file at base and head, and assembles a `DiffReport`.
//! The Markdown documentation diff renderer in `mehen-report` consumes
//! this report. Per the rewrite plan §4.6, per-file analysis is the
//! parallelism unit; this initial implementation runs serially and
//! follow-up commits will switch to a thread-per-file pool.

use std::sync::Arc;

use camino::Utf8PathBuf;

use mehen_core::{LanguageAnalysis, ParseDiagnostic, SourceFile, Threshold, ThresholdEvaluation};
use mehen_git::{ChangeStatus, GitError};

use crate::detection::detect_language;
use crate::registry::AnalyzerRegistry;
use crate::top_offenders::read_metric;
use mehen_core::{
    AnalysisErrorRecord, DiffFile, DiffInput, DiffReport, DiffSide, ThresholdViolation,
};

/// Run `mehen diff` against the workspace and produce a report.
///
/// Errors flow through the report's `analysis_errors` array (per rewrite
/// plan review §3.5: `analysis_errors` separate from
/// `threshold_violations`); only IO/git-fatal failures bubble up as
/// `Err` so callers can short-circuit the rendering step.
pub fn analyze_diff(input: DiffInput) -> Result<DiffReport, DiffError> {
    let registry = Arc::new(AnalyzerRegistry::default_set());
    let repo = mehen_git::open_repo().map_err(DiffError::Git)?;
    let changed =
        mehen_git::changed_files(&repo, &input.from, &input.to).map_err(DiffError::Git)?;

    let mut report = DiffReport {
        schema_version: "1.0".to_string(),
        base: input.from.clone(),
        head: input.to.clone(),
        files: Vec::new(),
        markdown_files: Vec::new(),
        analysis_errors: Vec::new(),
        threshold_violations: Vec::new(),
    };

    for cf in changed {
        // mehen-git returns `PathBuf` paths; convert at the boundary.
        let Ok(utf8_path) = Utf8PathBuf::try_from(cf.path.clone()) else {
            continue;
        };

        // Filter by `--paths` prefix matching.
        if !path_is_selected(&utf8_path, &input.paths) {
            continue;
        }

        let Some(language) = detect_language(&utf8_path) else {
            // Skip files we don't recognize.
            continue;
        };

        let base_text = if cf.status == ChangeStatus::Added {
            None
        } else {
            mehen_git::read_blob(&repo, &input.from, &cf.path)
                .map_err(DiffError::Git)?
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        };
        let head_text = if cf.status == ChangeStatus::Deleted {
            None
        } else {
            mehen_git::read_blob(&repo, &input.to, &cf.path)
                .map_err(DiffError::Git)?
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        };

        let analyzer = registry.analyzer_for(language);
        let Some(analyzer) = analyzer else {
            // Language detected but no analyzer registered (feature off);
            // surface as a non-fatal analysis error.
            record_unavailable(&mut report, &utf8_path, language);
            continue;
        };

        let mut head_analysis: Option<LanguageAnalysis> = None;
        for (text, side) in [
            (base_text.as_deref(), DiffSide::Base),
            (head_text.as_deref(), DiffSide::Head),
        ] {
            let Some(text) = text else { continue };
            let source = SourceFile::new(utf8_path.clone(), language, text.to_string());
            match analyzer.analyze(&source, &input.config) {
                Ok(analysis) => {
                    collect_diagnostics(&mut report, &utf8_path, side, &analysis);
                    if matches!(side, DiffSide::Head) {
                        head_analysis = Some(analysis);
                    }
                }
                Err(err) => {
                    report.analysis_errors.push(AnalysisErrorRecord {
                        path: utf8_path.clone(),
                        side,
                        diagnostics: vec![ParseDiagnostic::error(
                            "analysis.error",
                            err.to_string(),
                        )],
                    });
                }
            }
        }

        // Threshold evaluation runs against the head analysis (the
        // post-change state) so policy gates like "head cyclomatic must
        // not exceed 30" mean what callers expect. Files with a
        // blocking diagnostic on the head side are skipped — the
        // analysis is incomplete and folding a partial number into a
        // policy decision would be a false positive.
        if let Some(analysis) = head_analysis.as_ref()
            && !has_blocking_diagnostic(&analysis.diagnostics)
        {
            evaluate_thresholds(&mut report, &utf8_path, &input.thresholds, analysis);
        }

        if matches!(language, mehen_core::Language::Markdown) {
            report.markdown_files.push(DiffFile { path: utf8_path });
        } else {
            report.files.push(DiffFile { path: utf8_path });
        }
    }

    Ok(report)
}

/// Apply each `Threshold` to the head analysis's metrics and append a
/// `ThresholdViolation` to the report for every rule that fails. Done
/// per-file so the violation entry carries the originating path.
fn evaluate_thresholds(
    report: &mut DiffReport,
    path: &Utf8PathBuf,
    thresholds: &[Threshold],
    analysis: &LanguageAnalysis,
) {
    for threshold in thresholds {
        let actual = read_metric(&threshold.selector, &analysis.root);
        let violated = threshold.violated_by(actual);
        if violated {
            report.threshold_violations.push(ThresholdViolation {
                path: path.to_string(),
                evaluation: ThresholdEvaluation {
                    selector: threshold.selector.clone(),
                    actual,
                    limit: threshold.value,
                    polarity: threshold.polarity,
                    violated: true,
                },
            });
        }
    }
}

fn path_is_selected(path: &Utf8PathBuf, paths: &[Utf8PathBuf]) -> bool {
    if paths.is_empty() {
        return true;
    }
    paths.iter().any(|prefix| path.starts_with(prefix))
}

fn collect_diagnostics(
    report: &mut DiffReport,
    path: &Utf8PathBuf,
    side: DiffSide,
    analysis: &LanguageAnalysis,
) {
    // Surface every non-empty diagnostic batch — including
    // warning-only batches. Per plan §9.3 a `Warning` is
    // *informational* (CLI keeps exit 0 unless thresholds fail), but
    // it still has to be visible to callers; otherwise a Ruff-style
    // recoverable parse issue or a markdown cross-reference warning
    // is silently swallowed before it reaches the JSON output.
    // Severity-based exit-code routing happens at the CLI layer
    // against this same `analysis_errors` list, which carries the
    // severity on every entry via `ParseDiagnostic::severity`.
    if analysis.diagnostics.is_empty() {
        return;
    }
    report.analysis_errors.push(AnalysisErrorRecord {
        path: path.clone(),
        side,
        diagnostics: analysis.diagnostics.clone(),
    });
}

/// Classify a diagnostic batch for diff-side severity gating.
///
/// Per the diagnostic contract (rewrite plan §9.3), `Warning` is
/// informational, while `Error` or `Fatal` signals that the analysis is
/// incomplete — diff orchestrators must surface those (CLI exit 1, JSON
/// `analysis_errors`). Returns `true` iff any diagnostic in `diagnostics`
/// reaches the blocking threshold. Lives in the post-1.0 `diff` module
/// so it survives the legacy-engine teardown; the legacy diff path
/// re-uses it via `pub(crate)`.
pub(crate) fn has_blocking_diagnostic(diagnostics: &[ParseDiagnostic]) -> bool {
    diagnostics.iter().any(|d| {
        matches!(
            d.severity,
            mehen_core::DiagnosticSeverity::Error | mehen_core::DiagnosticSeverity::Fatal
        )
    })
}

fn record_unavailable(report: &mut DiffReport, path: &Utf8PathBuf, language: mehen_core::Language) {
    report.analysis_errors.push(AnalysisErrorRecord {
        path: path.clone(),
        side: DiffSide::Head,
        diagnostics: vec![ParseDiagnostic::warning(
            "engine.analyzer_unavailable",
            format!(
                "no analyzer registered for `{}` in this build",
                language.canonical()
            ),
        )],
    });
}

#[derive(Debug)]
pub enum DiffError {
    Git(GitError),
}

impl core::fmt::Display for DiffError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Git(e) => write!(f, "git: {e}"),
        }
    }
}

impl core::error::Error for DiffError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_diagnostics_are_not_blocking() {
        assert!(!has_blocking_diagnostic(&[]));
    }

    #[test]
    fn warning_only_is_not_blocking() {
        let diags = vec![ParseDiagnostic::warning("python.style", "long line")];
        assert!(!has_blocking_diagnostic(&diags));
    }

    #[test]
    fn error_severity_is_blocking() {
        let diags = vec![ParseDiagnostic::error(
            "ruby.syntax_error",
            "unterminated string",
        )];
        assert!(has_blocking_diagnostic(&diags));
    }

    #[test]
    fn fatal_severity_is_blocking() {
        let diags = vec![ParseDiagnostic::fatal(
            "rust.parse_error",
            "tree-sitter-rust failed",
        )];
        assert!(has_blocking_diagnostic(&diags));
    }

    #[test]
    fn warning_mixed_with_error_is_blocking() {
        let diags = vec![
            ParseDiagnostic::warning("python.style", "long line"),
            ParseDiagnostic::error("python.syntax_error", "invalid syntax"),
        ];
        assert!(has_blocking_diagnostic(&diags));
    }

    use mehen_core::{
        AnalysisBackend, Language, MetricKey, MetricSpace, Polarity, SourceSpan, SpaceId, SpaceKind,
    };

    fn analysis_with_metric(key: &str, value: f64) -> LanguageAnalysis {
        let mut root = MetricSpace::new(SpaceId(0), SpaceKind::Unit, SourceSpan::empty());
        root.metrics.insert(MetricKey::new(key), value);
        LanguageAnalysis {
            language: Language::Rust,
            backend: AnalysisBackend::TreeSitter,
            diagnostics: Vec::new(),
            root,
            contributions: Vec::new(),
        }
    }

    fn empty_report() -> DiffReport {
        DiffReport {
            schema_version: "1.0".to_string(),
            base: "HEAD~1".to_string(),
            head: "HEAD".to_string(),
            files: Vec::new(),
            markdown_files: Vec::new(),
            analysis_errors: Vec::new(),
            threshold_violations: Vec::new(),
        }
    }

    fn analysis_with_diagnostics(diagnostics: Vec<ParseDiagnostic>) -> LanguageAnalysis {
        LanguageAnalysis {
            language: Language::Rust,
            backend: AnalysisBackend::TreeSitter,
            diagnostics,
            root: MetricSpace::new(SpaceId(0), SpaceKind::Unit, SourceSpan::empty()),
            contributions: Vec::new(),
        }
    }

    #[test]
    fn collect_diagnostics_records_warning_only_batches() {
        // Regression: prior gate dropped warning-only batches before
        // they reached `analysis_errors`, so a Ruff-style recoverable
        // parse warning or a markdown cross-reference warning would
        // never surface in `mehen diff --format json`. The
        // `analysis_errors` field carries `severity` per entry, so
        // CLI exit-code routing can still distinguish warning vs.
        // error vs. fatal — but emitting them is required so callers
        // can see them at all.
        let analysis =
            analysis_with_diagnostics(vec![ParseDiagnostic::warning("python.style", "long line")]);
        let mut report = empty_report();
        collect_diagnostics(
            &mut report,
            &Utf8PathBuf::from("src/main.py"),
            DiffSide::Head,
            &analysis,
        );
        assert_eq!(report.analysis_errors.len(), 1);
        let rec = &report.analysis_errors[0];
        assert_eq!(rec.path, Utf8PathBuf::from("src/main.py"));
        assert_eq!(rec.diagnostics.len(), 1);
        assert_eq!(rec.diagnostics[0].code, "python.style");
    }

    #[test]
    fn collect_diagnostics_skips_empty_batch() {
        let analysis = analysis_with_diagnostics(Vec::new());
        let mut report = empty_report();
        collect_diagnostics(
            &mut report,
            &Utf8PathBuf::from("src/main.py"),
            DiffSide::Head,
            &analysis,
        );
        assert!(report.analysis_errors.is_empty());
    }

    #[test]
    fn collect_diagnostics_records_blocking_batch() {
        let analysis = analysis_with_diagnostics(vec![
            ParseDiagnostic::warning("python.style", "long line"),
            ParseDiagnostic::error("python.syntax_error", "unexpected token"),
        ]);
        let mut report = empty_report();
        collect_diagnostics(
            &mut report,
            &Utf8PathBuf::from("src/main.py"),
            DiffSide::Base,
            &analysis,
        );
        assert_eq!(report.analysis_errors.len(), 1);
        // Both diagnostics are preserved, so CLI exit-code routing
        // still sees the error severity.
        assert_eq!(report.analysis_errors[0].diagnostics.len(), 2);
    }

    #[test]
    fn higher_is_worse_threshold_above_limit_violates() {
        let analysis = analysis_with_metric("cognitive.sum", 42.0);
        let thresholds = vec![Threshold::new(
            "cognitive.sum".parse().unwrap(),
            30.0,
            Polarity::HigherIsWorse,
        )];
        let mut report = empty_report();
        evaluate_thresholds(
            &mut report,
            &Utf8PathBuf::from("src/main.rs"),
            &thresholds,
            &analysis,
        );
        assert_eq!(report.threshold_violations.len(), 1);
        let v = &report.threshold_violations[0];
        assert_eq!(v.path, "src/main.rs");
        assert_eq!(v.evaluation.actual, 42.0);
        assert_eq!(v.evaluation.limit, 30.0);
        assert!(v.evaluation.violated);
    }

    #[test]
    fn higher_is_worse_threshold_at_or_below_limit_does_not_violate() {
        let analysis = analysis_with_metric("cognitive.sum", 30.0);
        let thresholds = vec![Threshold::new(
            "cognitive.sum".parse().unwrap(),
            30.0,
            Polarity::HigherIsWorse,
        )];
        let mut report = empty_report();
        evaluate_thresholds(
            &mut report,
            &Utf8PathBuf::from("src/main.rs"),
            &thresholds,
            &analysis,
        );
        assert!(report.threshold_violations.is_empty());
    }

    #[test]
    fn higher_is_better_threshold_below_limit_violates() {
        let analysis = analysis_with_metric("mi.visual_studio", 49.0);
        let thresholds = vec![Threshold::new(
            "mi.visual_studio".parse().unwrap(),
            50.0,
            Polarity::HigherIsBetter,
        )];
        let mut report = empty_report();
        evaluate_thresholds(
            &mut report,
            &Utf8PathBuf::from("src/main.rs"),
            &thresholds,
            &analysis,
        );
        assert_eq!(report.threshold_violations.len(), 1);
        assert!(report.threshold_violations[0].evaluation.violated);
    }

    #[test]
    fn multiple_thresholds_each_evaluated_independently() {
        let mut analysis = analysis_with_metric("cyclomatic.sum", 50.0);
        analysis
            .root
            .metrics
            .insert(MetricKey::new("cognitive.sum"), 5.0);
        let thresholds = vec![
            Threshold::new(
                "cyclomatic.sum".parse().unwrap(),
                10.0,
                Polarity::HigherIsWorse,
            ),
            Threshold::new(
                "cognitive.sum".parse().unwrap(),
                30.0,
                Polarity::HigherIsWorse,
            ),
        ];
        let mut report = empty_report();
        evaluate_thresholds(
            &mut report,
            &Utf8PathBuf::from("src/main.rs"),
            &thresholds,
            &analysis,
        );
        // Only cyclomatic.sum exceeds its limit; cognitive.sum is fine.
        assert_eq!(report.threshold_violations.len(), 1);
        assert_eq!(
            report.threshold_violations[0]
                .evaluation
                .selector
                .key
                .as_str(),
            "cyclomatic"
        );
    }

    #[test]
    fn empty_thresholds_produce_no_violations() {
        let analysis = analysis_with_metric("cognitive.sum", 999.0);
        let mut report = empty_report();
        evaluate_thresholds(
            &mut report,
            &Utf8PathBuf::from("src/main.rs"),
            &[],
            &analysis,
        );
        assert!(report.threshold_violations.is_empty());
    }
}
