//! NPM tests for the Phase 9 ruby-prism walker.

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
fn ruby_npm_counts_methods_as_public() {
    let a = analyze(
        "class C
             def a; end
             def b; end
         end",
    );
    let npm = mehen_report::metrics_json::npm(&a.root.metrics);
    insta::assert_json_snapshot!(
        npm,
        @r#"
    {
      "classes": 2.0,
      "interfaces": 0.0,
      "class_methods": 2.0,
      "interface_methods": 0.0,
      "classes_average": 1.0,
      "interfaces_average": null,
      "total": 2.0,
      "total_methods": 2.0,
      "average": 1.0
    }
    "#
    );
}
