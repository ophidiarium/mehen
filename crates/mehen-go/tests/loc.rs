//! LOC tests for the Go walker.
//!
//! Every legacy `check_metrics::<GoParser>` LOC test from
//! `crates/mehen-engine/src/legacy/metrics/loc.rs` is ported here
//! byte-identical so the parity contract (plan §12.3.1) is visibly
//! maintained.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_go::GoAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    // Mirror the legacy `check_func_space` harness: trim trailing
    // newlines/whitespace and re-append exactly one `\n`. Without this
    // normalization the SLOC differs by ±1 depending on whether the
    // raw fixture has a trailing newline.
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = GoAnalyzer::new();
    let file = SourceFile::new("foo.go".into(), Language::Go, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn go_sloc() {
    let a = analyze(
        "package main

            // A comment
            func main() {
                x := 1
            }
            ",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(
        loc,
        @r###"
    {
      "sloc": 6.0,
      "ploc": 4.0,
      "lloc": 1.0,
      "cloc": 1.0,
      "blank": 1.0,
      "sloc_average": 3.0,
      "ploc_average": 2.0,
      "lloc_average": 0.5,
      "cloc_average": 0.5,
      "blank_average": 0.5,
      "sloc_min": 3.0,
      "sloc_max": 3.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 3.0,
      "ploc_max": 3.0,
      "lloc_min": 1.0,
      "lloc_max": 1.0,
      "blank_min": 0.0,
      "blank_max": 0.0
    }"###
    );
}

#[test]
fn go_lloc() {
    let a = analyze(
        "package main

            func main() {
                x := 1
                y := 2
                if x > y {
                    return
                }
            }",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(
        loc,
        @r###"
    {
      "sloc": 9.0,
      "ploc": 8.0,
      "lloc": 4.0,
      "cloc": 0.0,
      "blank": 1.0,
      "sloc_average": 4.5,
      "ploc_average": 4.0,
      "lloc_average": 2.0,
      "cloc_average": 0.0,
      "blank_average": 0.5,
      "sloc_min": 7.0,
      "sloc_max": 7.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 7.0,
      "ploc_max": 7.0,
      "lloc_min": 4.0,
      "lloc_max": 4.0,
      "blank_min": 0.0,
      "blank_max": 0.0
    }"###
    );
}

#[test]
fn go_lloc_counts_go_declaration_specs_and_receive_statements() {
    let a = analyze(
        "package main

            import (
                \"fmt\"
                _ \"net/http\"
            )

            var (
                a = 1
                b = 2
            )

            func main(ch chan int) {
            Loop:
                <-ch
                fmt.Println(a, b)
            }",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(
        loc,
        @r###"
    {
      "sloc": 17.0,
      "ploc": 14.0,
      "lloc": 7.0,
      "cloc": 0.0,
      "blank": 3.0,
      "sloc_average": 8.5,
      "ploc_average": 7.0,
      "lloc_average": 3.5,
      "cloc_average": 0.0,
      "blank_average": 1.5,
      "sloc_min": 5.0,
      "sloc_max": 5.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 5.0,
      "ploc_max": 5.0,
      "lloc_min": 3.0,
      "lloc_max": 3.0,
      "blank_min": 0.0,
      "blank_max": 0.0
    }"###
    );
}
