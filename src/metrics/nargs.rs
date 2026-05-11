use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::checker::Checker;
use crate::langs::{
    CCode, GoCode, KotlinCode, PowershellCode, PythonCode, RubyCode, RustCode, TsxCode,
    TypescriptCode,
};
#[cfg(test)]
use crate::langs::{CParser, GoParser, KotlinParser, PythonParser, RubyParser, RustParser};
use crate::languages::{C, Go, Kotlin, Powershell};
use crate::macros::implement_metric_trait;
use crate::node::Node;
use crate::traits::Search;

/// The `NArgs` metric.
///
/// This metric counts the number of arguments
/// of functions/closures.
#[derive(Debug, Clone)]
pub(crate) struct Stats {
    fn_nargs: usize,
    closure_nargs: usize,
    fn_nargs_sum: usize,
    closure_nargs_sum: usize,
    fn_nargs_min: usize,
    closure_nargs_min: usize,
    fn_nargs_max: usize,
    closure_nargs_max: usize,
    total_functions: usize,
    total_closures: usize,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            fn_nargs: 0,
            closure_nargs: 0,
            fn_nargs_sum: 0,
            closure_nargs_sum: 0,
            fn_nargs_min: usize::MAX,
            closure_nargs_min: usize::MAX,
            fn_nargs_max: 0,
            closure_nargs_max: 0,
            total_functions: 0,
            total_closures: 0,
        }
    }
}

impl Serialize for Stats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("nargs", 10)?;
        st.serialize_field("total_functions", &self.fn_args_sum())?;
        st.serialize_field("total_closures", &self.closure_args_sum())?;
        st.serialize_field("average_functions", &self.fn_args_average())?;
        st.serialize_field("average_closures", &self.closure_args_average())?;
        st.serialize_field("total", &self.nargs_total())?;
        st.serialize_field("average", &self.nargs_average())?;
        st.serialize_field("functions_min", &self.fn_args_min())?;
        st.serialize_field("functions_max", &self.fn_args_max())?;
        st.serialize_field("closures_min", &self.closure_args_min())?;
        st.serialize_field("closures_max", &self.closure_args_max())?;
        st.end()
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "total_functions: {}, total_closures: {}, average_functions: {}, average_closures: {}, total: {}, average: {}, functions_min: {}, functions_max: {}, closures_min: {}, closures_max: {}",
            self.fn_args(),
            self.closure_args(),
            self.fn_args_average(),
            self.closure_args_average(),
            self.nargs_total(),
            self.nargs_average(),
            self.fn_args_min(),
            self.fn_args_max(),
            self.closure_args_min(),
            self.closure_args_max()
        )
    }
}

impl Stats {
    /// Merges a second `NArgs` metric into the first one
    pub(crate) fn merge(&mut self, other: &Self) {
        self.closure_nargs_min = self.closure_nargs_min.min(other.closure_nargs_min);
        self.closure_nargs_max = self.closure_nargs_max.max(other.closure_nargs_max);
        self.fn_nargs_min = self.fn_nargs_min.min(other.fn_nargs_min);
        self.fn_nargs_max = self.fn_nargs_max.max(other.fn_nargs_max);
        self.fn_nargs_sum += other.fn_nargs_sum;
        self.closure_nargs_sum += other.closure_nargs_sum;
    }

    /// Returns the number of function arguments in a space.
    #[inline(always)]
    pub(crate) fn fn_args(&self) -> f64 {
        self.fn_nargs as f64
    }

    /// Returns the number of closure arguments in a space.
    #[inline(always)]
    pub(crate) fn closure_args(&self) -> f64 {
        self.closure_nargs as f64
    }

    /// Returns the number of function arguments sum in a space.
    #[inline(always)]
    pub(crate) fn fn_args_sum(&self) -> f64 {
        self.fn_nargs_sum as f64
    }

    /// Returns the number of closure arguments sum in a space.
    #[inline(always)]
    pub(crate) fn closure_args_sum(&self) -> f64 {
        self.closure_nargs_sum as f64
    }

    /// Returns the average number of functions arguments in a space.
    #[inline(always)]
    pub(crate) fn fn_args_average(&self) -> f64 {
        self.fn_nargs_sum as f64 / self.total_functions.max(1) as f64
    }

