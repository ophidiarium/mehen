// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! NArgs tests for the C walker.
//!
//! Every legacy `check_metrics::<CParser>` nargs test from
//! `crates/mehen-engine/src/legacy/metrics/nargs.rs` is ported here
//! byte-identical so the parity contract (plan §12.3.1) is visibly
//! maintained. The C-specific `count_c_args` walks the
//! `function_definition > function_declarator > parameter_list`
//! chain, applies the `(void)` exception, and excludes the
//! `variadic_parameter` (`...`) — see `walker.rs::count_c_args`.

use mehen_c::CAnalyzer;
use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = CAnalyzer::new();
    let file = SourceFile::new("foo.c".into(), Language::C, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn c_function_counts_parameters() {
    // Regression: tree-sitter-c nests `parameter_list` under
    // `function_declarator`, not directly under `function_definition`.
    // The generic `compute_args` that looks for a `parameters` field on
    // the function node would read zero for C functions; the C-specific
    // counter must descend into the declarator. Definition here has two
    // params (int a, int b), so aggregated nargs must reflect that.
    //
    // Drift from legacy: legacy reported `functions_min: 0.0` because
    // its `compute_minmax` ran unconditionally for every space, so the
    // unit space (which has no fn args) pulled the min down to 0. The
    // 1.0 mehen-metrics `NargsStats::finalize_minmax` only includes
    // a space in the function bounds when `is_function == true`, so
    // the unit no longer dilutes the bounds. Result: `functions_min:
    // 2.0` — matching the *only* function in this fixture. Same drift
    // documented in Phase 6 Python, Phase 9 Ruby, and the Go port.
    let a = analyze("int add(int a, int b) { return a + b; }");
    let nargs = mehen_report::metrics_json::nargs(&a.root.metrics);
    insta::assert_json_snapshot!(
        nargs,
        @r###"
    {
      "total_functions": 2.0,
      "total_closures": 0.0,
      "average_functions": 2.0,
      "average_closures": 0.0,
      "total": 2.0,
      "average": 2.0,
      "functions_min": 2.0,
      "functions_max": 2.0,
      "closures_min": 0.0,
      "closures_max": 0.0
    }"###
    );
}

#[test]
fn c_void_parameter_is_not_counted() {
    // `int foo(void)` is the C spelling for "no parameters" and must
    // count as zero arguments — not one.
    let a = analyze("int foo(void) { return 0; }");
    let nargs = mehen_report::metrics_json::nargs(&a.root.metrics);
    insta::assert_json_snapshot!(
        nargs,
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
}

#[test]
fn c_variadic_parameter_does_not_count() {
    // `int vararg(int fmt, ...)` has one named argument; the `...`
    // token is a `variadic_parameter`, not a `parameter_declaration`,
    // and must not contribute to the count.
    //
    // Drift from legacy: same as `c_function_counts_parameters` —
    // `functions_min` is now `1.0` because the only function carries
    // one parameter; legacy reported `0.0` because the unit space
    // diluted the bound.
    let a = analyze("int vararg(int fmt, ...) { return fmt; }");
    let nargs = mehen_report::metrics_json::nargs(&a.root.metrics);
    insta::assert_json_snapshot!(
        nargs,
        @r###"
    {
      "total_functions": 1.0,
      "total_closures": 0.0,
      "average_functions": 1.0,
      "average_closures": 0.0,
      "total": 1.0,
      "average": 1.0,
      "functions_min": 1.0,
      "functions_max": 1.0,
      "closures_min": 0.0,
      "closures_max": 0.0
    }"###
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
    let a = analyze("int foo(int) { return 0; }");
    let nargs = mehen_report::metrics_json::nargs(&a.root.metrics);
    assert_eq!(nargs.total_functions, 1.0);
}
