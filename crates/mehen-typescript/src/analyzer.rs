use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, ParseDiagnostic,
    Result, SourceFile, SourceSpan, SpaceKind, byte_offset_clamped,
};
use mehen_tree_sitter::{
    LanguageRules, NodeFacts, ScopeOpen, TreeSitterParser, empty_space, text_of, walk,
};
use tree_sitter::Node;

/// Shared LanguageRules for the JS/TS/JSX/TSX family.
///
/// All four flavors share the same decision set per pre-1.0
/// `Cyclomatic for TypescriptCode` / `for TsxCode`. The grammar entry
/// point differs (TypeScript vs TSX) but the node-kind names overlap on
/// the constructs we examine.
struct TsLikeRules;

impl LanguageRules for TsLikeRules {
    fn scope_for(&self, node: &Node<'_>, source: &[u8]) -> Option<ScopeOpen> {
        let kind = node.kind();
        let opened = match kind {
            "function_declaration"
            | "function_expression"
            | "method_definition"
            | "generator_function"
            | "generator_function_declaration" => ScopeOpen::Open {
                kind: SpaceKind::Function,
                name: node
                    .child_by_field_name("name")
                    .map(|n| text_of(&n, source).to_string()),
            },
            "arrow_function" => ScopeOpen::Open {
                kind: SpaceKind::Closure,
                name: None,
            },
            // `class` (without `_declaration`) is the keyword token, not
            // the declaration, so it must not open a scope.
            "class_declaration" | "class_expression" => ScopeOpen::Open {
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
            _ => return None,
        };
        Some(opened)
    }

    fn classify(&self, node: &Node<'_>) -> NodeFacts {
        let kind = node.kind();
        // Mirrors src/metrics/cyclomatic.rs:137-148 (Typescript) and
        // 150-161 (Tsx). Both cover JS/TS/JSX/TSX in one set.
        let cyclomatic_decision = matches!(
            kind,
            "if_statement"
                | "for_statement"
                | "for_in_statement"
                | "for_of_statement"
                | "while_statement"
                | "do_statement"
                | "switch_case"
                | "catch_clause"
                | "ternary_expression"
                | "&&"
                | "||"
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
                | "="
                | "+="
                | "-="
                | "*="
                | "/="
                | "%="
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
                | "??"
                | "?"
                | "!"
                | "..."
        );
        let halstead_operand = matches!(
            kind,
            "identifier"
                | "property_identifier"
                | "type_identifier"
                | "number"
                | "string"
                | "true"
                | "false"
                | "null"
                | "undefined"
                | "this"
                | "super"
        );
        let abc_assignment = matches!(
            kind,
            "assignment_expression" | "augmented_assignment_expression"
        );
        let abc_branch = matches!(kind, "call_expression" | "new_expression");
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

macro_rules! ts_analyzer {
    ($name:ident, $lang:expr, $grammar:expr) => {
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
                let parser = match TreeSitterParser::new($grammar, source.text.clone().into_bytes())
                {
                    Ok(p) => p,
                    Err(e) => {
                        let span = SourceSpan {
                            start_byte: 0,
                            end_byte: byte_offset_clamped(source.text.len()),
                            start_line: 1,
                            end_line: source.line_index.line_count(),
                        };
                        return Ok(LanguageAnalysis {
                            language: $lang,
                            backend: AnalysisBackend::TreeSitter,
                            diagnostics: vec![ParseDiagnostic::fatal(
                                concat!(stringify!($name), "_parse_error"),
                                format!("tree-sitter failed: {e}"),
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
                    &TsLikeRules,
                );
                Ok(LanguageAnalysis {
                    language: $lang,
                    backend: AnalysisBackend::TreeSitter,
                    diagnostics: Vec::new(),
                    root: result.root,
                    contributions: Vec::new(),
                })
            }
        }
    };
}

ts_analyzer!(
    TypeScriptAnalyzer,
    Language::TypeScript,
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
);
ts_analyzer!(
    JavaScriptAnalyzer,
    Language::JavaScript,
    tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
);
ts_analyzer!(
    TsxAnalyzer,
    Language::Tsx,
    tree_sitter_typescript::LANGUAGE_TSX.into()
);
ts_analyzer!(
    JsxAnalyzer,
    Language::Jsx,
    tree_sitter_typescript::LANGUAGE_TSX.into()
);

#[cfg(test)]
mod tests {
    use super::*;
    use mehen_core::{AnalysisConfig, Language, MetricKey, SourceFile};
    use mehen_metrics::keys;

    fn analyze_ts(source: &str) -> LanguageAnalysis {
        let analyzer = TypeScriptAnalyzer::new();
        let file = SourceFile::new("a.ts".into(), Language::TypeScript, source.to_string());
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