    /// Returns the average number of closures arguments in a space.
    #[inline(always)]
    pub(crate) fn closure_args_average(&self) -> f64 {
        self.closure_nargs_sum as f64 / self.total_closures.max(1) as f64
    }

    /// Returns the total number of arguments of each function and
    /// closure in a space.
    #[inline(always)]
    pub(crate) fn nargs_total(&self) -> f64 {
        self.fn_args_sum() + self.closure_args_sum()
    }

    /// Returns the `NArgs` metric average value
    ///
    /// This value is computed dividing the `NArgs` value
    /// for the total number of functions/closures in a space.
    #[inline(always)]
    pub(crate) fn nargs_average(&self) -> f64 {
        self.nargs_total() / (self.total_functions + self.total_closures).max(1) as f64
    }
    /// Returns the minimum number of function arguments in a space.
    #[inline(always)]
    pub(crate) fn fn_args_min(&self) -> f64 {
        self.fn_nargs_min as f64
    }
    /// Returns the maximum number of function arguments in a space.
    #[inline(always)]
    pub(crate) fn fn_args_max(&self) -> f64 {
        self.fn_nargs_max as f64
    }
    /// Returns the minimum number of closure arguments in a space.
    #[inline(always)]
    pub(crate) fn closure_args_min(&self) -> f64 {
        self.closure_nargs_min as f64
    }
    /// Returns the maximum number of closure arguments in a space.
    #[inline(always)]
    pub(crate) fn closure_args_max(&self) -> f64 {
        self.closure_nargs_max as f64
    }
    #[inline(always)]
    pub(crate) fn compute_sum(&mut self) {
        self.closure_nargs_sum += self.closure_nargs;
        self.fn_nargs_sum += self.fn_nargs;
    }
    #[inline(always)]
    pub(crate) fn compute_minmax(&mut self) {
        self.closure_nargs_min = self.closure_nargs_min.min(self.closure_nargs);
        self.closure_nargs_max = self.closure_nargs_max.max(self.closure_nargs);
        self.fn_nargs_min = self.fn_nargs_min.min(self.fn_nargs);
        self.fn_nargs_max = self.fn_nargs_max.max(self.fn_nargs);
        self.compute_sum();
    }
    pub(crate) fn finalize(&mut self, total_functions: usize, total_closures: usize) {
        self.total_functions = total_functions;
        self.total_closures = total_closures;
    }
}

#[inline(always)]
fn compute_args<T: Checker>(node: &Node, nargs: &mut usize) {
    if let Some(params) = node.child_by_field_name("parameters") {
        let node_params = params;
        node_params.act_on_child(&mut |n| {
            if !T::is_non_arg(n) {
                *nargs += 1;
            }
        });
    }
}

#[inline(always)]
fn compute_go_args(node: &Node, nargs: &mut usize) {
    if let Some(params) = node.child_by_field_name("parameters") {
        params.act_on_child(&mut |n| match n.kind_id().into() {
            Go::ParameterDeclaration | Go::VariadicParameterDeclaration => {
                let mut names = 0;
                n.act_on_child(&mut |child| {
                    if matches!(
                        child.kind_id().into(),
                        Go::Identifier | Go::Identifier2 | Go::Identifier3 | Go::BlankIdentifier
                    ) {
                        names += 1;
                    }
                });
                *nargs += names.max(1);
            }
            _ => {}
        });
    }
}

#[inline(always)]
fn compute_kotlin_parameter_list(params: &Node, nargs: &mut usize) {
    params.act_on_child(&mut |n| match n.kind_id().into() {
        Kotlin::ClassParameter
        | Kotlin::FunctionValueParameter
        | Kotlin::Parameter
        | Kotlin::ParameterWithOptionalType
        | Kotlin::VariableDeclaration => *nargs += 1,
        _ => {}
    });
}

#[inline(always)]
fn compute_kotlin_args(node: &Node, nargs: &mut usize) {
    node.act_on_child(&mut |child| match child.kind_id().into() {
        Kotlin::FunctionValueParameters | Kotlin::LambdaParameters => {
            compute_kotlin_parameter_list(child, nargs);
        }
        Kotlin::ParameterWithOptionalType if node.kind_id() == Kotlin::Setter => {
            *nargs += 1;
        }
        _ => {}
    });
}

