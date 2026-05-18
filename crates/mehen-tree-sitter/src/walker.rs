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
    AbcStats, CognitiveStats, CyclomaticStats, HalsteadBuilder, HalsteadStats, LocStats,
    MetricTreeBuilder, MiStats, NargsStats, NexitStats, NomStats, NpaStats, NpmStats, WmcStats,
    keys,
};
use tree_sitter::Node;

use crate::span::node_span;

/// Per-space accumulator state. The walker pushes one of these for the
/// `Unit` root and for every space the language rules open via
/// [`LanguageRules::scope_for`].
#[derive(Default, Clone)]
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
    /// LOC classification of this node. See [`LocFact`].
    pub loc: LocFact,
}

/// LOC family classification: how this node contributes to the LOC
/// suite (PLOC / LLOC / CLOC). Mirrors the pre-1.0 per-language
/// `Loc::compute` match arms (`src/metrics/loc.rs`).
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocFact {
    /// Node is a container (statement list, block, parameter list,
    /// string interior, …) and must NOT contribute to PLOC. The walker
    /// ignores this node for LOC purposes but still recurses into it.
    Container,
    /// Node is a statement-shaped construct that bumps LLOC by one.
    /// Statement classification is per-language (e.g. `pipeline`,
    /// `if_statement`, `function_statement` for PowerShell;
    /// `expression_statement`, `function_definition` for PHP).
    Lloc,
    /// Node is a comment. The walker uses `node.start_row()` /
    /// `node.end_row()` to update CLOC (distinguishing comment-on-code
    /// vs. independent-line comments per the legacy algorithm).
    Comment,
    /// Default: any other node — its `start_row()` is added to the
    /// PLOC line set.
    #[default]
    Code,
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
    // The unit space's LOC span covers the full source.
    walker.stack[0].loc.set_span(
        root_node.start_position().row as u32,
        root_node.end_position().row as u32,
        true,
    );
    walker.visit(root_node);
    let mut unit_state = walker.stack.pop().expect("walker stack underflow");
    finalize_state(&mut unit_state);
    apply_state_to(unit_state, walker.tree.metrics_mut());
    WalkResult {
        root: walker.tree.finish(),
    }
}

/// Snapshot the per-space "current" values into rolled-up
/// sum/min/max/avg fields. Called on every space close before the
/// per-space MetricSet is published or merged into the parent.
fn finalize_state(state: &mut State) {
    state.cyclomatic.finalize_minmax();
    state.cyclomatic.finalize_average();
    state.loc.finalize_minmax();
    state.nom.finalize_minmax();
    state.nexit.finalize_minmax();
    state.nexit.finalize_average(state.nom.total());
}

/// Fold a finalized child state's rolled-up totals (sum/min/max/n)
/// into the parent state. The parent's per-space "current" values are
/// not affected — children contribute only via the bounds.
fn merge_child_into_parent(parent: &mut State, child: &State) {
    parent.cyclomatic.merge(&child.cyclomatic);
    parent.cyclomatic.finalize_average();
    parent.loc.merge(&child.loc);
    parent.nom.merge(&child.nom);
    parent.nexit.merge(&child.nexit);
    parent.nexit.finalize_average(parent.nom.total());
}

/// Publish a finalized `State` into a `MetricSet` using the shared key
/// names. Called by the walker on every space close, *after*
/// `state.cyclomatic.finalize_minmax()` has snapshotted the McCabe
/// value into `cyclomatic_sum`/`min`/`max`/`n`.
///
/// Per the rewrite plan §5.1 each metric publishes the rolled-up
/// `{ sum, min, max, average }` set under aggregator-suffixed selectors
/// (`cyclomatic.sum`, `cyclomatic.min`, …) plus the bare per-space
/// value at the metric's root key. The selector format defined in
/// `mehen-core::selector` already understands those suffixes.
pub fn apply_state_to(state: State, target: &mut MetricSet) {
    publish_cyclomatic(&state.cyclomatic, target);
    publish_loc(&state.loc, target);
    publish_nom(&state.nom, target);
    publish_nexit(&state.nexit, target);

    target.insert(
        MetricKey::new(keys::COGNITIVE),
        state.cognitive.cognitive as i64,
    );

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
    target.insert(MetricKey::new(keys::NPA), state.npa.public as i64);
    target.insert(MetricKey::new(keys::NPM), state.npm.public as i64);
    target.insert(MetricKey::new(keys::WMC), state.wmc.wmc as i64);
}

