// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

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

/// Regression: methods inside a class must carry their own Halstead
/// counts in the per-space JSON. PR #95 discussion_r3265658502 flagged
/// the same bug on the Python walker; Ruby's helper methods recorded
/// every event onto `stack[0]`, leaving inner methods with zero
/// `halstead.N1`/`halstead.N2`.
#[test]
fn ruby_method_halstead_is_non_zero() {
    let a = analyze(
        "class C
  def m(a, b)
    a + b
  end
end",
    );
    assert_eq!(a.root.spaces.len(), 1, "expected one class space");
    let class = &a.root.spaces[0];
    assert_eq!(class.spaces.len(), 1, "expected one method space");
    let method = &class.spaces[0];
    let method_h = mehen_report::metrics_json::halstead(&method.metrics);
    assert!(
        method_h.big_n1 > 0.0,
        "method must record at least `def` / `+` operators, got {}",
        serde_json::to_string(&method_h).unwrap()
    );
    assert!(
        method_h.big_n2 > 0.0,
        "method must record `m`, `a`, `b` operands, got {}",
        serde_json::to_string(&method_h).unwrap()
    );
    let class_h = mehen_report::metrics_json::halstead(&class.metrics);
    assert!(
        class_h.big_n1 >= method_h.big_n1,
        "class N1 must roll up method: class={} method={}",
        serde_json::to_string(&class_h).unwrap(),
        serde_json::to_string(&method_h).unwrap()
    );
}
