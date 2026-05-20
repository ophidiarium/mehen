//! NOM tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/nom.rs::tests`.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_rust::RustAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = RustAnalyzer::new();
    let file = SourceFile::new("foo.rs".into(), Language::Rust, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn rust_nom() {
    // Drift from pre-1.0: `functions_min` was `0.0` in legacy because
    // the unit's always-zero `nom.functions` was folded into the per-
    // space min. Phase-9 NargsStats-mirroring NomStats gating preserves
    // the `_min` only for spaces that actually open a function/closure.
    let a = analyze(
        "mod A { fn foo() {}}
         mod B { fn foo() {}}
         let closure = |i: i32| -> i32 { i + 42 };",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::nom(&a.root.metrics),
        @r###"
    {
      "functions": 2.0,
      "closures": 1.0,
      "functions_average": 0.5,
      "closures_average": 0.25,
      "total": 3.0,
      "average": 0.75,
      "functions_min": 0.0,
      "functions_max": 1.0,
      "closures_min": 0.0,
      "closures_max": 1.0
    }"###
    );
}
