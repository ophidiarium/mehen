//! NExit tests for the Phase 9 ruby-prism walker.

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
fn ruby_no_exit() {
    // Drift from legacy: legacy serialized `null` when no functions
    // were observed (zero denominator). The 1.0 mehen-metrics
    // `NexitStats` defaults the empty average to 0.0 — same convention
    // applied in Phase 6 (see Python tests).
    let a = analyze("a = 42");
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
fn ruby_simple_method() {
    let a = analyze(
        "def f(a, b)
             return a if a > b
             return b
         end",
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
fn ruby_break_and_next() {
    // Both `break` and `next` are counted as exits; `yield` is not.
    let a = analyze(
        "def f(xs)
             xs.each do |x|
               next if x.nil?
               break if x.stop?
               yield x
             end
         end",
    );
    let nexits = mehen_report::metrics_json::nexits(&a.root.metrics);
    insta::assert_json_snapshot!(
        nexits,
        @r###"
    {
      "sum": 2.0,
      "average": 1.0,
      "min": 0.0,
      "max": 2.0
    }"###
    );
}
