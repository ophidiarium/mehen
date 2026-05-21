// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! PowerShell cyclomatic-complexity tests, ported from
//! `src/metrics/cyclomatic.rs::tests` per rewrite plan §8.2.
//!
//! Each test runs [`PowerShellAnalyzer`] over a script and asserts the
//! `cyclomatic` family object rendered by
//! [`mehen_report::metrics_json::cyclomatic`] (the JSON shape used by
//! `mehen metrics --format json`, plan §9.1). The expected snapshot
//! strings are copied verbatim from the corresponding pre-1.0 tests, so
//! a difference here is a numeric regression — not a representation
//! drift.
//!
//! Decision-point classification mirrors the original
//! `Cyclomatic for PowershellCode`: `if`/`elseif`/loops/`switch_clause`/
//! `catch_clause`/`trap_statement`, v7 ternary `?` and null-coalesce
//! `??` (operator + argument forms), short-circuit `&&` / `||`, and
//! logical `-and` / `-or`. `-xor` is intentionally excluded (Sonar's
//! rule: only short-circuit operators introduce a new path).

use mehen_core::{AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, SourceFile};
use mehen_powershell::PowerShellAnalyzer;

fn analyze(source: &str) -> LanguageAnalysis {
    let analyzer = PowerShellAnalyzer::new();
    let file = SourceFile::new("foo.ps1".into(), Language::PowerShell, source.to_string());
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn powershell_simple_function() {
    let a = analyze(
        "function Greet($name) { # +2 (+1 unit, +1 function)
                 if ($name) {         # +1
                     Write-Host \"hi, $name\"
                 }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cyclomatic(&a.root.metrics),
        @r###"
    {
      "sum": 3.0,
      "average": 1.5,
      "min": 1.0,
      "max": 2.0
    }
    "###
    );
}

#[test]
fn powershell_counts_each_switch_clause() {
    // The `switch` statement itself does NOT add a decision; each
    // `switch_clause` does. Aligns with Sonar's general cyclomatic rule.
    let a = analyze(
        "function Grade($score) {
                 switch ($score) {
                     1 { 'A' }
                     2 { 'B' }
                     3 { 'C' }
                     default { 'F' }
                 }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cyclomatic(&a.root.metrics),
        @r###"
    {
      "sum": 6.0,
      "average": 3.0,
      "min": 1.0,
      "max": 5.0
    }
    "###
    );
}

#[test]
fn powershell_short_circuit_and_word_form_boolean_operators() {
    // PowerShell has two boolean operator pairs:
    //   - short-circuit `&&` / `||` (inside `pipeline_chain`)
    //   - logical `-and` / `-or` / `-xor` (inside `logical_expression`)
    // Each occurrence contributes +1.
    let a = analyze(
        "function Check($a, $b, $c) { # +2 (+1 unit, +1 function)
                 if ($a -and $b -or $c) { # +3 (+1 if, +1 -and, +1 -or)
                     return $true
                 }
                 return $false
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cyclomatic(&a.root.metrics),
        @r###"
    {
      "sum": 5.0,
      "average": 2.5,
      "min": 1.0,
      "max": 4.0
    }
    "###
    );
}

#[test]
fn powershell_ternary_and_null_coalesce_wrappers_do_not_false_trigger() {
    // Regression: tree-sitter-pwsh emits `ternary_expression`,
    // `null_coalesce_expression`, and `logical_expression` as wrapper
    // kinds in the precedence cascade even for plain expressions like
    // `$a + $b`. Those wrappers must NOT contribute to cyclomatic; only
    // the real `?` / `??` / `-and` / `-or` operator tokens do.
    let a = analyze(
        "function Plain { # +2 (+1 unit, +1 function)
                 $x = $a + $b  # no decision point
                 return $x     # no decision point
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cyclomatic(&a.root.metrics),
        @r###"
    {
      "sum": 2.0,
      "average": 1.0,
      "min": 1.0,
      "max": 1.0
    }
    "###
    );
}

#[test]
fn powershell_real_ternary_and_null_coalesce_count() {
    // Real `?` / `??` expressions add one decision each.
    let a = analyze(
        "$a = $cond ? 1 : 2   # +1 ternary
             $b = $x ?? 0         # +1 null-coalesce",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cyclomatic(&a.root.metrics),
        @r###"
    {
      "sum": 3.0,
      "average": 3.0,
      "min": 3.0,
      "max": 3.0
    }
    "###
    );
}

#[test]
fn powershell_argument_form_ternary_and_null_coalesce_count() {
    // Regression: tree-sitter-pwsh emits a parallel family of
    // `*_argument_expression` kinds for expressions that live inside a
    // method-invocation `argument_list` (e.g.
    // `[Foo]::Bar($cond ? 1 : 2)`). Those argument-form decision
    // operators must count the same as their regular-form twins.
    let a = analyze(
        "function F($a, $b, $x, $cond) { # +2 (+1 unit, +1 function)
                 [Foo]::Bar($a -eq $b)        # comparison: no decision
                 [Foo]::Baz($cond ? 1 : 2)    # +1 ternary
                 [Foo]::Qux($x ?? 3)          # +1 null-coalesce
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cyclomatic(&a.root.metrics),
        @r###"
    {
      "sum": 4.0,
      "average": 2.0,
      "min": 1.0,
      "max": 3.0
    }
    "###
    );
}

#[test]
fn powershell_xor_is_not_a_cyclomatic_decision_point() {
    // Regression: `-xor` is intentionally excluded from the cyclomatic
    // decision-point set. Sonar's cyclomatic rule counts only
    // *short-circuit* boolean operators across every language it
    // analyzes; `-xor` always evaluates both operands so it cannot
    // introduce a new control-flow path. `-and` / `-or` are counted
    // because they short-circuit.
    let a = analyze(
        "function f($a, $b, $c) {
                 if ($a -xor $b) { }        # +1 if, NOT +1 -xor
                 if ($a -and $b) { }        # +1 if, +1 -and
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cyclomatic(&a.root.metrics),
        @r###"
    {
      "sum": 5.0,
      "average": 2.5,
      "min": 1.0,
      "max": 4.0
    }
    "###
    );
}
