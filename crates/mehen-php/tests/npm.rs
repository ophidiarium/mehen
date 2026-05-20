//! NPM tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/npm.rs::tests`.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_php::PhpAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = PhpAnalyzer::new();
    let file = SourceFile::new("foo.php".into(), Language::Php, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn php_npm_visibility_keywords_are_case_insensitive() {
    // PHP keywords are case-insensitive per the language spec, so
    // `PRIVATE` / `Protected` must be recognized as non-public.
    // public: a. non-public: b, c.
    let a = analyze(
        "<?php
         class C {
             public function a() {}
             PRIVATE function b() {}
             Protected function c() {}
         }",
    );
    let npm = mehen_report::metrics_json::npm(&a.root.metrics);
    insta::assert_json_snapshot!(
        npm,
        @r#"
    {
      "classes": 1.0,
      "interfaces": 0.0,
      "class_methods": 3.0,
      "interface_methods": 0.0,
      "classes_average": 0.3333333333333333,
      "interfaces_average": null,
      "total": 1.0,
      "total_methods": 3.0,
      "average": 0.3333333333333333
    }
    "#
    );
}
