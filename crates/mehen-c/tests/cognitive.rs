// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Cognitive complexity tests for the C walker.
//!
//! Every legacy `check_metrics::<CParser>` cognitive test from
//! `crates/mehen-engine/src/legacy/metrics/cognitive.rs` is ported
//! here byte-identical so the parity contract (plan §12.3.1) is
//! visibly maintained. No drift expected — this is a tree-sitter→
//! tree-sitter migration, not a parser swap.

use mehen_c::CAnalyzer;
use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = CAnalyzer::new();
    let file = SourceFile::new("foo.c".into(), Language::C, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn c_boolean_sequence_does_not_leak_across_else_if() {
    // Regression: tree-sitter-c parses `else if` as
    // `else_clause { if_statement }`. The outer `if (a && b)`'s
    // boolean-sequence tracker must not bleed into the inner
    // `else if (c && d)` condition — otherwise the second `&&` would
    // collapse with the first (same operator) and cognitive would be
    // undercounted.
    //
    // Expected breakdown for `int f(int a, int b, int c, int d)`:
    //   +1 outer `if`                 (nesting = 0 -> 1)
    //   +1 outer `&&`                 (first op in sequence)
    //   +1 `else` clause              (no nesting)
    //   +0 inner `if` (else-if arm)   (structural cost paid by `else`)
    //   +1 inner `&&`                 (fresh sequence — IF reset works)
    // total = 4.
    let a = analyze(
        "int f(int a, int b, int c, int d) {
             if (a && b) {
                 return 1;
             } else if (c && d) {
                 return 2;
             }
             return 0;
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    assert_eq!(cog.sum, 4.0);
}
