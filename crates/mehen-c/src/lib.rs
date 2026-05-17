//! `mehen-c` — C language analyzer.
//!
//! Phase 1 scope: skeleton with tree-sitter-c wired through
//! `LanguageAnalyzer`. Tree-sitter is the 1.0 backend per the rewrite plan
//! §6.1.

#![forbid(unsafe_code)]

use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, MetricSpace,
    Result, SourceFile, SourceSpan, SpaceId, SpaceKind, byte_offset_clamped,
};

pub struct CAnalyzer;

impl CAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageAnalyzer for CAnalyzer {
    fn language(&self) -> Language {
        Language::C
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
            language: Language::C,
            backend: AnalysisBackend::TreeSitter,
            diagnostics: Vec::new(),
            root: MetricSpace::new(SpaceId(0), SpaceKind::Unit, span),
            contributions: Vec::new(),
        })
    }
}
