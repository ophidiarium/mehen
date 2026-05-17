//! `mehen-powershell` — PowerShell language analyzer.
//!
//! Phase 1 scope: skeleton with tree-sitter-pwsh wired through
//! `LanguageAnalyzer`. Tree-sitter is the 1.0 backend per the rewrite plan
//! §6.1; PowerShell-specific complexity rules (pipeline chains, script
//! blocks, command flow) stay PowerShell-specific.

#![forbid(unsafe_code)]

use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, MetricSpace,
    Result, SourceFile, SourceSpan, SpaceId, SpaceKind,
};

pub struct PowerShellAnalyzer;

impl PowerShellAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PowerShellAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageAnalyzer for PowerShellAnalyzer {
    fn language(&self) -> Language {
        Language::PowerShell
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
            language: Language::PowerShell,
            backend: AnalysisBackend::TreeSitter,
            diagnostics: Vec::new(),
            root: MetricSpace::new(SpaceId(0), SpaceKind::Unit, span),
            contributions: Vec::new(),
        })
    }
}
