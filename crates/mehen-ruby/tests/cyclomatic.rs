// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Cyclomatic complexity tests for the Phase 9 ruby-prism walker.
//!
//! Every legacy `check_metrics::<RubyParser>` cyclomatic test from
//! `crates/mehen-engine/src/legacy/metrics/cyclomatic.rs` is ported
//! here byte-identical so the parity contract (plan §12.3.1) is
//! visibly maintained.

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
fn ruby_simple_method() {
    let a = analyze(
        "def f(a, b) # +2 (+1 unit space)
             if a && b # +2 (+1 if, +1 &&)
                 return 1
             end
             if c or d # +2 (+1 if, +1 or)
                 return 1
             end
         end",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    insta::assert_json_snapshot!(
        cy,
        @r###"
    {
      "sum": 6.0,
      "average": 3.0,
      "min": 1.0,
      "max": 5.0
    }"###
    );
}

#[test]
fn ruby_modifier_forms() {
    // Each trailing-modifier form contributes +1 like its block form.
    let a = analyze(
        "def f(a)      # +1 unit space +1 method
             return a if a > 0   # +1 if_modifier
             return -a unless a == 0 # +1 unless_modifier
         end",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    insta::assert_json_snapshot!(
        cy,
        @r###"
    {
      "sum": 4.0,
      "average": 2.0,
      "min": 1.0,
      "max": 3.0
    }"###
    );
}

#[test]
fn ruby_case_when() {
    let a = analyze(
        "def f(x)      # +1 unit +1 method
             case x    # case itself doesn't add; each `when` does
             when 1 then 'a' # +1
             when 2 then 'b' # +1
             when 3 then 'c' # +1
             else 'z'
             end
         end",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    insta::assert_json_snapshot!(
        cy,
        @r###"
    {
      "sum": 5.0,
      "average": 2.5,
      "min": 1.0,
      "max": 4.0
    }"###
    );
}
