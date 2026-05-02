use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::checker::Checker;
use crate::langs::{GoCode, KotlinCode, PythonCode, RubyCode, RustCode, TsxCode, TypescriptCode};
use crate::languages::{Go, Kotlin, Python, Ruby, Rust, Tsx, Typescript};
use crate::node::Node;

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

    /// Returns the `NExit` metric value
    pub(crate) fn exit(&self) -> f64 {
        self.exit as f64
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

impl Exit for PythonCode {
    fn compute(node: &Node, stats: &mut Stats) {
        if matches!(
            node.kind_id().into(),
            Python::ReturnStatement | Python::RaiseStatement
        ) {
            stats.exit += 1;
        }
    }
}

impl Exit for TypescriptCode {
    fn compute(node: &Node, stats: &mut Stats) {
        if matches!(
            node.kind_id().into(),
            Typescript::ReturnStatement | Typescript::ThrowStatement
        ) {
            stats.exit += 1;
        }
    }
}

impl Exit for TsxCode {
    fn compute(node: &Node, stats: &mut Stats) {
        if matches!(
            node.kind_id().into(),
            Tsx::ReturnStatement | Tsx::ThrowStatement
        ) {
            stats.exit += 1;
        }
    }
}

impl Exit for RustCode {
    fn compute(node: &Node, stats: &mut Stats) {
        if matches!(
            node.kind_id().into(),
            Rust::ReturnExpression | Rust::TryExpression
        ) || Self::is_func(node) && node.child_by_field_name("return_type").is_some()
        {
            stats.exit += 1;
        }
    }
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
        // Function exit points in Kotlin are `return` and `throw` — both
        // transfer control out of the enclosing function. The grammar wraps
        // all jumps (`return`, `throw`, `continue`, `break`, and their `@label`
        // forms) in a single `jump_expression` named node, so we look at the
        // lead keyword child to filter out `continue` / `break`, which stay
        // within the loop and don't exit the function.
        //
        // Matches the spirit of mozilla/rust-code-analysis's exit metric for
        // other languages (e.g. Python counts `return`/`raise`, Rust counts
        // `return`/`?`, TypeScript counts `return`/`throw`).
        if node.kind_id() == Kotlin::JumpExpression {
            let lead = node.child(0).map(|c| c.kind_id().into());
            if matches!(
                lead,
                Some(Kotlin::Return) | Some(Kotlin::ReturnAT) | Some(Kotlin::Throw)
            ) {
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

#[cfg(test)]
mod tests {
    use crate::langs::{
        GoParser, KotlinParser, PythonParser, RubyParser, RustParser, TypescriptParser,
    };
    use crate::tools::check_metrics;

    #[test]
    fn python_no_exit() {
        check_metrics::<PythonParser>("a = 42", "foo.py", |metric| {
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
    fn rust_no_exit() {
        check_metrics::<RustParser>("let a = 42;", "foo.rs", |metric| {
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
    fn rust_question_mark() {
        check_metrics::<RustParser>("let _ = a? + b? + c?;", "foo.rs", |metric| {
            // 0 functions
            insta::assert_json_snapshot!(
                metric.nexits,
                @r###"
                    {
                      "sum": 3.0,
                      "average": null,
                      "min": 3.0,
                      "max": 3.0
                    }"###
            );
        });
    }

    #[test]
    fn python_simple_function() {
        check_metrics::<PythonParser>(
            "def f(a, b):
                 if a:
                     return a",
            "foo.py",
            |metric| {
                println!("{:?}", metric.nexits);
                // 1 function
                insta::assert_json_snapshot!(
                    metric.nexits,
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
    fn python_more_functions() {
        check_metrics::<PythonParser>(
            "def f(a, b):
                 if a:
                     return a
            def f(a, b):
                 if b:
                     return b",
            "foo.py",
            |metric| {
                // 2 functions
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
    fn python_nested_functions() {
        check_metrics::<PythonParser>(
            "def f(a, b):
                 def foo(a):
                     if a:
                         return 1
                 bar = lambda a: lambda b: b or True or True
                 return bar(foo(a))(a)",
            "foo.py",
            |metric| {
                // 2 functions + 2 lambdas = 4
                insta::assert_json_snapshot!(
                    metric.nexits,
                    @r###"
                    {
                      "sum": 2.0,
                      "average": 0.5,
                      "min": 0.0,
                      "max": 1.0
                    }"###
                );
            },
        );
    }

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
    fn python_raise_counts_as_exit() {
        check_metrics::<PythonParser>(
            "def f(a):
                 if a < 0:
                     raise ValueError('bad')
                 return a",
            "foo.py",
            |metric| {
                // 1 function, 2 exits (raise + return)
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
    fn typescript_throw_counts_as_exit() {
        check_metrics::<TypescriptParser>(
            "function f(a: number): number {
                 if (a < 0) {
                     throw new Error('bad');
                 }
                 return a;
             }",
            "foo.ts",
            |metric| {
                // 1 function, 2 exits (throw + return)
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
