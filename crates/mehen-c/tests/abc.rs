//! ABC tests for the C walker.
//!
//! Every legacy `check_metrics::<CParser>` ABC test from
//! `crates/mehen-engine/src/legacy/metrics/abc.rs` is ported here
//! byte-identical so the parity contract (plan §12.3.1) is visibly
//! maintained.

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
fn c_abc_counts_else_clause_in_conditions() {
    // Per Fitzpatrick (1997), `else` is a branch-point that contributes
    // to the `C` (Conditions) component. tree-sitter-c exposes it as a
    // dedicated `else_clause` named node, so an `if (x > 0) {...} else
    // {...}` should yield: +1 if + 1 `>` comparison + 1 else = 3
    // conditions. A: 0 (no assignments). B: 0 (no calls).
    let a = analyze(
        "int f(int x) {
             if (x > 0) {
                 return 1;
             } else {
                 return 0;
             }
         }",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    assert_eq!(abc.assignments, 0.0);
    assert_eq!(abc.branches, 0.0);
    assert_eq!(abc.conditions, 3.0);
}
