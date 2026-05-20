//! Cyclomatic complexity tests for the tree-sitter-kotlin walker.
//!
//! Every legacy `check_metrics::<KotlinParser>` cyclomatic test from
//! `crates/mehen-engine/src/legacy/metrics/cyclomatic.rs` is ported
//! here byte-identical so the parity contract (plan §12.3.1) is
//! visibly maintained.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_kotlin::KotlinAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = KotlinAnalyzer::new();
    let file = SourceFile::new("foo.kt".into(), Language::Kotlin, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn kotlin_simple_function() {
    let a = analyze(
        "fun f(a: Int, b: Int): Int { // +2 (+1 unit space, +1 fun)
             if (a > b) { // +1
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
fn kotlin_when_branches_count() {
    // `when` itself doesn't add; each branch (`when_entry`) does.
    let a = analyze(
        "fun grade(score: Int): String { // +2 (+1 unit, +1 fun)
             return when { // +0
                 score >= 90 -> \"A\" // +1
                 score >= 80 -> \"B\" // +1
                 score >= 70 -> \"C\" // +1
                 else -> \"F\"       // +1 (else is its own when_entry)
             }
         }",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    insta::assert_json_snapshot!(
        cy,
        @r###"
    {
      "sum": 6.0,
      "average": 3.0,
      "min": 1.0,
      "max": 5.0
    }"###
    );
}

#[test]
fn kotlin_try_catch_counts_catch_not_try() {
    // Aligns with SonarKotlin's `CyclomaticComplexityVisitor`: `try`
    // itself is NOT a decision point, and `catch` is NOT either —
    // SonarKotlin counts `catch` only in cognitive complexity, not
    // cyclomatic. Reference:
    //   sonar-kotlin-metrics/.../CyclomaticComplexityVisitor.kt
    let a = analyze(
        "fun f() { // +2 (+1 unit, +1 fun)
             try {
                 risky()
             } catch (e: Exception) {
                 // catch does not add cyclomatic complexity per SonarKotlin
             }
         }",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    insta::assert_json_snapshot!(
        cy,
        @r###"
    {
      "sum": 2.0,
      "average": 1.0,
      "min": 1.0,
      "max": 1.0
    }"###
    );
}

#[test]
fn kotlin_logical_operators() {
    let a = analyze(
        "fun check(a: Boolean, b: Boolean, c: Boolean): Boolean { // +2
             if (a && b || c) { // +3 (+1 if, +1 &&, +1 ||)
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
