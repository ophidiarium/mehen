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

use mehen_core::{LanguageAnalysis, ParseDiagnostic, SourceFile};
use mehen_git::{ChangeStatus, GitError};

use crate::detection::detect_language;
use crate::registry::AnalyzerRegistry;
use mehen_core::{AnalysisErrorRecord, DiffFile, DiffInput, DiffReport, DiffSide};

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

        for (text, side) in [
            (base_text.as_deref(), DiffSide::Base),
            (head_text.as_deref(), DiffSide::Head),
        ] {
            let Some(text) = text else { continue };
            let source = SourceFile::new(utf8_path.clone(), language, text.to_string());
            match analyzer.analyze(&source, &input.config) {
                Ok(analysis) => collect_diagnostics(&mut report, &utf8_path, side, &analysis),
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

        if matches!(language, mehen_core::Language::Markdown) {
            report.markdown_files.push(DiffFile { path: utf8_path });
        } else {
            report.files.push(DiffFile { path: utf8_path });
        }
    }

    Ok(report)
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
    if analysis.diagnostics.is_empty() {
        return;
    }
    let any_fatal = analysis.diagnostics.iter().any(|d| {
        d.severity == mehen_core::DiagnosticSeverity::Fatal
            || d.severity == mehen_core::DiagnosticSeverity::Error
    });
    if any_fatal {
        report.analysis_errors.push(AnalysisErrorRecord {
            path: path.clone(),
            side,
            diagnostics: analysis.diagnostics.clone(),
        });
    }
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
