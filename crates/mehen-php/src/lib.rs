//! `mehen-php` — PHP language analyzer.
//!
//! Phase 1 scope: skeleton with tree-sitter-php wired through
//! `LanguageAnalyzer`. Phase 8 replaces the tree-sitter backend with Mago
//! syntax — the MSRV bump prerequisite is documented in the rewrite plan
//! review §2.1.

#![forbid(unsafe_code)]

use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, MetricSpace,
    Result, SourceFile, SourceSpan, SpaceId, SpaceKind, byte_offset_clamped,
};

pub struct PhpAnalyzer;

impl PhpAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PhpAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageAnalyzer for PhpAnalyzer {
    fn language(&self) -> Language {
        Language::Php
    }

    fn backend(&self) -> AnalysisBackend {
        AnalysisBackend::TreeSitter
    }

    fn analyze(&self, source: &SourceFile, _config: &AnalysisConfig) -> Result<LanguageAnalysis> {
        let span = SourceSpan {
            start_byte: 0,
            end_byte: byte_offset_clamped(source.text.len()),
            start_line: 1,
            end_line: source.line_index.line_count(),
        };
        Ok(LanguageAnalysis {
            language: Language::Php,
            backend: AnalysisBackend::TreeSitter,
            diagnostics: Vec::new(),
            root: MetricSpace::new(SpaceId(0), SpaceKind::Unit, span),
            contributions: Vec::new(),
        })
    }
}
