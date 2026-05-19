use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::legacy::checker::Checker;
use crate::legacy::langs::{CCode, GoCode, KotlinCode};
use crate::legacy::languages::{C, Go, Kotlin};
use crate::legacy::node::Node;

/// The `ABC` metric.
///
/// The `ABC` metric measures the size of a source code by counting
/// the number of Assignments (`A`), Branches (`B`) and Conditions (`C`).
/// The metric defines an ABC score as a vector of three elements (`<A,B,C>`).
/// The ABC score can be represented by its individual components (`A`, `B` and `C`)
/// or by the magnitude of the vector (`|<A,B,C>| = sqrt(A^2 + B^2 + C^2)`).
///
/// Official paper and definition:
///
/// Fitzpatrick, Jerry (1997). "Applying the ABC metric to C, C++ and Java". C++ Report.
///
/// <https://www.softwarerenovation.com/Articles.aspx>
#[derive(Debug, Clone)]
pub(crate) struct Stats {
    assignments: f64,
    assignments_sum: f64,
    assignments_min: f64,
    assignments_max: f64,
    branches: f64,
    branches_sum: f64,
    branches_min: f64,
    branches_max: f64,
    conditions: f64,
    conditions_sum: f64,
    conditions_min: f64,
    conditions_max: f64,
    space_count: usize,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            assignments: 0.,
            assignments_sum: 0.,
            assignments_min: f64::MAX,
            assignments_max: 0.,
            branches: 0.,
            branches_sum: 0.,
            branches_min: f64::MAX,
            branches_max: 0.,
            conditions: 0.,
            conditions_sum: 0.,
            conditions_min: f64::MAX,
            conditions_max: 0.,
            space_count: 1,
        }
    }
}

impl Serialize for Stats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("abc", 13)?;
        st.serialize_field("assignments", &self.assignments_sum())?;
        st.serialize_field("branches", &self.branches_sum())?;
        st.serialize_field("conditions", &self.conditions_sum())?;
        st.serialize_field("magnitude", &self.magnitude_sum())?;
        st.serialize_field("assignments_average", &self.assignments_average())?;
        st.serialize_field("branches_average", &self.branches_average())?;
        st.serialize_field("conditions_average", &self.conditions_average())?;
        st.serialize_field("assignments_min", &self.assignments_min())?;
        st.serialize_field("assignments_max", &self.assignments_max())?;
        st.serialize_field("branches_min", &self.branches_min())?;
        st.serialize_field("branches_max", &self.branches_max())?;
        st.serialize_field("conditions_min", &self.conditions_min())?;
        st.serialize_field("conditions_max", &self.conditions_max())?;
        st.end()
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "assignments: {}, branches: {}, conditions: {}, magnitude: {}, \
            assignments_average: {}, branches_average: {}, conditions_average: {}, \
            assignments_min: {}, assignments_max: {}, \
            branches_min: {}, branches_max: {}, \
            conditions_min: {}, conditions_max: {}",
            self.assignments_sum(),
            self.branches_sum(),
            self.conditions_sum(),
            self.magnitude_sum(),
            self.assignments_average(),
            self.branches_average(),
            self.conditions_average(),
            self.assignments_min(),
            self.assignments_max(),
            self.branches_min(),
            self.branches_max(),
            self.conditions_min(),
            self.conditions_max()
        )
    }
}

impl Stats {
    /// Merges a second `Abc` metric into the first one.
    pub(crate) fn merge(&mut self, other: &Self) {
        // Calculates minimum and maximum values
        self.assignments_min = self.assignments_min.min(other.assignments_min);
        self.assignments_max = self.assignments_max.max(other.assignments_max);
        self.branches_min = self.branches_min.min(other.branches_min);
        self.branches_max = self.branches_max.max(other.branches_max);
        self.conditions_min = self.conditions_min.min(other.conditions_min);
        self.conditions_max = self.conditions_max.max(other.conditions_max);

        self.assignments_sum += other.assignments_sum;
        self.branches_sum += other.branches_sum;
        self.conditions_sum += other.conditions_sum;

        self.space_count += other.space_count;
    }

    /// Returns the `Abc` assignments sum metric value.
    pub(crate) fn assignments_sum(&self) -> f64 {
        self.assignments_sum
    }