fn publish_nom(stats: &mehen_metrics::NomStats, target: &mut MetricSet) {
    // Legacy `metric.nom` JSON: { functions, closures, functions_average,
    //   closures_average, total, average,
    //   functions_min, functions_max, closures_min, closures_max }.
    // The flat MetricSet maps each to a dotted selector key; the per-
    // metric renderer in `mehen-report` reassembles them into the
    // family object.
    target.insert(MetricKey::new(keys::NOM), stats.total() as i64);
    target.insert(
        MetricKey::new(format!("{}.functions", keys::NOM)),
        stats.functions_sum as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.closures", keys::NOM)),
        stats.closures_sum as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.functions_average", keys::NOM)),
        stats.functions_average(),
    );
    target.insert(
        MetricKey::new(format!("{}.closures_average", keys::NOM)),
        stats.closures_average(),
    );
    target.insert(
        MetricKey::new(format!("{}.average", keys::NOM)),
        stats.average(),
    );
    target.insert(
        MetricKey::new(format!("{}.functions_min", keys::NOM)),
        stats.functions_min as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.functions_max", keys::NOM)),
        stats.functions_max as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.closures_min", keys::NOM)),
        stats.closures_min as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.closures_max", keys::NOM)),
        stats.closures_max as i64,
    );
}

fn publish_nexit(stats: &mehen_metrics::NexitStats, target: &mut MetricSet) {
    // Legacy `metric.nexits` JSON: { sum, average, min, max }.
    target.insert(MetricKey::new(keys::NEXIT), stats.exits as i64);
    target.insert(
        MetricKey::new(format!("{}.sum", keys::NEXIT)),
        stats.sum as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.average", keys::NEXIT)),
        stats.average,
    );
    target.insert(
        MetricKey::new(format!("{}.min", keys::NEXIT)),
        stats.min as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.max", keys::NEXIT)),
        stats.max as i64,
    );
}

fn publish_cyclomatic(stats: &mehen_metrics::CyclomaticStats, target: &mut MetricSet) {
    // Per-space McCabe value at the bare key.
    let mccabe = stats.cyclomatic.saturating_add(1) as i64;
    target.insert(MetricKey::new(keys::CYCLOMATIC), mccabe);
    target.insert(
        MetricKey::new(format!("{}.sum", keys::CYCLOMATIC)),
        stats.cyclomatic_sum as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.min", keys::CYCLOMATIC)),
        stats.min as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.max", keys::CYCLOMATIC)),
        stats.max as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.avg", keys::CYCLOMATIC)),
        stats.cyclomatic_average,
    );
}

