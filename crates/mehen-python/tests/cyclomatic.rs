//! Cyclomatic complexity ports from
//! `crates/mehen-engine/src/legacy/metrics/cyclomatic.rs` Python tests.
//!
//! Each test reproduces the legacy fixture and the legacy expected JSON.
//! Drift from the pre-1.0 tree-sitter-python output is classified per
//! the rewrite plan §12.3.1 and `docs/python-ruff-spec.md`.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_python::PythonAnalyzer;

fn analyze(source: &str, filename: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = PythonAnalyzer::new();
    let file = SourceFile::new(filename.into(), Language::Python, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

/// Legacy `python_simple_function` from
/// `crates/mehen-engine/src/legacy/metrics/cyclomatic.rs`.
#[test]
fn python_simple_function() {
    let a = analyze(
        "def f(a, b): # +2 (+1 unit space)
                if a and b:  # +2 (+1 and)
                   return 1
                if c and d: # +2 (+1 and)
                   return 1",
        "foo.py",
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

/// Legacy `python_1_level_nesting` from
/// `crates/mehen-engine/src/legacy/metrics/cyclomatic.rs`.
#[test]
fn python_1_level_nesting() {
    let a = analyze(
        "def f(a, b): # +2 (+1 unit space)
                if a:  # +1
                    for i in range(b):  # +1
                        return 1",
        "foo.py",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    insta::assert_json_snapshot!(
        cy,
        @r###"
    {
      "sum": 4.0,
      "average": 2.0,
      "min": 1.0,
      "max": 3.0
    }"###
    );
}
