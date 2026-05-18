use std::collections::HashMap;

use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::legacy::checker::Checker;
use crate::legacy::langs::{
    CCode, GoCode, KotlinCode, PythonCode, PythonParser, RubyCode, RustCode,
};
use crate::legacy::languages::{C, Kotlin, Python, Ruby, Rust};
use crate::legacy::node::Node;
use crate::legacy::rust_metric_helpers::is_inside_rust_macro_tokens;

// TODO: Find a way to increment the cognitive complexity value
// for recursive code. For some kind of languages, such as C++, it is pretty
// hard to detect, just parsing the code, if a determined function is recursive
// because the call graph of a function is solved at runtime.
// So a possible solution could be searching for a crate which implements
// a light language interpreter, computing the call graph, and then detecting
// if there are cycles. At this point, it is possible to figure out if a
// function is recursive or not.

/// The `Cognitive Complexity` metric.
#[derive(Debug, Clone)]
pub(crate) struct Stats {
    structural: usize,
    structural_sum: usize,
    structural_min: usize,
    structural_max: usize,
    nesting: usize,
    total_space_functions: usize,
    boolean_seq: BoolSequence,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            structural: 0,
            structural_sum: 0,
            structural_min: usize::MAX,
            structural_max: 0,
            nesting: 0,
            total_space_functions: 1,
            boolean_seq: BoolSequence::default(),
        }
    }
}

impl Serialize for Stats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("cognitive", 4)?;
        st.serialize_field("sum", &self.cognitive_sum())?;
        st.serialize_field("average", &self.cognitive_average())?;
        st.serialize_field("min", &self.cognitive_min())?;
        st.serialize_field("max", &self.cognitive_max())?;
        st.end()
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "sum: {}, average: {}, min:{}, max: {}",
            self.cognitive(),
            self.cognitive_average(),
            self.cognitive_min(),
            self.cognitive_max()
        )
    }
}

impl Stats {
    /// Merges a second `Cognitive Complexity` metric into the first one
    pub(crate) fn merge(&mut self, other: &Self) {
        self.structural_min = self.structural_min.min(other.structural_min);
        self.structural_max = self.structural_max.max(other.structural_max);
        self.structural_sum += other.structural_sum;
    }

    /// Returns the `Cognitive Complexity` metric value
    pub(crate) fn cognitive(&self) -> f64 {
        self.structural as f64
    }
    /// Returns the `Cognitive Complexity` sum metric value
    pub(crate) fn cognitive_sum(&self) -> f64 {
        self.structural_sum as f64
    }

    /// Returns the `Cognitive Complexity` minimum metric value
    pub(crate) fn cognitive_min(&self) -> f64 {
        self.structural_min as f64
    }
    /// Returns the `Cognitive Complexity` maximum metric value
    pub(crate) fn cognitive_max(&self) -> f64 {
        self.structural_max as f64
    }

    /// Returns the `Cognitive Complexity` metric average value
    ///
    /// This value is computed dividing the `Cognitive Complexity` value
    /// for the total number of functions/closures in a space.
    ///
    /// If there are no functions in a code, its value is `NAN`.
    pub(crate) fn cognitive_average(&self) -> f64 {
        self.cognitive_sum() / self.total_space_functions as f64
    }
    #[inline(always)]
    pub(crate) fn compute_sum(&mut self) {
        self.structural_sum += self.structural;
    }
    #[inline(always)]
    pub(crate) fn compute_minmax(&mut self) {
        self.structural_min = self.structural_min.min(self.structural);
        self.structural_max = self.structural_max.max(self.structural);
        self.compute_sum();
    }

    pub(crate) fn finalize(&mut self, total_space_functions: usize) {
        self.total_space_functions = total_space_functions;
    }
}

pub(crate) trait Cognitive
where
    Self: Checker,
{
    fn compute(
        node: &Node,
        stats: &mut Stats,
        nesting_map: &mut HashMap<usize, (usize, usize, usize)>,
    );
}

fn compute_booleans<T: PartialEq + From<u16>>(
    node: &Node,
    stats: &mut Stats,
    typs1: &T,
    typs2: &T,
) {
    for child in node.children() {
        if *typs1 == child.kind_id().into() || *typs2 == child.kind_id().into() {
            stats.structural = stats
                .boolean_seq
                .eval_based_on_prev(child.kind_id(), stats.structural);
        }
    }
}

/// Same-operator sequence collapser for nodes that may expose more than two
/// boolean operators (e.g. PowerShell's `logical_expression` carrying
/// `-and` / `-or` / `-xor`). Each direct child whose kind matches any
/// element of `typs` feeds the `BoolSequence` tracker, so mixed sequences
/// still get +1 per transition and same-operator runs collapse to +1.
fn compute_booleans_in<T: PartialEq + From<u16>>(node: &Node, stats: &mut Stats, typs: &[T]) {
    for child in node.children() {
        let child_kind: T = child.kind_id().into();
        if typs.contains(&child_kind) {
            stats.structural = stats
                .boolean_seq
                .eval_based_on_prev(child.kind_id(), stats.structural);
        }
    }
}

#[derive(Debug, Default, Clone)]
struct BoolSequence {
    boolean_op: Option<u16>,
}

impl BoolSequence {
    fn reset(&mut self) {
        self.boolean_op = None;
    }

    fn not_operator(&mut self, not_id: u16) {
        self.boolean_op = Some(not_id);
    }

    fn eval_based_on_prev(&mut self, bool_id: u16, structural: usize) -> usize {
        if let Some(prev) = self.boolean_op {
            if prev != bool_id {
                // The boolean operator is different from the previous one, so
                // the counter is incremented.
                structural + 1
            } else {
                // The boolean operator is equal to the previous one, so
                // the counter is not incremented.
                structural
            }
        } else {
            // Save the first boolean operator in a sequence of
            // logical operators and increment the counter.
            self.boolean_op = Some(bool_id);
            structural + 1
        }
    }
}