pub(crate) trait NArgs
where
    Self: Checker + Sized,
{
    fn compute(node: &Node, stats: &mut Stats) {
        if Self::is_func(node) {
            compute_args::<Self>(node, &mut stats.fn_nargs);
            return;
        }

        if Self::is_closure(node) {
            compute_args::<Self>(node, &mut stats.closure_nargs);
        }
    }
}

impl NArgs for GoCode {
    fn compute(node: &Node, stats: &mut Stats) {
        if Self::is_func(node) {
            compute_go_args(node, &mut stats.fn_nargs);
            return;
        }

        if Self::is_closure(node) {
            compute_go_args(node, &mut stats.closure_nargs);
        }
    }
}

impl NArgs for KotlinCode {
    fn compute(node: &Node, stats: &mut Stats) {
        if Self::is_func(node) {
            compute_kotlin_args(node, &mut stats.fn_nargs);
            return;
        }

        if Self::is_closure(node) {
            compute_kotlin_args(node, &mut stats.closure_nargs);
        }
    }
}

#[inline(always)]
fn compute_powershell_args(node: &Node, nargs: &mut usize) {
    use Powershell::*;

    // PowerShell parameter declarations can appear in three shapes:
    //   1. `function_statement` > `function_parameter_declaration` >
    //      `parameter_list` > `script_parameter` (each `script_parameter`
    //      is one named `$var`).
    //   2. For script-block closures: `script_block_expression` >
    //      `param_block` > `parameter_list` > `script_parameter`.
    //   3. For class methods: `class_method_definition` >
    //      `class_method_parameter_list` > `class_method_parameter`.
    //
    // The walker recurses *only* through the immediate structural
    // wrappers that sit between the entry node and the parameter list
    // (`function_parameter_declaration` for functions, `param_block` for
    // closures). It deliberately does NOT recurse into the body
    // `script_block` / `script_block_body` / `statement_list`: each
    // nested function or closure inside a body is its own FuncSpace and
    // its own call to `NArgs::compute`, so descending into the body
    // would double-count nested params against the enclosing
    // function / closure. Regression test:
    // `powershell_nested_closure_params_do_not_count_toward_outer_fn`.
    enum Kind {
        Script,
        Method,
    }

    fn walk(node: &Node, nargs: &mut usize, kind: &Kind) {
        for child in node.children() {
            match child.kind_id().into() {
                Powershell::ParameterList => {
                    if matches!(kind, Kind::Script) {
                        for p in child.children() {
                            if p.kind_id() == Powershell::ScriptParameter {
                                *nargs += 1;
                            }
                        }
                    }
                }
                Powershell::ClassMethodParameterList => {
                    if matches!(kind, Kind::Method) {
                        for p in child.children() {
                            if p.kind_id() == Powershell::ClassMethodParameter {
                                *nargs += 1;
                            }
                        }
                    }
                }
                // Recurse only into the thin structural wrappers that
                // directly enclose the parameter list. See the comment
                // above for why the body `script_block` is intentionally
                // excluded.
                Powershell::FunctionParameterDeclaration | Powershell::ParamBlock => {
                    walk(&child, nargs, kind)
                }
                _ => {}
            }
        }
    }

    match node.kind_id().into() {
        FunctionStatement | ScriptBlockExpression => walk(node, nargs, &Kind::Script),
        ClassMethodDefinition => walk(node, nargs, &Kind::Method),
        _ => {}
    }
}

impl NArgs for PowershellCode {
    fn compute(node: &Node, stats: &mut Stats) {
        if Self::is_func(node) {
            compute_powershell_args(node, &mut stats.fn_nargs);
            return;
        }

        if Self::is_closure(node) {
            compute_powershell_args(node, &mut stats.closure_nargs);
        }
    }
}

