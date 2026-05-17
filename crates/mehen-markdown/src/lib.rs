//! `mehen-markdown` — Markdown documentation metrics analyzer.
//!
//! Markdown is special: it is not source-code function/class analysis. The
//! Phase 1 skeleton:
//! - registers `MarkdownAnalyzer` so the engine can dispatch by `Language`,
//! - exposes `analyze_markdown_with_dispatcher` for the embedded-code path
//!   (rewrite plan §4.7 / review §3.3 / §4.1) — the seam Markdown uses to
//!   recursively analyze a fenced code block in another language without
//!   pulling every language crate as a compile-time dependency,
//! - leaves the existing pre-1.0 Markdown metric implementations
//!   (in the original `src/markdown/`) untouched until Phase 4 ports them.

#![forbid(unsafe_code)]

use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer,
    LanguageDispatcher, MetricSpace, Result, SourceFile, SourceSpan, SpaceId, SpaceKind,
    byte_offset_clamped,
};

pub struct MarkdownAnalyzer;

impl MarkdownAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MarkdownAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageAnalyzer for MarkdownAnalyzer {
    fn language(&self) -> Language {
        Language::Markdown
    }

    fn backend(&self) -> AnalysisBackend {
        AnalysisBackend::MarkdownLegacy
    }

    fn analyze(&self, source: &SourceFile, _config: &AnalysisConfig) -> Result<LanguageAnalysis> {
        // Phase 1 placeholder — Phase 4 ports the existing Markdown metric
        // implementation here. The dispatcher-aware variant below is the
        // embedded-code path that requires a `LanguageDispatcher`.
        let span = SourceSpan {
            start_byte: 0,
            end_byte: byte_offset_clamped(source.text.len()),
            start_line: 1,
            end_line: source.line_index.line_count(),
        };
        Ok(LanguageAnalysis {
            language: Language::Markdown,
            backend: AnalysisBackend::MarkdownLegacy,
            diagnostics: Vec::new(),
            root: MetricSpace::new(SpaceId(0), SpaceKind::Unit, span),
            contributions: Vec::new(),
        })
    }
}

/// Markdown analysis with embedded-code support.
///
/// This is the entry point Phase 4 will call from `mehen-engine` when the
/// engine needs to roll up embedded-code complexity into Markdown metrics.
/// Tests can pass a mock `&dyn LanguageDispatcher` that returns canned
/// `LanguageAnalysis` values.
pub fn analyze_markdown_with_dispatcher(
    source: &SourceFile,
    config: &AnalysisConfig,
    _dispatcher: &dyn LanguageDispatcher,
) -> Result<LanguageAnalysis> {
    // Phase 1 placeholder — the dispatcher is unused until Phase 4 ports
    // the embedded-code roll-up logic from `src/markdown/embedded_code.rs`.
    MarkdownAnalyzer::new().analyze(source, config)
}