#[inline(always)]
fn increment(stats: &mut Stats) {
    stats.structural += stats.nesting + 1;
}

#[inline(always)]
fn increment_by_one(stats: &mut Stats) {
    stats.structural += 1;
}

fn get_nesting_from_map(
    node: &Node,
    nesting_map: &HashMap<usize, (usize, usize, usize)>,
) -> (usize, usize, usize) {
    if let Some(parent) = node.parent() {
        if let Some(n) = nesting_map.get(&parent.id()) {
            *n
        } else {
            (0, 0, 0)
        }
    } else {
        (0, 0, 0)
    }
}

fn increment_function_depth<T: PartialEq + From<u16>>(depth: &mut usize, node: &Node, stop: &T) {
    increment_function_depth_any(depth, node, std::slice::from_ref(stop));
}

fn increment_function_depth_any<T: PartialEq + From<u16>>(
    depth: &mut usize,
    node: &Node,
    stops: &[T],
) {
    // Increase depth function nesting if needed
    let mut child = *node;
    while let Some(parent) = child.parent() {
        let parent_kind = parent.kind_id().into();
        if stops.iter().any(|stop| stop == &parent_kind) {
            *depth += 1;
            break;
        }
        child = parent;
    }
}

#[inline(always)]
fn increase_nesting(stats: &mut Stats, nesting: &mut usize, depth: usize, lambda: usize) {
    stats.nesting = *nesting + depth + lambda;
    increment(stats);
    *nesting += 1;
    stats.boolean_seq.reset();
}

impl Cognitive for PythonCode {
    fn compute(
        node: &Node,
        stats: &mut Stats,
        nesting_map: &mut HashMap<usize, (usize, usize, usize)>,
    ) {
        use Python::*;

        // Get nesting of the parent
        let (mut nesting, mut depth, mut lambda) = get_nesting_from_map(node, nesting_map);

        match node.kind_id().into() {
            IfStatement
            | ForStatement
            | WhileStatement
            | TryStatement
            | ExceptClause
            | ConditionalExpression => {
                increase_nesting(stats, &mut nesting, depth, lambda);
            }
            ElifClause => {
                // No nesting increment for them because their cost has already
                // been paid by the if construct
                increment_by_one(stats);
                // Reset the boolean sequence
                stats.boolean_seq.reset();
            }
            ElseClause | FinallyClause => {
                // No nesting increment for them because their cost has already
                // been paid by the if construct
                increment_by_one(stats);
            }
            ExpressionList | ExpressionStatement | Tuple => {
                stats.boolean_seq.reset();
            }
            NotOperator => {
                stats.boolean_seq.not_operator(node.kind_id());
            }
            BooleanOperator => {
                if node.count_specific_ancestors::<PythonParser>(
                    |node| node.kind_id() == BooleanOperator,
                    |node| node.kind_id() == Lambda,
                ) == 0
                {
                    stats.structural += node.count_specific_ancestors::<PythonParser>(
                        |node| node.kind_id() == Lambda,
                        |node| {
                            matches!(
                                node.kind_id().into(),
                                ExpressionList | IfStatement | ForStatement | WhileStatement
                            )
                        },
                    );
                }
                compute_booleans::<Python>(node, stats, &And, &Or);
            }
            Lambda => {
                // Increase lambda nesting
                lambda += 1;
            }
            FunctionDefinition => {
                // Increase depth function nesting if needed
                increment_function_depth::<Python>(&mut depth, node, &FunctionDefinition);
            }
            _ => {}
        }
        // Add node to nesting map
        nesting_map.insert(node.id(), (nesting, depth, lambda));
    }
}

impl Cognitive for RustCode {
    fn compute(
        node: &Node,
        stats: &mut Stats,
        nesting_map: &mut HashMap<usize, (usize, usize, usize)>,
    ) {
        use Rust::*;
        let (mut nesting, mut depth, mut lambda) = get_nesting_from_map(node, nesting_map);

        if is_inside_rust_macro_tokens(node) {
            nesting_map.insert(node.id(), (nesting, depth, lambda));
            return;
        }

        match node.kind_id().into() {
            IfExpression if !Self::is_else_if(node) => {
                // Check if a node is not an else-if
                increase_nesting(stats, &mut nesting, depth, lambda);
            }
            IfExpression => {}
            ForExpression | WhileExpression | LoopExpression | MatchExpression => {
                increase_nesting(stats, &mut nesting, depth, lambda);
            }
            Else /*else-if also */ => {
                increment_by_one(stats);
            }
            TryExpression => {
                // `?` short-circuits on error; contributes +1 without nesting,
                // matching labeled break/continue treatment.
                increment_by_one(stats);
            }
            BreakExpression | ContinueExpression => {
                if let Some(label_child) = node.child(1)
                    && label_child.kind_id() == Label
                {
                    increment_by_one(stats);
                }
            }
            UnaryExpression => {
                stats.boolean_seq.not_operator(node.kind_id());
            }
            BinaryExpression => {
                compute_booleans::<Rust>(node, stats, &AMPAMP, &PIPEPIPE);
            }
            LetChain | LetChain2 => {
                compute_booleans::<Rust>(node, stats, &AMPAMP, &PIPEPIPE);
            }
            FunctionItem => {
                nesting = 0;
                // Increase depth function nesting if needed
                increment_function_depth::<Rust>(&mut depth, node, &FunctionItem);
            }
            ClosureExpression => {
                lambda += 1;
            }
            _ => {}
        }
        nesting_map.insert(node.id(), (nesting, depth, lambda));
    }
}

