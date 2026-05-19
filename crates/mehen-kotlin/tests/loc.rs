//! LOC tests for the tree-sitter-kotlin walker.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_kotlin::KotlinAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    // Match the legacy `check_metrics` test harness: trim whitespace,
    // append a single trailing newline. The line-count helpers in
    // `LineIndex` count `\n` boundaries so the trailing newline pushes
    // SLOC up by one row, matching the legacy snapshots.
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = KotlinAnalyzer::new();
    let file = SourceFile::new("foo.kt".into(), Language::Kotlin, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn kotlin_simple_loc() {
    let a = analyze(
        "// header
         fun greet(name: String) {
             println(\"hi, \" + name)
         }",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(loc);
}

#[test]
fn kotlin_nested_calls_do_not_add_extra_lloc() {
    let a = analyze(
        "fun f() {
             val x = foo(bar())
             foo(bar())
         }",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(
        loc,
        @r###"
    {
      "sloc": 4.0,
      "ploc": 4.0,
      "lloc": 3.0,
      "cloc": 0.0,
      "blank": 0.0,
      "sloc_average": 2.0,
      "ploc_average": 2.0,
      "lloc_average": 1.5,
      "cloc_average": 0.0,
      "blank_average": 0.0,
      "sloc_min": 4.0,
      "sloc_max": 4.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 4.0,
      "ploc_max": 4.0,
      "lloc_min": 3.0,
      "lloc_max": 3.0,
      "blank_min": 0.0,
      "blank_max": 0.0
    }"###
    );
}

#[test]
fn kotlin_counts_companion_and_accessors_as_lloc() {
    let a = analyze(
        "class C {
             companion object {
                 fun make() = C()
             }

             var x: Int = 0
                 get() = field
                 set(value) { field = value }
         }",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(loc.lloc, 7.0);
}
