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

#![deny(unsafe_code)]

pub mod ci;
mod detection;
mod diff;
mod dispatcher;
mod registry;
mod top_offenders;

/// Pre-1.0 metric machinery relocated from `mehen/src/`. This is an
/// internal implementation detail of `mehen-engine` for the duration of
/// the v1 transition; the published `run_diff`, `run_top_offenders`,
/// `DiffOpts`, and `TopOffendersOpts` are re-exported at the crate root
/// so `mehen-cli` does not need to reach into a `legacy::` submodule.
/// Plan §8.2/§8.3 ultimately splits this content across the per-language
/// crates and `mehen-metrics`; until each language reaches parity through
/// its own analyzer, the legacy dispatch supplies the metric tree.
mod legacy;

pub use legacy::diff::{DiffOpts, run_diff};
pub use legacy::top_offenders::{TopOffendersOpts, run_top_offenders};

/// Register the embedded-code dispatch callback the moved
/// [`mehen_markdown::analyze_markdown`] uses to fold fenced source
/// snippets into Markdown metrics. Idempotent — backed by a
/// `OnceLock` inside `mehen-markdown`, so repeat calls are silent
/// no-ops.
#[cfg(feature = "markdown")]
pub fn init_markdown() {
    use mehen_markdown::{EmbeddedFenceMetrics, FenceLanguage};

    fn legacy_dispatch(lang: FenceLanguage, body: String) -> Option<EmbeddedFenceMetrics> {
        let bytes = body.into_bytes();
        let path = synthetic_path(lang);
        let legacy_lang = legacy_lang_for(lang);
        let space = crate::legacy::langs::get_function_spaces(
            &legacy_lang,
            bytes,
            std::path::Path::new(&path),
            None,
        )?;
        Some(EmbeddedFenceMetrics {
            volume: space.metrics.halstead.volume(),
            cognitive_sum: space.metrics.cognitive.cognitive_sum(),
            sloc: space.metrics.loc.sloc(),
        })
    }

    fn synthetic_path(lang: FenceLanguage) -> std::path::PathBuf {
        let name = match lang {
            FenceLanguage::Rust => "fence.rs",
            FenceLanguage::Python => "fence.py",
            FenceLanguage::Typescript => "fence.ts",
            FenceLanguage::Tsx => "fence.tsx",
            FenceLanguage::Go => "fence.go",
            FenceLanguage::Ruby => "fence.rb",
            FenceLanguage::Kotlin => "fence.kt",
            FenceLanguage::Powershell => "fence.ps1",
            FenceLanguage::C => "fence.c",
            FenceLanguage::Php => "fence.php",
        };
        std::path::PathBuf::from(name)
    }

    fn legacy_lang_for(lang: FenceLanguage) -> crate::legacy::langs::LANG {
        use crate::legacy::langs::LANG;
        match lang {
            FenceLanguage::Rust => LANG::Rust,
            FenceLanguage::Python => LANG::Python,
            FenceLanguage::Typescript => LANG::Typescript,
            FenceLanguage::Tsx => LANG::Tsx,
            FenceLanguage::Go => LANG::Go,
            FenceLanguage::Ruby => LANG::Ruby,
            FenceLanguage::Kotlin => LANG::Kotlin,
            FenceLanguage::Powershell => LANG::Powershell,
            FenceLanguage::C => LANG::C,
            FenceLanguage::Php => LANG::Php,
        }
    }

    mehen_markdown::set_legacy_dispatch(legacy_dispatch);
}

pub use detection::detect_language;
pub use diff::analyze_diff;
pub use dispatcher::EngineDispatcher;
pub use mehen_core::{
    AnalysisErrorRecord, AnalyzeMetricsInput, DiffFile, DiffInput, DiffReport, DiffSide,
    MetricsReport, TopOffenderEntry, TopOffendersInput, TopOffendersReport,
};
pub use registry::{AnalyzerRegistry, RegistryError};
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