#[inline(always)]
fn compute_c_args(node: &Node, nargs: &mut usize) {
    // tree-sitter-c nests the parameter list under the innermost
    // `function_declarator`: `function_definition > function_declarator >
    // parameter_list`. Pointer (`int (*f)(...)`) and attributed declarators
    // wrap the `function_declarator`, so walk inward via the `declarator`
    // field until we find the `function_declarator` whose direct child is
    // the `parameter_list`.
    let mut cur = node.0.child_by_field_name("declarator");
    while let Some(current) = cur {
        if current.kind_id() == C::FunctionDeclarator {
            let mut cursor = current.walk();
            let Some(param_list) = current
                .children(&mut cursor)
                .find(|c| c.kind_id() == C::ParameterList)
            else {
                return;
            };
            let mut list_cursor = param_list.walk();
            let params: Vec<_> = param_list
                .children(&mut list_cursor)
                .filter(|p| p.kind_id() == C::ParameterDeclaration)
                .collect();
            // `(void)` — a sole `parameter_declaration` whose only child is
            // a `primitive_type` (no named declarator) — is C's spelling
            // for "no parameters" and must not be counted. Function
            // *definitions* require named parameters when any exist, so a
            // nameless sole parameter is reliably `(void)` in practice.
            // `variadic_parameter` (`...`) is already filtered above.
            let is_void_only = params.len() == 1
                && params[0].child_count() == 1
                && params[0]
                    .child(0)
                    .is_some_and(|c| c.kind_id() == C::PrimitiveType);
            if !is_void_only {
                *nargs += params.len();
            }
            return;
        }
        cur = current.child_by_field_name("declarator");
    }
}

impl NArgs for CCode {
    fn compute(node: &Node, stats: &mut Stats) {
        if Self::is_func(node) {
            compute_c_args(node, &mut stats.fn_nargs);
        }
        // C has no closures; `is_closure` is always false.
    }
}

implement_metric_trait!(
    [NArgs],
    PythonCode,
    TypescriptCode,
    TsxCode,
    RustCode,
    RubyCode
);

#[cfg(test)]
mod tests {
    use crate::tools::check_metrics;

    use super::*;

    #[test]
    fn python_no_functions_and_closures() {
        check_metrics::<PythonParser>("a = 42", "foo.py", |metric| {
            // 0 functions + 0 closures
            insta::assert_json_snapshot!(
                metric.nargs,
                @r###"
                    {
                      "total_functions": 0.0,
                      "total_closures": 0.0,
                      "average_functions": 0.0,
                      "average_closures": 0.0,
                      "total": 0.0,
                      "average": 0.0,
                      "functions_min": 0.0,
                      "functions_max": 0.0,
                      "closures_min": 0.0,
                      "closures_max": 0.0
                    }"###
            );
        });
    }

    #[test]
    fn rust_no_functions_and_closures() {
        check_metrics::<RustParser>("let a = 42;", "foo.rs", |metric| {
            // 0 functions + 0 closures
            insta::assert_json_snapshot!(
                metric.nargs,
                @r###"
                    {
                      "total_functions": 0.0,
                      "total_closures": 0.0,
                      "average_functions": 0.0,
                      "average_closures": 0.0,
                      "total": 0.0,
                      "average": 0.0,
                      "functions_min": 0.0,
                      "functions_max": 0.0,
                      "closures_min": 0.0,
                      "closures_max": 0.0
                    }"###
            );
        });
    }

