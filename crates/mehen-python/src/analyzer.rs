use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, MetricKey,
    Result, SourceFile, SourceSpan, SpaceKind, byte_offset_clamped,
};
use mehen_metrics::{
    CognitiveStats, CyclomaticStats, HalsteadBuilder, HalsteadOperand, HalsteadOperator,
    HalsteadStats, LineClass, LocStats, MetricTreeBuilder, MiStats, keys,
};
use mehen_tree_sitter::{TreeSitterParser, node_span, text_of};
use smol_str::SmolStr;
use tree_sitter::Node;

/// Tree-sitter-backed Python analyzer.
///
/// Phase 3 reference implementation: walks the tree-sitter Python AST,
/// classifies LOC, computes cyclomatic complexity (per the pre-1.0
/// `src/metrics/cyclomatic.rs::Cyclomatic for PythonCode` rules), assembles
/// a `MetricSpace` tree per Python `function_definition` and
/// `class_definition`, and publishes the results via `MetricSet` keyed by
/// the shared `keys::*` namespace.
///
/// Phase 6 replaces the tree-sitter backend with Ruff parser + semantic.
/// The replacement keeps the same public interface — `analyze` returns
/// `LanguageAnalysis` — so the engine wiring does not change.
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
                let mut report = empty_analysis(source);
                report.diagnostics.push(mehen_core::ParseDiagnostic::fatal(
                    "python.parse_error",
                    format!("tree-sitter-python failed: {e}"),
                ));
                return Ok(report);
            }
        };

        let root = parser.root();
        let unit_span = node_span(&root, &source.line_index);

        let mut walker = Walker {
            tree: MetricTreeBuilder::new(unit_span),
            source_text: parser.source(),
            line_index: &source.line_index,
            stack: Vec::new(),
        };
        walker.push_state();
        walker.visit(root);
        let mut unit_state = walker.pop_state();
        // LOC classification is computed once over the whole source for
        // the unit-level state; nested spaces fill in `total` only from
        // their span. The pre-1.0 implementation does richer per-space
        // line accounting that Phase 3+ will port over.
        classify_unit_loc(&source.text, &mut unit_state.loc);
        // Top-level metric set comes from the unit-level state we just
        // popped. Any spaces opened during the walk are already attached
        // to the root unit by the tree builder.
        apply_state_to(unit_state, walker.tree.metrics_mut());
        let root_space = walker.tree.finish();

        Ok(LanguageAnalysis {
            language: Language::Python,
            backend: AnalysisBackend::TreeSitter,
            diagnostics: Vec::new(),
            root: root_space,
            contributions: Vec::new(),
        })
    }
}

fn empty_analysis(source: &SourceFile) -> LanguageAnalysis {
    let span = SourceSpan {
        start_byte: 0,
        end_byte: byte_offset_clamped(source.text.len()),
        start_line: 1,
        end_line: source.line_index.line_count(),
    };
    LanguageAnalysis {
        language: Language::Python,
        backend: AnalysisBackend::TreeSitter,
        diagnostics: Vec::new(),
        root: mehen_core::MetricSpace::new(mehen_core::SpaceId(0), SpaceKind::Unit, span),
        contributions: Vec::new(),
    }
}

/// Per-space accumulator state. Mirrors the structure the engine builds
/// for every `MetricSpace`; `Walker` keeps a stack of these and pops them
/// at the matching `function_definition` / `class_definition` boundary.
struct State {
    loc: LocStats,
    cyclomatic: CyclomaticStats,
    cognitive: CognitiveStats,
    halstead: HalsteadBuilder,
}

impl State {
    fn new() -> Self {
        Self {
            loc: LocStats::default(),
            cyclomatic: CyclomaticStats::default(),
            cognitive: CognitiveStats::default(),
            halstead: HalsteadBuilder::new(),
        }
    }
}

struct Walker<'a> {
    tree: MetricTreeBuilder,
    source_text: &'a [u8],
    line_index: &'a mehen_core::LineIndex,
    stack: Vec<State>,
}

impl<'a> Walker<'a> {
    fn push_state(&mut self) {
        self.stack.push(State::new());
    }

    fn pop_state(&mut self) -> State {
        self.stack.pop().expect("Walker: stack underflow")
    }

    fn current_state(&mut self) -> &mut State {
        self.stack.last_mut().expect("Walker: empty stack")
    }

