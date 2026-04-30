use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::checker::Checker;
use crate::langs::{GoCode, PythonCode, RubyCode, RustCode, TsxCode, TypescriptCode};
use crate::languages::{Go, Python, Ruby, Rust, Tsx, Typescript};
use crate::node::Node;

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

impl Abc for RustCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use Rust::*;

        match node.kind_id().into() {
            // A: assignments (`=`), compound assignments (`+=`, `-=`, ...) and
            // `let` bindings with an initializer.
            AssignmentExpression | CompoundAssignmentExpr => {
                stats.assignments += 1.;
            }
            LetDeclaration if node.is_child(EQ as u16) => {
                stats.assignments += 1.;
            }
            // B: function/method calls and macro invocations transfer control.
            CallExpression | MacroInvocation => {
                stats.branches += 1.;
            }
            // C: conditional constructs, `?`, match arms, and comparison/logical operators.
            IfExpression | MatchExpression | WhileExpression | LoopExpression | ForExpression
            | MatchArm | MatchArm2 | TryExpression | EQEQ | BANGEQ | LT | GT | LTEQ | GTEQ
            | AMPAMP | PIPEPIPE => {
                stats.conditions += 1.;
            }
            _ => {}
        }
    }
}

impl Abc for PythonCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use Python::*;

        match node.kind_id().into() {
            // A: assignment and augmented assignment (`x = ...`, `x += ...`).
            Assignment | AugmentedAssignment => {
                stats.assignments += 1.;
            }
            // B: function/method calls.
            Call => {
                stats.branches += 1.;
            }
            // C: conditional constructs, comparison and boolean operators.
            IfStatement
            | ElifClause
            | ElseClause
            | ConditionalExpression
            | MatchStatement
            | CaseClause
            | ForStatement
            | WhileStatement
            | TryStatement
            | ExceptClause
            | ComparisonOperator
            | BooleanOperator => {
                stats.conditions += 1.;
            }
            _ => {}
        }
    }
}

impl Abc for TypescriptCode {
    fn compute(node: &Node, stats: &mut Stats) {
        ts_abc_compute::<Typescript>(node, stats);
    }
}

impl Abc for TsxCode {
    fn compute(node: &Node, stats: &mut Stats) {
        ts_abc_compute::<Tsx>(node, stats);
    }
}

trait TsAbcKinds {
    const ASSIGNMENT_EXPRESSION: u16;
    const AUGMENTED_ASSIGNMENT_EXPRESSION: u16;
    const UPDATE_EXPRESSION: u16;
    const VARIABLE_DECLARATOR: u16;
    const CALL_EXPRESSION: u16;
    const CALL_EXPRESSION_2: u16;
    const NEW_EXPRESSION: u16;
    const IF_STATEMENT: u16;
    const ELSE_CLAUSE: u16;
    const TERNARY_EXPRESSION: u16;
    const SWITCH_CASE: u16;
    const SWITCH_DEFAULT: u16;
    const FOR_STATEMENT: u16;
    const FOR_IN_STATEMENT: u16;
    const WHILE_STATEMENT: u16;
    const DO_STATEMENT: u16;
    const CATCH_CLAUSE: u16;
    const EQEQ: u16;
    const EQEQEQ: u16;
    const BANGEQ: u16;
    const BANGEQEQ: u16;
    const LT: u16;
    const LTEQ: u16;
    const GT: u16;
    const GTEQ: u16;
    const AMPAMP: u16;
    const PIPEPIPE: u16;
    const EQ: u16;
}