impl Cognitive for GoCode {
    fn compute(
        node: &Node,
        stats: &mut Stats,
        nesting_map: &mut HashMap<usize, (usize, usize, usize)>,
    ) {
        use crate::legacy::languages::Go::*;

        let (mut nesting, mut depth, mut lambda) = get_nesting_from_map(node, nesting_map);

        match node.kind_id().into() {
            IfStatement if !Self::is_else_if(node) => {
                increase_nesting(stats, &mut nesting, depth, lambda);
            }
            IfStatement => {}
            ForStatement | ExpressionSwitchStatement | TypeSwitchStatement | SelectStatement => {
                increase_nesting(stats, &mut nesting, depth, lambda);
            }
            Else /* else-if also */ => {
                increment_by_one(stats);
            }
            ExpressionStatement | SendStatement | ReceiveStatement | IncStatement
            | DecStatement | AssignmentStatement | ShortVarDeclaration | VarSpec | ConstSpec
            | ReturnStatement => {
                stats.boolean_seq.reset();
            }
            UnaryExpression => {
                stats.boolean_seq.not_operator(node.kind_id());
            }
            BinaryExpression => {
                compute_booleans::<crate::legacy::languages::Go>(node, stats, &AMPAMP, &PIPEPIPE);
            }
            FuncLiteral => {
                lambda += 1;
            }
            FunctionDeclaration | MethodDeclaration => {
                nesting = 0;
                increment_function_depth::<crate::legacy::languages::Go>(
                    &mut depth,
                    node,
                    &FunctionDeclaration,
                );
            }
            _ => {}
        }
        nesting_map.insert(node.id(), (nesting, depth, lambda));
    }
}

impl Cognitive for RubyCode {
    fn compute(
        node: &Node,
        stats: &mut Stats,
        nesting_map: &mut HashMap<usize, (usize, usize, usize)>,
    ) {
        use Ruby::*;

        let (mut nesting, mut depth, mut lambda) = get_nesting_from_map(node, nesting_map);

        match node.kind_id().into() {
            // Nesting-increasing control-flow constructs.
            If | Unless | While | Until | For | Case | CaseMatch | Conditional => {
                increase_nesting(stats, &mut nesting, depth, lambda);
            }
            // elsif: cost already paid by `if`; +1 without nesting.
            Elsif => {
                increment_by_one(stats);
                stats.boolean_seq.reset();
            }
            // else: +1 without nesting.
            Else => {
                increment_by_one(stats);
            }
            // rescue: treated like `except`, nesting-increasing.
            Rescue => {
                nesting += 1;
                increment(stats);
            }
            // Trailing-modifier forms: `expr if cond`, `expr unless cond`,
            // `expr while cond`, `expr until cond`, `expr rescue expr`.
            // Each contributes +1 without altering nesting (per Sonar spec).
            IfModifier | UnlessModifier | WhileModifier | UntilModifier | RescueModifier
            | RescueModifier2 | RescueModifier3 => {
                increment_by_one(stats);
            }
            // Reset boolean-sequence tracking at statement boundaries.
            Statement => {
                stats.boolean_seq.reset();
            }
            // Handle `not` / `!` unary forms.
            Unary | Unary2 | Unary3 | Unary4 | Unary5 => {
                stats.boolean_seq.not_operator(node.kind_id());
            }
            // Sequence of boolean binary operators with sequence collapsing.
            Binary | Binary2 | Binary3 => {
                // Collapse `&&`/`and` vs `||`/`or` sequences.
                compute_booleans::<Ruby>(node, stats, &AMPAMP, &PIPEPIPE);
                compute_booleans::<Ruby>(node, stats, &And, &Or);
            }
            // Blocks and lambdas bump lambda nesting, but a lambda-owned block
            // is the lambda body, not an additional nested lambda.
            Lambda => {
                lambda += 1;
            }
            Block | DoBlock
                if node
                    .parent()
                    .is_none_or(|parent| parent.kind_id() != Ruby::Lambda) =>
            {
                lambda += 1;
            }
            // Method definitions reset structural nesting and bump function depth.
            Method | SingletonMethod => {
                nesting = 0;
                increment_function_depth_any::<Ruby>(&mut depth, node, &[Method, SingletonMethod]);
            }
            _ => {}
        }
        nesting_map.insert(node.id(), (nesting, depth, lambda));
    }
}

impl Cognitive for KotlinCode {
    fn compute(
        node: &Node,
        stats: &mut Stats,
        nesting_map: &mut HashMap<usize, (usize, usize, usize)>,
    ) {
        use Kotlin::*;

        let (mut nesting, mut depth, mut lambda) = get_nesting_from_map(node, nesting_map);

        // Increment set and nesting model are aligned with SonarKotlin's
        // `CognitiveComplexity` check. Nesting-incrementing structures:
        // `if` (not else-if), loops, `when`, `catch`. Note: `try` itself is
        // NOT a nesting structure — only `catch_block` is. Label-qualified
        // `break`/`continue` add +1 without nesting. `else` adds +1 without
        // nesting. Mixed-sequence booleans are handled per conjunction /
        // disjunction subtree, matching the Sonar "sequence of like
        // operators" rule.
        // Reference:
        //   sonar-kotlin-checks/.../CognitiveComplexity.kt
        match node.kind_id().into() {
            IfExpression if !Self::is_else_if(node) => {
                increase_nesting(stats, &mut nesting, depth, lambda);
            }
            IfExpression => {}
            ForStatement | WhileStatement | DoWhileStatement | WhenExpression | CatchBlock => {
                increase_nesting(stats, &mut nesting, depth, lambda);
            }
            Else /* else-if also */ => {
                increment_by_one(stats);
            }
            // Label-qualified `break@label` / `continue@label` jumps break
            // the linear flow like a goto and earn +1 without nesting.
            JumpExpression if matches!(
                node.child(0).map(|c| c.kind_id().into()),
                Some(BreakAT) | Some(ContinueAT)
            ) => {
                increment_by_one(stats);
            }
            // Statement-boundary reset for the boolean-sequence tracker.
            // Kotlin's grammar aliases `_statement` as an (unemitted)
            // supertype, so concrete statement-like kinds are listed here.
            PropertyDeclaration | Assignment | CallExpression | JumpExpression => {
                stats.boolean_seq.reset();
            }
            PrefixExpression => {
                stats.boolean_seq.not_operator(node.kind_id());
            }
            ConjunctionExpression => {
                compute_booleans::<Kotlin>(node, stats, &AMPAMP, &AMPAMP);
            }
            DisjunctionExpression => {
                compute_booleans::<Kotlin>(node, stats, &PIPEPIPE, &PIPEPIPE);
            }
            FunctionDeclaration | AnonymousFunction | SecondaryConstructor => {
                nesting = 0;
                lambda = 0;
                increment_function_depth_any::<Kotlin>(
                    &mut depth,
                    node,
                    &[FunctionDeclaration, AnonymousFunction, SecondaryConstructor],
                );
            }
            LambdaLiteral => {
                lambda += 1;
            }
            _ => {}
        }
        nesting_map.insert(node.id(), (nesting, depth, lambda));
    }
}

