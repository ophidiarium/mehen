//! PowerShell NArgs tests, ported from
//! `src/metrics/nargs.rs::tests` per rewrite plan §8.2.
//!
//! Snapshots are byte-identical to the pre-1.0 `metric.nargs` strings.

use mehen_core::{AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, SourceFile};
use mehen_powershell::PowerShellAnalyzer;

fn analyze(source: &str) -> LanguageAnalysis {
    let analyzer = PowerShellAnalyzer::new();
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let file = SourceFile::new("foo.ps1".into(), Language::PowerShell, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn powershell_function_counts_script_parameters() {
    let a = analyze(
        "function Add($a, $b) {
                 $a + $b
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
      "functions_min": 0.0,
      "functions_max": 2.0,
      "closures_min": 0.0,
      "closures_max": 0.0
    }
    "###
    );
}

#[test]
fn powershell_class_method_counts_method_parameters() {
    let a = analyze(
        "class C {
                 [int] Add([int]$a, [int]$b, [int]$c) {
                     return $a + $b + $c
                 }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::nargs(&a.root.metrics),
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
    }
    "###
    );
}

#[test]
fn powershell_script_block_with_param_counts_as_closure() {
    let a = analyze("$sb = { param($x, $y) $x + $y }");
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::nargs(&a.root.metrics),
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
    }
    "###
    );
}

#[test]
fn powershell_nested_closure_params_do_not_count_toward_outer_fn() {
    // `function f($a) { $sb = { param($x, $y) ... } }` — `f` owns 1
    // function arg ($a) and the inner closure owns 2 closure args ($x,
    // $y). Neither bleeds into the other counter.
    let a = analyze(
        "function f($a) {
                 $sb = { param($x, $y) $x + $y }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::nargs(&a.root.metrics),
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
    }
    "###
    );
}
