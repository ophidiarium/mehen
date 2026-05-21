// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! NExit tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/exit.rs::tests`.

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
fn rust_no_exit() {
    // Drift from pre-1.0: `average: null` for empty function buckets has
    // become `0.0` in Phase-1+ accumulators (same convention as cognitive).
    let a = analyze("let a = 42;");
    let nx = mehen_report::metrics_json::nexits(&a.root.metrics);
    insta::assert_json_snapshot!(
        nx,
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
fn rust_question_mark() {
    // Three `?` operators, all at the unit level (no functions). Each
    // contributes +1 NExit.
    let a = analyze("let _ = a? + b? + c?;");
    let nx = mehen_report::metrics_json::nexits(&a.root.metrics);
    insta::assert_json_snapshot!(
        nx,
        @r###"
    {
      "sum": 3.0,
      "average": 0.0,
      "min": 3.0,
      "max": 3.0
    }"###
    );
}

#[test]
fn rust_return_type_is_not_an_exit() {
    let a = analyze(
        "fn typed() -> () {}
         fn explicit() {
             return;
         }
         fn question() {
             a?;
         }",
    );
    let nx = mehen_report::metrics_json::nexits(&a.root.metrics);
    assert_eq!(nx.sum, 2.0, "got {}", serde_json::to_string(&nx).unwrap());
    assert_eq!(nx.max, 1.0, "got {}", serde_json::to_string(&nx).unwrap());
}
