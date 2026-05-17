//! `mehen-powershell` — PowerShell language analyzer.
//!
//! Phase 3 implementation: walks tree-sitter-pwsh with PowerShell-specific
//! decision rules mirroring the pre-1.0 `Cyclomatic for PowershellCode`
//! (`src/metrics/cyclomatic.rs:250-306`).

#![forbid(unsafe_code)]

use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, ParseDiagnostic,
    Result, SourceFile, SourceSpan, SpaceKind, byte_offset_clamped,
};
use mehen_tree_sitter::{
    LanguageRules, NodeFacts, ScopeOpen, TreeSitterParser, empty_space, text_of, walk,
};
use tree_sitter::Node;

pub struct PowerShellAnalyzer;

impl PowerShellAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PowerShellAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

struct PowerShellRules;

impl LanguageRules for PowerShellRules {
    fn scope_for(&self, node: &Node<'_>, source: &[u8]) -> Option<ScopeOpen> {
        let kind = node.kind();
        let opened = match kind {
            "function_statement" | "function_definition" => ScopeOpen::Open {
                kind: SpaceKind::Function,
                name: node
                    .child_by_field_name("name")
                    .map(|n| text_of(&n, source).to_string()),
            },
            "script_block_expression" | "script_block" => ScopeOpen::Open {
                kind: SpaceKind::Closure,
                name: None,
            },
            "class_statement" => ScopeOpen::Open {
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
        // Per pre-1.0 src/metrics/cyclomatic.rs:250-306. PowerShell adds
        // -and / -or as short-circuit and v7's null-coalesce / ternary.
        let cyclomatic_decision = matches!(
            kind,
            "if_statement"
                | "elseif_clause"
                | "for_statement"
                | "foreach_statement"
                | "while_statement"
                | "do_statement"
                | "switch_clause"
                | "catch_clause"
                | "trap_statement"
                | "ternary_expression"
                | "ternary_argument_expression"
                | "null_coalesce_expression"
                | "null_coalesce_argument_expression"
                | "&&"
                | "||"
                | "-and"
                | "-or"
        );
        let nexit = matches!(
            kind,
            "return_statement"
                | "throw_statement"
                | "break_statement"
                | "continue_statement"
                | "exit_statement"
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
                | "-eq"
                | "-ne"
                | "-lt"
                | "-gt"
                | "-le"
                | "-ge"
                | "&&"
                | "||"
                | "-and"
                | "-or"
                | "-not"
                | "!"
                | "??"
                | "?"
        );
        let halstead_operand = matches!(
            kind,
            "variable" | "simple_name" | "integer_literal" | "real_literal" | "string_literal"
        );
        let abc_assignment = matches!(kind, "assignment_expression");
        let abc_branch = matches!(
            kind,
            "command" | "command_expression" | "invocation_expression"
        );
        let abc_condition = matches!(kind, "comparison_expression" | "logical_expression");
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
        } else if trimmed.starts_with('#') || trimmed.starts_with("<#") {
            mehen_metrics::LineClass::Comment
        } else {
            mehen_metrics::LineClass::Code
        }
    }
}

impl LanguageAnalyzer for PowerShellAnalyzer {
    fn language(&self) -> Language {
        Language::PowerShell
    }

    fn backend(&self) -> AnalysisBackend {
        AnalysisBackend::TreeSitter
    }

    fn analyze(&self, source: &SourceFile, _config: &AnalysisConfig) -> Result<LanguageAnalysis> {
        let parser = match TreeSitterParser::new(
            tree_sitter_pwsh::LANGUAGE.into(),
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
                    language: Language::PowerShell,
                    backend: AnalysisBackend::TreeSitter,
                    diagnostics: vec![ParseDiagnostic::fatal(
                        "powershell.parse_error",
                        format!("tree-sitter-pwsh failed: {e}"),
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
            &PowerShellRules,
        );
        Ok(LanguageAnalysis {
            language: Language::PowerShell,
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
    fn analyzes_simple_script() {
        let a = PowerShellAnalyzer::new()
            .analyze(
                &SourceFile::new(
                    "a.ps1".into(),
                    Language::PowerShell,
                    "function Foo { 1 }".to_string(),
                ),
                &AnalysisConfig::default(),
            )
            .unwrap();
        assert_eq!(a.root.kind, SpaceKind::Unit);
    }
}
