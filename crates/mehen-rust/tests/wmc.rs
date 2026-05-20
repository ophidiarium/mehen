//! WMC tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/wmc.rs::tests`.

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
fn rust_wmc_impl_sums_function_cyclomatics() {
    // impl S: a cyc=2 (if), b cyc=1 -> classes = 3
    let a = analyze(
        "struct S;
         impl S {
             fn a(&self, x: bool) -> u32 {
                 if x { 1 } else { 0 }
             }
             fn b(&self) -> u32 { 1 }
         }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::wmc(&a.root.metrics),
        @r###"
    {
      "classes": 3.0,
      "interfaces": 0.0,
      "total": 3.0
    }"###
    );
}

#[test]
fn rust_wmc_empty_impl_still_emitted() {
    let a = analyze(
        "struct S;
         impl S {}",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::wmc(&a.root.metrics),
        @r###"
    {
      "classes": 0.0,
      "interfaces": 0.0,
      "total": 0.0
    }"###
    );
}
