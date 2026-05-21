// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Cyclomatic complexity tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/cyclomatic.rs::tests` per
//! rewrite plan §12.3 (parity contract). Every pre-1.0
//! `check_metrics::<RustParser>` Rust test is reproduced here against
//! the Phase 9 ra_ap_syntax-backed walker.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_rust::RustAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    // Match legacy `check_metrics`: trim trailing newlines and append one.
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = RustAnalyzer::new();
    let file = SourceFile::new("foo.rs".into(), Language::Rust, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn rust_1_level_nesting() {
    let a = analyze(
        "fn f() {
             if true {
                 match true {
                     true => println!(\"test\"),
                     false => println!(\"test\"),
                 }
             }
         }",
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

#[test]
fn rust_macro_tokens_are_opaque_for_cyclomatic() {
    let a = analyze(
        "fn f() {
             maybe!(a && b, if c { d() });
         }",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    // Unit (1) + function baseline (1) = 2. Macro body tokens (`&&`,
    // `if`) do not count — they are not parsed Rust control flow.
    assert_eq!(cy.sum, 2.0, "got {}", serde_json::to_string(&cy).unwrap());
}
