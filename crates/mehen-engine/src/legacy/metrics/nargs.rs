use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::legacy::checker::Checker;
use crate::legacy::langs::CCode;
#[cfg(test)]
use crate::legacy::langs::CParser;
use crate::legacy::languages::C;
use crate::legacy::node::Node;
use crate::legacy::traits::Search;

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

pub(crate) trait NArgs
where
    Self: Checker + Sized,
{
    fn compute(node: &Node, _code: &[u8], stats: &mut Stats) {
        if Self::is_func(node) {
            compute_args::<Self>(node, &mut stats.fn_nargs);
            return;
        }

        if Self::is_closure(node) {
            compute_args::<Self>(node, &mut stats.closure_nargs);
        }
    }
}

#[inline(always)]
fn is_c_function_declarator(kind: u16) -> bool {
    // The tree-sitter-c grammar emits `function_declarator` under five
    // distinct IDs (context-dependent alternates for nested / abstract /
    // attributed declarators). All alias the same rule, so any of them
    // can hold our `parameter_list`.
    matches!(
        C::from(kind),
        C::FunctionDeclarator
            | C::FunctionDeclarator2
            | C::FunctionDeclarator3
            | C::FunctionDeclarator4
            | C::FunctionDeclarator5
    )
}

#[inline(always)]
fn is_c_parameter_list(kind: u16) -> bool {
    matches!(C::from(kind), C::ParameterList | C::ParameterList2)
}

#[inline(always)]
fn compute_c_args(node: &Node, code: &[u8], nargs: &mut usize) {
    // tree-sitter-c nests the parameter list under the innermost
    // `function_declarator`: `function_definition > function_declarator >
    // parameter_list`. Pointer (`int (*f)(...)`) and attributed declarators
    // wrap the `function_declarator`, so walk inward via the `declarator`
    // field until we find the `function_declarator` whose direct child is
    // the `parameter_list`. Both the declarator and parameter-list rules
    // expose multiple positional IDs (231..=235 / 259) alongside the
    // canonical 230 / 258; any of them is a valid match.
    let mut cur = node.0.child_by_field_name("declarator");
    while let Some(current) = cur {
        if is_c_function_declarator(current.kind_id()) {
            let mut cursor = current.walk();
            let Some(param_list) = current
                .children(&mut cursor)
                .find(|c| is_c_parameter_list(c.kind_id()))
            else {
                return;
            };
            let mut list_cursor = param_list.walk();
            let params: Vec<_> = param_list
                .children(&mut list_cursor)
                .filter(|p| p.kind_id() == C::ParameterDeclaration)
                .collect();
            // `(void)` is C's spelling for "no parameters" and must not be
            // counted. Detect it precisely by checking that the sole
            // parameter's text literally matches `void` — a nameless
            // `parameter_declaration` alone isn't enough to disambiguate
            // `(void)` from `(int)` (a bare type in an old-style
            // prototype), which tree-sitter-c parses with the same shape.
            // `variadic_parameter` (`...`) is already filtered out above.
            let is_void_only = params.len() == 1
                && code
                    .get(params[0].start_byte()..params[0].end_byte())
                    .is_some_and(|bytes| bytes == b"void");
            if !is_void_only {
                *nargs += params.len();
            }
            return;
        }
        cur = current.child_by_field_name("declarator");
    }
}

impl NArgs for CCode {
    fn compute(node: &Node, code: &[u8], stats: &mut Stats) {
        if Self::is_func(node) {
            compute_c_args(node, code, &mut stats.fn_nargs);
        }
        // C has no closures; `is_closure` is always false.
    }
}

// Markdown documents have no function parameters.
#[cfg(feature = "markdown")]
impl NArgs for crate::legacy::langs::MarkdownCode {}

#[cfg(test)]
mod tests {
    use crate::legacy::tools::check_metrics;

    use super::*;

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

    #[test]
    fn c_bare_type_parameter_counts_as_one() {
        // `int foo(int)` — a K&R / old-style prototype-esque definition
        // with a bare type and no parameter name — has ONE parameter.
        // tree-sitter-c parses it with the same AST shape as `int foo(void)`
        // (sole `parameter_declaration` holding just a `primitive_type`),
        // so the `(void)` detection must look at the literal text, not
        // just the structural shape, to avoid undercounting this case.
        check_metrics::<CParser>("int foo(int) { return 0; }", "foo.c", |metric| {
            assert_eq!(metric.nargs.fn_args_sum(), 1.0);
        });
    }
}
