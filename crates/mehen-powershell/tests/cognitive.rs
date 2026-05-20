//! PowerShell cognitive-complexity tests, ported from
//! `src/metrics/cognitive.rs::tests` per rewrite plan §8.2.
//!
//! Snapshots are byte-identical to the pre-1.0 `metric.cognitive`
//! strings.

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
fn powershell_nested_if_increments_nesting() {
    // function f($a, $b) {
    //     if ($a) {        # +1
    //         if ($b) {    # +2 (nesting = 1)
    //             ...
    //         }
    //     }
    // }
    // sum = 3, average = sum / nom.total() = 3 / 1 = 3.
    let a = analyze(
        "function f($a, $b) {
                 if ($a) {
                     if ($b) {
                         Write-Host \"hi\"
                     }
                 }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cognitive(&a.root.metrics),
        @r###"
    {
      "sum": 3.0,
      "average": 3.0,
      "min": 0.0,
      "max": 3.0
    }
    "###
    );
}

#[test]
fn powershell_try_catch_nesting() {
    // `try` itself does NOT add; each `catch` does and bumps nesting.
    // `finally` adds +1 without nesting.
    let a = analyze(
        "function f {
                 try {
                     if ($a) {
                         Write-Host \"a\"
                     }
                 } catch {
                     if ($b) {
                         throw
                     }
                 } finally {
                     Write-Host \"done\"
                 }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cognitive(&a.root.metrics),
        @r###"
    {
      "sum": 5.0,
      "average": 5.0,
      "min": 0.0,
      "max": 5.0
    }
    "###
    );
}

#[test]
fn powershell_elseif_and_else_flatten() {
    let a = analyze(
        "function f($a) {
                 if ($a -gt 0) {
                     'pos'
                 } elseif ($a -lt 0) {
                     'neg'
                 } else {
                     'zero'
                 }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cognitive(&a.root.metrics),
        @r###"
    {
      "sum": 3.0,
      "average": 3.0,
      "min": 0.0,
      "max": 3.0
    }
    "###
    );
}

#[test]
fn powershell_same_op_boolean_sequence_collapses() {
    // Same-op `-and -and` collapses → +2 (1 if + 1 sequence).
    let a = analyze(
        "function f($a, $b, $c) {
                 if ($a -and $b -and $c) {
                     'ok'
                 }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cognitive(&a.root.metrics),
        @r###"
    {
      "sum": 2.0,
      "average": 2.0,
      "min": 0.0,
      "max": 2.0
    }
    "###
    );

    // Mixed `-and -or` adds +1 each → +3 (1 if + 1 -and + 1 -or).
    let b = analyze(
        "function f($a, $b, $c) {
                 if ($a -and $b -or $c) {
                     'ok'
                 }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cognitive(&b.root.metrics),
        @r###"
    {
      "sum": 3.0,
      "average": 3.0,
      "min": 0.0,
      "max": 3.0
    }
    "###
    );
}

#[test]
fn powershell_wrappers_without_operators_do_not_false_trigger() {
    // tree-sitter-pwsh emits `ternary_expression` /
    // `null_coalesce_expression` / `logical_expression` ONLY when an
    // actual operator is present. Plain `$a + $b` doesn't trigger them.
    // Cognitive must be zero.
    let a = analyze("function Plain { $x = $a + $b }");
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cognitive(&a.root.metrics),
        @r###"
    {
      "sum": 0.0,
      "average": 0.0,
      "min": 0.0,
      "max": 0.0
    }
    "###
    );
}

#[test]
fn powershell_boolean_sequence_does_not_leak_across_else() {
    // Outer `if ($a -and $b)`'s boolean-sequence tracker must not bleed
    // into an inner `if ($c -and $d)` in the `else` body. The inner
    // `if`'s condition is wrapped in a `pipeline` node, which is the
    // statement boundary that resets the tracker.
    let a = analyze(
        "function f($a, $b, $c, $d) {
                 if ($a -and $b) {
                     'x'
                 } else {
                     if ($c -and $d) {
                         'y'
                     }
                 }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cognitive(&a.root.metrics),
        @r###"
    {
      "sum": 6.0,
      "average": 6.0,
      "min": 0.0,
      "max": 6.0
    }
    "###
    );
}

