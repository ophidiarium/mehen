//! ABC tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/abc.rs::tests`.

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
fn rust_abc_basic() {
    let a = analyze(
        "fn f(a: i32, b: i32) -> i32 {
             let mut x = a;
             x += b;
             log(x);
             if x > b {
                 return x;
             }
             x
         }",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    insta::assert_json_snapshot!(
        abc,
        @r###"
    {
      "assignments": 2.0,
      "branches": 1.0,
      "conditions": 2.0,
      "magnitude": 3.0,
      "assignments_average": 1.0,
      "branches_average": 0.5,
      "conditions_average": 1.0,
      "assignments_min": 0.0,
      "assignments_max": 2.0,
      "branches_min": 0.0,
      "branches_max": 1.0,
      "conditions_min": 0.0,
      "conditions_max": 2.0
    }"###
    );
}

#[test]
fn rust_abc_scopes_type_and_macro_tokens() {
    // Type parameters do not contribute to ABC. Macro body tokens are
    // opaque, so the inner `&&` and `if` do not register either. Only
    // the macro call itself counts as a branch.
    let a = analyze(
        "fn generic(a: Option<i32>, b: Result<u8, E>) {}
         fn macro_call() {
             maybe!(a && b, if c { d() });
         }",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    assert_eq!(
        abc.conditions,
        0.0,
        "got {}",
        serde_json::to_string(&abc).unwrap()
    );
    assert_eq!(
        abc.branches,
        1.0,
        "got {}",
        serde_json::to_string(&abc).unwrap()
    );
}

#[test]
fn rust_abc_counts_let_chain_operators_in_conditions() {
    let a = analyze(
        "fn f(a: Option<i32>, b: Option<i32>) {
             if let Some(x) = a && let Some(y) = b && x > y {
                 work();
             }
         }",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    assert_eq!(
        abc.conditions,
        4.0,
        "got {}",
        serde_json::to_string(&abc).unwrap()
    );
    assert_eq!(
        abc.branches,
        1.0,
        "got {}",
        serde_json::to_string(&abc).unwrap()
    );
}
