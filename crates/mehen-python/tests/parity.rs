// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Parity / divergence snapshots for the Ruff-backed Python analyzer.
//!
//! Each test reproduces a relevant pre-1.0 `mehen-engine::legacy`
//! Python assertion using the same fixture and expected JSON. The
//! snapshots come from the legacy `check_metrics::<PythonParser>`
//! body — every drift from the pre-1.0 tree-sitter-python output is
//! classified per the rewrite plan §12.3.1 and `docs/python-ruff-spec.md`.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_python::PythonAnalyzer;

fn analyze(source: &str, filename: &str) -> mehen_core::LanguageAnalysis {
    // The legacy `check_metrics` strips trailing newlines and pushes a
    // single one — match that precisely so any LOC line-count drift is
    // not just a whitespace artifact.
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = PythonAnalyzer::new();
    let file = SourceFile::new(filename.into(), Language::Python, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

/// Legacy `python_simple_function` from
/// `crates/mehen-engine/src/legacy/metrics/cyclomatic.rs:323` — the
/// numbers below come straight from that test's inline JSON.
#[test]
fn python_simple_function_cyclomatic() {
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

/// Legacy `python_1_level_nesting`.
#[test]
fn python_1_level_nesting_cyclomatic() {
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

/// Match/case is a Phase 6 *improvement* over tree-sitter-python:
/// each case is a real structural branch, so each contributes +1
/// cyclomatic. The legacy walker also counted matches (`+1 per case`),
/// so this is a parity check, not a drift.
#[test]
fn python_match_each_case_counts_as_decision() {
    let a = analyze(
        "def f(x):
            match x:
                case 1:
                    return 'one'
                case 2:
                    return 'two'
                case _:
                    return 'other'",
        "foo.py",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    // 1 (base) + 3 cases = 4 in the function. Unit space adds nothing
    // structural. Sum = 4 + 1 (unit) = 5; max = 4.
    insta::assert_json_snapshot!(
        cy,
        @r###"
    {
      "sum": 5.0,
      "average": 2.5,
      "min": 1.0,
      "max": 4.0
    }"###
    );
}

/// `try`/`except*` (PEP 654 exception groups) — each `except*` handler
/// counts the same as `except`. New-parser improvement: tree-sitter
/// grammar may not parse this correctly; Ruff does.
#[test]
fn python_except_star_handler_counts_as_decision() {
    let a = analyze(
        "def f():
            try:
                do()
            except* ValueError:
                handle()",
        "foo.py",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    insta::assert_json_snapshot!(
        cy,
        @r###"
    {
      "sum": 3.0,
      "average": 1.5,
      "min": 1.0,
      "max": 2.0
    }"###
    );
}

/// Type annotations are runtime-accessible objects in Python (Pydantic,
/// `typing.get_type_hints`, dataclasses), so identifiers inside them
/// DO contribute to Halstead operands. This is the deliberate
/// difference from `mehen-typescript`, where TS types are erased.
#[test]
fn python_type_annotations_participate_in_halstead() {
    let a = analyze(
        "def f(x: int, y: str = 'hi') -> bool:
            return False",
        "foo.py",
    );
    let h = mehen_report::metrics_json::halstead(&a.root.metrics);
    // Identifiers `int`, `str`, `bool` are operands; the `:` and `->`
    // are operators. Without this rule, the metric would understate
    // the program's Halstead difficulty.
    assert!(
        h.n2 >= 4.0,
        "expected n2>=4 (x, y, int, str, bool, False, ...) got {}",
        h.n2
    );
}

/// Module-level docstring (PEP 257) is excluded from Halstead — it is
/// structural documentation, not running code. Compare to a module
/// without a docstring to confirm the exclusion.
#[test]
fn python_module_docstring_excluded_from_halstead() {
    let with_doc = analyze(
        "\"\"\"This is a module docstring.\"\"\"
x = 1",
        "with.py",
    );
    let without_doc = analyze("x = 1\n", "without.py");
    let h_with = mehen_report::metrics_json::halstead(&with_doc.root.metrics);
    let h_without = mehen_report::metrics_json::halstead(&without_doc.root.metrics);
    // The docstring's tokens (`"""`, `T h i s …`) must not contribute
    // any Halstead operators or operands beyond what `x = 1` already
    // produced. Token-level Halstead totals must therefore match
    // exactly when the docstring is removed.
    assert_eq!(h_with.n1, h_without.n1, "n1 should match");
    assert_eq!(h_with.n2, h_without.n2, "n2 should match");
    assert_eq!(h_with.big_n1, h_without.big_n1, "N1 should match");
    assert_eq!(h_with.big_n2, h_without.big_n2, "N2 should match");
}

/// NPM: only methods of class bodies count. Public/private follow the
/// PEP 8 leading-underscore convention; dunders (`__init__` etc.) are
/// public.
#[test]
fn python_npm_counts_class_methods() {
    let a = analyze(
        "class C:
    def __init__(self):
        pass
    def public(self):
        pass
    def _internal(self):
        pass",
        "foo.py",
    );
    let class_space = &a.root.spaces[0];
    let npm = mehen_report::metrics_json::npm(&class_space.metrics);
    // 3 methods total in the class. Public: __init__ + public = 2.
    assert_eq!(npm.class_methods, 3.0);
    assert_eq!(
        npm.classes, 2.0,
        "expected 2 public methods, got {}",
        npm.classes
    );
}

/// NPA: top-level class assignments (annotated or not) count as
/// attributes. Legacy walked `expression_statement -> assignment`.
/// Ruff's AST gives us `StmtAssign` / `StmtAnnAssign` directly.
#[test]
fn python_npa_counts_class_attributes() {
    let a = analyze(
        "class C:
    x: int = 1
    y = 2
    _internal = 3",
        "foo.py",
    );
    let class_space = &a.root.spaces[0];
    let npa = mehen_report::metrics_json::npa(&class_space.metrics);
    assert_eq!(npa.class_attributes, 3.0);
    assert_eq!(
        npa.classes, 2.0,
        "expected 2 public attributes, got {}",
        npa.classes
    );
}

/// NExit: `return` and `raise` count.
#[test]
fn python_nexit_counts_return_and_raise() {
    let a = analyze(
        "def f(x):
    if x:
        return 1
    raise ValueError('oops')",
        "foo.py",
    );
    let func = &a.root.spaces[0];
    let nx = mehen_report::metrics_json::nexits(&func.metrics);
    assert_eq!(nx.sum, 2.0);
}

/// PEP 701 / PEP 750: f-strings and t-strings expose embedded
/// expressions as proper AST nodes. The interpolated `{ ... }` parts
/// reach Halstead via the AST `visit_expr` traversal — each
/// `Expr::FString` walks every `InterpolatedStringElement` and emits
/// the embedded expression's tokens as ordinary operators / operands.
/// The legacy tree-sitter-python's f-string handling lumped the entire
/// f-string into one `string` operand and missed the interpolation's
/// embedded identifiers. Ruff's structurally-richer representation
/// captures `x` and `y` as Halstead operands.
#[test]
fn python_f_string_interpolation_contributes_to_halstead() {
    let plain = analyze(
        "def fmt(x, y):
    return 'static'",
        "plain.py",
    );
    let interp = analyze(
        "def fmt(x, y):
    return f'{x + y!r}'",
        "interp.py",
    );
    let h_plain = mehen_report::metrics_json::halstead(&plain.root.metrics);
    let h_interp = mehen_report::metrics_json::halstead(&interp.root.metrics);
    // The interpolated expression contributes at least 2 extra operand
    // tokens (`x` and `y`) and at least one extra operator (`+`).
    assert!(
        h_interp.big_n2 > h_plain.big_n2,
        "f-string interpolation should add operands; plain N2={} interp N2={}",
        h_plain.big_n2,
        h_interp.big_n2
    );
    assert!(
        h_interp.big_n1 > h_plain.big_n1,
        "f-string interpolation should add operators; plain N1={} interp N1={}",
        h_plain.big_n1,
        h_interp.big_n1
    );
}

/// Default-value expressions in parameters are runtime-evaluated at
/// definition time. They should reach ABC (calls, comparisons) and
/// Halstead (operators / operands). This is a Phase-6-friendly check
/// — Ruff's AST gives us each `ParameterWithDefault` directly.
#[test]
fn python_parameter_defaults_count_as_definition_time_code() {
    let a = analyze(
        "def f(x=1, y=int('42'), z=[1, 2, 3]):
    return x + y",
        "foo.py",
    );
    // The unit's ABC must include the `int('42')` call as a branch.
    let abc_unit = mehen_report::metrics_json::abc(&a.root.metrics);
    assert!(
        abc_unit.branches >= 1.0,
        "expected `int('42')` default to count as a branch, got {}",
        serde_json::to_string(&abc_unit).unwrap()
    );
}

/// Walrus / named expression `(x := 42)` is an assignment that
/// returns its value. Legacy walker counted it as `named_expression`;
/// Ruff exposes it as `Expr::Named`. ABC.assignments must increment.
#[test]
fn python_walrus_counts_as_assignment() {
    let a = analyze(
        "def f():
    if (n := 10) > 5:
        return n",
        "foo.py",
    );
    let func = &a.root.spaces[0];
    let abc = mehen_report::metrics_json::abc(&func.metrics);
    assert!(
        abc.assignments >= 1.0,
        "walrus must count as assignment, got {}",
        serde_json::to_string(&abc).unwrap()
    );
}
