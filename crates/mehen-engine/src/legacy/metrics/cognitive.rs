use std::collections::HashMap;

use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::legacy::checker::Checker;
use crate::legacy::langs::{CCode, KotlinCode};
use crate::legacy::languages::{C, Kotlin};
use crate::legacy::node::Node;

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
    use crate::legacy::langs::{CParser, KotlinParser};
    use crate::legacy::tools::check_metrics;

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
