// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! NExit tests for the Go walker.
//!
//! Every legacy `check_metrics::<GoParser>` exit test from
//! `crates/mehen-engine/src/legacy/metrics/exit.rs` is ported here
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
fn go_no_exit() {
    // Drift from legacy: legacy serialized `null` when no functions
    // were observed (so the average's denominator was zero). The 1.0
    // mehen-metrics `NexitStats` defaults the empty average to 0.0 —
    // same convention applied in Phase 6 Python and Phase 9 Ruby.
    let a = analyze("var a = 42");
    let nexits = mehen_report::metrics_json::nexits(&a.root.metrics);
    insta::assert_json_snapshot!(
        nexits,
        @r###"
    {
      "sum": 0.0,
      "average": 0.0,
      "min": 0.0,
      "max": 0.0
    }"###
    );
}

#[test]
fn go_simple_function() {
    let a = analyze(
        "package main

            func max(a, b int) int {
                if a > b {
                    return a
                }
                return b
            }",
    );
    let nexits = mehen_report::metrics_json::nexits(&a.root.metrics);
    // 2 exits / 1 function
    insta::assert_json_snapshot!(
        nexits,
        @r###"
    {
      "sum": 2.0,
      "average": 2.0,
      "min": 0.0,
      "max": 2.0
    }"###
    );
}

#[test]
fn go_multiple_functions() {
    let a = analyze(
        "package main

            func f1() int {
                return 1
            }

            func f2() int {
                return 2
            }",
    );
    let nexits = mehen_report::metrics_json::nexits(&a.root.metrics);
    // 2 exits / 2 functions
    insta::assert_json_snapshot!(
        nexits,
        @r###"
    {
      "sum": 2.0,
      "average": 1.0,
      "min": 0.0,
      "max": 1.0
    }"###
    );
}
