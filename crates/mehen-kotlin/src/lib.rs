//! `mehen-kotlin` — Kotlin language analyzer.
//!
//! Tree-sitter-kotlin walker that produces per-space metric output
//! matching the pre-1.0 `legacy::metrics::*::compute for KotlinCode`
//! arms. See [`walker`] for the metric coverage table.

#![forbid(unsafe_code)]

mod grammar;
mod walker;

use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, ParseDiagnostic,
    Result, SourceFile, SourceSpan, byte_offset_clamped,
};
use mehen_tree_sitter::{TreeSitterParser, collect_recovered_errors, empty_space};

pub struct KotlinAnalyzer;

impl KotlinAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KotlinAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageAnalyzer for KotlinAnalyzer {
    fn language(&self) -> Language {
        Language::Kotlin
    }

    fn backend(&self) -> AnalysisBackend {
        AnalysisBackend::TreeSitter
    }

    fn analyze(&self, source: &SourceFile, _config: &AnalysisConfig) -> Result<LanguageAnalysis> {
        let parser = match TreeSitterParser::new(
            tree_sitter_kotlin::LANGUAGE.into(),
            source.text.clone().into_bytes(),
        ) {
            Ok(p) => p,
            Err(e) => {
                let span = SourceSpan {
                    start_byte: 0,
                    end_byte: byte_offset_clamped(source.text.len()),
                    start_line: 1,
                    end_line: source.line_index.line_count(),
                };
                return Ok(LanguageAnalysis {
                    language: Language::Kotlin,
                    backend: AnalysisBackend::TreeSitter,
                    diagnostics: vec![ParseDiagnostic::fatal(
                        "kotlin.parse_error",
                        format!("tree-sitter-kotlin failed: {e}"),
                    )],
                    root: empty_space(span),
                    contributions: Vec::new(),
                });
            }
        };

        let root = walker::walk_program(parser.root(), parser.source(), &source.line_index);
        // Tree-sitter recovers from syntax errors by inserting ERROR /
        // missing nodes; surface them as `error` diagnostics so the
        // metric output can't masquerade as clean (plan §9.3).
        let diagnostics = collect_recovered_errors(parser.root(), "kotlin.syntax_error", 16);
        Ok(LanguageAnalysis {
            language: Language::Kotlin,
            backend: AnalysisBackend::TreeSitter,
            diagnostics,
            root,
            contributions: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mehen_core::{AnalysisConfig, Language, SourceFile, SpaceKind};

    fn analyze(source: &str, path: &str) -> LanguageAnalysis {
        let analyzer = KotlinAnalyzer::new();
        let file = SourceFile::new(path.into(), Language::Kotlin, source.to_string());
        analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
    }

    #[test]
    fn empty_file_yields_root_unit() {
        let a = analyze("", "test.kt");
        assert_eq!(a.root.kind, SpaceKind::Unit);
        assert!(a.root.spaces.is_empty());
    }

    #[test]
    fn fun_creates_function_space() {
        let a = analyze("fun foo(): Int { return 1 }\n", "test.kt");
        assert!(a.root.spaces.iter().any(|s| s.kind == SpaceKind::Function));
        assert_eq!(a.root.spaces[0].name.as_deref(), Some("foo"));
    }

    #[test]
    fn class_creates_class_space_with_method() {
        let a = analyze("class C { fun m() {} }\n", "test.kt");
        assert_eq!(a.root.spaces.len(), 1);
        assert_eq!(a.root.spaces[0].kind, SpaceKind::Class);
        assert_eq!(a.root.spaces[0].name.as_deref(), Some("C"));
        assert_eq!(a.root.spaces[0].spaces.len(), 1);
    }

    #[test]
    fn interface_creates_interface_space() {
        let a = analyze("interface I { fun m() }\n", "test.kt");
        assert_eq!(a.root.spaces.len(), 1);
        assert_eq!(a.root.spaces[0].kind, SpaceKind::Interface);
        assert_eq!(a.root.spaces[0].name.as_deref(), Some("I"));
    }
}