    #[test]
    fn python_single_function() {
        check_metrics::<PythonParser>(
            "def f(a, b):
                 if a:
                     return a",
            "foo.py",
            |metric| {
                // 1 function
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 2.0,
                      "total_closures": 0.0,
                      "average_functions": 2.0,
                      "average_closures": 0.0,
                      "total": 2.0,
                      "average": 2.0,
                      "functions_min": 0.0,
                      "functions_max": 2.0,
                      "closures_min": 0.0,
                      "closures_max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_single_function() {
        check_metrics::<RustParser>(
            "fn f(a: bool, b: usize) {
                 if a {
                     return a;
                }
             }",
            "foo.rs",
            |metric| {
                // 1 function
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 2.0,
                      "total_closures": 0.0,
                      "average_functions": 2.0,
                      "average_closures": 0.0,
                      "total": 2.0,
                      "average": 2.0,
                      "functions_min": 0.0,
                      "functions_max": 2.0,
                      "closures_min": 0.0,
                      "closures_max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn kotlin_counts_function_constructor_and_lambda_parameters() {
        check_metrics::<KotlinParser>(
            "class C {
                 constructor(a: Int, b: Int)
             }

             fun f(a: Int, b: String = \"x\", vararg xs: Int) {}

             fun g(items: List<Int>) {
                 items.map { item -> item + 1 }
             }",
            "foo.kt",
            |metric| {
                assert_eq!(metric.nargs.fn_args_sum(), 6.0);
                assert_eq!(metric.nargs.closure_args_sum(), 1.0);
                assert_eq!(metric.nargs.fn_args_max(), 3.0);
                assert_eq!(metric.nargs.closure_args_max(), 1.0);
            },
        );
    }

    #[test]
    fn python_single_lambda() {
        check_metrics::<PythonParser>("bar = lambda a: True", "foo.py", |metric| {
            // 1 lambda
            insta::assert_json_snapshot!(
                metric.nargs,
                @r###"
                    {
                      "total_functions": 0.0,
                      "total_closures": 1.0,
                      "average_functions": 0.0,
                      "average_closures": 1.0,
                      "total": 1.0,
                      "average": 1.0,
                      "functions_min": 0.0,
                      "functions_max": 0.0,
                      "closures_min": 1.0,
                      "closures_max": 1.0
                    }"###
            );
        });
    }

    #[test]
    fn rust_single_closure() {
        check_metrics::<RustParser>("let bar = |i: i32| -> i32 { i + 1 };", "foo.rs", |metric| {
            // 1 lambda
            insta::assert_json_snapshot!(
                metric.nargs,
                @r###"
                    {
                      "total_functions": 0.0,
                      "total_closures": 1.0,
                      "average_functions": 0.0,
                      "average_closures": 1.0,
                      "total": 1.0,
                      "average": 1.0,
                      "functions_min": 0.0,
                      "functions_max": 0.0,
                      "closures_min": 0.0,
                      "closures_max": 1.0
                    }"###
            );
        });
    }

    #[test]
    fn python_functions() {
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
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 4.0,
                      "total_closures": 0.0,
                      "average_functions": 2.0,
                      "average_closures": 0.0,
                      "total": 4.0,
                      "average": 2.0,
                      "functions_min": 0.0,
                      "functions_max": 2.0,
                      "closures_min": 0.0,
                      "closures_max": 0.0
                    }"###
                );
            },
        );

