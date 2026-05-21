// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! NPA tests for the Phase 9 ruby-prism walker.

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
fn ruby_npa_counts_instance_variables_under_body_statement() {
    // 2 ivar attributes, both non-public by convention.
    let a = analyze(
        "class C
             @x = 1
             @y = 2
         end",
    );
    let npa = mehen_report::metrics_json::npa(&a.root.metrics);
    insta::assert_json_snapshot!(
        npa,
        @r###"
    {
      "classes": 0.0,
      "interfaces": 0.0,
      "class_attributes": 2.0,
      "interface_attributes": 0.0,
      "classes_average": 0.0,
      "interfaces_average": null,
      "total": 0.0,
      "total_attributes": 2.0,
      "average": 0.0
    }"###
    );
}
