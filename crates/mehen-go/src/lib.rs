//! `mehen-go` — Go language analyzer.
//!
//! Phase 3 implementation: walks tree-sitter-go with Go-specific decision
//! rules mirroring the pre-1.0 `Cyclomatic for GoCode`
//! (`src/metrics/cyclomatic.rs:184-206`).

#![forbid(unsafe_code)]

use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, ParseDiagnostic,
    Result, SourceFile, SourceSpan, SpaceKind, byte_offset_clamped,
};
use mehen_tree_sitter::{
    LanguageRules, NodeFacts, ScopeOpen, TreeSitterParser, empty_space, text_of, walk,
};
use tree_sitter::Node;

pub struct GoAnalyzer;

impl GoAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GoAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

struct GoRules;

impl LanguageRules for GoRules {
    fn scope_for(&self, node: &Node<'_>, source: &[u8]) -> Option<ScopeOpen> {
        let kind = node.kind();
        let opened = match kind {
            "function_declaration" | "method_declaration" => ScopeOpen::Open {
                kind: SpaceKind::Function,
                name: node
                    .child_by_field_name("name")
                    .map(|n| text_of(&n, source).to_string()),
            },
            "func_literal" => ScopeOpen::Open {
                kind: SpaceKind::Closure,
                name: None,
            },
            "type_declaration" => ScopeOpen::Open {
                kind: SpaceKind::Class,
                name: None,
            },
            _ => return None,
        };
        Some(opened)
    }

    fn classify(&self, node: &Node<'_>) -> NodeFacts {
        let kind = node.kind();
        let cyclomatic_decision = matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "expression_case"
                | "type_case"
                | "communication_case"
                | "&&"
                | "||"
        );
        let nexit = matches!(
            kind,
            "return_statement" | "break_statement" | "continue_statement"
        );
        let halstead_operator = matches!(
            kind,
            "+" | "-"
                | "*"
                | "/"
                | "%"
                | "="
                | ":="
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
        );
        let halstead_operand = matches!(
            kind,
            "identifier"
                | "field_identifier"
                | "type_identifier"
                | "int_literal"
                | "float_literal"
                | "interpreted_string_literal"
                | "raw_string_literal"
                | "true"
                | "false"
                | "nil"
        );
        let abc_assignment = matches!(kind, "assignment_statement" | "short_var_declaration");
        let abc_branch = matches!(kind, "call_expression");
        let abc_condition = matches!(kind, "binary_expression" | "unary_expression");
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

impl LanguageAnalyzer for GoAnalyzer {
    fn language(&self) -> Language {
        Language::Go
    }

    fn backend(&self) -> AnalysisBackend {
        AnalysisBackend::TreeSitter
    }

    fn analyze(&self, source: &SourceFile, _config: &AnalysisConfig) -> Result<LanguageAnalysis> {
        let parser = match TreeSitterParser::new(
            tree_sitter_go::LANGUAGE.into(),
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
                    language: Language::Go,
                    backend: AnalysisBackend::TreeSitter,
                    diagnostics: vec![ParseDiagnostic::fatal(
                        "go.parse_error",
                        format!("tree-sitter-go failed: {e}"),
                    )],
                    root: empty_space(span),
                    contributions: Vec::new(),
                });
            }
        };

        let result = walk(parser.root(), parser.source(), &source.line_index, &GoRules);
        Ok(LanguageAnalysis {
            language: Language::Go,
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
    use mehen_core::{AnalysisConfig, Language, MetricKey, SourceFile};
    use mehen_metrics::keys;

    fn analyze(source: &str) -> LanguageAnalysis {
        GoAnalyzer::new()
            .analyze(
                &SourceFile::new("a.go".into(), Language::Go, source.to_string()),
                &AnalysisConfig::default(),
            )
            .unwrap()
    }

    #[test]
    fn func_creates_function_space() {
        let a = analyze("package main\nfunc Foo() int { return 1 }\n");
        assert!(a.root.spaces.iter().any(|s| s.kind == SpaceKind::Function));
    }

    #[test]
    fn cyclomatic_counts_branches() {
        let a = analyze(
            "package main\nfunc f(x int) int { if x > 0 && x < 10 { return 1 }; return 2 }\n",
        );
        let func = a
            .root
            .spaces
            .iter()
            .find(|s| s.kind == SpaceKind::Function)
            .unwrap();
        let cy = func
            .metrics
            .get(&MetricKey::new(keys::CYCLOMATIC))
            .unwrap()
            .as_f64();
        assert!(cy >= 3.0, "expected >= 3, got {cy}");
    }
}
