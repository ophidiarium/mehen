//! `mehen-engine` — pipeline orchestration.
//!
//! This crate owns:
//! - language analyzer registry,
//! - language detection by extension and content,
//! - the public engine APIs (`analyze_metrics`, `analyze_diff`,
//!   `rank_top_offenders`),
//! - per-file concurrency (per the rewrite plan §4.6: per-file analysis is
//!   the parallelism unit; analyzers are constructed per worker; parser
//!   arenas live for one analyze call),
//! - the only `LanguageDispatcher` implementation in 1.0, exposed to
//!   `mehen-markdown` for the embedded-code path.
//!
//! Phase 1 wired the registry and the dispatcher; Phase 5 added the
//! `analyze_diff` and `rank_top_offenders` orchestrators. The per-file
//! parallelism unit and recursion/depth limits land in follow-up
//! commits; this implementation keeps each operation single-threaded
//! and predictable.

#![forbid(unsafe_code)]

mod detection;
mod diff;
mod dispatcher;
mod registry;
mod report;
mod top_offenders;

pub use detection::detect_language;
pub use diff::analyze_diff;
pub use dispatcher::EngineDispatcher;
pub use registry::{AnalyzerRegistry, RegistryError};
pub use report::{
    AnalysisErrorRecord, AnalyzeMetricsInput, DiffFile, DiffInput, DiffReport, DiffSide,
    MetricsReport, TopOffenderEntry, TopOffendersInput, TopOffendersReport,
};
pub use top_offenders::rank_top_offenders;

use mehen_core::{AnalysisError, Result};

/// Run a single-file analysis using the default registry.
///
/// The returned report has its `path` populated from the input, so callers
/// don't need to set it manually after the conversion from
/// `LanguageAnalysis` (`LanguageAnalysis` itself does not carry the path).
///
/// Phase 1 implementation; Phase 5 expands this to the full `mehen metrics`
/// orchestration (output formatting, diagnostics → exit codes, …).
pub fn analyze_metrics(input: AnalyzeMetricsInput) -> Result<MetricsReport> {
    let registry = AnalyzerRegistry::default_set();
    let path = input.source.path.clone();
    let analyzer = registry
        .analyzer_for(input.source.language)
        .ok_or(AnalysisError::AnalyzerUnavailable(input.source.language))?;
    let analysis = analyzer.analyze(&input.source, &input.config)?;
    let mut report = MetricsReport::from(analysis);
    report.path = path;
    Ok(report)
}
