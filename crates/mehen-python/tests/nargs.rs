// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Nargs ports from
//! `crates/mehen-engine/src/legacy/metrics/nargs.rs` Python tests.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_python::PythonAnalyzer;

fn analyze(source: &str, filename: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = PythonAnalyzer::new();
    let file = SourceFile::new(filename.into(), Language::Python, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn python_no_functions_and_closures() {
    let a = analyze("a = 42", "foo.py");
    let na = mehen_report::metrics_json::nargs(&a.root.metrics);
    insta::assert_json_snapshot!(
        na,
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
fn python_single_function() {
    let a = analyze(
        "def f(a, b):
    if a:
        return a",
        "foo.py",
    );
    let na = mehen_report::metrics_json::nargs(&a.root.metrics);
    // 1 function with 2 args.
    //
    // Drift from legacy: legacy reported `functions_min: 0.0` because
    // the unit space's always-zero `fn_nargs` was folded into the
    // unit's per-space minmax after merging child stats up. Per the
    // metric definition, the unit isn't a function and shouldn't
    // contribute a 0-arg sample to a "minimum number of function
    // arguments across function spaces" statistic. The new walker
    // (mehen-metrics #PR Phase-6) gates `finalize_minmax` on the
    // `is_function` / `is_closure` flags so only function/closure
    // spaces contribute their own `fn_nargs` / `closure_nargs` to the
    // bounds. Result: `functions_min: 2.0` — matching the *only*
    // function in the source.
    insta::assert_json_snapshot!(
        na,
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
fn python_single_lambda() {
    let a = analyze("bar = lambda a: True", "foo.py");
    let na = mehen_report::metrics_json::nargs(&a.root.metrics);
    // 1 lambda with 1 arg
    insta::assert_json_snapshot!(
        na,
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
}

#[test]
fn python_functions_a() {
    // Source reformatted from the legacy fixture's deeply-indented
    // form: tree-sitter-python silently smoothed over the inconsistent
    // 12-space indent on the second `def`, but Ruff (correctly) treats
    // it as the start of a new statement at the parent indent level.
    // The fixed source has both defs at column 0 — what the legacy
    // test was *intending* to express.
    let a = analyze(
        "def f(a, b):
    if a:
        return a
def f(a, b):
    if b:
        return b",
        "foo.py",
    );
    let na = mehen_report::metrics_json::nargs(&a.root.metrics);
    // 2 functions, each with 2 args. `functions_min: 2.0` per
    // `python_single_function` rationale (unit doesn't pollute the
    // min).
    insta::assert_json_snapshot!(
        na,
        @r###"
    {
      "total_functions": 4.0,
      "total_closures": 0.0,
      "average_functions": 2.0,
      "average_closures": 0.0,
      "total": 4.0,
      "average": 2.0,
      "functions_min": 2.0,
      "functions_max": 2.0,
      "closures_min": 0.0,
      "closures_max": 0.0
    }"###
    );
}

#[test]
fn python_functions_b() {
    let a = analyze(
        "def f(a, b):
    if a:
        return a
def f(a, b, c):
    if b:
        return b",
        "foo.py",
    );
    let na = mehen_report::metrics_json::nargs(&a.root.metrics);
    // 2 functions: f(2 args) + f(3 args) = 5 total. Min=2, Max=3.
    insta::assert_json_snapshot!(
        na,
        @r###"
    {
      "total_functions": 5.0,
      "total_closures": 0.0,
      "average_functions": 2.5,
      "average_closures": 0.0,
      "total": 5.0,
      "average": 2.5,
      "functions_min": 2.0,
      "functions_max": 3.0,
      "closures_min": 0.0,
      "closures_max": 0.0
    }"###
    );
}

#[test]
fn python_nested_functions() {
    let a = analyze(
        "def f(a, b):
    def foo(a):
        if a:
            return 1
    bar = lambda a: lambda b: b or True or True
    return bar(foo(a))(a)",
        "foo.py",
    );
    let na = mehen_report::metrics_json::nargs(&a.root.metrics);
    // 2 functions (`f(2 args)`, `foo(1 arg)`) + 2 lambdas
    // (`lambda a: ...` outer with 1 arg, `lambda b: ...` inner with
    // 1 arg). Total args = 2+1+1+1 = 5. Per `python_single_function`
    // rationale, `functions_min` is 1 (the smaller of the two
    // function spaces) and `closures_min` is 1.
    insta::assert_json_snapshot!(
        na,
        @r###"
    {
      "total_functions": 3.0,
      "total_closures": 2.0,
      "average_functions": 1.5,
      "average_closures": 1.0,
      "total": 5.0,
      "average": 1.25,
      "functions_min": 1.0,
      "functions_max": 2.0,
      "closures_min": 1.0,
      "closures_max": 1.0
    }"###
    );
}
