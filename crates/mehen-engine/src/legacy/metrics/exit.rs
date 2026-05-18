use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::legacy::checker::Checker;
use crate::legacy::langs::{CCode, GoCode, KotlinCode, RubyCode};
use crate::legacy::languages::{C, Go, Kotlin, Ruby};
use crate::legacy::node::Node;

/// The `NExit` metric.
///
/// This metric counts the number of possible exit points
/// from a function/method.
#[derive(Debug, Clone)]
pub(crate) struct Stats {
    exit: usize,
    exit_sum: usize,
    total_space_functions: usize,
    exit_min: usize,
    exit_max: usize,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            exit: 0,
            exit_sum: 0,
            total_space_functions: 1,
            exit_min: usize::MAX,
            exit_max: 0,
        }
    }
}

impl Serialize for Stats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("nexits", 4)?;
        st.serialize_field("sum", &self.exit_sum())?;
        st.serialize_field("average", &self.exit_average())?;
        st.serialize_field("min", &self.exit_min())?;
        st.serialize_field("max", &self.exit_max())?;
        st.end()
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "sum: {}, average: {} min: {}, max: {}",
            self.exit_sum(),
            self.exit_average(),
            self.exit_min(),
            self.exit_max()
        )
    }
}

impl Stats {
    /// Merges a second `NExit` metric into the first one
    pub(crate) fn merge(&mut self, other: &Self) {
        self.exit_max = self.exit_max.max(other.exit_max);
        self.exit_min = self.exit_min.min(other.exit_min);
        self.exit_sum += other.exit_sum;
    }

    /// Returns the `NExit` metric sum value
    pub(crate) fn exit_sum(&self) -> f64 {
        self.exit_sum as f64
    }
    /// Returns the `NExit` metric  minimum value
    pub(crate) fn exit_min(&self) -> f64 {
        self.exit_min as f64
    }
    /// Returns the `NExit` metric maximum value
    pub(crate) fn exit_max(&self) -> f64 {
        self.exit_max as f64
    }

    /// Returns the `NExit` metric average value
    ///
    /// This value is computed dividing the `NExit` value
    /// for the total number of functions/closures in a space.
    ///
    /// If there are no functions in a code, its value is `NAN`.
    pub(crate) fn exit_average(&self) -> f64 {
        self.exit_sum() / self.total_space_functions as f64
    }
    #[inline(always)]
    pub(crate) fn compute_sum(&mut self) {
        self.exit_sum += self.exit;
    }
    #[inline(always)]
    pub(crate) fn compute_minmax(&mut self) {
        self.exit_max = self.exit_max.max(self.exit);
        self.exit_min = self.exit_min.min(self.exit);
        self.compute_sum();
    }
    pub(crate) fn finalize(&mut self, total_space_functions: usize) {
        self.total_space_functions = total_space_functions;
    }
}

pub(crate) trait Exit
where
    Self: Checker,
{
    fn compute(node: &Node, stats: &mut Stats);
}

impl Exit for GoCode {
    fn compute(node: &Node, stats: &mut Stats) {
        if matches!(node.kind_id().into(), Go::ReturnStatement) {
            stats.exit += 1;
        }
    }
}

impl Exit for KotlinCode {
    fn compute(node: &Node, stats: &mut Stats) {
        // Function exit points in Kotlin are bare `return` and `throw` — both
        // transfer control out of the enclosing function. The grammar wraps
        // all jumps (`return`, `throw`, `continue`, `break`, and their `@label`
        // forms) in a single `jump_expression` named node, so we look at the
        // lead keyword child to filter out loop-local `continue` / `break` and
        // lambda-local `return@label`.
        //
        // Matches the spirit of mozilla/rust-code-analysis's exit metric for
        // other languages (e.g. Rust counts `return`/`?`, TypeScript counts
        // `return`/`throw`).
        if node.kind_id() == Kotlin::JumpExpression {
            let lead = node.child(0).map(|c| c.kind_id().into());
            if matches!(lead, Some(Kotlin::Return) | Some(Kotlin::Throw)) {
                stats.exit += 1;
            }
        }
    }
}

impl Exit for RubyCode {
    fn compute(node: &Node, stats: &mut Stats) {
        // Count language-level exits from a method/closure:
        // `return`, `break`, and `next`. `yield` hands control back to the
        // caller's block but does not exit the enclosing method, so it is
        // intentionally excluded.
        if matches!(
            node.kind_id().into(),
            Ruby::Return | Ruby::Return2 | Ruby::Break | Ruby::Break2 | Ruby::Next | Ruby::Next2
        ) {
            stats.exit += 1;
        }
    }
}

// No languages require empty Exit implementations
// implement_metric_trait!(Exit);

impl Exit for CCode {
    fn compute(node: &Node, stats: &mut Stats) {
        // Only `return` exits a function. `break` / `continue` are loop
        // flow, not function-exit — same convention used for Ruby / Kotlin.
        // `goto` stays within the same function.
        if matches!(node.kind_id().into(), C::ReturnStatement) {
            stats.exit += 1;
        }
    }
}

// Markdown documents have no return statements.
#[cfg(feature = "markdown")]
impl Exit for crate::legacy::langs::MarkdownCode {
    fn compute(_node: &Node, _stats: &mut Stats) {}
}

#[cfg(test)]
mod tests {
    use crate::legacy::langs::{GoParser, KotlinParser, RubyParser};
    use crate::legacy::tools::check_metrics;

    #[test]
    fn go_no_exit() {
        check_metrics::<GoParser>("var a = 42", "foo.go", |metric| {
            // 0 functions
            insta::assert_json_snapshot!(
                metric.nexits,
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
    fn go_simple_function() {
        check_metrics::<GoParser>(
            "package main

            func max(a, b int) int {
                if a > b {
                    return a
                }
                return b
            }",
            "foo.go",
            |metric| {
                // 2 exits / 1 function
                insta::assert_json_snapshot!(
                    metric.nexits,
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
    fn go_multiple_functions() {
        check_metrics::<GoParser>(
            "package main

            func f1() int {
                return 1
            }

            func f2() int {
                return 2
            }",
            "foo.go",
            |metric| {
                // 2 exits / 2 functions
                insta::assert_json_snapshot!(
                    metric.nexits,
                    @r###"
                    {
                      "sum": 2.0,
                      "average": 1.0,
                      "min": 0.0,
                      "max": 1.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_no_exit() {
        check_metrics::<RubyParser>("a = 42", "foo.rb", |metric| {
            insta::assert_json_snapshot!(
                metric.nexits,
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
                 return a if a > b
                 return b
             end",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.nexits,
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
    fn kotlin_return_and_throw_count_as_exits() {
        check_metrics::<KotlinParser>(
            "fun f(a: Int): Int {
                 if (a < 0) {
                     throw IllegalArgumentException(\"bad\")
                 }
                 return a
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.nexits,
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
    fn kotlin_labeled_lambda_return_does_not_count_as_function_exit() {
        check_metrics::<KotlinParser>(
            "fun f(xs: List<Int>) {
                 xs.forEach { x ->
                     if (x < 0) return@forEach
                 }
             }",
            "foo.kt",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.nexits,
                    @r###"
                    {
                      "sum": 0.0,
                      "average": 0.0,
                      "min": 0.0,
                      "max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_break_and_next() {
        // Both `break` and `next` are counted as exits; `yield` is not.
        check_metrics::<RubyParser>(
            "def f(xs)
                 xs.each do |x|
                   next if x.nil?
                   break if x.stop?
                   yield x
                 end
             end",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.nexits,
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
}
