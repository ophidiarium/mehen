//! Cyclomatic complexity tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/cyclomatic.rs::tests` per
//! rewrite plan §12.3 (parity contract). Every pre-1.0
//! `check_metrics::<PhpParser>` PHP test is reproduced here against
//! the Phase 8 mago-syntax-backed walker.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_php::PhpAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    // Match legacy `check_metrics`: trim trailing newlines and append one.
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = PhpAnalyzer::new();
    let file = SourceFile::new("foo.php".into(), Language::Php, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn php_basic_decision_points() {
    // Decision points: function f opens unit (+1), function (+1),
    // if (+1), elseif (+1), else (no), && (+1), || (+1).
    let a = analyze(
        "<?php
         function f($a, $b) { // +2 (+1 unit space)
             if ($a > 0 && $b > 0) {  // +2 (if + &&)
                 return 1;
             } elseif ($a < 0 || $b < 0) {  // +2 (elseif + ||)
                 return -1;
             }
             return 0;
         }",
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
