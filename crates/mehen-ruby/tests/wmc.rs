//! WMC tests for the Phase 9 ruby-prism walker.

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
fn ruby_wmc_class_sums_method_cyclomatics() {
    let a = analyze(
        "class C
             def a(x)
                 return 1 if x
                 return 0
             end
             def b
                 1
             end
         end",
    );
    let wmc = mehen_report::metrics_json::wmc(&a.root.metrics);
    insta::assert_json_snapshot!(
        wmc,
        @r###"
    {
      "classes": 3.0,
      "interfaces": 0.0,
      "total": 3.0
    }"###
    );
}