// No languages require empty Cognitive implementations
// implement_metric_trait!(Cognitive);

impl Cognitive for CCode {
    fn compute(
        node: &Node,
        stats: &mut Stats,
        nesting_map: &mut HashMap<usize, (usize, usize, usize)>,
    ) {
        use C::*;

        let (mut nesting, mut depth, lambda) = get_nesting_from_map(node, nesting_map);

        // Sonar cognitive scoring for C:
        //   - Nesting-increasing: `if` (not else-if), loops, `switch`, and
        //     the ternary `conditional_expression`.
        //   - Non-nesting +1: `else` (covers else-if).
        //   - Same-operator collapsing on `&&` / `||` sequences.
        //   - `function_definition` resets structural nesting and bumps
        //     function depth. C has no closures or lambdas.
        match node.kind_id().into() {
            IfStatement if !Self::is_else_if(node) => {
                increase_nesting(stats, &mut nesting, depth, lambda);
            }
            IfStatement => {
                // `else if`: structural +1 is already paid by the outer
                // `else_clause`, but the condition starts a new
                // boolean-operator sequence. Defense-in-depth with the
                // `ElseClause` reset: keeps the invariant local to each
                // `if`, so a future visitor between `else_clause` and the
                // inner `if`'s operators won't reintroduce cross-branch
                // sequence bleed.
                stats.boolean_seq.reset();
            }
            ForStatement | WhileStatement | DoStatement | SwitchStatement
            | ConditionalExpression => {
                increase_nesting(stats, &mut nesting, depth, lambda);
            }
            ElseClause /* covers else-if */ => {
                increment_by_one(stats);
                // Mirror the ElseClause / ElseifClause resets in every
                // other language's cognitive impl so chained conditions
                // don't inherit state from the preceding branch.
                stats.boolean_seq.reset();
            }
            ExpressionStatement | ExpressionStatement2 | ReturnStatement | Declaration => {
                stats.boolean_seq.reset();
            }
            BinaryExpression | BinaryExpression2 => {
                compute_booleans::<C>(node, stats, &AMPAMP, &PIPEPIPE);
            }
            FunctionDefinition | FunctionDefinition2 => {
                nesting = 0;
                increment_function_depth_any::<C>(
                    &mut depth,
                    node,
                    &[FunctionDefinition, FunctionDefinition2],
                );
            }
            _ => {}
        }
        nesting_map.insert(node.id(), (nesting, depth, lambda));
    }
}

impl Cognitive for crate::legacy::langs::PhpCode {
    fn compute(
        node: &Node,
        stats: &mut Stats,
        nesting_map: &mut HashMap<usize, (usize, usize, usize)>,
    ) {
        use crate::legacy::languages::Php::*;

        let (mut nesting, mut depth, mut lambda) = get_nesting_from_map(node, nesting_map);

        match node.kind_id().into() {
            IfStatement if !Self::is_else_if(node) => {
                increase_nesting(stats, &mut nesting, depth, lambda);
            }
            IfStatement => {}
            ForStatement
            | ForeachStatement
            | WhileStatement
            | DoStatement
            | SwitchStatement
            | MatchExpression
            | TryStatement
            | CatchClause
            | ConditionalExpression => {
                increase_nesting(stats, &mut nesting, depth, lambda);
            }
            ElseIfClause | ElseIfClause2 => {
                increment_by_one(stats);
                stats.boolean_seq.reset();
            }
            ElseClause | ElseClause2 => {
                increment_by_one(stats);
                // Reset boolean-operator sequence tracking so `&&` / `||` /
                // `and` / `or` chains in the `else` body do not extend the
                // sequence from the preceding branch. Mirrors the reset in
                // `ElseIfClause` above.
                stats.boolean_seq.reset();
            }
            ExpressionStatement | ReturnStatement | EchoStatement => {
                stats.boolean_seq.reset();
            }
            UnaryOpExpression | UnaryOpExpression2 => {
                stats.boolean_seq.not_operator(node.kind_id());
            }
            BinaryExpression => {
                compute_booleans::<crate::legacy::languages::Php>(node, stats, &AMPAMP, &PIPEPIPE);
                compute_booleans::<crate::legacy::languages::Php>(node, stats, &And, &Or);
            }
            AnonymousFunction | ArrowFunction => {
                lambda += 1;
            }
            FunctionDefinition | MethodDeclaration => {
                nesting = 0;
                increment_function_depth_any::<crate::legacy::languages::Php>(
                    &mut depth,
                    node,
                    &[FunctionDefinition, MethodDeclaration],
                );
            }
            _ => {}
        }
        nesting_map.insert(node.id(), (nesting, depth, lambda));
    }
}

