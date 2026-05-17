//! `mehen-rust` — Rust language analyzer.
//!
//! Phase 3 implementation: drives the shared walker from
//! `mehen-tree-sitter` against tree-sitter-rust with Rust-specific
//! decision rules mirroring the pre-1.0 `Cyclomatic for RustCode`
//! (`src/metrics/cyclomatic.rs:163-182`).

#![forbid(unsafe_code)]

use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, ParseDiagnostic,
    Result, SourceFile, SourceSpan, SpaceKind, byte_offset_clamped,
};
use mehen_tree_sitter::{
    LanguageRules, NodeFacts, ScopeOpen, TreeSitterParser, empty_space, text_of, walk,
};
use tree_sitter::Node;

pub struct RustAnalyzer;

impl RustAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RustAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

struct RustRules;

impl LanguageRules for RustRules {
    fn scope_for(&self, node: &Node<'_>, source: &[u8]) -> Option<ScopeOpen> {
        let kind = node.kind();
        let opened = match kind {
            "function_item" => ScopeOpen::Open {
                kind: SpaceKind::Function,
                name: node
                    .child_by_field_name("name")
                    .map(|n| text_of(&n, source).to_string()),
            },
            "closure_expression" => ScopeOpen::Open {
                kind: SpaceKind::Closure,
                name: None,
            },
            "struct_item" | "enum_item" | "union_item" => ScopeOpen::Open {
                kind: SpaceKind::Class,
                name: node
                    .child_by_field_name("name")
                    .map(|n| text_of(&n, source).to_string()),
            },
            "trait_item" => ScopeOpen::Open {
                kind: SpaceKind::Trait,
                name: node
                    .child_by_field_name("name")
                    .map(|n| text_of(&n, source).to_string()),
            },
            "impl_item" => ScopeOpen::Open {
                kind: SpaceKind::Impl,
                name: node
                    .child_by_field_name("type")
                    .map(|n| text_of(&n, source).to_string()),
            },
            _ => return None,
        };
        Some(opened)
    }

    fn classify(&self, node: &Node<'_>) -> NodeFacts {
        let kind = node.kind();
        // Mirrors src/metrics/cyclomatic.rs:163-182 — `if`, `for`, `while`,
        // `loop`, match arms, `?`, and short-circuit `&&`/`||`.
        let cyclomatic_decision = matches!(
            kind,
            "if_expression"
                | "for_expression"
                | "while_expression"
                | "loop_expression"
                | "match_arm"
                | "try_expression"
                | "&&"
                | "||"
        );
        let nexit = matches!(
            kind,
            "return_expression" | "break_expression" | "continue_expression"
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
                | "?"
                | "!"
        );
        let halstead_operand = matches!(
            kind,
            "identifier"
                | "type_identifier"
                | "field_identifier"
                | "integer_literal"
                | "float_literal"
                | "string_literal"
                | "char_literal"
                | "boolean_literal"
                | "self"
        );
        let abc_assignment = matches!(kind, "assignment_expression" | "compound_assignment_expr");
        let abc_branch = matches!(kind, "call_expression" | "macro_invocation");
        let abc_condition = matches!(kind, "binary_expression" | "unary_expression");
        NodeFacts {
            cyclomatic_decision,
            cognitive_increment: u32::from(cyclomatic_decision),
            halstead_operator,
            halstead_operand,
            nexit,
            abc_branch,
            abc_condition,
            abc_assignment,
        }
    }

    fn classify_line(&self, line: &str) -> mehen_metrics::LineClass {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            mehen_metrics::LineClass::Blank
        } else if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*')
        {
            mehen_metrics::LineClass::Comment
        } else {
            mehen_metrics::LineClass::Code
        }
    }
}

impl LanguageAnalyzer for RustAnalyzer {
    fn language(&self) -> Language {
        Language::Rust
    }

    fn backend(&self) -> AnalysisBackend {
        AnalysisBackend::TreeSitter
    }

    fn analyze(&self, source: &SourceFile, _config: &AnalysisConfig) -> Result<LanguageAnalysis> {
        let parser = match TreeSitterParser::new(
            tree_sitter_rust::LANGUAGE.into(),
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
                    language: Language::Rust,
                    backend: AnalysisBackend::TreeSitter,
                    diagnostics: vec![ParseDiagnostic::fatal(
                        "rust.parse_error",
                        format!("tree-sitter-rust failed: {e}"),
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
            &RustRules,
        );
        Ok(LanguageAnalysis {
            language: Language::Rust,
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
        RustAnalyzer::new()
            .analyze(
                &SourceFile::new("a.rs".into(), Language::Rust, source.to_string()),
                &AnalysisConfig::default(),
            )
            .unwrap()
    }

    #[test]
    fn fn_creates_function_space() {
        let a = analyze("fn foo() { 1 }");
        assert_eq!(a.root.spaces.len(), 1);
        assert_eq!(a.root.spaces[0].kind, SpaceKind::Function);
        assert_eq!(a.root.spaces[0].name.as_deref(), Some("foo"));
    }

    #[test]
    fn struct_creates_class_space() {
        let a = analyze("struct S { x: i32 }");
        assert_eq!(a.root.spaces.len(), 1);
        assert_eq!(a.root.spaces[0].kind, SpaceKind::Class);
    }

    #[test]
    fn cyclomatic_counts_match_arms() {
        let a = analyze("fn f(x: i32) -> i32 { match x { 1 => 1, 2 => 2, _ => 3 } }");
        let cy = a.root.spaces[0]
            .metrics
            .get(&MetricKey::new(keys::CYCLOMATIC))
            .unwrap()
            .as_f64();
        assert!(cy >= 4.0, "expected >= 4, got {cy}");
    }
}
