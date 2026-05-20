//! NOM tests for the Phase 9 ruby-prism walker.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_ruby::RubyAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = RubyAnalyzer::new();
    let file = SourceFile::new("foo.rb".into(), Language::Ruby, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn ruby_nom() {
    let a = analyze(
        "def a
             1
         end
         def b
             2
         end
         def c
             3
         end
         x = -> (a) { a + 42 }",
    );
    let nom = mehen_report::metrics_json::nom(&a.root.metrics);
    insta::assert_json_snapshot!(
        nom,
        @r###"
    {
      "functions": 3.0,
      "closures": 1.0,
      "functions_average": 0.6,
      "closures_average": 0.2,
      "total": 4.0,
      "average": 0.8,
      "functions_min": 0.0,
      "functions_max": 1.0,
      "closures_min": 0.0,
      "closures_max": 1.0
    }"###
    );
}

#[test]
fn ruby_do_lambda_counts_as_one_closure() {
    let a = analyze(
        "x = -> (a) do
             a + 42
         end",
    );
    let nom = mehen_report::metrics_json::nom(&a.root.metrics);
    insta::assert_json_snapshot!(
        nom,
        @r###"
    {
      "functions": 0.0,
      "closures": 1.0,
      "functions_average": 0.0,
      "closures_average": 0.5,
      "total": 1.0,
      "average": 0.5,
      "functions_min": 0.0,
      "functions_max": 0.0,
      "closures_min": 0.0,
      "closures_max": 1.0
    }"###
    );
}
