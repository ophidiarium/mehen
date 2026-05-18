//! `mehen-kotlin` — Kotlin language analyzer.
//!
//! Phase 3 implementation: walks `tree-sitter-kotlin` (the
//! `tree-sitter-kotlin-sg` crate, aliased as `tree-sitter-kotlin` in the
//! workspace's dependency table) with Kotlin-specific decision rules
//! mirroring the pre-1.0 `Cyclomatic for KotlinCode`
//! (`src/metrics/cyclomatic.rs:226-245`).

#![forbid(unsafe_code)]

use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, ParseDiagnostic,
    Result, SourceFile, SourceSpan, SpaceKind, byte_offset_clamped,
};
use mehen_tree_sitter::{
    LanguageRules, NodeFacts, ScopeOpen, TreeSitterParser, empty_space, text_of, walk,
};
use tree_sitter::Node;

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

struct KotlinRules;

impl LanguageRules for KotlinRules {
    fn scope_for(&self, node: &Node<'_>, source: &[u8]) -> Option<ScopeOpen> {
        let kind = node.kind();
        let opened = match kind {
            "function_declaration" => ScopeOpen::Open {
                kind: SpaceKind::Function,
                name: node
                    .child_by_field_name("name")
                    .map(|n| text_of(&n, source).to_string()),
            },
            "lambda_literal" | "anonymous_function" => ScopeOpen::Open {
                kind: SpaceKind::Closure,
                name: None,
            },
            "class_declaration" | "object_declaration" => ScopeOpen::Open {
                kind: SpaceKind::Class,
                name: node
                    .child_by_field_name("name")
                    .map(|n| text_of(&n, source).to_string()),
            },
            _ => return None,
        };
        Some(opened)
    }

    fn classify(&self, node: &Node<'_>) -> NodeFacts {
        let kind = node.kind();
        // Per pre-1.0 src/metrics/cyclomatic.rs:226-245 (catch is excluded).
        let cyclomatic_decision = matches!(
            kind,
            "if_expression"
                | "for_statement"
                | "while_statement"
                | "do_while_statement"
                | "when_entry"
                | "&&"
                | "||"
        );
        let nexit = matches!(
            kind,
            "jump_expression" | "return_expression" | "break" | "continue" | "throw_expression"
        );
        let halstead_operator = matches!(
            kind,
            "+" | "-"
                | "*"
                | "/"
                | "%"
                | "="
                | "+="
                | "-="
                | "*="
                | "/="
                | "%="
                | "=="
                | "!="
                | "<"
                | ">"
                | "<="
                | ">="
                | "&&"
                | "||"
                | "!"
                | "?:"
        );
        let halstead_operand = matches!(
            kind,
            "simple_identifier"
                | "integer_literal"
                | "real_literal"
                | "string_literal"
                | "boolean_literal"
                | "null_literal"
                | "this_expression"
        );
        let abc_assignment = matches!(kind, "assignment");
        let abc_branch = matches!(kind, "call_expression");
        let abc_condition = matches!(kind, "comparison_expression" | "equality_expression");
        NodeFacts {
            cyclomatic_decision,
            cognitive: if cyclomatic_decision {
                mehen_tree_sitter::CognitiveFact::IncreaseNesting
            } else {
                mehen_tree_sitter::CognitiveFact::None
            },
            halstead_operator,
            halstead_operand,
            nexit,
            abc_branch,
            abc_condition,
            abc_assignment,
            loc: mehen_tree_sitter::LocFact::Code,
        }
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

        let result = walk(
            parser.root(),
            parser.source(),
            &source.line_index,
            &KotlinRules,
        );
        Ok(LanguageAnalysis {
            language: Language::Kotlin,
            backend: AnalysisBackend::TreeSitter,
            diagnostics: Vec::new(),
            root: result.root,
            contributions: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mehen_core::{AnalysisConfig, Language, SourceFile};

    #[test]
    fn fun_creates_function_space() {
        let a = KotlinAnalyzer::new()
            .analyze(
                &SourceFile::new(
                    "a.kt".into(),
                    Language::Kotlin,
                    "fun foo(): Int { return 1 }\n".to_string(),
                ),
                &AnalysisConfig::default(),
            )
            .unwrap();
        assert!(a.root.spaces.iter().any(|s| s.kind == SpaceKind::Function));
    }
}
