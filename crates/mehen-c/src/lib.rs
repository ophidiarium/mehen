//! `mehen-c` — C language analyzer.
//!
//! Phase 3 implementation: walks tree-sitter-c with C-specific decision
//! rules mirroring the pre-1.0 `Cyclomatic for CCode`
//! (`src/metrics/cyclomatic.rs:308-331`).

#![forbid(unsafe_code)]

use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, ParseDiagnostic,
    Result, SourceFile, SourceSpan, SpaceKind, byte_offset_clamped,
};
use mehen_tree_sitter::{
    LanguageRules, NodeFacts, ScopeOpen, TreeSitterParser, empty_space, text_of, walk,
};
use tree_sitter::Node;

pub struct CAnalyzer;

impl CAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

struct CRules;

impl LanguageRules for CRules {
    fn scope_for(&self, node: &Node<'_>, source: &[u8]) -> Option<ScopeOpen> {
        let kind = node.kind();
        let opened = match kind {
            "function_definition" => {
                // tree-sitter-c puts the name inside `declarator`; walk
                // into it to find the bare identifier.
                let name = node
                    .child_by_field_name("declarator")
                    .and_then(|d| find_function_name(&d, source));
                ScopeOpen::Open {
                    kind: SpaceKind::Function,
                    name,
                }
            }
            "struct_specifier" | "union_specifier" | "enum_specifier" => ScopeOpen::Open {
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
        let cyclomatic_decision = matches!(
            kind,
            "if_statement"
                | "case_statement"
                | "for_statement"
                | "while_statement"
                | "do_statement"
                | "conditional_expression"
                | "&&"
                | "||"
        );
        let nexit = matches!(
            kind,
            "return_statement" | "break_statement" | "continue_statement" | "goto_statement"
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
                | "&"
                | "|"
                | "^"
                | "<<"
                | ">>"
        );
        let halstead_operand = matches!(
            kind,
            "identifier"
                | "field_identifier"
                | "type_identifier"
                | "number_literal"
                | "string_literal"
                | "char_literal"
                | "true"
                | "false"
                | "null"
        );
        let abc_assignment = matches!(kind, "assignment_expression");
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

fn find_function_name(node: &Node<'_>, source: &[u8]) -> Option<String> {
    if node.kind() == "identifier" {
        return Some(text_of(node, source).to_string());
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            if let Some(name) = find_function_name(&cursor.node(), source) {
                return Some(name);
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    None
}

impl LanguageAnalyzer for CAnalyzer {
    fn language(&self) -> Language {
        Language::C
    }

    fn backend(&self) -> AnalysisBackend {
        AnalysisBackend::TreeSitter
    }

    fn analyze(&self, source: &SourceFile, _config: &AnalysisConfig) -> Result<LanguageAnalysis> {
        let parser = match TreeSitterParser::new(
            tree_sitter_c::LANGUAGE.into(),
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
                    language: Language::C,
                    backend: AnalysisBackend::TreeSitter,
                    diagnostics: vec![ParseDiagnostic::fatal(
                        "c.parse_error",
                        format!("tree-sitter-c failed: {e}"),
                    )],
                    root: empty_space(span),
                    contributions: Vec::new(),
                });
            }
        };

        let result = walk(parser.root(), parser.source(), &source.line_index, &CRules);
        Ok(LanguageAnalysis {
            language: Language::C,
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
    fn func_creates_function_space() {
        let a = CAnalyzer::new()
            .analyze(
                &SourceFile::new(
                    "a.c".into(),
                    Language::C,
                    "int foo(int x) { return x; }".to_string(),
                ),
                &AnalysisConfig::default(),
            )
            .unwrap();
        assert!(a.root.spaces.iter().any(|s| s.kind == SpaceKind::Function));
    }
}
