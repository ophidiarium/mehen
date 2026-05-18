use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, ParseDiagnostic,
    Result, SourceFile, SourceSpan, SpaceKind, byte_offset_clamped,
};
use mehen_tree_sitter::{
    LanguageRules, LocFact, NodeFacts, ScopeOpen, TreeSitterParser, empty_space, walk,
};
use tree_sitter::Node;

struct PythonRules;

impl LanguageRules for PythonRules {
    fn scope_for(&self, node: &Node<'_>, source: &[u8]) -> Option<ScopeOpen> {
        python_scope(node, source)
    }

    fn classify(&self, node: &Node<'_>) -> NodeFacts {
        python_facts(node)
    }
}

/// Tree-sitter-backed Python analyzer.
///
/// Phase 3 implementation: drives the shared walker from
/// `mehen-tree-sitter` against tree-sitter-python with Python-specific
/// decision/operator/operand/exit/scope rules. Phase 6 replaces the
/// tree-sitter backend with Ruff while keeping the same outer interface.
pub struct PythonAnalyzer;

impl PythonAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PythonAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageAnalyzer for PythonAnalyzer {
    fn language(&self) -> Language {
        Language::Python
    }

    fn backend(&self) -> AnalysisBackend {
        AnalysisBackend::TreeSitter
    }

    fn analyze(&self, source: &SourceFile, _config: &AnalysisConfig) -> Result<LanguageAnalysis> {
        let parser = match TreeSitterParser::new(
            tree_sitter_python::LANGUAGE.into(),
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
                    language: Language::Python,
                    backend: AnalysisBackend::TreeSitter,
                    diagnostics: vec![ParseDiagnostic::fatal(
                        "python.parse_error",
                        format!("tree-sitter-python failed: {e}"),
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
            &PythonRules,
        );
        Ok(LanguageAnalysis {
            language: Language::Python,
            backend: AnalysisBackend::TreeSitter,
            diagnostics: Vec::new(),
            root: result.root,
            contributions: Vec::new(),
        })
    }
}

/// Whether `node` opens a Python space and what kind.
pub(crate) fn python_scope(node: &Node<'_>, source: &[u8]) -> Option<ScopeOpen> {
    let kind = node.kind();
    let opened = match kind {
        "function_definition" => ScopeOpen::Open {
            kind: SpaceKind::Function,
            name: node
                .child_by_field_name("name")
                .map(|n| mehen_tree_sitter::text_of(&n, source).to_string()),
        },
        "class_definition" => ScopeOpen::Open {
            kind: SpaceKind::Class,
            name: node
                .child_by_field_name("name")
                .map(|n| mehen_tree_sitter::text_of(&n, source).to_string()),
        },
        "lambda" => ScopeOpen::Open {
            kind: SpaceKind::Closure,
            name: None,
        },
        _ => return None,
    };
    Some(opened)
}

/// Python rules: which AST nodes count toward which metrics.
///
/// Mirrors the pre-1.0 `Cyclomatic for PythonCode`
/// (`src/metrics/cyclomatic.rs:117-135`). Phase 3+ adds richer cognitive
/// rules (nesting penalties, binary-sequence handling) when the pre-1.0
/// `Cognitive for PythonCode` is fully ported.
pub(crate) fn python_facts(node: &Node<'_>) -> NodeFacts {
    let kind = node.kind();
    let cyclomatic_decision = matches!(
        kind,
        "if_statement"
            | "elif_clause"
            | "for_statement"
            | "while_statement"
            | "except_clause"
            | "and"
            | "or"
            | "boolean_operator"
            | "conditional_expression"
    );
    let nexit = matches!(kind, "return_statement" | "raise_statement");
    let halstead_operator = matches!(
        kind,
        "+" | "-"
            | "*"
            | "/"
            | "%"
            | "**"
            | "//"
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
            | "and"
            | "or"
            | "not"
            | "if"
            | "elif"
            | "else"
            | "for"
            | "while"
            | "return"
            | "in"
            | "is"
            | "lambda"
    );
    let halstead_operand = matches!(
        kind,
        "identifier" | "integer" | "float" | "string" | "true" | "false" | "none"
    );
    let abc_assignment = matches!(kind, "assignment" | "augmented_assignment");
    let abc_branch = matches!(kind, "call");
    let abc_condition = matches!(
        kind,
        "comparison_operator" | "boolean_operator" | "not_operator"
    );
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
        loc: LocFact::Code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mehen_core::{AnalysisConfig, Language, MetricKey, SourceFile};
    use mehen_metrics::keys;

    fn analyze(source: &str) -> LanguageAnalysis {
        let analyzer = PythonAnalyzer::new();
        let file = SourceFile::new("test.py".into(), Language::Python, source.to_string());
        analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
    }

    #[test]
    fn empty_file_yields_root_unit() {
        let a = analyze("");
        assert_eq!(a.root.kind, SpaceKind::Unit);
        assert!(a.root.spaces.is_empty());
    }

    #[test]
    fn def_creates_function_space() {
        let a = analyze("def foo():\n    pass\n");
        assert_eq!(a.root.spaces.len(), 1);
        assert_eq!(a.root.spaces[0].kind, SpaceKind::Function);
        assert_eq!(a.root.spaces[0].name.as_deref(), Some("foo"));
    }

    #[test]
    fn class_creates_class_space_with_method() {
        let a = analyze("class C:\n    def m(self):\n        pass\n");
        assert_eq!(a.root.spaces.len(), 1);
        assert_eq!(a.root.spaces[0].kind, SpaceKind::Class);
        assert_eq!(a.root.spaces[0].name.as_deref(), Some("C"));
        assert_eq!(a.root.spaces[0].spaces.len(), 1);
        assert_eq!(a.root.spaces[0].spaces[0].kind, SpaceKind::Function);
    }

    #[test]
    fn cyclomatic_counts_decision_points() {
        // 1 (base) + if + elif + or = 4
        let a =
            analyze("def f(x):\n    if x or x:\n        return 1\n    elif x:\n        return 2\n");
        let func = &a.root.spaces[0];
        let cyclomatic = func
            .metrics
            .get(&MetricKey::new(keys::CYCLOMATIC))
            .unwrap()
            .as_f64();
        assert!(cyclomatic >= 4.0, "expected >= 4, got {cyclomatic}");
    }
}
