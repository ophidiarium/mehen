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
//! Phase 1 wires the registry and the dispatcher; the diff/top-offenders
//! orchestrators are skeleton functions that Phase 5 fills in. This keeps
//! the workspace compiling without removing the existing pre-1.0 CLI
//! functionality (which still lives in the root `mehen` crate).

#![forbid(unsafe_code)]

mod detection;
mod dispatcher;
mod registry;
mod report;

pub use detection::detect_language;
pub use dispatcher::EngineDispatcher;
pub use registry::{AnalyzerRegistry, RegistryError};
pub use report::{
    AnalysisErrorRecord, AnalyzeMetricsInput, DiffFile, DiffInput, DiffReport, DiffSide,
    MetricsReport, TopOffenderEntry, TopOffendersInput, TopOffendersReport,
};

use mehen_core::{AnalysisError, LanguageAnalysis, Result};

/// Run a single-file analysis using the default registry.
///
/// Phase 1 implementation; Phase 5 expands this to the full `mehen metrics`
/// orchestration (output formatting, diagnostics → exit codes, …).
pub fn analyze_metrics(input: AnalyzeMetricsInput) -> Result<MetricsReport> {
    let registry = AnalyzerRegistry::default_set();
    let analysis = analyze_one(&registry, input)?;
    Ok(MetricsReport::from(analysis))
}

fn analyze_one(
    registry: &AnalyzerRegistry,
    input: AnalyzeMetricsInput,
) -> Result<LanguageAnalysis> {
    let analyzer = registry
        .analyzer_for(input.source.language)
        .ok_or(AnalysisError::AnalyzerUnavailable(input.source.language))?;
    analyzer.analyze(&input.source, &input.config)
}
