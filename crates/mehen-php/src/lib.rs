//! `mehen-php` — PHP language analyzer.
//!
//! Phase 3 implementation: walks tree-sitter-php with PHP-specific
//! decision rules mirroring the pre-1.0 `Cyclomatic for PhpCode`
//! (`src/metrics/cyclomatic.rs:333-364`). Phase 8 replaces the
//! tree-sitter backend with Mago.

#![forbid(unsafe_code)]

use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, ParseDiagnostic,
    Result, SourceFile, SourceSpan, SpaceKind, byte_offset_clamped,
};
use mehen_tree_sitter::{
    LanguageRules, NodeFacts, ScopeOpen, TreeSitterParser, empty_space, text_of, walk,
};
use tree_sitter::Node;

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

struct PhpRules;

impl LanguageRules for PhpRules {
    fn scope_for(&self, node: &Node<'_>, source: &[u8]) -> Option<ScopeOpen> {
        let kind = node.kind();
        let opened = match kind {
            "function_definition" | "method_declaration" => ScopeOpen::Open {
                kind: SpaceKind::Function,
                name: node
                    .child_by_field_name("name")
                    .map(|n| text_of(&n, source).to_string()),
            },
            "anonymous_function_creation_expression" | "arrow_function" => ScopeOpen::Open {
                kind: SpaceKind::Closure,
                name: None,
            },
            "class_declaration" => ScopeOpen::Open {
                kind: SpaceKind::Class,
                name: node
                    .child_by_field_name("name")
                    .map(|n| text_of(&n, source).to_string()),
            },
            "interface_declaration" => ScopeOpen::Open {
                kind: SpaceKind::Interface,
                name: node
                    .child_by_field_name("name")
                    .map(|n| text_of(&n, source).to_string()),
            },
            "trait_declaration" => ScopeOpen::Open {
                kind: SpaceKind::Trait,
                name: node
                    .child_by_field_name("name")
                    .map(|n| text_of(&n, source).to_string()),
            },
            "enum_declaration" => ScopeOpen::Open {
                kind: SpaceKind::Enum,
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
        // Per pre-1.0 src/metrics/cyclomatic.rs:333-364.
        let cyclomatic_decision = matches!(
            kind,
            "if_statement"
                | "else_if_clause"
                | "case_statement"
                | "for_statement"
                | "foreach_statement"
                | "while_statement"
                | "do_statement"
                | "conditional_expression"
                | "match_conditional_expression"
                | "catch_clause"
                | "&&"
                | "||"
                | "and"
                | "or"
        );
        let nexit = matches!(
            kind,
            "return_statement" | "throw_statement" | "break_statement" | "continue_statement"
        );
        let halstead_operator = matches!(
            kind,
            "+" | "-"
                | "*"
                | "/"
                | "%"
                | "**"
                | "."
                | "="
                | "+="
                | "-="
                | "*="
                | "/="
                | "%="
                | ".="
                | "=="
                | "==="
                | "!="
                | "!=="
                | "<"
                | ">"
                | "<="
                | ">="
                | "&&"
                | "||"
                | "!"
                | "and"
                | "or"
                | "not"
                | "??"
        );
        let halstead_operand = matches!(
            kind,
            "name"
                | "variable_name"
                | "qualified_name"
                | "integer"
                | "float"
                | "string"
                | "encapsed_string"
                | "boolean"
                | "null"
        );
        let abc_assignment = matches!(
            kind,
            "assignment_expression" | "augmented_assignment_expression"
        );
        let abc_branch = matches!(
            kind,
            "function_call_expression" | "method_call_expression" | "object_creation_expression"
        );
        let abc_condition = matches!(kind, "binary_expression" | "unary_op_expression");
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

impl LanguageAnalyzer for PhpAnalyzer {
    fn language(&self) -> Language {
        Language::Php
    }

    fn backend(&self) -> AnalysisBackend {
        AnalysisBackend::TreeSitter
    }

    fn analyze(&self, source: &SourceFile, _config: &AnalysisConfig) -> Result<LanguageAnalysis> {
        let parser = match TreeSitterParser::new(
            tree_sitter_php::LANGUAGE_PHP.into(),
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
                    language: Language::Php,
                    backend: AnalysisBackend::TreeSitter,
                    diagnostics: vec![ParseDiagnostic::fatal(
                        "php.parse_error",
                        format!("tree-sitter-php failed: {e}"),
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
            &PhpRules,
        );
        Ok(LanguageAnalysis {
            language: Language::Php,
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
    fn function_creates_function_space() {
        let a = PhpAnalyzer::new()
            .analyze(
                &SourceFile::new(
                    "a.php".into(),
                    Language::Php,
                    "<?php\nfunction foo() { return 1; }\n".to_string(),
                ),
                &AnalysisConfig::default(),
            )
            .unwrap();
        assert!(a.root.spaces.iter().any(|s| s.kind == SpaceKind::Function));
    }
}