    fn visit(&mut self, node: Node<'_>) {
        let kind = node.kind();

        // Open a new space for each definition.
        let opened_space = match kind {
            "function_definition" => {
                let name = function_name(&node, self.source_text);
                let span = node_span(&node, self.line_index);
                self.tree.open(SpaceKind::Function, span, name);
                self.push_state();
                true
            }
            "class_definition" => {
                let name = class_name(&node, self.source_text);
                let span = node_span(&node, self.line_index);
                self.tree.open(SpaceKind::Class, span, name);
                self.push_state();
                true
            }
            "lambda" => {
                let span = node_span(&node, self.line_index);
                self.tree.open(SpaceKind::Closure, span, None);
                self.push_state();
                true
            }
            _ => false,
        };

        // Record cyclomatic decisions at every node — Python rules from
        // pre-1.0 `Cyclomatic for PythonCode`.
        if is_python_decision(&node) {
            self.current_state().cyclomatic.record_decision();
            // Phase 1 cognitive uses the cyclomatic decision set as a
            // first approximation. The pre-1.0 cognitive rules add
            // nesting penalties and binary-sequence handling that
            // Phase 3+ will port over from `Cognitive for PythonCode`.
            self.current_state().cognitive.record_increment(1);
        }

        // Halstead operator/operand events. The classification here is
        // intentionally minimal for the Phase 1 demo; Phase 3+ refines
        // the rules (docstring detection, normalized numeric literals,
        // f-string handling, …).
        if is_python_operator(kind) {
            self.current_state()
                .halstead
                .observe_operator(HalsteadOperator {
                    kind: SmolStr::new(kind),
                    text: None,
                });
        }
        if is_python_operand(kind) {
            let text = text_of(&node, self.source_text);
            self.current_state()
                .halstead
                .observe_operand(HalsteadOperand {
                    kind: SmolStr::new(kind),
                    text: Some(SmolStr::new(text)),
                });
        }

        // Visit children.
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                self.visit(child);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        // Finalize the opened space, if any.
        if opened_space {
            let mut state = self.pop_state();
            // Per-space LOC: classify the lines covered by this space's
            // span. The pre-1.0 analyzer does richer accounting (visible
            // statement detection, comment-on-code-line handling); the
            // Phase 1 demo classifies lines as Code/Comment/Blank from
            // the source text alone.
            classify_span_loc(self.source_text, &node, &mut state.loc);
            // Emit metrics into the just-finished space's MetricSet, then
            // close it so it attaches to the parent.
            apply_state_to(state, self.tree.metrics_mut());
            self.tree.close();
        }
    }
}

/// Classify every line of `source` and feed the result into `loc`.
fn classify_unit_loc(source: &str, loc: &mut LocStats) {
    for line in source.lines() {
        loc.observe(classify_line(line));
    }
}

/// Classify the lines covered by `node`'s byte range.
fn classify_span_loc(source: &[u8], node: &Node<'_>, loc: &mut LocStats) {
    let start = node.start_byte().min(source.len());
    let end = node.end_byte().min(source.len());
    if start >= end {
        return;
    }
    let slice = match core::str::from_utf8(&source[start..end]) {
        Ok(s) => s,
        Err(_) => return,
    };
    for line in slice.lines() {
        loc.observe(classify_line(line));
    }
}

fn classify_line(line: &str) -> LineClass {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        LineClass::Blank
    } else if trimmed.starts_with('#') {
        LineClass::Comment
    } else {
        LineClass::Code
    }
}

fn apply_state_to(state: State, target: &mut mehen_core::MetricSet) {
    // Cyclomatic.
    target.insert(
        MetricKey::new(keys::CYCLOMATIC),
        state.cyclomatic.cyclomatic.max(1) as i64,
    );
    // Cognitive.
    target.insert(
        MetricKey::new(keys::COGNITIVE),
        state.cognitive.cognitive as i64,
    );
    // LOC family.
    target.insert(MetricKey::new(keys::LOC_LLOC), state.loc.lloc as i64);
    target.insert(MetricKey::new(keys::LOC_SLOC), state.loc.sloc as i64);
    target.insert(MetricKey::new(keys::LOC_PLOC), state.loc.ploc as i64);
    target.insert(MetricKey::new(keys::LOC_CLOC), state.loc.cloc as i64);
    target.insert(MetricKey::new(keys::LOC_BLANK), state.loc.blank as i64);
    target.insert(MetricKey::new(keys::LOC), state.loc.total as i64);
    // Halstead.
    let halstead = HalsteadStats::from_counts(state.halstead.counts());
    target.insert(MetricKey::new(keys::HALSTEAD_VOLUME), halstead.volume());
    target.insert(
        MetricKey::new(keys::HALSTEAD_DIFFICULTY),
        halstead.difficulty(),
    );
    target.insert(MetricKey::new(keys::HALSTEAD_EFFORT), halstead.effort());
    target.insert(
        MetricKey::new(keys::HALSTEAD_VOCABULARY),
        halstead.vocabulary(),
    );
    target.insert(MetricKey::new(keys::HALSTEAD_LENGTH), halstead.length());
    // Maintainability index.
    let mi = MiStats::compute(&state.loc, &state.cyclomatic, &halstead);
    target.insert(MetricKey::new(keys::MI_VS), mi.mi_visual_studio);
    target.insert(MetricKey::new(keys::MI_ORIGINAL), mi.mi_original);
    target.insert(MetricKey::new(keys::MI_SEI), mi.mi_sei);
}

fn function_name(node: &Node<'_>, source: &[u8]) -> Option<String> {
    let name_node = node.child_by_field_name("name")?;
    Some(text_of(&name_node, source).to_string())
}

fn class_name(node: &Node<'_>, source: &[u8]) -> Option<String> {
    let name_node = node.child_by_field_name("name")?;
    Some(text_of(&name_node, source).to_string())
}

/// Python cyclomatic decision points, mirroring pre-1.0
/// `Cyclomatic for PythonCode` (`src/metrics/cyclomatic.rs:117-135`).
fn is_python_decision(node: &Node<'_>) -> bool {
    matches!(
        node.kind(),
        "if_statement"
            | "elif_clause"
            | "for_statement"
            | "while_statement"
            | "except_clause"
            | "and"
            | "or"
            | "boolean_operator"
            | "conditional_expression"
    )
}

fn is_python_operator(kind: &str) -> bool {
    matches!(
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
    )
}

fn is_python_operand(kind: &str) -> bool {
    matches!(
        kind,
        "identifier" | "integer" | "float" | "string" | "true" | "false" | "none"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
