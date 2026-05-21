// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! PowerShell ABC tests, ported from
//! `src/metrics/abc.rs::tests` per rewrite plan §8.2.
//!
//! Snapshots are byte-identical to the pre-1.0 `metric.abc` strings.

use mehen_core::{
    AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, MetricKey, MetricValue,
    SourceFile,
};
use mehen_powershell::PowerShellAnalyzer;

fn analyze(source: &str) -> LanguageAnalysis {
    let analyzer = PowerShellAnalyzer::new();
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let file = SourceFile::new("foo.ps1".into(), Language::PowerShell, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

fn metric(report: &LanguageAnalysis, key: &str) -> f64 {
    match report.root.metrics.get(&MetricKey::new(key)).unwrap() {
        MetricValue::Int(i) => i as f64,
        MetricValue::Float(f) => f,
    }
}

#[test]
fn powershell_abc_basic() {
    // function f($a, $b) { $c = $a + $b; Write-Host $c; if ($c -gt 0) { return $c } }
    // A=1, B=1, C=2 (1 if + 1 comparison). Magnitude = sqrt(1 + 1 + 4) ≈ 2.449.
    let a = analyze(
        "function f($a, $b) {
                 $c = $a + $b
                 Write-Host $c
                 if ($c -gt 0) {
                     return $c
                 }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::abc(&a.root.metrics),
        @r###"
    {
      "assignments": 1.0,
      "branches": 1.0,
      "conditions": 2.0,
      "magnitude": 2.449489742783178,
      "assignments_average": 0.5,
      "branches_average": 0.5,
      "conditions_average": 1.0,
      "assignments_min": 0.0,
      "assignments_max": 1.0,
      "branches_min": 0.0,
      "branches_max": 1.0,
      "conditions_min": 0.0,
      "conditions_max": 2.0
    }
    "###
    );
}

#[test]
fn powershell_abc_counts_argument_form_decision_operators() {
    // tree-sitter-pwsh emits `*_argument_expression` for expressions
    // inside method-invocation argument lists (e.g.
    // `[Foo]::Bar($a -eq $b)`). Argument-form comparison / ternary /
    // null-coalesce must contribute to ABC conditions.
    let a = analyze(
        "function f($a, $b, $cond, $x) {
                 [Foo]::Bar($a -eq $b)
                 [Foo]::Baz($cond ? 1 : 2)
                 [Foo]::Qux($x ?? 3)
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::abc(&a.root.metrics),
        @r###"
    {
      "assignments": 0.0,
      "branches": 3.0,
      "conditions": 3.0,
      "magnitude": 4.242640687119285,
      "assignments_average": 0.0,
      "branches_average": 1.5,
      "conditions_average": 1.5,
      "assignments_min": 0.0,
      "assignments_max": 0.0,
      "branches_min": 0.0,
      "branches_max": 3.0,
      "conditions_min": 0.0,
      "conditions_max": 3.0
    }
    "###
    );
}

#[test]
fn powershell_abc_logical_operators_are_not_double_counted() {
    // Regression: ABC conditions match only the *leaf* logical-operator
    // tokens (`-and` / `-or` / `-xor` / `&&` / `||`), NOT the
    // `logical_expression` / `pipeline_chain` wrappers — a single
    // wrapper can hold multiple leaves (`$a -and $b -and $c`),
    // counting both wrapper and leaves would double-count.
    let a = analyze(
        "function f($a, $b, $c) {
                 if ($a -and $b -and $c) { 'x' }
                 [Foo]::Bar($a -or $b)
             }",
    );
    // Conditions: 1 (if) + 2 (-and, -and) + 1 (-or) = 4.
    // Branches: 1 (`[Foo]::Bar` invocation_expression).
    // Assignments: 0.
    assert_eq!(metric(&a, "abc.conditions"), 4.0);
    assert_eq!(metric(&a, "abc.branches"), 1.0);
    assert_eq!(metric(&a, "abc.assignments"), 0.0);
}
