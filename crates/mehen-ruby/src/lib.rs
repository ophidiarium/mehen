//! `mehen-ruby` — Ruby language analyzer.
//!
//! Phase 1 scope: skeleton with tree-sitter-ruby wired through
//! `LanguageAnalyzer`. Phase 9 replaces the tree-sitter backend with Prism;
//! the license/build prerequisites are documented in the rewrite plan
//! review §2.2.

#![forbid(unsafe_code)]

use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, MetricSpace,
    Result, SourceFile, SourceSpan, SpaceId, SpaceKind,
};

pub struct RubyAnalyzer;

impl RubyAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RubyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageAnalyzer for RubyAnalyzer {
    fn language(&self) -> Language {
        Language::Ruby
    }

    fn backend(&self) -> AnalysisBackend {
        AnalysisBackend::TreeSitter
    }

    fn analyze(&self, source: &SourceFile, _config: &AnalysisConfig) -> Result<LanguageAnalysis> {
        let span = SourceSpan {
            start_byte: 0,
            end_byte: source.text.len() as u32,
            start_line: 1,
            end_line: source.line_index.line_count(),
        };
        Ok(LanguageAnalysis {
            language: Language::Ruby,
            backend: AnalysisBackend::TreeSitter,
            diagnostics: Vec::new(),
            root: MetricSpace::new(SpaceId(0), SpaceKind::Unit, span),
            contributions: Vec::new(),
        })
    }
}
