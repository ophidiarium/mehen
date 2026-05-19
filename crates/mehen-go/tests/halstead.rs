//! Halstead tests for the Go walker.
//!
//! Every legacy `check_metrics::<GoParser>` Halstead test from
//! `crates/mehen-engine/src/legacy/metrics/halstead.rs` is ported here
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
fn go_operators_and_operands() {
    let a = analyze(
        "package main

            func add(a, b int) int {
                return a + b
            }",
    );
    let halstead = mehen_report::metrics_json::halstead(&a.root.metrics);
    insta::assert_json_snapshot!(
        halstead,
        @r###"
    {
      "n1": 7.0,
      "N1": 7.0,
      "n2": 5.0,
      "N2": 8.0,
      "length": 15.0,
      "estimated_program_length": 31.26112492884004,
      "purity_ratio": 2.0840749952560027,
      "vocabulary": 12.0,
      "volume": 53.77443751081734,
      "difficulty": 5.6,
      "level": 0.17857142857142858,
      "effort": 301.1368500605771,
      "time": 16.729825003365395,
      "bugs": 0.014975730436275946
    }"###
    );
}
