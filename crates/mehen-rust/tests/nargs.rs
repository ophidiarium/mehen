//! NArgs tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/nargs.rs::tests` per the
//! same `functions_min`/`closures_min` correction documented for
//! Python and PowerShell in Phase 6/7. Per-space `_min` is gated on
//! `is_function`/`is_closure`, so a unit space with no own arguments
//! no longer pollutes the rolled-up minimum to `0.0`.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_rust::RustAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = RustAnalyzer::new();
    let file = SourceFile::new("foo.rs".into(), Language::Rust, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn rust_no_functions_and_closures() {
    let a = analyze("let a = 42;");
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::nargs(&a.root.metrics),
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
fn rust_single_function() {
    let a = analyze(
        "fn f(a: bool, b: usize) {
             if a {
                 return a;
             }
         }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::nargs(&a.root.metrics),
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
fn rust_single_closure() {
    let a = analyze("let bar = |i: i32| -> i32 { i + 1 };");
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::nargs(&a.root.metrics),
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
fn rust_functions_two() {
    let a = analyze(
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
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::nargs(&a.root.metrics),
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
fn rust_functions_uneven() {
    let a = analyze(
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
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::nargs(&a.root.metrics),
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
fn rust_nested_functions() {
    let a = analyze(
        "fn f(a: i32, b: i32) -> i32 {
             fn foo(a: i32) -> i32 {
                 return a;
             }
             let bar = |a: i32, b: i32| -> i32 { a + 1 };
             let bar1 = |b: i32| -> i32 { b + 1 };
             return bar(foo(a), a);
         }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::nargs(&a.root.metrics),
        @r###"
    {
      "total_functions": 3.0,
      "total_closures": 3.0,
      "average_functions": 1.5,
      "average_closures": 1.5,
      "total": 6.0,
      "average": 1.5,
      "functions_min": 1.0,
      "functions_max": 2.0,
      "closures_min": 1.0,
      "closures_max": 2.0
    }"###
    );
}
