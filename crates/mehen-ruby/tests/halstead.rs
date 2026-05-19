//! Halstead tests for the Phase 9 ruby-prism walker.

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
fn ruby_operators_and_operands() {
    let a = analyze(
        "def add(a, b)
             a + b
         end",
    );
    let h = mehen_report::metrics_json::halstead(&a.root.metrics);
    insta::assert_json_snapshot!(
        h,
        {
            ".estimated_program_length" => "[masked]",
            ".purity_ratio" => "[masked]",
            ".volume" => "[masked]",
            ".difficulty" => "[masked]",
            ".level" => "[masked]",
            ".effort" => "[masked]",
            ".time" => "[masked]",
            ".bugs" => "[masked]"
        },
        @r###"
    {
      "n1": 4.0,
      "N1": 4.0,
      "n2": 3.0,
      "N2": 5.0,
      "length": 9.0,
      "estimated_program_length": "[masked]",
      "purity_ratio": "[masked]",
      "vocabulary": 7.0,
      "volume": "[masked]",
      "difficulty": "[masked]",
      "level": "[masked]",
      "effort": "[masked]",
      "time": "[masked]",
      "bugs": "[masked]"
    }"###
    );
}
