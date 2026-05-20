//! Cognitive complexity tests for the Phase 9 ruby-prism walker —
//! ported from `crates/mehen-engine/src/legacy/metrics/cognitive.rs`.

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
fn ruby_no_cognitive() {
    // Drift from legacy: legacy serialized `null` when no functions
    // were observed (so the average's denominator was zero). The 1.0
    // mehen-metrics `CognitiveStats` defaults the empty average to
    // 0.0 — same convention applied in Phase 6 Python (see
    // `crates/mehen-python/tests/cognitive.rs::python_no_cognitive`).
    let a = analyze("a = 42");
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
fn ruby_simple_method() {
    let a = analyze(
        "def f(a, b)
             if a && b  # +2 (+1 if, +1 &&)
                return 1
             end
             if c && d  # +2 (+1 if, +1 &&)
                return 1
             end
         end",
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

#[test]
fn ruby_nested_if_and_else() {
    let a = analyze(
        "def f(a, b)
             if a          # +1
                if b        # +2 (nesting = 1)
                   return 1
                else        # +1
                   return 2
                end
             end
         end",
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

#[test]
fn ruby_modifier_and_rescue() {
    let a = analyze(
        "def f(a)
             return a if a > 0  # +1 if_modifier
             begin
                risky!
             rescue StandardError  # +1 (nesting +1 because in begin)
                retry
             end
         end",
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
fn ruby_rescue_modifier() {
    let a = analyze(
        "def f
             value = risky rescue fallback  # +1 rescue_modifier
         end",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 1.0,
      "average": 1.0,
      "min": 0.0,
      "max": 1.0
    }"###
    );
}

#[test]
fn ruby_lambda_with_block() {
    let a = analyze(
        "def f
             x = -> { if a then 1 end }
         end",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 2.0,
      "average": 1.0,
      "min": 0.0,
      "max": 2.0
    }"###
    );
}

#[test]
fn ruby_nested_method_in_singleton_method() {
    let a = analyze(
        "def self.outer
             def inner
               if x then 1 end
             end
         end",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 2.0,
      "average": 1.0,
      "min": 0.0,
      "max": 2.0
    }"###
    );
}