#[test]
fn powershell_boolean_sequence_does_not_leak_across_finally() {
    // Same invariant as the else-leak regression but for `finally`. The
    // reset comes from the `pipeline` wrapping the inner `if`'s
    // condition, not from `finally_clause` itself.
    let a = analyze(
        "function f($a, $b, $c, $d) {
                 try {
                     if ($a -and $b) { 'x' }
                 } finally {
                     if ($c -and $d) { 'y' }
                 }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cognitive(&a.root.metrics),
        @r###"
    {
      "sum": 5.0,
      "average": 5.0,
      "min": 0.0,
      "max": 5.0
    }
    "###
    );
}

#[test]
fn powershell_cognitive_counts_argument_form_decision_operators() {
    // tree-sitter-pwsh emits `*_argument_expression` for expressions
    // inside method-invocation argument lists. Argument-form ternary
    // and null-coalesce are nesting-increasing, and argument-form
    // `logical_argument_expression` participates in same-operator
    // sequence collapsing.
    let a = analyze(
        "function f($a, $b, $cond, $x) {
                 [Foo]::Baz($cond ? 1 : 2)
                 [Foo]::Qux($x ?? 3)
                 [Foo]::Zig($a -and $b -and $cond)
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cognitive(&a.root.metrics),
        @r###"
    {
      "sum": 3.0,
      "average": 3.0,
      "min": 0.0,
      "max": 3.0
    }
    "###
    );
}

#[test]
fn powershell_cognitive_xor_participates_in_boolean_sequence() {
    // `-xor` is a direct child of `logical_expression`; it must
    // participate in the boolean-sequence tracker so a standalone `-xor`
    // adds +1 and mixed chains add +1 per operator-transition.
    let a = analyze(
        "function f($a, $b, $c, $d) {
                 if ($a -xor $b) { 'x' }
                 if ($a -xor $b -xor $c) { 'y' }
                 if ($a -and $b -xor $c -or $d) {
                     'z'
                 }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cognitive(&a.root.metrics),
        @r###"
    {
      "sum": 8.0,
      "average": 8.0,
      "min": 0.0,
      "max": 8.0
    }
    "###
    );
}

#[test]
fn powershell_cognitive_unary_wrappers_do_not_break_boolean_collapsing() {
    // `expression_with_unary_operator` is the grammar wrapper for *all*
    // unary forms (`+$x`, `-$x`, `[int]$x`, `,$x`, `++$x`, `-split $x`,
    // …), not just `-not` / `!`. Storing the wrapper's kind as the
    // "previous boolean" would poison subsequent same-operator
    // collapsing. Only the actual negation tokens (`-not` / `!` /
    // `-bnot`) feed `not_operator`.
    let a = analyze(
        "function f($a, $b, $c) {
                 if ($a -and $b -and $c) { }
                 if (+$a -and $b -and $c) { }
                 if ([int]$a -and $b -and $c) { }
                 if (,$a -and $b -and $c) { }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cognitive(&a.root.metrics),
        @r###"
    {
      "sum": 8.0,
      "average": 8.0,
      "min": 0.0,
      "max": 8.0
    }
    "###
    );
}

#[test]
fn powershell_cognitive_not_negation_still_tracked() {
    // The restricted `DASHnot | BANG | DASHbnot` arm must still feed
    // `BoolSequence::not_operator` so real negation chains collapse.
    // `if (-not $a -and -not $b)` has two same-shape negated operands
    // joined by a single `-and` — total is +1 if +1 -and.
    let a = analyze(
        "function f($a, $b) {
                 if (-not $a -and -not $b) { }
                 if (!$a -or !$b -or !$c) { }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cognitive(&a.root.metrics),
        @r###"
    {
      "sum": 4.0,
      "average": 4.0,
      "min": 0.0,
      "max": 4.0
    }
    "###
    );
}

#[test]
fn powershell_cognitive_pipeline_chain_tail_operators_count() {
    // `cmd1 && cmd2 || cmd3` parses as one `pipeline` containing
    // alternating `pipeline_chain` / `pipeline_chain_tail` children,
    // where each `pipeline_chain_tail` wraps a single `&&` or `||`
    // token. The Pipeline arm scans those tails and feeds the
    // boolean-sequence tracker, so mixed chains add +1 per transition
    // and same-op runs collapse to +1.
    let a = analyze(
        "function f {
                 Get-Thing && Write-Host 'ok' || Write-Error 'bad'
                 Get-A && Get-B && Get-C
                 Get-D
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::cognitive(&a.root.metrics),
        @r###"
    {
      "sum": 3.0,
      "average": 3.0,
      "min": 0.0,
      "max": 3.0
    }
    "###
    );
}
