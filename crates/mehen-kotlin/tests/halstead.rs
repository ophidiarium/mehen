// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Halstead tests for the tree-sitter-kotlin walker.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_kotlin::KotlinAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = KotlinAnalyzer::new();
    let file = SourceFile::new("foo.kt".into(), Language::Kotlin, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

/// Regression: `get`/`set` accessor keywords must be Halstead operators
/// and `field` must be an operand. Mirrors legacy
/// `getter.rs::kotlin_accessor_tokens_are_classified_for_halstead` —
/// that test reached into the private `KotlinCode::get_op_type` table;
/// the walker's `halstead_op_type` is also private, so we lock the
/// assertion to the per-property accessor's full Halstead snapshot.
/// A regression that drops `get`/`set` from the operator table or
/// `field` from the operand table would visibly shift these counts.
#[test]
fn kotlin_accessor_tokens_contribute_to_halstead() {
    let a = analyze(
        "class C {
             var x: Int = 0
                 get() = field
                 set(value) { field = value }
         }",
    );
    let h = mehen_report::metrics_json::halstead(&a.root.metrics);
    // Operators in this fragment: `class`, `var`, `=`, `get`, `set`,
    // `(`, `{`, `.` (none here) — distinct keyword/punctuation kinds.
    // Operands: `C`, `x`, `Int`, `0`, `value`, `field`.
    assert_eq!(h.n1, 8.0, "distinct operators (class/var/=/get/set/(/{{/:)");
    assert_eq!(h.big_n1, 12.0);
    assert_eq!(h.n2, 6.0, "distinct operands (C/x/Int/0/value/field)");
    assert_eq!(h.big_n2, 8.0);
}

#[test]
fn kotlin_operators_and_operands() {
    let a = analyze(
        "fun add(a: Int, b: Int): Int {
             return a + b
         }",
    );
    let h = mehen_report::metrics_json::halstead(&a.root.metrics);
    // Only core counts are locked in; derived measures shift with the
    // vocabulary in ways that aren't meaningful to assert.
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
      "n1": 7.0,
      "N1": 9.0,
      "n2": 4.0,
      "N2": 8.0,
      "length": 17.0,
      "estimated_program_length": "[masked]",
      "purity_ratio": "[masked]",
      "vocabulary": 11.0,
      "volume": "[masked]",
      "difficulty": "[masked]",
      "level": "[masked]",
      "effort": "[masked]",
      "time": "[masked]",
      "bugs": "[masked]"
    }"###
    );
}
