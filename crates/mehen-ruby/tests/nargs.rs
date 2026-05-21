// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! NArgs tests for the Phase 9 ruby-prism walker.

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
fn ruby_single_method() {
    // Drift from legacy: legacy reported `functions_min: 0.0` because
    // its `compute_minmax` ran unconditionally for every space, so the
    // unit space (which has no fn args) pulled the min down to 0. The
    // 1.0 mehen-metrics `NargsStats::finalize_minmax` only includes
    // a space in the function bounds if `is_function == true`, so the
    // unit no longer dilutes the bounds. Result: `functions_min: 2.0`
    // — matching the *only* function in this fixture (`def f(a, b)`).
    // This drift is shared with Phase 6 Python (see
    // `crates/mehen-python/tests/nargs.rs::python_single_function`).
    let a = analyze(
        "def f(a, b)
             a + b
         end",
    );
    let nargs = mehen_report::metrics_json::nargs(&a.root.metrics);
    insta::assert_json_snapshot!(
        nargs,
        @r###"
    {
      "total_functions": 2.0,
      "total_closures": 0.0,
      "average_functions": 2.0,
      "average_closures": 0.0,
      "total": 2.0,
      "average": 2.0,
      "functions_min": 2.0,
      "functions_max": 2.0,
      "closures_min": 0.0,
      "closures_max": 0.0
    }"###
    );
}

#[test]
fn ruby_block_and_lambda_args() {
    // `do |a, b| ... end` is a block (closure); `-> (x) { ... }` is a lambda.
    //
    // Drift from legacy: see `ruby_single_method` above. `closures_min`
    // is now 1.0 because both closures bring 1+ args; the legacy 0 came
    // from including the non-closure unit space in the closure bounds.
    let a = analyze(
        "xs.each do |a, b|
             a + b
         end
         f = -> (x) { x * 2 }",
    );
    let nargs = mehen_report::metrics_json::nargs(&a.root.metrics);
    insta::assert_json_snapshot!(
        nargs,
        @r###"
    {
      "total_functions": 0.0,
      "total_closures": 3.0,
      "average_functions": 0.0,
      "average_closures": 1.5,
      "total": 3.0,
      "average": 1.5,
      "functions_min": 0.0,
      "functions_max": 0.0,
      "closures_min": 1.0,
      "closures_max": 2.0
    }"###
    );
}