// Markdown is a documentation language; Cognitive is a code metric. The
// dedicated Markdown pipeline computes its own MCC analogue (Phase B).
#[cfg(feature = "markdown")]
impl Cognitive for crate::legacy::langs::MarkdownCode {
    fn compute(
        _node: &Node,
        _stats: &mut Stats,
        _nesting_map: &mut HashMap<usize, (usize, usize, usize)>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use crate::legacy::langs::{
        CParser, GoParser, KotlinParser, PhpParser, PythonParser, RubyParser, RustParser,
    };
    use crate::legacy::tools::check_metrics;

    #[test]
    fn php_else_branch_resets_boolean_sequence() {
        // The boolean-operator sequence must reset when entering the
        // `else` branch so that operators inside the else body start a
        // fresh sequence rather than continuing the sequence from the
        // `if` condition. Without the reset, two same-operator runs
        // separated only by an `else` collapse — undercounting cognitive
        // complexity.
        //
        // The bodies are intentionally empty: a non-empty body's
        // `expression_statement` would itself reset `boolean_seq` and
        // mask the bug.
        //
        // Breakdown WITH reset (correct):
        //   - outer `if`: +1 nesting -> structural=1
        //   - outer `&&`: fresh sequence, +1 -> 2
        //   - `else` clause: +1 (no nesting), reset -> 3
        //   - inner `else if`: parses as nested `if_statement` whose
        //     `is_else_if` is true; counted as elseif (no extra nesting)
        //   - inner `&&`: with the reset, fresh sequence again, +1 -> 4
        //
        // WITHOUT the reset, the inner `&&` collapses with the outer
        // (same operator) and contributes 0, yielding 3.
        check_metrics::<PhpParser>(
            "<?php
             function f($a, $b, $c, $d) {
                 if ($a && $b) {} else if ($c && $d) {}
             }",
            "foo.php",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 4.0,
                      "min": 0.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_no_cognitive() {
        check_metrics::<PythonParser>("a = 42", "foo.py", |metric| {
            insta::assert_json_snapshot!(
                metric.cognitive,
                @r###"
                    {
                      "sum": 0.0,
                      "average": null,
                      "min": 0.0,
                      "max": 0.0
                    }"###
            );
        });
    }

    #[test]
    fn rust_no_cognitive() {
        check_metrics::<RustParser>("let a = 42;", "foo.rs", |metric| {
            insta::assert_json_snapshot!(
                metric.cognitive,
                @r###"
                    {
                      "sum": 0.0,
                      "average": null,
                      "min": 0.0,
                      "max": 0.0
                    }"###
            );
        });
    }

    #[test]
    fn python_simple_function() {
        check_metrics::<PythonParser>(
            "def f(a, b):
                if a and b:  # +2 (+1 and)
                   return 1
                if c and d: # +2 (+1 and)
                   return 1",
            "foo.py",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 4.0,
                      "min": 0.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_expression_statement() {
        // Boolean expressions containing `And` and `Or` operators were not
        // considered in assignments
        check_metrics::<PythonParser>(
            "def f(a, b):
                c = True and True",
            "foo.py",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 1.0,
                      "average": 1.0,
                      "min": 0.0,
                      "max": 1.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_tuple() {
        // Boolean expressions containing `And` and `Or` operators were not
        // considered inside tuples
        check_metrics::<PythonParser>(
            "def f(a, b):
                return \"%s%s\" % (a and \"Get\" or \"Set\", b)",
            "foo.py",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 2.0,
                      "average": 2.0,
                      "min": 0.0,
                      "max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_nested_if_in_else_is_not_else_if() {
        // Python has no `else if`; `elif` is a dedicated grammar node. A plain
        // `if` inside an `else:` block must therefore be counted as a nested
        // `if`, not skipped as else-if. This verifies that `is_else_if = false`
        // for Python is correct.
        check_metrics::<PythonParser>(
            "def f(a, b):
                if a:          # +1
                    pass
                else:          # +1 else
                    if b:      # +2 (+1 if, +1 nesting)
                        pass",
            "foo.py",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 4.0,
                      "min": 0.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_elif_function() {
        // Boolean expressions containing `And` and `Or` operators were not
        // considered in `elif` statements
        check_metrics::<PythonParser>(
            "def f(a, b):
                if a and b:  # +2 (+1 and)
                   return 1
                elif c and d: # +2 (+1 and)
                   return 1",
            "foo.py",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 4.0,
                      "min": 0.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_more_elifs_function() {
        // Boolean expressions containing `And` and `Or` operators were not
        // considered when there were more `elif` statements
        check_metrics::<PythonParser>(
            "def f(a, b):
                if a and b:  # +2 (+1 and)
                   return 1
                elif c and d: # +2 (+1 and)
                   return 1
                elif e and f: # +2 (+1 and)
                   return 1",
            "foo.py",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 6.0,
                      "average": 6.0,
                      "min": 0.0,
                      "max": 6.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_simple_function() {
        check_metrics::<RustParser>(
            "fn f() {
                 if a && b { // +2 (+1 &&)
                     println!(\"test\");
                 }
                 if c && d { // +2 (+1 &&)
                     println!(\"test\");
                 }
             }",
            "foo.rs",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 4.0,
                      "min": 0.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_sequence_same_booleans() {
        check_metrics::<PythonParser>(
            "def f(a, b):
                if a and b and True:  # +2 (+1 sequence of and)
                   return 1",
            "foo.py",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 2.0,
                      "average": 2.0,
                      "min": 0.0,
                      "max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_sequence_same_booleans() {
        check_metrics::<RustParser>(
            "fn f() {
                 if a && b && true { // +2 (+1 sequence of &&)
                     println!(\"test\");
                 }
             }",
            "foo.rs",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 2.0,
                      "average": 2.0,
                      "min": 0.0,
                      "max": 2.0
                    }"###
                );
            },
        );

        check_metrics::<RustParser>(
            "fn f() {
                 if a || b || c || d { // +2 (+1 sequence of ||)
                     println!(\"test\");
                 }
             }",
            "foo.rs",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 2.0,
                      "average": 2.0,
                      "min": 0.0,
                      "max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_not_booleans() {
        check_metrics::<RustParser>(
            "fn f() {
                 if !a && !b { // +2 (+1 &&)
                     println!(\"test\");
                 }
             }",
            "foo.rs",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 2.0,
                      "average": 2.0,
                      "min": 0.0,
                      "max": 2.0
                    }"###
                );
            },
        );

        check_metrics::<RustParser>(
            "fn f() {
                 if a && !(b && c) { // +3 (+1 &&, +1 &&)
                     println!(\"test\");
                 }
             }",
            "foo.rs",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 3.0,
                      "min": 0.0,
                      "max": 3.0
                    }"###
                );
            },
        );

        check_metrics::<RustParser>(
            "fn f() {
                 if !(a || b) && !(c || d) { // +4 (+1 ||, +1 &&, +1 ||)
                     println!(\"test\");
                 }
             }",
            "foo.rs",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 4.0,
                      "min": 0.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_sequence_different_booleans() {
        check_metrics::<PythonParser>(
            "def f(a, b):
                if a and b or True:  # +3 (+1 and, +1 or)
                   return 1",
            "foo.py",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 3.0,
                      "min": 0.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_sequence_different_booleans() {
        check_metrics::<RustParser>(
            "fn f() {
                 if a && b || true { // +3 (+1 &&, +1 ||)
                     println!(\"test\");
                 }
             }",
            "foo.rs",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 3.0,
                      "min": 0.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_let_chain_boolean_sequence() {
        check_metrics::<RustParser>(
            "fn f(a: Option<i32>, b: Option<i32>) {
                 if let Some(x) = a && let Some(y) = b && x > y {
                     work();
                 }
             }",
            "foo.rs",
            |metric| {
                // +1 for the if, +1 for the same-operator `&&` let-chain.
                assert_eq!(metric.cognitive.cognitive_sum(), 2.0);
            },
        );
    }

    #[test]
    fn rust_macro_tokens_are_opaque_for_cognitive() {
        check_metrics::<RustParser>(
            "fn f() {
                 maybe!(a && b, if c { d() });
             }",
            "foo.rs",
            |metric| {
                assert_eq!(metric.cognitive.cognitive_sum(), 0.0);
            },
        );
    }

    #[test]
    fn python_formatted_sequence_different_booleans() {
        check_metrics::<PythonParser>(
            "def f(a, b):
                if (  # +1
                    a and b and  # +1
                    (c or d)  # +1
                ):
                   return 1",
            "foo.py",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 3.0,
                      "min": 0.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_1_level_nesting() {
        check_metrics::<PythonParser>(
            "def f(a, b):
                if a:  # +1
                    for i in range(b):  # +2
                        return 1",
            "foo.py",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 3.0,
                      "min": 0.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_1_level_nesting() {
        check_metrics::<RustParser>(
            "fn f() {
                 if true { // +1
                     if true { // +2 (nesting = 1)
                         println!(\"test\");
                     } else if 1 == 1 { // +1
                         if true { // +3 (nesting = 2)
                             println!(\"test\");
                         }
                     } else { // +1
                         if true { // +3 (nesting = 2)
                             println!(\"test\");
                         }
                     }
                 }
             }",
            "foo.rs",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 11.0,
                      "average": 11.0,
                      "min": 0.0,
                      "max": 11.0
                    }"###
                );
            },
        );

        check_metrics::<RustParser>(
            "fn f() {
                 if true { // +1
                     match true { // +2 (nesting = 1)
                         true => println!(\"test\"),
                         false => println!(\"test\"),
                     }
                 }
             }",
            "foo.rs",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 3.0,
                      "min": 0.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_2_level_nesting() {
        check_metrics::<PythonParser>(
            "def f(a, b):
                if a:  # +1
                    for i in range(b):  # +2
                        if b:  # +3
                            return 1",
            "foo.py",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 6.0,
                      "average": 6.0,
                      "min": 0.0,
                      "max": 6.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_2_level_nesting() {
        check_metrics::<RustParser>(
            "fn f() {
                 if true { // +1
                     for i in 0..4 { // +2 (nesting = 1)
                         match true { // +3 (nesting = 2)
                             true => println!(\"test\"),
                             false => println!(\"test\"),
                         }
                     }
                 }
             }",
            "foo.rs",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 6.0,
                      "average": 6.0,
                      "min": 0.0,
                      "max": 6.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_try_construct() {
        check_metrics::<PythonParser>(
            "def f(a, b):
                try:                 # +1
                    for foo in bar:  # +2 (nesting = 1)
                        return a
                except Exception:    # +2 (nesting = 1)
                    if a < 0:        # +3 (nesting = 2)
                        return a",
            "foo.py",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 8.0,
                      "average": 8.0,
                      "min": 0.0,
                      "max": 8.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_break_continue() {
        // Only labeled break and continue statements are considered
        check_metrics::<RustParser>(
            "fn f() {
                 'tens: for ten in 0..3 { // +1
                     '_units: for unit in 0..=9 { // +2 (nesting = 1)
                         if unit % 2 == 0 { // +3 (nesting = 2)
                             continue;
                         } else if unit == 5 { // +1
                             continue 'tens; // +1
                         } else if unit == 6 { // +1
                             break;
                         } else { // +1
                             break 'tens; // +1
                         }
                     }
                 }
             }",
            "foo.rs",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 11.0,
                      "average": 11.0,
                      "min": 0.0,
                      "max": 11.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_ternary_operator() {
        check_metrics::<PythonParser>(
            "def f(a, b):
                 if a % 2:  # +1
                     return 'c' if a else 'd'  # +2
                 return 'a' if a else 'b'  # +1",
            "foo.py",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 4.0,
                      "min": 0.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_nested_functions_lambdas() {
        check_metrics::<PythonParser>(
            "def f(a, b):
                 def foo(a):
                     if a:  # +2 (+1 nesting)
                         return 1
                 # +3 (+1 for boolean sequence +2 for lambda nesting)
                 bar = lambda a: lambda b: b or True or True
                 return bar(foo(a))(a)",
            "foo.py",
            |metric| {
                // 2 functions + 2 lambdas = 4
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 5.0,
                      "average": 1.25,
                      "min": 0.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn python_real_function() {
        check_metrics::<PythonParser>(
            "def process_raw_constant(constant, min_word_length):
                 processed_words = []
                 raw_camelcase_words = []
                 for raw_word in re.findall(r'[a-z]+', constant):  # +1
                     word = raw_word.strip()
                         if (  # +2 (+1 if and +1 nesting)
                             len(word) >= min_word_length
                             and not (word.startswith('-') or word.endswith('-')) # +2 operators
                         ):
                             if is_camel_case_word(word):  # +3 (+1 if and +2 nesting)
                                 raw_camelcase_words.append(word)
                             else: # +1 else
                                 processed_words.append(word.lower())
                 return processed_words, raw_camelcase_words",
            "foo.py",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 9.0,
                      "average": 9.0,
                      "min": 0.0,
                      "max": 9.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_if_let_else_if_else() {
        check_metrics::<RustParser>(
            "pub fn create_usage_no_title(p: &Parser, used: &[&str]) -> String {
                 debugln!(\"usage::create_usage_no_title;\");
                 if let Some(u) = p.meta.usage_str { // +1
                     String::from(&*u)
                 } else if used.is_empty() { // +1
                     create_help_usage(p, true)
                 } else { // +1
                     create_smart_usage(p, used)
                }
            }",
            "foo.rs",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 3.0,
                      "min": 0.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_loop_and_try() {
        check_metrics::<RustParser>(
            "fn f() -> Option<i32> {
                 loop {          // +1
                     let x = g()?;  // +1 try
                     if x > 0 {   // +2 (nesting = 1)
                         return Some(x);
                     }
                 }
             }",
            "foo.rs",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 4.0,
                      "min": 0.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_no_cognitive() {
        check_metrics::<GoParser>(
            "package main

            var x = 42",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 0.0,
                      "average": null,
                      "min": 0.0,
                      "max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_simple_function() {
        check_metrics::<GoParser>(
            "package main

            func f() {
                if true { // +1
                    if false { // +2 (nesting = 1)
                        println(\"test\")
                    }
                }
            }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 3.0,
                      "min": 0.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_for_loop() {
        check_metrics::<GoParser>(
            "package main

            func f() {
                for i := 0; i < 10; i++ { // +1
                    if i > 5 { // +2 (nesting = 1)
                        println(i)
                    }
                }
            }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 3.0,
                      "min": 0.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_logical_operators() {
        check_metrics::<GoParser>(
            "package main

            func f(a, b, c bool) {
                if a && b && c { // +1 (if) +1 (sequence of &&)
                    println(\"all true\")
                }
            }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 2.0,
                      "average": 2.0,
                      "min": 0.0,
                      "max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_logical_operator_sequences_reset_between_statements() {
        check_metrics::<GoParser>(
            "package main

            func f(a, b, c, d bool) {
                _ = a && b
                _ = c && d
            }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 2.0,
                      "average": 2.0,
                      "min": 0.0,
                      "max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_logical_operator_sequences_reset_between_declaration_specs() {
        check_metrics::<GoParser>(
            "package main

            func f(a, b, c, d bool) {
                var x = a && b
                var y = c && d
                const p = true && false
                const q = false && true
            }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 4.0,
                      "min": 0.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_no_cognitive() {
        check_metrics::<RubyParser>("a = 42", "foo.rb", |metric| {
            insta::assert_json_snapshot!(
                metric.cognitive,
                @r###"
                {
                  "sum": 0.0,
                  "average": null,
                  "min": 0.0,
                  "max": 0.0
                }"###
            );
        });
    }

    #[test]
    fn ruby_simple_method() {
        check_metrics::<RubyParser>(
            "def f(a, b)
                 if a && b  # +2 (+1 if, +1 &&)
                    return 1
                 end
                 if c && d  # +2 (+1 if, +1 &&)
                    return 1
                 end
             end",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 4.0,
                      "min": 0.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_nested_if_and_else() {
        check_metrics::<RubyParser>(
            "def f(a, b)
                 if a          # +1
                    if b        # +2 (nesting = 1)
                       return 1
                    else        # +1
                       return 2
                    end
                 end
             end",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 4.0,
                      "min": 0.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_modifier_and_rescue() {
        check_metrics::<RubyParser>(
            "def f(a)
                 return a if a > 0  # +1 if_modifier
                 begin
                    risky!
                 rescue StandardError  # +1 (nesting +1 because in begin)
                    retry
                 end
             end",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 2.0,
                      "average": 2.0,
                      "min": 0.0,
                      "max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_rescue_modifier() {
        check_metrics::<RubyParser>(
            "def f
                 value = risky rescue fallback  # +1 rescue_modifier
             end",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 1.0,
                      "average": 1.0,
                      "min": 0.0,
                      "max": 1.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_lambda_with_block() {
        check_metrics::<RubyParser>(
            "def f
                 x = -> { if a then 1 end }
             end",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 2.0,
                      "average": 1.0,
                      "min": 0.0,
                      "max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_nested_if_increments_nesting() {
        check_metrics::<KotlinParser>(
            "fun f(a: Boolean, b: Boolean) {
                 if (a) {      // +1
                     if (b) {  // +2 (nesting = 1)
                         println(\"hi\")
                     }
                 }
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 3.0,
                      "min": 0.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_try_catch_nesting() {
        // SonarKotlin's `CognitiveComplexity` increments and bumps nesting on
        // `KtCatchClause`, not on the enclosing `try`. An `if` inside the
        // catch block therefore sees nesting=1 at the +1 structural cost.
        check_metrics::<KotlinParser>(
            "fun f() {
                 try {
                     if (a) {       // +1 (try itself contributes 0)
                         println(\"a\")
                     }
                 } catch (e: Exception) { // +1 catch
                     if (b) {               // +2 (nesting = 1 from catch)
                         println(\"b\")
                     }
                 }
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 4.0,
                      "min": 0.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_labeled_break_and_continue() {
        // Label-qualified `break@label` / `continue@label` flip the linear
        // flow and earn +1 each per the Sonar whitepaper. Unlabelled forms
        // don't.
        check_metrics::<KotlinParser>(
            "fun f() {
                 outer@ for (i in 0..10) {        // +1 for
                     for (j in 0..10) {           // +2 (nesting=1)
                         if (i == j) {            // +3 (nesting=2)
                             continue@outer       // +1 labelled continue
                         }
                         if (j > 5) {             // +3 (nesting=2)
                             break@outer          // +1 labelled break
                         }
                     }
                 }
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 11.0,
                      "average": 11.0,
                      "min": 0.0,
                      "max": 11.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_else_if_counts_as_one() {
        // `else if` in Kotlin parses as an `if_expression` whose parent is
        // another `if_expression`. It should NOT increase nesting; only the
        // `else` keyword adds +1, matching other C-style languages.
        check_metrics::<KotlinParser>(
            "fun f(a: Int) {
                 if (a > 0) {          // +1
                     println(\"pos\")
                 } else if (a < 0) {   // +1
                     println(\"neg\")
                 } else {              // +1
                     println(\"zero\")
                 }
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 3.0,
                      "min": 0.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_nested_if_in_then_branch_is_not_else_if() {
        // Regression: an unbraced nested `if` in the *then* branch of an
        // outer `if` parses as `if_expression > control_structure_body >
        // if_expression`. The grammar also uses `control_structure_body`
        // for the `else` branch, so `is_else_if` must specifically check
        // that the body it lives in is the outer if's `alternative`, not
        // its `consequence`. Otherwise this nested-if is misclassified as
        // `else if` and cognitive complexity undercounts by 2 (no +1
        // structural cost and no +1 nesting).
        check_metrics::<KotlinParser>(
            "fun f(a: Boolean, b: Boolean) {
                 if (a)            // +1
                     if (b)        // +2 (nesting = 1)
                         println(\"hi\")
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 3.0,
                      "min": 0.0,
                      "max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_nested_if_inside_else_if_chain_counts() {
        // Mixed shape: a nested `if` inside both the then-branch of the
        // outer `if` AND the body of an `else if`. The outer `if` counts
        // +1, the nested `if` in the then-branch counts +2 (nesting=1),
        // the `else if` counts +1 (flattened, no nesting), and its nested
        // `if` counts +2 (nesting=1) for a total of 6. This locks in that
        // the fix only flattens the else-branch, not the then-branch.
        check_metrics::<KotlinParser>(
            "fun f(a: Int, b: Int) {
                 if (a > 0) {            // +1
                     if (b > 0) {        // +2 (nesting = 1)
                         println(\"x\")
                     }
                 } else if (a < 0) {     // +1 (flattened else-if)
                     if (b > 0) {        // +2 (nesting = 1)
                         println(\"y\")
                     }
                 }
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 6.0,
                      "average": 6.0,
                      "min": 0.0,
                      "max": 6.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_nested_method_in_singleton_method() {
        check_metrics::<RubyParser>(
            "def self.outer
                 def inner
                   if x then 1 end
                 end
             end",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cognitive,
                    @r###"
                    {
                      "sum": 2.0,
                      "average": 1.0,
                      "min": 0.0,
                      "max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn c_boolean_sequence_does_not_leak_across_else_if() {
        // Regression: tree-sitter-c parses `else if` as `else_clause { if_statement }`.
        // The outer `if (a && b)`'s boolean-sequence tracker must not bleed
        // into the inner `else if (c && d)` condition — otherwise the
        // second `&&` would collapse with the first (same operator) and
        // cognitive would be undercounted.
        //
        // Expected breakdown for `int f(int a, int b, int c, int d)`:
        //   +1 outer `if`                 (nesting = 0 -> 1)
        //   +1 outer `&&`                 (first op in sequence)
        //   +1 `else` clause              (no nesting)
        //   +0 inner `if` (else-if arm)   (structural cost paid by `else`)
        //   +1 inner `&&`                 (fresh sequence — IF reset works)
        // total = 4.
        check_metrics::<CParser>(
            "int f(int a, int b, int c, int d) {
                 if (a && b) {
                     return 1;
                 } else if (c && d) {
                     return 2;
                 }
                 return 0;
             }",
            "foo.c",
            |metric| {
                assert_eq!(metric.cognitive.cognitive_sum(), 4.0);
            },
        );
    }
}
