// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! NOM tests for the tree-sitter-kotlin walker.

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
fn kotlin_init_block_is_not_counted_as_function() {
    let a = analyze(
        "class C {
             init {
                 println(\"ready\")
             }
         }",
    );
    let nom = mehen_report::metrics_json::nom(&a.root.metrics);
    insta::assert_json_snapshot!(
        nom,
        @r###"
    {
      "functions": 0.0,
      "closures": 0.0,
      "functions_average": 0.0,
      "closures_average": 0.0,
      "total": 0.0,
      "average": 0.0,
      "functions_min": 0.0,
      "functions_max": 0.0,
      "closures_min": 0.0,
      "closures_max": 0.0
    }"###
    );
}
