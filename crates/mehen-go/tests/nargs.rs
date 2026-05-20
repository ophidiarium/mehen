//! NArgs tests for the Go walker.
//!
//! Every legacy `check_metrics::<GoParser>` nargs test from
//! `crates/mehen-engine/src/legacy/metrics/nargs.rs` is ported here
//! byte-identical so the parity contract (plan §12.3.1) is visibly
//! maintained.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_go::GoAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = GoAnalyzer::new();
    let file = SourceFile::new("foo.go".into(), Language::Go, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn go_grouped_and_variadic_parameters() {
    // Drift from legacy: legacy reported `functions_min: 0.0` because
    // its `compute_minmax` ran unconditionally for every space, so the
    // unit space (which has no fn args) pulled the min down to 0. The
    // 1.0 mehen-metrics `NargsStats::finalize_minmax` only includes
    // a space in the function bounds if `is_function == true`, so the
    // unit no longer dilutes the bounds. Result: `functions_min: 3.0`
    // — matching the *only* function in this fixture. This drift is
    // shared with Phase 6 Python and Phase 9 Ruby.
    let a = analyze(
        "package main

             func add(a, b int, rest ...string) int {
                 return a + b
             }",
    );
    let nargs = mehen_report::metrics_json::nargs(&a.root.metrics);
    insta::assert_json_snapshot!(
        nargs,
        @r###"
    {
      "total_functions": 3.0,
      "total_closures": 0.0,
      "average_functions": 3.0,
      "average_closures": 0.0,
      "total": 3.0,
      "average": 3.0,
      "functions_min": 3.0,
      "functions_max": 3.0,
      "closures_min": 0.0,
      "closures_max": 0.0
    }"###
    );
}

#[test]
fn go_func_literal_parameters_are_counted_as_closures() {
    // Drift from legacy: same as `go_grouped_and_variadic_parameters`
    // — `closures_min` is now `3.0` because the only closure (the
    // `func(x, y int, done chan bool)` literal) carries three
    // parameters. Legacy reported `0.0` because the unit space (which
    // has no closure args) pulled the bound down.
    let a = analyze(
        "package main

             func main() {
                 _ = func(x, y int, done chan bool) {
                     done <- x > y
                 }
             }",
    );
    let nargs = mehen_report::metrics_json::nargs(&a.root.metrics);
    insta::assert_json_snapshot!(
        nargs,
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
      "closures_min": 3.0,
      "closures_max": 3.0
    }"###
    );
}