        check_metrics::<PythonParser>(
            "def f(a, b):
                 if a:
                     return a
            def f(a, b, c):
                 if b:
                     return b",
            "foo.py",
            |metric| {
                // 2 functions
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 5.0,
                      "total_closures": 0.0,
                      "average_functions": 2.5,
                      "average_closures": 0.0,
                      "total": 5.0,
                      "average": 2.5,
                      "functions_min": 0.0,
                      "functions_max": 3.0,
                      "closures_min": 0.0,
                      "closures_max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_functions() {
        check_metrics::<RustParser>(
            "fn f(a: bool, b: usize) {
                 if a {
                     return a;
                }
             }
             fn f1(a: bool, b: usize) {
                 if a {
                     return a;
                }
             }",
            "foo.rs",
            |metric| {
                // 2 functions
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 4.0,
                      "total_closures": 0.0,
                      "average_functions": 2.0,
                      "average_closures": 0.0,
                      "total": 4.0,
                      "average": 2.0,
                      "functions_min": 0.0,
                      "functions_max": 2.0,
                      "closures_min": 0.0,
                      "closures_max": 0.0
                    }"###
                );
            },
        );

        check_metrics::<RustParser>(
            "fn f(a: bool, b: usize) {
                 if a {
                     return a;
                }
             }
             fn f1(a: bool, b: usize, c: usize) {
                 if a {
                     return a;
                }
             }",
            "foo.rs",
            |metric| {
                // 2 functions
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 5.0,
                      "total_closures": 0.0,
                      "average_functions": 2.5,
                      "average_closures": 0.0,
                      "total": 5.0,
                      "average": 2.5,
                      "functions_min": 0.0,
                      "functions_max": 3.0,
                      "closures_min": 0.0,
                      "closures_max": 0.0
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
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 3.0,
                      "total_closures": 2.0,
                      "average_functions": 1.5,
                      "average_closures": 1.0,
                      "total": 5.0,
                      "average": 1.25,
                      "functions_min": 0.0,
                      "functions_max": 2.0,
                      "closures_min": 0.0,
                      "closures_max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_nested_functions() {
        check_metrics::<RustParser>(
            "fn f(a: i32, b: i32) -> i32 {
                 fn foo(a: i32) -> i32 {
                     return a;
                 }
                 let bar = |a: i32, b: i32| -> i32 { a + 1 };
                 let bar1 = |b: i32| -> i32 { b + 1 };
                 return bar(foo(a), a);
             }",
            "foo.rs",
            |metric| {
                // 2 functions + 2 lambdas = 4
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 3.0,
                      "total_closures": 3.0,
                      "average_functions": 1.5,
                      "average_closures": 1.5,
                      "total": 6.0,
                      "average": 1.5,
                      "functions_min": 0.0,
                      "functions_max": 2.0,
                      "closures_min": 0.0,
                      "closures_max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_grouped_and_variadic_parameters() {
        check_metrics::<GoParser>(
            "package main

             func add(a, b int, rest ...string) int {
                 return a + b
             }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 3.0,
                      "total_closures": 0.0,
                      "average_functions": 3.0,
                      "average_closures": 0.0,
                      "total": 3.0,
                      "average": 3.0,
                      "functions_min": 0.0,
                      "functions_max": 3.0,
                      "closures_min": 0.0,
                      "closures_max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn go_func_literal_parameters_are_counted_as_closures() {
        check_metrics::<GoParser>(
            "package main

             func main() {
                 _ = func(x, y int, done chan bool) {
                     done <- x > y
                 }
             }",
            "foo.go",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 0.0,
                      "total_closures": 3.0,
                      "average_functions": 0.0,
                      "average_closures": 3.0,
                      "total": 3.0,
                      "average": 1.5,
                      "functions_min": 0.0,
                      "functions_max": 0.0,
                      "closures_min": 0.0,
                      "closures_max": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_single_method() {
        check_metrics::<RubyParser>(
            "def f(a, b)
                 a + b
             end",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 2.0,
                      "total_closures": 0.0,
                      "average_functions": 2.0,
                      "average_closures": 0.0,
                      "total": 2.0,
                      "average": 2.0,
                      "functions_min": 0.0,
                      "functions_max": 2.0,
                      "closures_min": 0.0,
                      "closures_max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_block_and_lambda_args() {
        // `do |a, b| ... end` is a block (closure); `-> (x) { ... }` is a lambda.
        check_metrics::<RubyParser>(
            "xs.each do |a, b|
                 a + b
             end
             f = -> (x) { x * 2 }",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 0.0,
                      "total_closures": 3.0,
                      "average_functions": 0.0,
                      "average_closures": 1.5,
                      "total": 3.0,
                      "average": 1.5,
                      "functions_min": 0.0,
                      "functions_max": 0.0,
                      "closures_min": 0.0,
                      "closures_max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn powershell_function_counts_script_parameters() {
        check_metrics::<crate::langs::PowershellParser>(
            "function Add($a, $b) {
                 $a + $b
             }",
            "foo.ps1",
            |metric| {
                // 1 function with 2 parameters.
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 2.0,
                      "total_closures": 0.0,
                      "average_functions": 2.0,
                      "average_closures": 0.0,
                      "total": 2.0,
                      "average": 2.0,
                      "functions_min": 0.0,
                      "functions_max": 2.0,
                      "closures_min": 0.0,
                      "closures_max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn powershell_class_method_counts_method_parameters() {
        check_metrics::<crate::langs::PowershellParser>(
            "class C {
                 [int] Add([int]$a, [int]$b, [int]$c) {
                     return $a + $b + $c
                 }
             }",
            "foo.ps1",
            |metric| {
                // 1 method with 3 parameters.
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 3.0,
                      "total_closures": 0.0,
                      "average_functions": 3.0,
                      "average_closures": 0.0,
                      "total": 3.0,
                      "average": 3.0,
                      "functions_min": 0.0,
                      "functions_max": 3.0,
                      "closures_min": 0.0,
                      "closures_max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn powershell_script_block_with_param_counts_as_closure() {
        check_metrics::<crate::langs::PowershellParser>(
            "$sb = { param($x, $y) $x + $y }",
            "foo.ps1",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 0.0,
                      "total_closures": 2.0,
                      "average_functions": 0.0,
                      "average_closures": 2.0,
                      "total": 2.0,
                      "average": 2.0,
                      "functions_min": 0.0,
                      "functions_max": 0.0,
                      "closures_min": 0.0,
                      "closures_max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn powershell_nested_closure_params_do_not_count_toward_outer_fn() {
        // Regression: `compute_powershell_args` must not recurse into
        // the body `script_block` of a `function_statement` or
        // `script_block_expression`, or the `param_block` of a nested
        // closure inside the body would leak into the outer function's
        // arg count. For `function f($a) { $sb = { param($x, $y) ... } }`,
        // `f` owns 1 function arg and the inner closure owns 2 closure
        // args; neither should leak into the other's counters.
        check_metrics::<crate::langs::PowershellParser>(
            "function f($a) {
                 $sb = { param($x, $y) $x + $y }
             }",
            "foo.ps1",
            |metric| {
                // At the outer function's space, the aggregated counts
                // are: 1 function arg (f's $a) + 2 closure args (the
                // inner scriptblock's $x, $y). Neither bleeds into the
                // other counter. `functions_max = 1` (not 3) pins that
                // f itself owns exactly 1 arg.
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 1.0,
                      "total_closures": 2.0,
                      "average_functions": 1.0,
                      "average_closures": 2.0,
                      "total": 3.0,
                      "average": 1.5,
                      "functions_min": 0.0,
                      "functions_max": 1.0,
                      "closures_min": 0.0,
                      "closures_max": 2.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn c_function_counts_parameters() {
        // Regression: tree-sitter-c nests `parameter_list` under
        // `function_declarator`, not directly under `function_definition`.
        // The generic `compute_args` that looks for a `parameters` field on
        // the function node would read zero for C functions; the C-specific
        // counter must descend into the declarator. Definition here has two
        // params (int a, int b), so aggregated nargs must reflect that.
        check_metrics::<CParser>(
            "int add(int a, int b) { return a + b; }",
            "foo.c",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 2.0,
                      "total_closures": 0.0,
                      "average_functions": 2.0,
                      "average_closures": 0.0,
                      "total": 2.0,
                      "average": 2.0,
                      "functions_min": 0.0,
                      "functions_max": 2.0,
                      "closures_min": 0.0,
                      "closures_max": 0.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn c_void_parameter_is_not_counted() {
        // `int foo(void)` is the C spelling for "no parameters" and must
        // count as zero arguments — not one.
        check_metrics::<CParser>("int foo(void) { return 0; }", "foo.c", |metric| {
            insta::assert_json_snapshot!(
                metric.nargs,
                @r###"
                    {
                      "total_functions": 0.0,
                      "total_closures": 0.0,
                      "average_functions": 0.0,
                      "average_closures": 0.0,
                      "total": 0.0,
                      "average": 0.0,
                      "functions_min": 0.0,
                      "functions_max": 0.0,
                      "closures_min": 0.0,
                      "closures_max": 0.0
                    }"###
            );
        });
    }

    #[test]
    fn c_variadic_parameter_does_not_count() {
        // `int vararg(int fmt, ...)` has one named argument; the `...`
        // token is a `variadic_parameter`, not a `parameter_declaration`,
        // and must not contribute to the count.
        check_metrics::<CParser>(
            "int vararg(int fmt, ...) { return fmt; }",
            "foo.c",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.nargs,
                    @r###"
                    {
                      "total_functions": 1.0,
                      "total_closures": 0.0,
                      "average_functions": 1.0,
                      "average_closures": 0.0,
                      "total": 1.0,
                      "average": 1.0,
                      "functions_min": 0.0,
                      "functions_max": 1.0,
                      "closures_min": 0.0,
                      "closures_max": 0.0
                    }"###
                );
            },
        );
    }
}
