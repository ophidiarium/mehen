//! Oxc-backed analyzer entry points for TypeScript / JavaScript / TSX / JSX.
//!
//! The four `LanguageAnalyzer` impls all funnel into a single
//! [`crate::walker::analyze`] call with a different [`SourceType`] —
//! every other piece of behavior (scope detection, decision points,
//! LOC, Halstead) is shared.

use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, ParseDiagnostic,
    Result, SourceFile, SourceSpan, byte_offset_clamped,
};
use mehen_metrics::MetricTreeBuilder;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_parser::config::TokensParserConfig;
use oxc_span::SourceType;

use crate::walker;

/// Run the parser and walker, returning a populated `LanguageAnalysis`.
///
/// This is the single dispatch point shared by every flavor of the
/// analyzer. The Oxc parser is constructed with `TokensParserConfig` so
/// the lexer captures every punctuation, keyword, identifier, and
/// literal token — the walker uses the token stream for Halstead
/// operator/operand classification while it walks the AST for everything
/// else (scopes, decision points, ABC, NPA/NPM/WMC, NOM, LOC).
fn analyze_with_source_type(
    language: Language,
    source: &SourceFile,
    source_type: SourceType,
) -> LanguageAnalysis {
    let allocator = Allocator::default();
    let parser_return = Parser::new(&allocator, source.text.as_str(), source_type)
        .with_config(TokensParserConfig)
        .parse();

    if parser_return.panicked {
        let span = SourceSpan {
            start_byte: 0,
            end_byte: byte_offset_clamped(source.text.len()),
            start_line: 1,
            end_line: source.line_index.line_count(),
        };
        let builder = MetricTreeBuilder::new(span);
        let diagnostics = vec![ParseDiagnostic::fatal(
            "typescript.parse_error",
            format!(
                "oxc_parser panicked with {} error(s)",
                parser_return.errors.len()
            ),
        )];
        return LanguageAnalysis {
            language,
            backend: AnalysisBackend::Oxc,
            diagnostics,
            root: builder.finish(),
            contributions: Vec::new(),
        };
    }

    let root = walker::walk_program(
        &parser_return.program,
        &parser_return.tokens,
        source.text.as_str(),
        &source.line_index,
    );

    LanguageAnalysis {
        language,
        backend: AnalysisBackend::Oxc,
        diagnostics: Vec::new(),
        root,
        contributions: Vec::new(),
    }
}

macro_rules! ts_analyzer {
    ($name:ident, $lang:expr, $source_type:expr) => {
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
                AnalysisBackend::Oxc
            }

            fn analyze(
                &self,
                source: &SourceFile,
                _config: &AnalysisConfig,
            ) -> Result<LanguageAnalysis> {
                Ok(analyze_with_source_type($lang, source, $source_type))
            }
        }
    };
}

ts_analyzer!(TypeScriptAnalyzer, Language::TypeScript, SourceType::ts());
ts_analyzer!(TsxAnalyzer, Language::Tsx, SourceType::tsx());
ts_analyzer!(JavaScriptAnalyzer, Language::JavaScript, SourceType::mjs());
ts_analyzer!(JsxAnalyzer, Language::Jsx, SourceType::jsx());

#[cfg(test)]
mod tests {
    use super::*;
    use mehen_core::{AnalysisConfig, Language, MetricKey, SourceFile, SpaceKind};
    use mehen_metrics::keys;

    fn analyze_ts(src: &str) -> LanguageAnalysis {
        let analyzer = TypeScriptAnalyzer::new();
        let file = SourceFile::new("a.ts".into(), Language::TypeScript, src.to_string());
        analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
    }

    #[test]
    fn function_creates_function_space() {
        let a = analyze_ts("function foo() { return 1; }");
        assert_eq!(a.root.spaces.len(), 1);
        assert_eq!(a.root.spaces[0].kind, SpaceKind::Function);
        assert_eq!(a.root.spaces[0].name.as_deref(), Some("foo"));
    }

    #[test]
    fn class_creates_class_space() {
        let a = analyze_ts("class C { m() { return 1; } }");
        assert_eq!(a.root.spaces.len(), 1);
        assert_eq!(a.root.spaces[0].kind, SpaceKind::Class);
        assert_eq!(a.root.spaces[0].spaces.len(), 1);
    }

    #[test]
    fn cyclomatic_counts_decision_points() {
        let a = analyze_ts("function f(x) { if (x && x) return 1; return 2; }");
        let cy = a.root.spaces[0]
            .metrics
            .get(&MetricKey::new(keys::CYCLOMATIC))
            .unwrap()
            .as_f64();
        assert!(cy >= 3.0, "expected >= 3, got {cy}");
    }
}
