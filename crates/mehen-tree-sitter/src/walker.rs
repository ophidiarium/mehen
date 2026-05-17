//! Reusable walker scaffolding for tree-sitter-backed analyzers.
//!
//! Each language analyzer crate plugs its decision/operator/operand rules
//! into this walker; the shared bookkeeping (`MetricTreeBuilder` updates,
//! per-space accumulator stack, LOC line classification at unit and span
//! level, Halstead event emission, MI/Halstead/LOC publishing into the
//! `MetricSet`) lives here so language crates focus on their own syntax.
//!
//! The walker is intentionally minimal: it does not implement cognitive
//! nesting state machines or context-sensitive Halstead classification.
//! Those live in the owning language crate and are wired in via the
//! [`LanguageRules`] trait.

use mehen_core::{LineIndex, MetricKey, MetricSet, MetricSpace, SourceSpan, SpaceId, SpaceKind};
use mehen_metrics::{
    AbcStats, CognitiveStats, CyclomaticStats, HalsteadBuilder, HalsteadStats, LineClass, LocStats,
    MetricTreeBuilder, MiStats, NargsStats, NexitStats, NomStats, NpaStats, NpmStats, WmcStats,
    keys,
};
use tree_sitter::Node;

use crate::span::node_span;

/// Per-space accumulator state. The walker pushes one of these for the
/// `Unit` root and for every space the language rules open via
/// [`LanguageRules::scope_for`].
#[derive(Default)]
pub struct State {
    pub loc: LocStats,
    pub cyclomatic: CyclomaticStats,
    pub cognitive: CognitiveStats,
    pub halstead: HalsteadBuilder,
    pub abc: AbcStats,
    pub nargs: NargsStats,
    pub nom: NomStats,
    pub nexit: NexitStats,
    pub npa: NpaStats,
    pub npm: NpmStats,
    pub wmc: WmcStats,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }
}

/// What a language reports about an AST node.
///
/// This is the "language interpretation" surface from the rewrite plan
/// §5.2: each language crate decides which constructs are decisions,
/// operators, operands, exits, etc. The shared walker accumulates them.
#[derive(Default, Clone, Copy, Debug)]
pub struct NodeFacts {
    /// Counts toward cyclomatic complexity (`if`, `for`, `&&`, …).
    pub cyclomatic_decision: bool,
    /// Counts toward cognitive complexity. Phase 1 demo: same set as
    /// cyclomatic; full nesting/binary-sequence rules are language-owned
    /// and land per language.
    pub cognitive_increment: u32,
    /// Halstead operator with the node kind as its key.
    pub halstead_operator: bool,
    /// Halstead operand with the node text as its dedup key.
    pub halstead_operand: bool,
    /// Counts toward NExit (`return`, `throw`, `raise`, …).
    pub nexit: bool,
    /// Counts toward ABC's `B` (branches: function calls, `goto`, …).
    pub abc_branch: bool,
    /// Counts toward ABC's `C` (conditionals: comparisons, boolean ops).
    pub abc_condition: bool,
    /// Counts toward ABC's `A` (assignments).
    pub abc_assignment: bool,
}

/// What kind of scope a node opens, if any. `None` means the node is not
/// itself a space boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeOpen {
    /// Open a space with the given kind. `name` may be `None` if the
    /// language can't extract one cheaply.
    Open {
        kind: SpaceKind,
        name: Option<String>,
    },
}

