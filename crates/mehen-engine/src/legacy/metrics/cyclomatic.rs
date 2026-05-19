use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::legacy::checker::Checker;
use crate::legacy::langs::{CCode, GoCode, KotlinCode};
use crate::legacy::languages::{C, Kotlin};
use crate::legacy::node::Node;

/// The `Cyclomatic` metric.
#[derive(Debug, Clone)]
pub(crate) struct Stats {
    cyclomatic_sum: f64,
    cyclomatic: f64,
    n: usize,
    cyclomatic_max: f64,
    cyclomatic_min: f64,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            cyclomatic_sum: 0.,
            cyclomatic: 1.,
            n: 1,
            cyclomatic_max: 0.,
            cyclomatic_min: f64::MAX,
        }
    }
}

impl Serialize for Stats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("cyclomatic", 4)?;
        st.serialize_field("sum", &self.cyclomatic_sum())?;
        st.serialize_field("average", &self.cyclomatic_average())?;
        st.serialize_field("min", &self.cyclomatic_min())?;
        st.serialize_field("max", &self.cyclomatic_max())?;
        st.end()
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "sum: {}, average: {}, min: {}, max: {}",
            self.cyclomatic_sum(),
            self.cyclomatic_average(),
            self.cyclomatic_min(),
            self.cyclomatic_max()
        )
    }
}

impl Stats {
    /// Merges a second `Cyclomatic` metric into the first one
    pub(crate) fn merge(&mut self, other: &Self) {
        //Calculate minimum and maximum values
        self.cyclomatic_max = self.cyclomatic_max.max(other.cyclomatic_max);
        self.cyclomatic_min = self.cyclomatic_min.min(other.cyclomatic_min);

        self.cyclomatic_sum += other.cyclomatic_sum;
        self.n += other.n;
    }

    /// Returns the `Cyclomatic` metric value
    pub(crate) fn cyclomatic(&self) -> f64 {
        self.cyclomatic
    }
    /// Returns the sum
    pub(crate) fn cyclomatic_sum(&self) -> f64 {
        self.cyclomatic_sum
    }

    /// Returns the `Cyclomatic` metric average value
    ///
    /// This value is computed dividing the `Cyclomatic` value for the
    /// number of spaces.
    pub(crate) fn cyclomatic_average(&self) -> f64 {
        self.cyclomatic_sum() / self.n as f64
    }
    /// Returns the `Cyclomatic` maximum value
    pub(crate) fn cyclomatic_max(&self) -> f64 {
        self.cyclomatic_max
    }
    /// Returns the `Cyclomatic` minimum value
    pub(crate) fn cyclomatic_min(&self) -> f64 {
        self.cyclomatic_min
    }
    #[inline(always)]
    pub(crate) fn compute_sum(&mut self) {
        self.cyclomatic_sum += self.cyclomatic;
    }
    #[inline(always)]
    pub(crate) fn compute_minmax(&mut self) {
        self.cyclomatic_max = self.cyclomatic_max.max(self.cyclomatic);
        self.cyclomatic_min = self.cyclomatic_min.min(self.cyclomatic);
        self.compute_sum();
    }
}

pub(crate) trait Cyclomatic
where
    Self: Checker,
{
    fn compute(node: &Node, stats: &mut Stats);
}

impl Cyclomatic for GoCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use crate::legacy::languages::Go::*;

        match node.kind_id().into() {
            If | For | ExpressionCase | TypeCase | CommunicationCase | AMPAMP | PIPEPIPE => {
                stats.cyclomatic += 1.;
            }
            // `default_case` is shared between switch and select in the Go
            // grammar. In switch it is fallthrough (no branch), but in
            // `select` the default branch is an additional executable path
            // and should count as a decision point.
            DefaultCase
                if node
                    .parent()
                    .is_some_and(|p| p.kind_id() == SelectStatement as u16) =>
            {
                stats.cyclomatic += 1.;
            }
            _ => {}
        }
    }
}

impl Cyclomatic for KotlinCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use Kotlin::*;

        // Decision-point set is aligned with SonarKotlin's
        // `CyclomaticComplexityVisitor`: every `KtIfExpression` (including
        // else-if branches, because each parses as a nested `if_expression`),
        // every loop (`for`/`while`/`do-while`), every `when_entry`, and
        // each short-circuit `&&`/`||`. `catch` is intentionally excluded —
        // SonarKotlin does not count it towards cyclomatic complexity.
        // Reference: sonar-kotlin-metrics/.../CyclomaticComplexityVisitor.kt
        match node.kind_id().into() {
            IfExpression | ForStatement | WhileStatement | DoWhileStatement | WhenEntry
            | AMPAMP | PIPEPIPE => {
                stats.cyclomatic += 1.;
            }
            _ => {}
        }
    }
}

// No languages require empty Cyclomatic implementations
// implement_metric_trait!(Cyclomatic);