impl TsAbcKinds for Typescript {
    const ASSIGNMENT_EXPRESSION: u16 = Typescript::AssignmentExpression as u16;
    const AUGMENTED_ASSIGNMENT_EXPRESSION: u16 = Typescript::AugmentedAssignmentExpression as u16;
    const UPDATE_EXPRESSION: u16 = Typescript::UpdateExpression as u16;
    const VARIABLE_DECLARATOR: u16 = Typescript::VariableDeclarator as u16;
    const CALL_EXPRESSION: u16 = Typescript::CallExpression as u16;
    const CALL_EXPRESSION_2: u16 = Typescript::CallExpression2 as u16;
    const NEW_EXPRESSION: u16 = Typescript::NewExpression as u16;
    const IF_STATEMENT: u16 = Typescript::IfStatement as u16;
    const ELSE_CLAUSE: u16 = Typescript::ElseClause as u16;
    const TERNARY_EXPRESSION: u16 = Typescript::TernaryExpression as u16;
    const SWITCH_CASE: u16 = Typescript::SwitchCase as u16;
    const SWITCH_DEFAULT: u16 = Typescript::SwitchDefault as u16;
    const FOR_STATEMENT: u16 = Typescript::ForStatement as u16;
    const FOR_IN_STATEMENT: u16 = Typescript::ForInStatement as u16;
    const WHILE_STATEMENT: u16 = Typescript::WhileStatement as u16;
    const DO_STATEMENT: u16 = Typescript::DoStatement as u16;
    const CATCH_CLAUSE: u16 = Typescript::CatchClause as u16;
    const EQEQ: u16 = Typescript::EQEQ as u16;
    const EQEQEQ: u16 = Typescript::EQEQEQ as u16;
    const BANGEQ: u16 = Typescript::BANGEQ as u16;
    const BANGEQEQ: u16 = Typescript::BANGEQEQ as u16;
    const LT: u16 = Typescript::LT as u16;
    const LTEQ: u16 = Typescript::LTEQ as u16;
    const GT: u16 = Typescript::GT as u16;
    const GTEQ: u16 = Typescript::GTEQ as u16;
    const AMPAMP: u16 = Typescript::AMPAMP as u16;
    const PIPEPIPE: u16 = Typescript::PIPEPIPE as u16;
    const EQ: u16 = Typescript::EQ as u16;
}

impl TsAbcKinds for Tsx {
    const ASSIGNMENT_EXPRESSION: u16 = Tsx::AssignmentExpression as u16;
    const AUGMENTED_ASSIGNMENT_EXPRESSION: u16 = Tsx::AugmentedAssignmentExpression as u16;
    const UPDATE_EXPRESSION: u16 = Tsx::UpdateExpression as u16;
    const VARIABLE_DECLARATOR: u16 = Tsx::VariableDeclarator as u16;
    const CALL_EXPRESSION: u16 = Tsx::CallExpression as u16;
    const CALL_EXPRESSION_2: u16 = Tsx::CallExpression2 as u16;
    const NEW_EXPRESSION: u16 = Tsx::NewExpression as u16;
    const IF_STATEMENT: u16 = Tsx::IfStatement as u16;
    const ELSE_CLAUSE: u16 = Tsx::ElseClause as u16;
    const TERNARY_EXPRESSION: u16 = Tsx::TernaryExpression as u16;
    const SWITCH_CASE: u16 = Tsx::SwitchCase as u16;
    const SWITCH_DEFAULT: u16 = Tsx::SwitchDefault as u16;
    const FOR_STATEMENT: u16 = Tsx::ForStatement as u16;
    const FOR_IN_STATEMENT: u16 = Tsx::ForInStatement as u16;
    const WHILE_STATEMENT: u16 = Tsx::WhileStatement as u16;
    const DO_STATEMENT: u16 = Tsx::DoStatement as u16;
    const CATCH_CLAUSE: u16 = Tsx::CatchClause as u16;
    const EQEQ: u16 = Tsx::EQEQ as u16;
    const EQEQEQ: u16 = Tsx::EQEQEQ as u16;
    const BANGEQ: u16 = Tsx::BANGEQ as u16;
    const BANGEQEQ: u16 = Tsx::BANGEQEQ as u16;
    const LT: u16 = Tsx::LT as u16;
    const LTEQ: u16 = Tsx::LTEQ as u16;
    const GT: u16 = Tsx::GT as u16;
    const GTEQ: u16 = Tsx::GTEQ as u16;
    const AMPAMP: u16 = Tsx::AMPAMP as u16;
    const PIPEPIPE: u16 = Tsx::PIPEPIPE as u16;
    const EQ: u16 = Tsx::EQ as u16;
}

