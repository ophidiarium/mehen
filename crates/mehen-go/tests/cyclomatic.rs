//! Cyclomatic complexity tests for the Go walker.
//!
//! Every legacy `check_metrics::<GoParser>` cyclomatic test from
//! `crates/mehen-engine/src/legacy/metrics/cyclomatic.rs` is ported
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
fn go_simple_function() {
    let a = analyze(
        "package main

            func calculate(a, b int) int { // +2 (+1 unit space)
                if a > b { // +1
                    return a
                }
                return b
            }",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    insta::assert_json_snapshot!(
        cy,
        @r###"
    {
      "sum": 3.0,
      "average": 1.5,
      "min": 1.0,
      "max": 2.0
    }"###
    );
}

#[test]
fn go_switch_statement() {
    let a = analyze(
        "package main

            func grade(score int) string { // +2 (+1 unit space)
                switch { // switch itself doesn't add, cases do
                case score >= 90: // +1
                    return \"A\"
                case score >= 80: // +1
                    return \"B\"
                case score >= 70: // +1
                    return \"C\"
                default: // default is fallthrough, not a decision point
                    return \"F\"
                }
            }",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    insta::assert_json_snapshot!(
        cy,
        @r###"
    {
      "sum": 5.0,
      "average": 2.5,
      "min": 1.0,
      "max": 4.0
    }"###
    );
}

#[test]
fn go_select_default_counts() {
    // `default` in a `switch` is fallthrough and should NOT count,
    // but `default` in a `select` is an additional executable
    // communication branch and SHOULD count.
    let a = analyze(
        "package main

            func f(ch chan int) { // +2 (+1 unit space)
                select { // +1 CommunicationCase
                case v := <-ch:
                    _ = v
                default: // +1 default branch of select
                }
            }",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    insta::assert_json_snapshot!(
        cy,
        @r###"
    {
      "sum": 4.0,
      "average": 2.0,
      "min": 1.0,
      "max": 3.0
    }"###
    );
}

#[test]
fn go_logical_operators() {
    let a = analyze(
        "package main

            func check(a, b, c bool) bool { // +2 (+1 unit space)
                if a && b || c { // +3 (+1 if, +1 &&, +1 ||)
                    return true
                }
                return false
            }",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    insta::assert_json_snapshot!(
        cy,
        @r###"
    {
      "sum": 5.0,
      "average": 2.5,
      "min": 1.0,
      "max": 4.0
    }"###
    );
}
