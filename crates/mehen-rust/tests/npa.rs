//! NPA tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/npa.rs::tests`.

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
fn rust_npa_counts_struct_fields() {
    // 2 fields, 1 public.
    let a = analyze(
        "struct S {
             pub a: u32,
             b: u32,
         }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::npa(&a.root.metrics),
        @r###"
    {
      "classes": 1.0,
      "interfaces": 0.0,
      "class_attributes": 2.0,
      "interface_attributes": 0.0,
      "classes_average": 0.5,
      "interfaces_average": null,
      "total": 1.0,
      "total_attributes": 2.0,
      "average": 0.5
    }"###
    );
}

#[test]
fn rust_npa_counts_tuple_struct_fields() {
    // 2 positional fields, 1 public.
    let a = analyze("struct S(pub u32, u32);");
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::npa(&a.root.metrics),
        @r###"
    {
      "classes": 1.0,
      "interfaces": 0.0,
      "class_attributes": 2.0,
      "interface_attributes": 0.0,
      "classes_average": 0.5,
      "interfaces_average": null,
      "total": 1.0,
      "total_attributes": 2.0,
      "average": 0.5
    }"###
    );
}
