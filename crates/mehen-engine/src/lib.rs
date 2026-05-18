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
///
/// PowerShell fences route through the new [`AnalyzerRegistry`]
/// (the `mehen-powershell` analyzer is at parity per plan §8.2);
/// the remaining languages still flow through
/// `legacy::langs::get_function_spaces` until their per-language
/// crates reach parity. Each fence-language switch lives in
/// `dispatch_per_language` so the migration tracks one match arm
/// at a time.
#[cfg(feature = "markdown")]
pub fn init_markdown() {
    use mehen_markdown::{EmbeddedFenceMetrics, FenceLanguage};

    fn dispatch(lang: FenceLanguage, body: String) -> Option<EmbeddedFenceMetrics> {
        match lang {
            // PowerShell (plan §8.2 Phase 3), TypeScript / TSX
            // (Phase 7 Oxc migration), and Python (Phase 6 Ruff
            // migration) flow through the new registry. The remaining
            // languages still use legacy until each per-language crate
            // reaches parity.
            FenceLanguage::Powershell
            | FenceLanguage::Typescript
            | FenceLanguage::Tsx
            | FenceLanguage::Python => dispatch_via_registry(lang, body),
            _ => dispatch_via_legacy(lang, body),
        }
    }

    fn dispatch_via_registry(lang: FenceLanguage, body: String) -> Option<EmbeddedFenceMetrics> {
        use mehen_core::{AnalysisConfig, MetricKey, SourceFile, keys};

        let language = language_for(lang);
        let registry = AnalyzerRegistry::default_set();
        let analyzer = registry.analyzer_for(language)?;
        let path = camino::Utf8PathBuf::try_from(synthetic_path(lang)).ok()?;
        let source = SourceFile::new(path, language, body);
        let analysis = analyzer.analyze(&source, &AnalysisConfig::default()).ok()?;
        let read = |key: &str| {
            analysis
                .root
                .metrics
                .get(&MetricKey::new(key))
                .map(|v| v.as_f64())
                .unwrap_or(0.0)
        };
        Some(EmbeddedFenceMetrics {
            volume: read(keys::HALSTEAD_VOLUME),
            cognitive_sum: read("cognitive.sum"),
            sloc: read(keys::LOC_SLOC),
        })
    }

    fn dispatch_via_legacy(lang: FenceLanguage, body: String) -> Option<EmbeddedFenceMetrics> {
        let bytes = body.into_bytes();
        let path = synthetic_path(lang);
        let legacy_lang = legacy_lang_for(lang)?;
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

    /// Map a fence language to its legacy `LANG` variant. Languages
    /// that have completed their Oxc / Ruff / Mago migration (currently
    /// PowerShell + TypeScript / TSX + Python) return `None` — they
    /// route through `dispatch_via_registry`, which lets each migrated
    /// per-language crate's analyzer drive the embedded fence metrics.
    fn legacy_lang_for(lang: FenceLanguage) -> Option<crate::legacy::langs::LANG> {
        use crate::legacy::langs::LANG;
        Some(match lang {
            FenceLanguage::Rust => LANG::Rust,
            FenceLanguage::Go => LANG::Go,
            FenceLanguage::Ruby => LANG::Ruby,
            FenceLanguage::Kotlin => LANG::Kotlin,
            FenceLanguage::C => LANG::C,
            FenceLanguage::Php => LANG::Php,
            // Migrated to per-language crate analyzers; no legacy fallback.
            FenceLanguage::Powershell
            | FenceLanguage::Typescript
            | FenceLanguage::Tsx
            | FenceLanguage::Python => return None,
        })
    }

    fn language_for(lang: FenceLanguage) -> mehen_core::Language {
        use mehen_core::Language;
        match lang {
            FenceLanguage::Rust => Language::Rust,
            FenceLanguage::Python => Language::Python,
            FenceLanguage::Typescript => Language::TypeScript,
            FenceLanguage::Tsx => Language::Tsx,
            FenceLanguage::Go => Language::Go,
            FenceLanguage::Ruby => Language::Ruby,
            FenceLanguage::Kotlin => Language::Kotlin,
            FenceLanguage::Powershell => Language::PowerShell,
            FenceLanguage::C => Language::C,
            FenceLanguage::Php => Language::Php,
        }
    }

    mehen_markdown::set_legacy_dispatch(dispatch);
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
