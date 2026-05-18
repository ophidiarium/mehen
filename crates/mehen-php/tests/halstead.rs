//! Halstead tests for the Phase 8 mago-syntax-backed walker.
//!
//! The legacy tree-sitter PHP suite carried no Halstead snapshot
//! (Halstead was implemented but untested in `legacy/metrics/halstead.rs`),
//! so these tests are new. They lock in the operator/operand
//! classification table in `walker::classify_token` and the
//! token-sweep pipeline.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_php::PhpAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = PhpAnalyzer::new();
    let file = SourceFile::new("foo.php".into(), Language::Php, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn php_halstead_simple_function() {
    // `function add(int $a, int $b): int { return $a + $b; }` exercises
    // keyword (`function`, `return`), punctuation (`(`, `,`, `:`, `{`, `;`),
    // arithmetic (`+`), identifier (`add`, `int`), and variable
    // (`$a`, `$b`) classification. Closing parens / braces pair with
    // the openers (classical Halstead) so they don't appear in `n1`.
    let a = analyze(
        "<?php
         function add(int $a, int $b): int {
             return $a + $b;
         }",
    );
    let h = mehen_report::metrics_json::halstead(&a.root.metrics);
    insta::assert_json_snapshot!(
        h,
        @r###"
    {
      "n1": 8.0,
      "N1": 8.0,
      "n2": 4.0,
      "N2": 8.0,
      "length": 16.0,
      "estimated_program_length": 32.0,
      "purity_ratio": 2.0,
      "vocabulary": 12.0,
      "volume": 57.3594000115385,
      "difficulty": 8.0,
      "level": 0.125,
      "effort": 458.875200092308,
      "time": 25.49306667179489,
      "bugs": 0.01983087162785987
    }"###
    );
}

#[test]
fn php_halstead_string_part_is_skipped() {
    // Double-quoted strings emit `DoubleQuote` + `StringPart` tokens
    // around any interior characters. The classifier must skip those
    // so the wrapping span doesn't count twice — only literal,
    // closed strings (`LiteralString`) contribute as operands.
    let a = analyze(
        "<?php
         function f() {
             return 'plain';
         }",
    );
    let h = mehen_report::metrics_json::halstead(&a.root.metrics);
    // function (1), ( (2), ) - skip, { (3), return (4), ; (5), } - skip
    // Wait: ( and ) are paired in classical Halstead; ) is skipped.
    // Operators present: function, (, {, return, ;
    // n1 = 5, all single-occurrence -> N1 = 5
    // Operands: 'plain' (1× String), `f` (1× Identifier) -> n2=2, N2=2
    assert_eq!(h.n1, 5.0, "{}", serde_json::to_string(&h).unwrap());
    assert_eq!(h.n2, 2.0, "{}", serde_json::to_string(&h).unwrap());
    assert_eq!(h.length, 7.0, "{}", serde_json::to_string(&h).unwrap());
}
