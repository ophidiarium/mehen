//! Cognitive complexity tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/cognitive.rs::tests`.

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
fn rust_no_cognitive() {
    // Drift from pre-1.0: legacy reported `average: null` when the unit
    // has no enclosed functions because `cognitive_average` was a
    // `Option<f64>` that the JSON formatter rendered as `null`. The
    // Phase-1+ `mehen-metrics::cognitive::finalize` sets it to `0.0` for
    // empty inputs (no functions → average is 0, not undefined). The
    // metric definition is unchanged: with zero functions there is
    // nothing to average. `0.0` is mathematically defensible for
    // "no contribution." All other Phase-9 language ports (Python,
    // TypeScript) carry the same `0.0` here.
    let a = analyze("let a = 42;");
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 0.0,
      "average": 0.0,
      "min": 0.0,
      "max": 0.0
    }"###
    );
}

#[test]
fn rust_simple_function() {
    let a = analyze(
        "fn f() {
             if a && b {
                 println!(\"test\");
             }
             if c && d {
                 println!(\"test\");
             }
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 4.0,
      "average": 4.0,
      "min": 0.0,
      "max": 4.0
    }"###
    );
}

#[test]
fn rust_sequence_same_booleans_amp() {
    let a = analyze(
        "fn f() {
             if a && b && true {
                 println!(\"test\");
             }
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 2.0,
      "average": 2.0,
      "min": 0.0,
      "max": 2.0
    }"###
    );
}

#[test]
fn rust_sequence_same_booleans_pipe() {
    let a = analyze(
        "fn f() {
             if a || b || c || d {
                 println!(\"test\");
             }
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 2.0,
      "average": 2.0,
      "min": 0.0,
      "max": 2.0
    }"###
    );
}

#[test]
fn rust_not_booleans_simple() {
    let a = analyze(
        "fn f() {
             if !a && !b {
                 println!(\"test\");
             }
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 2.0,
      "average": 2.0,
      "min": 0.0,
      "max": 2.0
    }"###
    );
}

#[test]
fn rust_not_booleans_nested_amp() {
    let a = analyze(
        "fn f() {
             if a && !(b && c) {
                 println!(\"test\");
             }
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 3.0,
      "average": 3.0,
      "min": 0.0,
      "max": 3.0
    }"###
    );
}

#[test]
fn rust_not_booleans_nested_pipe() {
    let a = analyze(
        "fn f() {
             if !(a || b) && !(c || d) {
                 println!(\"test\");
             }
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 4.0,
      "average": 4.0,
      "min": 0.0,
      "max": 4.0
    }"###
    );
}

#[test]
fn rust_sequence_different_booleans() {
    let a = analyze(
        "fn f() {
             if a && b || true {
                 println!(\"test\");
             }
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 3.0,
      "average": 3.0,
      "min": 0.0,
      "max": 3.0
    }"###
    );
}

#[test]
fn rust_let_chain_boolean_sequence() {
    let a = analyze(
        "fn f(a: Option<i32>, b: Option<i32>) {
             if let Some(x) = a && let Some(y) = b && x > y {
                 work();
             }
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    // +1 for the if, +1 for the same-operator `&&` let-chain.
    assert_eq!(cog.sum, 2.0, "got {}", serde_json::to_string(&cog).unwrap());
}

#[test]
fn rust_macro_tokens_are_opaque_for_cognitive() {
    let a = analyze(
        "fn f() {
             maybe!(a && b, if c { d() });
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    assert_eq!(cog.sum, 0.0, "got {}", serde_json::to_string(&cog).unwrap());
}

#[test]
fn rust_1_level_nesting_complex() {
    let a = analyze(
        "fn f() {
             if true {
                 if true {
                     println!(\"test\");
                 } else if 1 == 1 {
                     if true {
                         println!(\"test\");
                     }
                 } else {
                     if true {
                         println!(\"test\");
                     }
                 }
             }
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 11.0,
      "average": 11.0,
      "min": 0.0,
      "max": 11.0
    }"###
    );
}

#[test]
fn rust_1_level_nesting_match() {
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
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 3.0,
      "average": 3.0,
      "min": 0.0,
      "max": 3.0
    }"###
    );
}

#[test]
fn rust_2_level_nesting() {
    let a = analyze(
        "fn f() {
             if true {
                 for i in 0..4 {
                     match true {
                         true => println!(\"test\"),
                         false => println!(\"test\"),
                     }
                 }
             }
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 6.0,
      "average": 6.0,
      "min": 0.0,
      "max": 6.0
    }"###
    );
}

#[test]
fn rust_break_continue() {
    let a = analyze(
        "fn f() {
             'tens: for ten in 0..3 {
                 '_units: for unit in 0..=9 {
                     if unit % 2 == 0 {
                         continue;
                     } else if unit == 5 {
                         continue 'tens;
                     } else if unit == 6 {
                         break;
                     } else {
                         break 'tens;
                     }
                 }
             }
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 11.0,
      "average": 11.0,
      "min": 0.0,
      "max": 11.0
    }"###
    );
}

#[test]
fn rust_if_let_else_if_else() {
    let a = analyze(
        "pub fn create_usage_no_title(p: &Parser, used: &[&str]) -> String {
             debugln!(\"usage::create_usage_no_title;\");
             if let Some(u) = p.meta.usage_str {
                 String::from(&*u)
             } else if used.is_empty() {
                 create_help_usage(p, true)
             } else {
                 create_smart_usage(p, used)
             }
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 3.0,
      "average": 3.0,
      "min": 0.0,
      "max": 3.0
    }"###
    );
}

#[test]
fn rust_loop_and_try() {
    let a = analyze(
        "fn f() -> Option<i32> {
             loop {
                 let x = g()?;
                 if x > 0 {
                     return Some(x);
                 }
             }
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 4.0,
      "average": 4.0,
      "min": 0.0,
      "max": 4.0
    }"###
    );
}