fn ts_abc_compute<K: TsAbcKinds>(node: &Node, stats: &mut Stats) {
    let kind = node.kind_id();

    if kind == K::ASSIGNMENT_EXPRESSION
        || kind == K::AUGMENTED_ASSIGNMENT_EXPRESSION
        || kind == K::UPDATE_EXPRESSION
        || (kind == K::VARIABLE_DECLARATOR && node.is_child(K::EQ))
    {
        stats.assignments += 1.;
    } else if kind == K::CALL_EXPRESSION
        || kind == K::CALL_EXPRESSION_2
        || kind == K::NEW_EXPRESSION
    {
        stats.branches += 1.;
    } else if kind == K::IF_STATEMENT
        || kind == K::ELSE_CLAUSE
        || kind == K::TERNARY_EXPRESSION
        || kind == K::SWITCH_CASE
        || kind == K::SWITCH_DEFAULT
        || kind == K::FOR_STATEMENT
        || kind == K::FOR_IN_STATEMENT
        || kind == K::WHILE_STATEMENT
        || kind == K::DO_STATEMENT
        || kind == K::CATCH_CLAUSE
        || kind == K::EQEQ
        || kind == K::EQEQEQ
        || kind == K::BANGEQ
        || kind == K::BANGEQEQ
        || kind == K::LT
        || kind == K::LTEQ
        || kind == K::GT
        || kind == K::GTEQ
        || kind == K::AMPAMP
        || kind == K::PIPEPIPE
    {
        stats.conditions += 1.;
    }
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
        use crate::languages::Go::*;

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

impl Abc for RubyCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use Ruby::*;

        match node.kind_id().into() {
            // A: every assignment, including compound ones.
            Assignment | Assignment2 | OperatorAssignment | OperatorAssignment2 => {
                stats.assignments += 1.;
            }
            // B: every method call and `yield` (transfers control to a block).
            Call | Call2 | Call3 | Call4 | Yield | Yield2 => {
                stats.branches += 1.;
            }
            // C: every conditional construct and every comparison operator.
            If | Unless | IfModifier | UnlessModifier | When | InClause | Conditional | Rescue
            | RescueModifier | RescueModifier2 | RescueModifier3 | EQEQ | BANGEQ | LT | GT
            | LTEQ | GTEQ | LTEQGT | EQEQEQ | EQTILDE | BANGTILDE => {
                stats.conditions += 1.;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::langs::{GoParser, PythonParser, RubyParser, RustParser, TypescriptParser};
    use crate::tools::check_metrics;

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
    fn rust_abc_basic() {
        check_metrics::<RustParser>(
            "fn f(a: i32, b: i32) -> i32 {
                 let mut x = a;           // +1 A
                 x += b;                  // +1 A
                 log(x);                  // +1 B
                 if x > b {               // +1 C (if) + +1 C (>)
                     return x;
                 }
                 x
             }",
            "foo.rs",
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
    fn python_abc_basic() {
        check_metrics::<PythonParser>(
            "def f(a, b):
    x = a          # +1 A
    x += b         # +1 A
    log(x)         # +1 B
    if x > b:      # +1 C (if) + +1 C (>)
        return x
    return b
",
            "foo.py",
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
    fn typescript_abc_basic() {
        check_metrics::<TypescriptParser>(
            "function f(a: number, b: number): number {
                 let x = a;          // +1 A (declarator with `=`)
                 x += b;             // +1 A
                 log(x);             // +1 B
                 if (x > b) {        // +1 C (if) + +1 C (>)
                     return x;
                 }
                 return b;
             }",
            "foo.ts",
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
    fn ruby_abc_basic() {
        check_metrics::<RubyParser>(
            "def f(a, b)
                 c = a + b    # +1 A
                 log(c)       # +1 B
                 return c if c > 0  # +1 C (if_modifier) + +1 C (>)
             end",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.abc,
                    @r###"
                    {
                      "assignments": 1.0,
                      "branches": 1.0,
                      "conditions": 2.0,
                      "magnitude": 2.449489742783178,
                      "assignments_average": 0.5,
                      "branches_average": 0.5,
                      "conditions_average": 1.0,
                      "assignments_min": 0.0,
                      "assignments_max": 1.0,
                      "branches_min": 0.0,
                      "branches_max": 1.0,
                      "conditions_min": 0.0,
                      "conditions_max": 2.0
                    }"###
                );
            },
        );
    }
}
