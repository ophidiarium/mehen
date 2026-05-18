//! `mehen-ruby` — Ruby language analyzer.
//!
//! Phase 3 implementation: walks tree-sitter-ruby with Ruby-specific
//! decision rules mirroring the pre-1.0 `Cyclomatic for RubyCode`
//! (`src/metrics/cyclomatic.rs:208-224`). Phase 9 replaces the
//! tree-sitter backend with Prism.

#![forbid(unsafe_code)]

use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, ParseDiagnostic,
    Result, SourceFile, SourceSpan, SpaceKind, byte_offset_clamped,
};
use mehen_tree_sitter::{
    LanguageRules, NodeFacts, ScopeOpen, TreeSitterParser, empty_space, text_of, walk,
};
use tree_sitter::Node;

pub struct RubyAnalyzer;

impl RubyAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RubyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

struct RubyRules;

impl LanguageRules for RubyRules {
    fn scope_for(&self, node: &Node<'_>, source: &[u8]) -> Option<ScopeOpen> {
        let kind = node.kind();
        let opened = match kind {
            "method" | "singleton_method" => ScopeOpen::Open {
                kind: SpaceKind::Function,
                name: node
                    .child_by_field_name("name")
                    .map(|n| text_of(&n, source).to_string()),
            },
            "block" | "lambda" => ScopeOpen::Open {
                kind: SpaceKind::Closure,
                name: None,
            },
            "class" => ScopeOpen::Open {
                kind: SpaceKind::Class,
                name: node
                    .child_by_field_name("name")
                    .map(|n| text_of(&n, source).to_string()),
            },
            "module" => ScopeOpen::Open {
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
        // Per pre-1.0 src/metrics/cyclomatic.rs:208-224.
        let cyclomatic_decision = matches!(
            kind,
            "if" | "elsif"
                | "unless"
                | "while"
                | "until"
                | "for"
                | "if_modifier"
                | "unless_modifier"
                | "while_modifier"
                | "until_modifier"
                | "when"
                | "in_clause"
                | "rescue"
                | "rescue_modifier"
                | "conditional"
                | "&&"
                | "||"
                | "and"
                | "or"
        );
        let nexit = matches!(kind, "return" | "break" | "next" | "redo" | "yield");
        let halstead_operator = matches!(
            kind,
            "+" | "-"
                | "*"
                | "/"
                | "%"
                | "**"
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
                | "and"
                | "or"
                | "not"
        );
        let halstead_operand = matches!(
            kind,
            "identifier"
                | "constant"
                | "instance_variable"
                | "class_variable"
                | "global_variable"
                | "integer"
                | "float"
                | "string"
                | "symbol"
                | "true"
                | "false"
                | "nil"
                | "self"
        );
        let abc_assignment = matches!(kind, "assignment" | "operator_assignment");
        let abc_branch = matches!(kind, "call" | "command");
        let abc_condition = matches!(kind, "binary" | "unary");
        NodeFacts {
            cyclomatic_decision,
            cognitive_increment: u32::from(cyclomatic_decision),
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

impl LanguageAnalyzer for RubyAnalyzer {
    fn language(&self) -> Language {
        Language::Ruby
    }

    fn backend(&self) -> AnalysisBackend {
        AnalysisBackend::TreeSitter
    }

    fn analyze(&self, source: &SourceFile, _config: &AnalysisConfig) -> Result<LanguageAnalysis> {
        let parser = match TreeSitterParser::new(
            tree_sitter_ruby::LANGUAGE.into(),
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
                    language: Language::Ruby,
                    backend: AnalysisBackend::TreeSitter,
                    diagnostics: vec![ParseDiagnostic::fatal(
                        "ruby.parse_error",
                        format!("tree-sitter-ruby failed: {e}"),
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
            &RubyRules,
        );
        Ok(LanguageAnalysis {
            language: Language::Ruby,
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
    fn def_creates_function_space() {
        let a = RubyAnalyzer::new()
            .analyze(
                &SourceFile::new(
                    "a.rb".into(),
                    Language::Ruby,
                    "def foo\n  1\nend\n".to_string(),
                ),
                &AnalysisConfig::default(),
            )
            .unwrap();
        assert!(a.root.spaces.iter().any(|s| s.kind == SpaceKind::Function));
    }
}
