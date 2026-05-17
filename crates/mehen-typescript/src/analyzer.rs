use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, MetricSpace,
    Result, SourceFile, SourceSpan, SpaceId, SpaceKind,
};

macro_rules! ts_analyzer {
    ($name:ident, $lang:expr) => {
        pub struct $name;

        impl $name {
            pub fn new() -> Self {
                Self
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl LanguageAnalyzer for $name {
            fn language(&self) -> Language {
                $lang
            }

            fn backend(&self) -> AnalysisBackend {
                AnalysisBackend::TreeSitter
            }

            fn analyze(
                &self,
                source: &SourceFile,
                _config: &AnalysisConfig,
            ) -> Result<LanguageAnalysis> {
                let span = SourceSpan {
                    start_byte: 0,
                    end_byte: source.text.len() as u32,
                    start_line: 1,
                    end_line: source.line_index.line_count(),
                };
                Ok(LanguageAnalysis {
                    language: $lang,
                    backend: AnalysisBackend::TreeSitter,
                    diagnostics: Vec::new(),
                    root: MetricSpace::new(SpaceId(0), SpaceKind::Unit, span),
                    contributions: Vec::new(),
                })
            }
        }
    };
}

ts_analyzer!(TypeScriptAnalyzer, Language::TypeScript);
ts_analyzer!(JavaScriptAnalyzer, Language::JavaScript);
ts_analyzer!(TsxAnalyzer, Language::Tsx);
ts_analyzer!(JsxAnalyzer, Language::Jsx);
