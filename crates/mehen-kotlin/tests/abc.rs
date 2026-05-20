//! ABC tests for the tree-sitter-kotlin walker.

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
fn kotlin_abc_basic() {
    let a = analyze(
        "fun f(a: Int, b: Int): Int {
             val c = a + b        // +1 A (val with initializer)
             log(c)               // +1 B
             if (c > 0) {         // +1 C (if) + +1 C (>)
                 return c
             }
             return 0
         }",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    insta::assert_json_snapshot!(
        abc,
        @r###"
    {
      "assignments": 1.0,
      "branches": 1.0,
      "conditions": 2.0,
      "magnitude": 2.449489742783178,
      "assignments_average": 0.5,
      "branches_average": 0.5,
      "conditions_average": 1.0,
      "assignments_min": 0.0,
      "assignments_max": 1.0,
      "branches_min": 0.0,
      "branches_max": 1.0,
      "conditions_min": 0.0,
      "conditions_max": 2.0
    }"###
    );
}
