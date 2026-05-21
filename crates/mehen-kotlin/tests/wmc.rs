// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! WMC tests for the tree-sitter-kotlin walker.

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
fn kotlin_wmc_class_sums_method_cyclomatics() {
    let a = analyze(
        "class C {
             fun a(x: Int): Int {
                 return if (x > 0) 1 else 0
             }
             fun b(): Int { return 1 }
         }",
    );
    let wmc = mehen_report::metrics_json::wmc(&a.root.metrics);
    // class C -> a cyc = 2 (if), b cyc = 1 -> 3
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