/// Trait implemented by each language analyzer to plug into the shared
/// walker.
pub trait LanguageRules {
    /// Returns the node kind names that mark a space boundary, alongside
    /// the kind to open. The walker consults this on the way down.
    fn scope_for(&self, node: &Node<'_>, source: &[u8]) -> Option<ScopeOpen>;

    /// Classify a single AST node for shared metrics.
    fn classify(&self, node: &Node<'_>) -> NodeFacts;

    /// Optional: classify a single physical line for LOC. The default
    /// implementation calls [`default_line_classifier`] which treats
    /// `#`-prefixed and `//`-prefixed lines as comments. Language crates
    /// that need richer behaviour (`/* */` blocks, heredocs, docstrings)
    /// override this.
    fn classify_line(&self, line: &str) -> LineClass {
        default_line_classifier(line)
    }
}

/// Default LOC line classifier: blank lines, lines starting with `#` or
/// `//` are comments, anything else is code.
pub fn default_line_classifier(line: &str) -> LineClass {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        LineClass::Blank
    } else if trimmed.starts_with('#') || trimmed.starts_with("//") {
        LineClass::Comment
    } else {
        LineClass::Code
    }
}

/// The result of running [`walk`] on a tree.
pub struct WalkResult {
    pub root: MetricSpace,
}

/// Walk `root_node` over `source_text` using `rules` and produce a
/// populated `MetricSpace` tree.
///
/// Generic over the rules so monomorphization gives each language its own
/// fast path. The walker:
/// 1. opens a Unit space at the root,
/// 2. on every node, asks `rules.scope_for(...)`; if it opens a scope it
///    pushes a fresh `State`,
/// 3. on every node, asks `rules.classify(...)` and accumulates the
///    facts into the current `State`,
/// 4. classifies every physical line of the source for the unit-level
///    `loc`, and every line covered by an opened scope for that scope's
///    `loc`,
/// 5. on close, publishes the per-space metric set via the shared
///    [`apply_state_to`] helper, then folds it back into the parent.
pub fn walk<R: LanguageRules>(
    root_node: Node<'_>,
    source_text: &[u8],
    line_index: &LineIndex,
    rules: &R,
) -> WalkResult {
    let unit_span = node_span(&root_node, line_index);
    let mut walker = Walker {
        tree: MetricTreeBuilder::new(unit_span),
        source_text,
        line_index,
        stack: vec![State::new()],
        rules,
    };
    walker.visit(root_node);
    let mut unit_state = walker.stack.pop().expect("walker stack underflow");
    classify_unit_loc(walker.source_text, rules, &mut unit_state.loc);
    apply_state_to(unit_state, walker.tree.metrics_mut());
    WalkResult {
        root: walker.tree.finish(),
    }
}

/// Publish a finalized `State` into a `MetricSet` using the shared key
/// names. Called by the walker on every space close.
pub fn apply_state_to(state: State, target: &mut MetricSet) {
    // McCabe convention: V(G) = decisions + 1. The accumulator stores
    // the raw decision count so the +1 is added at publish time, where
    // the conversion to a typed `MetricValue` happens.
    let cyclomatic = state.cyclomatic.cyclomatic.saturating_add(1);
    target.insert(MetricKey::new(keys::CYCLOMATIC), cyclomatic as i64);
    target.insert(
        MetricKey::new(keys::COGNITIVE),
        state.cognitive.cognitive as i64,
    );

    target.insert(MetricKey::new(keys::LOC_LLOC), state.loc.lloc as i64);
    target.insert(MetricKey::new(keys::LOC_SLOC), state.loc.sloc as i64);
    target.insert(MetricKey::new(keys::LOC_PLOC), state.loc.ploc as i64);
    target.insert(MetricKey::new(keys::LOC_CLOC), state.loc.cloc as i64);
    target.insert(MetricKey::new(keys::LOC_BLANK), state.loc.blank as i64);
    target.insert(MetricKey::new(keys::LOC), state.loc.total as i64);

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

    let mi = MiStats::compute(&state.loc, &state.cyclomatic, &halstead);
    target.insert(MetricKey::new(keys::MI_VS), mi.mi_visual_studio);
    target.insert(MetricKey::new(keys::MI_ORIGINAL), mi.mi_original);
    target.insert(MetricKey::new(keys::MI_SEI), mi.mi_sei);

    target.insert(MetricKey::new(keys::ABC), state.abc.magnitude());
    target.insert(
        MetricKey::new(keys::NARGS),
        (state.nargs.functions + state.nargs.closures) as i64,
    );
    target.insert(MetricKey::new(keys::NOM), state.nom.total() as i64);
    target.insert(MetricKey::new(keys::NEXIT), state.nexit.exits as i64);
    target.insert(MetricKey::new(keys::NPA), state.npa.public as i64);
    target.insert(MetricKey::new(keys::NPM), state.npm.public as i64);
    target.insert(MetricKey::new(keys::WMC), state.wmc.wmc as i64);
}

struct Walker<'a, R: LanguageRules> {
    tree: MetricTreeBuilder,
    source_text: &'a [u8],
    line_index: &'a LineIndex,
    stack: Vec<State>,
    rules: &'a R,
}

impl<'a, R: LanguageRules> Walker<'a, R> {
    fn current(&mut self) -> &mut State {
        self.stack.last_mut().expect("walker stack empty")
    }

    fn visit(&mut self, node: Node<'_>) {
        let opened_space = match self.rules.scope_for(&node, self.source_text) {
            Some(ScopeOpen::Open { kind, name }) => {
                let span = node_span(&node, self.line_index);
                self.tree.open(kind, span, name);
                self.stack.push(State::new());
                true
            }
            None => false,
        };

        let facts = self.rules.classify(&node);
        if facts.cyclomatic_decision {
            self.current().cyclomatic.record_decision();
        }
        if facts.cognitive_increment > 0 {
            self.current()
                .cognitive
                .record_increment(facts.cognitive_increment);
        }
        if facts.halstead_operator {
            let kind = node.kind();
            self.current()
                .halstead
                .observe_operator(mehen_metrics::HalsteadOperator {
                    kind: kind.into(),
                    text: None,
                });
        }
        if facts.halstead_operand {
            let kind = node.kind();
            let text = crate::span::text_of(&node, self.source_text);
            self.current()
                .halstead
                .observe_operand(mehen_metrics::HalsteadOperand {
                    kind: kind.into(),
                    text: Some(text.into()),
                });
        }
        if facts.nexit {
            self.current().nexit.record_exit();
        }
        if facts.abc_branch {
            self.current().abc.record_branch();
        }
        if facts.abc_condition {
            self.current().abc.record_condition();
        }
        if facts.abc_assignment {
            self.current().abc.record_assignment();
        }

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.visit(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        if opened_space {
            let mut state = self.stack.pop().expect("walker stack underflow on close");
            classify_span_loc(self.source_text, &node, self.rules, &mut state.loc);
            apply_state_to(state, self.tree.metrics_mut());
            self.tree.close();
        }
    }
}

fn classify_unit_loc<R: LanguageRules>(source: &[u8], rules: &R, loc: &mut LocStats) {
    let Ok(text) = core::str::from_utf8(source) else {
        return;
    };
    for line in text.lines() {
        loc.observe(rules.classify_line(line));
    }
}

fn classify_span_loc<R: LanguageRules>(
    source: &[u8],
    node: &Node<'_>,
    rules: &R,
    loc: &mut LocStats,
) {
    let start = node.start_byte().min(source.len());
    let end = node.end_byte().min(source.len());
    if start >= end {
        return;
    }
    let Ok(slice) = core::str::from_utf8(&source[start..end]) else {
        return;
    };
    for line in slice.lines() {
        loc.observe(rules.classify_line(line));
    }
}

/// Convenience: build an "empty" space (used by analyzers when the parser
/// fails before any walk can happen).
pub fn empty_space(span: SourceSpan) -> MetricSpace {
    MetricSpace::new(SpaceId(0), SpaceKind::Unit, span)
}