    /// Returns the `Abc` assignments average value.
    ///
    /// This value is computed dividing the `Abc`
    /// assignments value for the number of spaces.
    pub(crate) fn assignments_average(&self) -> f64 {
        self.assignments_sum() / self.space_count as f64
    }

    /// Returns the `Abc` assignments minimum value.
    pub(crate) fn assignments_min(&self) -> f64 {
        self.assignments_min
    }

    /// Returns the `Abc` assignments maximum value.
    pub(crate) fn assignments_max(&self) -> f64 {
        self.assignments_max
    }

    /// Returns the `Abc` branches sum metric value.
    pub(crate) fn branches_sum(&self) -> f64 {
        self.branches_sum
    }

    /// Returns the `Abc` branches average value.
    ///
    /// This value is computed dividing the `Abc`
    /// branches value for the number of spaces.
    pub(crate) fn branches_average(&self) -> f64 {
        self.branches_sum() / self.space_count as f64
    }

    /// Returns the `Abc` branches minimum value.
    pub(crate) fn branches_min(&self) -> f64 {
        self.branches_min
    }

    /// Returns the `Abc` branches maximum value.
    pub(crate) fn branches_max(&self) -> f64 {
        self.branches_max
    }

    /// Returns the `Abc` conditions sum metric value.
    pub(crate) fn conditions_sum(&self) -> f64 {
        self.conditions_sum
    }

    /// Returns the `Abc` conditions average value.
    ///
    /// This value is computed dividing the `Abc`
    /// conditions value for the number of spaces.
    pub(crate) fn conditions_average(&self) -> f64 {
        self.conditions_sum() / self.space_count as f64
    }

    /// Returns the `Abc` conditions minimum value.
    pub(crate) fn conditions_min(&self) -> f64 {
        self.conditions_min
    }

    /// Returns the `Abc` conditions maximum value.
    pub(crate) fn conditions_max(&self) -> f64 {
        self.conditions_max
    }

    /// Returns the `Abc` magnitude sum metric value.
    pub(crate) fn magnitude_sum(&self) -> f64 {
        self.conditions_sum
            .mul_add(
                self.conditions_sum,
                self.assignments_sum
                    .mul_add(self.assignments_sum, self.branches_sum.powi(2)),
            )
            .sqrt()
    }

    #[inline(always)]
    pub(crate) fn compute_sum(&mut self) {
        self.assignments_sum += self.assignments;
        self.branches_sum += self.branches;
        self.conditions_sum += self.conditions;
    }

    #[inline(always)]
    pub(crate) fn compute_minmax(&mut self) {
        self.assignments_min = self.assignments_min.min(self.assignments);
        self.assignments_max = self.assignments_max.max(self.assignments);
        self.branches_min = self.branches_min.min(self.branches);
        self.branches_max = self.branches_max.max(self.branches);
        self.conditions_min = self.conditions_min.min(self.conditions);
        self.conditions_max = self.conditions_max.max(self.conditions);
        self.compute_sum();
    }
}

pub(crate) trait Abc
where
    Self: Checker,
{
    fn compute(node: &Node, stats: &mut Stats);
}

#[inline(always)]
fn go_expression_list_len(node: &Node) -> f64 {
    node.children()
        .filter(|child| !matches!(child.kind_id().into(), Go::COMMA | Go::Comment))
        .count() as f64
}

#[inline(always)]
fn go_assignment_target_count(node: &Node) -> f64 {
    node.child_by_field_name("left")
        .map(|child| go_expression_list_len(&child))
        .unwrap_or(1.)
}

#[inline(always)]
fn go_spec_name_count(node: &Node) -> f64 {
    let mut count: f64 = 0.;
    for child in node.children() {
        match child.kind_id().into() {
            Go::EQ => break,
            Go::Identifier | Go::Identifier2 | Go::Identifier3 | Go::BlankIdentifier => count += 1.,
            _ => {}
        }
    }
    count.max(1.)
}

