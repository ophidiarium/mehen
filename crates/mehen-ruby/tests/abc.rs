// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! ABC tests for the Phase 9 ruby-prism walker.

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
fn ruby_abc_basic() {
    let a = analyze(
        "def f(a, b)
             c = a + b    # +1 A
             log(c)       # +1 B
             return c if c > 0  # +1 C (if_modifier) + +1 C (>)
         end",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    insta::assert_json_snapshot!(
        abc,
        @r###"
    {
      "assignments": 1.0,
      "branches": 1.0,
      "conditions": 2.0,
      "magnitude": 2.449489742783178,
      "assignments_average": 0.5,
      "branches_average": 0.5,
      "conditions_average": 1.0,
      "assignments_min": 0.0,
      "assignments_max": 1.0,
      "branches_min": 0.0,
      "branches_max": 1.0,
      "conditions_min": 0.0,
      "conditions_max": 2.0
    }"###
    );
}
