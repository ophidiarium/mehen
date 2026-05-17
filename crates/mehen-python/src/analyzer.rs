use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, MetricSpace,
    Result, SourceFile, SourceSpan, SpaceId, SpaceKind,
};

/// Tree-sitter-backed Python analyzer.
///
/// The Phase 1 implementation returns an empty file-level space so the
/// engine wiring can be exercised end-to-end. Phase 3 fills in the per-metric
/// interpretation by porting from `src/languages/language_python.rs` and the
/// Python branches of the metric files in the pre-1.0 layout.
pub struct PythonAnalyzer;

impl PythonAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PythonAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageAnalyzer for PythonAnalyzer {
    fn language(&self) -> Language {
        Language::Python
    }

    fn backend(&self) -> AnalysisBackend {
        AnalysisBackend::TreeSitter
    }

    fn analyze(&self, source: &SourceFile, _config: &AnalysisConfig) -> Result<LanguageAnalysis> {
        // Phase 1 placeholder: produces an empty unit space so engine
        // plumbing is exercisable. Phase 3 will replace this with real
        // metric computation ported from the pre-1.0 src/.
        let span = SourceSpan {
            start_byte: 0,
            end_byte: source.text.len() as u32,
            start_line: 1,
            end_line: source.line_index.line_count(),
        };
        let root = MetricSpace::new(SpaceId(0), SpaceKind::Unit, span);
        Ok(LanguageAnalysis {
            language: Language::Python,
            backend: AnalysisBackend::TreeSitter,
            diagnostics: Vec::new(),
            root,
            contributions: Vec::new(),
        })
    }
}
