//! NExit tests for the tree-sitter-kotlin walker.

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
fn kotlin_return_and_throw_count_as_exits() {
    let a = analyze(
        "fun f(a: Int): Int {
             if (a < 0) {
                 throw IllegalArgumentException(\"bad\")
             }
             return a
         }",
    );
    let nexits = mehen_report::metrics_json::nexits(&a.root.metrics);
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
fn kotlin_labeled_lambda_return_does_not_count_as_function_exit() {
    let a = analyze(
        "fun f(xs: List<Int>) {
             xs.forEach { x ->
                 if (x < 0) return@forEach
             }
         }",
    );
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