impl Abc for GoCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use crate::legacy::languages::Go::*;

        match node.kind_id().into() {
            AssignmentStatement | ShortVarDeclaration => {
                stats.assignments += go_assignment_target_count(node);
            }
            ReceiveStatement | RangeClause if node.child_by_field_name("left").is_some() => {
                stats.assignments += go_assignment_target_count(node);
            }
            IncStatement | DecStatement => {
                stats.assignments += 1.;
            }
            ConstSpec => {
                stats.assignments += go_spec_name_count(node);
            }
            VarSpec if node.is_child(EQ as u16) => {
                stats.assignments += go_spec_name_count(node);
            }
            CallExpression => {
                stats.branches += 1.;
            }
            IfStatement | ForStatement | ExpressionCase | DefaultCase | TypeCase
            | CommunicationCase | EQEQ | BANGEQ | LT | LTEQ | GT | GTEQ | AMPAMP | PIPEPIPE
            | BANG => {
                stats.conditions += 1.;
            }
            _ => {}
        }
    }
}

impl Abc for KotlinCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use Kotlin::*;

        match node.kind_id().into() {
            // A: assignments. `property_declaration` with an initializer is
            // also an assignment (`val x = ...`).
            Assignment => {
                stats.assignments += 1.;
            }
            PropertyDeclaration if node.is_child(EQ as u16) => {
                stats.assignments += 1.;
            }
            // B: function and constructor calls transfer control.
            CallExpression => {
                stats.branches += 1.;
            }
            // C: every conditional construct and comparison / logical operator.
            IfExpression | WhenEntry | CatchBlock | ForStatement | WhileStatement
            | DoWhileStatement | EQEQ | BANGEQ | EQEQEQ | BANGEQEQ | LT | LTEQ | GT | GTEQ
            | AMPAMP | PIPEPIPE | QMARKCOLON | QMARKDOT | BANGBANG => {
                stats.conditions += 1.;
            }
            _ => {}
        }
    }
}

impl Abc for CCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use C::*;

        match node.kind_id().into() {
            // A: assignments, compound assignments, `init_declarator` with
            // initializer (`int x = 1`), and increment/decrement.
            AssignmentExpression => {
                stats.assignments += 1.;
            }
            InitDeclarator if node.is_child(EQ as u16) => {
                stats.assignments += 1.;
            }
            UpdateExpression => {
                stats.assignments += 1.;
            }
            // B: every function/method call.
            CallExpression | CallExpression2 => {
                stats.branches += 1.;
            }
            // C: structural conditionals, switch cases, comparison and
            // short-circuit boolean operators. `else_clause` also contributes
            // per Fitzpatrick's original ABC specification, matching how
            // TypeScript / Tsx treat `ElseClause` in this crate.
            IfStatement
            | ElseClause
            | CaseStatement
            | ForStatement
            | WhileStatement
            | DoStatement
            | ConditionalExpression
            | EQEQ
            | BANGEQ
            | LT
            | LTEQ
            | GT
            | GTEQ
            | AMPAMP
            | PIPEPIPE
            | BANG => {
                stats.conditions += 1.;
            }
            _ => {}
        }
    }
}

// Markdown is a documentation language; ABC is a code metric and has no
// meaning on prose. The dedicated Markdown pipeline handles document metrics.
#[cfg(feature = "markdown")]
impl Abc for crate::legacy::langs::MarkdownCode {
    fn compute(_node: &Node, _stats: &mut Stats) {}
}

#[cfg(test)]
mod tests {
    use crate::legacy::langs::{CParser, GoParser, KotlinParser};
    use crate::legacy::tools::check_metrics;

