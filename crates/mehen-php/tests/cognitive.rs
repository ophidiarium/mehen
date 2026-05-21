// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Cognitive complexity tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/cognitive.rs::tests`.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_php::PhpAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = PhpAnalyzer::new();
    let file = SourceFile::new("foo.php".into(), Language::Php, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn php_else_branch_resets_boolean_sequence() {
    // The boolean-operator sequence must reset when entering the
    // `else` branch so that operators inside the else body start a
    // fresh sequence rather than continuing the sequence from the
    // `if` condition. Without the reset, two same-operator runs
    // separated only by an `else` collapse — undercounting cognitive
    // complexity.
    //
    // The bodies are intentionally empty: a non-empty body's
    // `expression_statement` would itself reset `boolean_seq` and
    // mask the bug.
    //
    // Breakdown WITH reset (correct):
    //   - outer `if`: +1 nesting -> structural=1
    //   - outer `&&`: fresh sequence, +1 -> 2
    //   - `else` clause: +1 (no nesting), reset -> 3
    //   - inner `else if`: parses as nested `if_statement` whose
    //     `is_else_if` is true; counted as elseif (no extra nesting)
    //   - inner `&&`: with the reset, fresh sequence again, +1 -> 4
    //
    // WITHOUT the reset, the inner `&&` collapses with the outer
    // (same operator) and contributes 0, yielding 3.
    let a = analyze(
        "<?php
         function f($a, $b, $c, $d) {
             if ($a && $b) {} else if ($c && $d) {}
         }",
    );
    let co = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        co,
        @r###"
    {
      "sum": 4.0,
      "average": 4.0,
      "min": 0.0,
      "max": 4.0
    }"###
    );
}