fn publish_loc(stats: &LocStats, target: &mut MetricSet) {
    // The bare keys carry the rolled-up sums per the legacy `loc` JSON
    // shape (`sloc`, `ploc`, … are the rolled-up totals across all
    // folded spaces). Per-aggregator selectors (`loc.sloc.min`, etc.)
    // hang off the same family.
    target.insert(MetricKey::new(keys::LOC_LLOC), stats.lloc() as i64);
    target.insert(MetricKey::new(keys::LOC_SLOC), stats.sloc() as i64);
    target.insert(MetricKey::new(keys::LOC_PLOC), stats.ploc() as i64);
    target.insert(MetricKey::new(keys::LOC_CLOC), stats.cloc() as i64);
    target.insert(MetricKey::new(keys::LOC_BLANK), stats.blank() as i64);
    target.insert(MetricKey::new(keys::LOC), stats.sloc() as i64);

    target.insert(
        MetricKey::new(format!("{}.min", keys::LOC_SLOC)),
        stats.sloc_min as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.max", keys::LOC_SLOC)),
        stats.sloc_max as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.avg", keys::LOC_SLOC)),
        stats.sloc_average(),
    );
    target.insert(
        MetricKey::new(format!("{}.min", keys::LOC_PLOC)),
        stats.ploc_min as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.max", keys::LOC_PLOC)),
        stats.ploc_max as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.avg", keys::LOC_PLOC)),
        stats.ploc_average(),
    );
    target.insert(
        MetricKey::new(format!("{}.min", keys::LOC_LLOC)),
        stats.lloc_min as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.max", keys::LOC_LLOC)),
        stats.lloc_max as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.avg", keys::LOC_LLOC)),
        stats.lloc_average(),
    );
    target.insert(
        MetricKey::new(format!("{}.min", keys::LOC_CLOC)),
        stats.cloc_min as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.max", keys::LOC_CLOC)),
        stats.cloc_max as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.avg", keys::LOC_CLOC)),
        stats.cloc_average(),
    );
    target.insert(
        MetricKey::new(format!("{}.min", keys::LOC_BLANK)),
        stats.blank_min as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.max", keys::LOC_BLANK)),
        stats.blank_max as i64,
    );
    target.insert(
        MetricKey::new(format!("{}.avg", keys::LOC_BLANK)),
        stats.blank_average(),
    );
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
                // Bump NOM on the *parent* — this child space contributes
                // to its parent's function/closure count. Mirrors the
                // pre-1.0 `Nom::compute` which incremented on `is_func` /
                // `is_closure` nodes encountered during the parent's walk.
                match kind {
                    SpaceKind::Function => self.current().nom.record_function(),
                    SpaceKind::Closure => self.current().nom.record_closure(),
                    _ => {}
                }
                let span = node_span(&node, self.line_index);
                self.tree.open(kind, span, name);
                let mut child_state = State::new();
                // The child space's LOC span is the AST node's row range.
                // Non-unit spaces use the `+1` convention (counts the
                // function-signature line as part of the space).
                child_state.loc.set_span(
                    node.start_position().row as u32,
                    node.end_position().row as u32,
                    false,
                );
                self.stack.push(child_state);
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
        // LOC: each AST node contributes per-language to PLOC / LLOC /
        // CLOC. The walker stays language-agnostic — it forwards the
        // language's `LocFact` decision into the per-space accumulator.
        let start_row = node.start_position().row as u32;
        let end_row = node.end_position().row as u32;
        match facts.loc {
            LocFact::Container => {}
            LocFact::Lloc => {
                self.current().loc.observe_lloc();
            }
            LocFact::Comment => {
                self.current().loc.observe_comment(start_row, end_row);
            }
            LocFact::Code => {
                self.current().loc.observe_code_line(start_row);
            }
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
            finalize_state(&mut state);
            apply_state_to_for_close(&state, self.tree.metrics_mut());
            // Fold this space's rolled-up bounds into the parent so the
            // unit's final stats reflect every nested space.
            if let Some(parent) = self.stack.last_mut() {
                merge_child_into_parent(parent, &state);
            }
            self.tree.close();
        }
    }
}

/// Variant of [`apply_state_to`] that takes a borrow — used at space
/// close where the state must also be merged into the parent. The
/// freestanding `apply_state_to(state, target)` continues to consume
/// its argument so external callers (the walker's unit close path)
/// don't pay for an extra clone.
fn apply_state_to_for_close(state: &State, target: &mut MetricSet) {
    apply_state_to(state.clone(), target);
}

/// Convenience: build an "empty" space (used by analyzers when the parser
/// fails before any walk can happen).
pub fn empty_space(span: SourceSpan) -> MetricSpace {
    MetricSpace::new(SpaceId(0), SpaceKind::Unit, span)
}
