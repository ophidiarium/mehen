// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Halstead ports from
//! `crates/mehen-engine/src/legacy/metrics/halstead.rs` Python tests.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_python::PythonAnalyzer;

fn analyze(source: &str, filename: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = PythonAnalyzer::new();
    let file = SourceFile::new(filename.into(), Language::Python, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

/// Phase 6 parity diagnostic — dumps the Halstead, cognitive and LOC
/// breakdown for the `embedded_code_large.md` Python fence so we can
/// compare directly against the legacy walker.
#[test]
fn legacy_python_fence_halstead_dump() {
    let body = r#"def fibonacci(n):
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a

def main():
    import sys
    for arg in sys.argv[1:]:
        try:
            n = int(arg)
        except ValueError:
            continue
        print(n, fibonacci(n))

if __name__ == "__main__":
    main()
"#;
    let a = analyze(body, "fence.py");
    let h = mehen_report::metrics_json::halstead(&a.root.metrics);
    let cg = mehen_report::metrics_json::cognitive(&a.root.metrics);
    let lc = mehen_report::metrics_json::loc(&a.root.metrics);
    eprintln!(
        "ruff python fence: volume={} cognitive_sum={} sloc={}",
        h.volume, cg.sum, lc.sloc
    );
}

#[test]
fn python_operators_and_operands() {
    let a = analyze(
        "def foo():
                 def bar():
                     def toto():
                        a = 1 + 1
                     b = 2 + a
                 c = 3 + 3",
        "foo.py",
    );
    // unique operators: def, =, +
    // operators: def, def, def, =, =, =, +, +, +
    // unique operands: foo, bar, toto, a, b, c, 1, 2, 3
    // operands: foo, bar, toto, a, b, c, 1, 1, 2, a, 3, 3
    let h = mehen_report::metrics_json::halstead(&a.root.metrics);
    insta::assert_json_snapshot!(
        h,
        @r#"
    {
      "n1": 5.0,
      "N1": 15.0,
      "n2": 9.0,
      "N2": 12.0,
      "length": 27.0,
      "estimated_program_length": 40.13896548741762,
      "purity_ratio": 1.4866283513858378,
      "vocabulary": 14.0,
      "volume": 102.79858289555531,
      "difficulty": 3.3333333333333335,
      "level": 0.3,
      "effort": 342.6619429851844,
      "time": 19.03677461028802,
      "bugs": 0.01632259960095138
    }
    "#
    );
}

/// Ruff vs tree-sitter-python: the legacy walker counted the bare
/// brackets `()[]{}` as Halstead operators because tree-sitter-python's
/// lossy CST silently treated them as standalone syntax nodes (CPython
/// rejects this same source as a `SyntaxError`). Ruff matches CPython
/// — `parse_module` returns an error and the analyzer emits a parse
/// diagnostic instead of attributing tokens to n1/N1. This is a
/// parser-correctness improvement, not a metric regression.
#[test]
fn python_wrong_operators() {
    let a = analyze("()[]{}", "foo.py");
    let h = mehen_report::metrics_json::halstead(&a.root.metrics);
    insta::assert_json_snapshot!(
        h,
        @r#"
    {
      "n1": 0.0,
      "N1": 0.0,
      "n2": 0.0,
      "N2": 0.0,
      "length": 0.0,
      "estimated_program_length": 0.0,
      "purity_ratio": 0.0,
      "vocabulary": 0.0,
      "volume": 0.0,
      "difficulty": 0.0,
      "level": 0.0,
      "effort": 0.0,
      "time": 0.0,
      "bugs": 0.0
    }
    "#
    );
}

#[test]
fn python_check_metrics() {
    let a = analyze(
        "def f():
                 pass",
        "foo.py",
    );
    let h = mehen_report::metrics_json::halstead(&a.root.metrics);
    insta::assert_json_snapshot!(
        h,
        @r#"
    {
      "n1": 4.0,
      "N1": 4.0,
      "n2": 1.0,
      "N2": 1.0,
      "length": 5.0,
      "estimated_program_length": 8.0,
      "purity_ratio": 1.6,
      "vocabulary": 5.0,
      "volume": 11.60964047443681,
      "difficulty": 2.0,
      "level": 0.5,
      "effort": 23.21928094887362,
      "time": 1.289960052715201,
      "bugs": 0.002712967490108627
    }
    "#
    );
}

/// Regression: nested function spaces must carry their own Halstead
/// counts in the per-space JSON. PR #95 discussion_r3265658502
/// flagged that the post-AST token sweep was writing every event onto
/// the unit space, leaving inner function spaces with zero
/// `halstead.N1` / `halstead.N2` even when they contained operators
/// and operands.
#[test]
fn python_nested_function_halstead_is_non_zero() {
    let a = analyze(
        "def outer():
    def inner():
        x = 1 + 2
    inner()
",
        "nested.py",
    );
    // Tree shape: unit -> outer -> inner.
    assert_eq!(a.root.spaces.len(), 1, "expected one outer function");
    let outer = &a.root.spaces[0];
    assert_eq!(
        outer.name.as_deref(),
        Some("outer"),
        "outer space should be `outer`"
    );
    assert_eq!(outer.spaces.len(), 1, "expected one nested function");
    let inner = &outer.spaces[0];
    assert_eq!(inner.name.as_deref(), Some("inner"));

    let inner_h = mehen_report::metrics_json::halstead(&inner.metrics);
    let inner_json = serde_json::to_string(&inner_h).unwrap();
    assert!(
        inner_h.big_n1 > 0.0,
        "inner function must record its `=` and `+` operators in the per-space JSON, got {inner_json}"
    );
    assert!(
        inner_h.big_n2 > 0.0,
        "inner function must record its `x`, `1`, `2` operands, got {inner_json}"
    );
    assert!(
        inner_h.volume > 0.0,
        "inner function volume must be > 0, got {inner_json}"
    );

    // The outer rollup must include the inner's distinct operators
    // and operands (set-union semantics from `HalsteadBuilder::merge`).
    let outer_h = mehen_report::metrics_json::halstead(&outer.metrics);
    let outer_json = serde_json::to_string(&outer_h).unwrap();
    assert!(
        outer_h.big_n1 >= inner_h.big_n1,
        "outer N1 must roll up the inner: outer={outer_json} inner={inner_json}"
    );
    assert!(
        outer_h.big_n2 >= inner_h.big_n2,
        "outer N2 must roll up the inner: outer={outer_json} inner={inner_json}"
    );
}