impl Cyclomatic for CCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use C::*;

        match node.kind_id().into() {
            // Decision-point set aligned with Sonar's cyclomatic rule for C:
            // `if`, every `case`, loops, ternary (`conditional_expression`),
            // and each short-circuit boolean operator (`&&` / `||`).
            // `switch` itself is not a decision (cases are); `default` is
            // fallthrough.
            IfStatement
            | CaseStatement
            | ForStatement
            | WhileStatement
            | DoStatement
            | ConditionalExpression
            | AMPAMP
            | PIPEPIPE => {
                stats.cyclomatic += 1.;
            }
            _ => {}
        }
    }
}

// Markdown is a documentation language; cyclomatic is a code metric. The
// dedicated Markdown pipeline computes its own MRPC analogue (Phase B).
#[cfg(feature = "markdown")]
impl Cyclomatic for crate::legacy::langs::MarkdownCode {
    fn compute(_node: &Node, _stats: &mut Stats) {}
}

#[cfg(test)]
mod tests {
    use crate::legacy::langs::{GoParser, KotlinParser};
    use crate::legacy::tools::check_metrics;

    #[test]
    fn go_simple_function() {
        check_metrics::<GoParser>(
            "package main

            func calculate(a, b int) int { // +2 (+1 unit space)
                if a > b { // +1
                    return a
                }
                return b
            }",
            "foo.go",
            |metric| {
                // nspace = 2 (func and unit)
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 1.5,
                      "min": 1.0,
                      "max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_switch_statement() {
        check_metrics::<GoParser>(
            "package main

            func grade(score int) string { // +2 (+1 unit space)
                switch { // switch itself doesn't add, cases do
                case score >= 90: // +1
                    return \"A\"
                case score >= 80: // +1
                    return \"B\"
                case score >= 70: // +1
                    return \"C\"
                default: // default is fallthrough, not a decision point
                    return \"F\"
                }
            }",
            "foo.go",
            |metric| {
                // nspace = 2 (func and unit)
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 5.0,
                      "average": 2.5,
                      "min": 1.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_select_default_counts() {
        // `default` in a `switch` is fallthrough and should NOT count,
        // but `default` in a `select` is an additional executable
        // communication branch and SHOULD count.
        check_metrics::<GoParser>(
            "package main

            func f(ch chan int) { // +2 (+1 unit space)
                select { // +1 CommunicationCase
                case v := <-ch:
                    _ = v
                default: // +1 default branch of select
                }
            }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 4.0,
                      "average": 2.0,
                      "min": 1.0,
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

            func check(a, b, c bool) bool { // +2 (+1 unit space)
                if a && b || c { // +3 (+1 if, +1 &&, +1 ||)
                    return true
                }
                return false
            }",
            "foo.go",
            |metric| {
                // nspace = 2 (func and unit)
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 5.0,
                      "average": 2.5,
                      "min": 1.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_simple_function() {
        check_metrics::<KotlinParser>(
            "fun f(a: Int, b: Int): Int { // +2 (+1 unit space, +1 fun)
                 if (a > b) { // +1
                     return a
                 }
                 return b
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 3.0,
                      "average": 1.5,
                      "min": 1.0,
                      "max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_when_branches_count() {
        // `when` itself doesn't add; each branch (`when_entry`) does.
        check_metrics::<KotlinParser>(
            "fun grade(score: Int): String { // +2 (+1 unit, +1 fun)
                 return when { // +0
                     score >= 90 -> \"A\" // +1
                     score >= 80 -> \"B\" // +1
                     score >= 70 -> \"C\" // +1
                     else -> \"F\"       // +1 (else is its own when_entry)
                 }
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 6.0,
                      "average": 3.0,
                      "min": 1.0,
                      "max": 5.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_try_catch_counts_catch_not_try() {
        // Aligns with SonarKotlin's `CyclomaticComplexityVisitor`: `try`
        // itself is NOT a decision point, but each `catch` is NOT either —
        // SonarKotlin counts `catch` only in cognitive complexity, not
        // cyclomatic. Reference:
        //   sonar-kotlin-metrics/.../CyclomaticComplexityVisitor.kt
        check_metrics::<KotlinParser>(
            "fun f() { // +2 (+1 unit, +1 fun)
                 try {
                     risky()
                 } catch (e: Exception) {
                     // catch does not add cyclomatic complexity per SonarKotlin
                 }
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 2.0,
                      "average": 1.0,
                      "min": 1.0,
                      "max": 1.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_logical_operators() {
        check_metrics::<KotlinParser>(
            "fun check(a: Boolean, b: Boolean, c: Boolean): Boolean { // +2
                 if (a && b || c) { // +3 (+1 if, +1 &&, +1 ||)
                     return true
                 }
                 return false
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.cyclomatic,
                    @r###"
                    {
                      "sum": 5.0,
                      "average": 2.5,
                      "min": 1.0,
                      "max": 4.0
                    }"###
                );
            },
        );
    }
}
