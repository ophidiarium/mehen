// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Cognitive complexity tests for the Go walker.
//!
//! Every legacy `check_metrics::<GoParser>` cognitive test from
//! `crates/mehen-engine/src/legacy/metrics/cognitive.rs` is ported
//! here byte-identical so the parity contract (plan §12.3.1) is
//! visibly maintained.

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
fn go_no_cognitive() {
    // Drift from legacy: legacy serialized `null` when no functions
    // were observed (so the average's denominator was zero). The 1.0
    // mehen-metrics `CognitiveStats` defaults the empty average to
    // 0.0 — same convention applied in Phase 6 Python and Phase 9
    // Ruby.
    let a = analyze(
        "package main

            var x = 42",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
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

            func f() {
                if true { // +1
                    if false { // +2 (nesting = 1)
                        println(\"test\")
                    }
                }
            }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 3.0,
      "average": 3.0,
      "min": 0.0,
      "max": 3.0
    }"###
    );
}

#[test]
fn go_for_loop() {
    let a = analyze(
        "package main

            func f() {
                for i := 0; i < 10; i++ { // +1
                    if i > 5 { // +2 (nesting = 1)
                        println(i)
                    }
                }
            }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 3.0,
      "average": 3.0,
      "min": 0.0,
      "max": 3.0
    }"###
    );
}

#[test]
fn go_logical_operators() {
    let a = analyze(
        "package main

            func f(a, b, c bool) {
                if a && b && c { // +1 (if) +1 (sequence of &&)
                    println(\"all true\")
                }
            }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
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
fn go_logical_operator_sequences_reset_between_statements() {
    let a = analyze(
        "package main

            func f(a, b, c, d bool) {
                _ = a && b
                _ = c && d
            }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
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
fn go_logical_operator_sequences_reset_between_declaration_specs() {
    let a = analyze(
        "package main

            func f(a, b, c, d bool) {
                var x = a && b
                var y = c && d
                const p = true && false
                const q = false && true
            }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 4.0,
      "average": 4.0,
      "min": 0.0,
      "max": 4.0
    }"###
    );
}
