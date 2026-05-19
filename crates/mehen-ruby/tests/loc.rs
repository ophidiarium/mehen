//! LOC tests for the Phase 9 ruby-prism walker.

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
fn ruby_simple_loc() {
    let a = analyze(
        "# header comment
         def greet(name)
             puts \"hi, #{name}\"
         end",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(
        loc,
        @r###"
    {
      "sloc": 4.0,
      "ploc": 3.0,
      "lloc": 2.0,
      "cloc": 1.0,
      "blank": 0.0,
      "sloc_average": 2.0,
      "ploc_average": 1.5,
      "lloc_average": 1.0,
      "cloc_average": 0.5,
      "blank_average": 0.0,
      "sloc_min": 3.0,
      "sloc_max": 3.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 3.0,
      "ploc_max": 3.0,
      "lloc_min": 2.0,
      "lloc_max": 2.0,
      "blank_min": 0.0,
      "blank_max": 0.0
    }"###
    );
}