    #[test]
    fn go_abc_basic() {
        check_metrics::<GoParser>(
            "package main

             func f(a, b int) int {
                 x, y := a, b
                 log(x)
                 if x > y {
                     return x
                 }
                 return y
             }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.abc,
                    @r###"
                    {
                      "assignments": 2.0,
                      "branches": 1.0,
                      "conditions": 2.0,
                      "magnitude": 3.0,
                      "assignments_average": 1.0,
                      "branches_average": 0.5,
                      "conditions_average": 1.0,
                      "assignments_min": 0.0,
                      "assignments_max": 2.0,
                      "branches_min": 0.0,
                      "branches_max": 1.0,
                      "conditions_min": 0.0,
                      "conditions_max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_abc_comments_in_targets_and_logical_conditions() {
        check_metrics::<GoParser>(
            "package main

             func f(a, b int) {
                 x, /* target comment */ y := a, b
                 _ = !((x > 0 && y > 0) || x == y)
             }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.abc,
                    @r###"
                    {
                      "assignments": 3.0,
                      "branches": 0.0,
                      "conditions": 6.0,
                      "magnitude": 6.708203932499369,
                      "assignments_average": 1.5,
                      "branches_average": 0.0,
                      "conditions_average": 3.0,
                      "assignments_min": 0.0,
                      "assignments_max": 3.0,
                      "branches_min": 0.0,
                      "branches_max": 0.0,
                      "conditions_min": 0.0,
                      "conditions_max": 6.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_abc_receive_assignments() {
        check_metrics::<GoParser>(
            "package main

             func f(ch chan int) {
                 x := <-ch
                 y, ok := <-ch
                 <-ch
             }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.abc,
                    @r###"
                    {
                      "assignments": 3.0,
                      "branches": 0.0,
                      "conditions": 0.0,
                      "magnitude": 3.0,
                      "assignments_average": 1.5,
                      "branches_average": 0.0,
                      "conditions_average": 0.0,
                      "assignments_min": 0.0,
                      "assignments_max": 3.0,
                      "branches_min": 0.0,
                      "branches_max": 0.0,
                      "conditions_min": 0.0,
                      "conditions_max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_abc_range_clause_assignments() {
        check_metrics::<GoParser>(
            "package main

             func f(m map[string]int) {
                 for k, v := range m {
                 }
                 for k = range m {
                 }
                 for range m {
                 }
             }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.abc,
                    @r###"
                    {
                      "assignments": 3.0,
                      "branches": 0.0,
                      "conditions": 3.0,
                      "magnitude": 4.242640687119285,
                      "assignments_average": 1.5,
                      "branches_average": 0.0,
                      "conditions_average": 1.5,
                      "assignments_min": 0.0,
                      "assignments_max": 3.0,
                      "branches_min": 0.0,
                      "branches_max": 0.0,
                      "conditions_min": 0.0,
                      "conditions_max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_abc_default_cases() {
        check_metrics::<GoParser>(
            "package main

             func f(x int) int {
                 switch x {
                 case 1:
                     return 1
                 default:
                     return 0
                 }
             }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.abc,
                    @r###"
                    {
                      "assignments": 0.0,
                      "branches": 0.0,
                      "conditions": 2.0,
                      "magnitude": 2.0,
                      "assignments_average": 0.0,
                      "branches_average": 0.0,
                      "conditions_average": 1.0,
                      "assignments_min": 0.0,
                      "assignments_max": 0.0,
                      "branches_min": 0.0,
                      "branches_max": 0.0,
                      "conditions_min": 0.0,
                      "conditions_max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_abc_basic() {
        check_metrics::<KotlinParser>(
            "fun f(a: Int, b: Int): Int {
                 val c = a + b        // +1 A (val with initializer)
                 log(c)               // +1 B
                 if (c > 0) {         // +1 C (if) + +1 C (>)
                     return c
                 }
                 return 0
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(metric.abc);
            },
        );
    }

    #[test]
    fn c_abc_counts_else_clause_in_conditions() {
        // Per Fitzpatrick (1997), `else` is a branch-point that contributes
        // to the `C` (Conditions) component. tree-sitter-c exposes it as a
        // dedicated `else_clause` named node, so an `if (x > 0) {...} else
        // {...}` should yield: +1 if + 1 `>` comparison + 1 else = 3
        // conditions. Matches how TypeScript / Tsx treat
        // `ElseClause` in this file.
        check_metrics::<CParser>(
            "int f(int x) {
                 if (x > 0) {
                     return 1;
                 } else {
                     return 0;
                 }
             }",
            "foo.c",
            |metric| {
                // A: 0 (no assignments). B: 0 (no calls). C: 3 (if + `>` + else).
                assert_eq!(metric.abc.assignments_sum(), 0.0);
                assert_eq!(metric.abc.branches_sum(), 0.0);
                assert_eq!(metric.abc.conditions_sum(), 3.0);
            },
        );
    }
}
