// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Cognitive complexity ports from
//! `crates/mehen-engine/src/legacy/metrics/cognitive.rs` Python tests.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_python::PythonAnalyzer;

fn analyze(source: &str, filename: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = PythonAnalyzer::new();
    let file = SourceFile::new(filename.into(), Language::Python, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn python_no_cognitive() {
    let a = analyze("a = 42", "foo.py");
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
fn python_simple_function() {
    let a = analyze(
        "def f(a, b):
                if a and b:  # +2 (+1 and)
                   return 1
                if c and d: # +2 (+1 and)
                   return 1",
        "foo.py",
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
fn python_expression_statement() {
    // Boolean expressions containing `And` and `Or` operators were not
    // considered in assignments
    let a = analyze(
        "def f(a, b):
                c = True and True",
        "foo.py",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 1.0,
      "average": 1.0,
      "min": 0.0,
      "max": 1.0
    }"###
    );
}

#[test]
fn python_tuple() {
    // Boolean expressions containing `And` and `Or` operators were not
    // considered inside tuples
    let a = analyze(
        "def f(a, b):
                return \"%s%s\" % (a and \"Get\" or \"Set\", b)",
        "foo.py",
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
fn python_nested_if_in_else_is_not_else_if() {
    // Python has no `else if`; `elif` is a dedicated grammar node. A plain
    // `if` inside an `else:` block must therefore be counted as a nested
    // `if`, not skipped as else-if. This verifies that `is_else_if = false`
    // for Python is correct.
    let a = analyze(
        "def f(a, b):
                if a:          # +1
                    pass
                else:          # +1 else
                    if b:      # +2 (+1 if, +1 nesting)
                        pass",
        "foo.py",
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
fn python_elif_function() {
    // Boolean expressions containing `And` and `Or` operators were not
    // considered in `elif` statements
    let a = analyze(
        "def f(a, b):
                if a and b:  # +2 (+1 and)
                   return 1
                elif c and d: # +2 (+1 and)
                   return 1",
        "foo.py",
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
fn python_more_elifs_function() {
    // Boolean expressions containing `And` and `Or` operators were not
    // considered when there were more `elif` statements
    let a = analyze(
        "def f(a, b):
                if a and b:  # +2 (+1 and)
                   return 1
                elif c and d: # +2 (+1 and)
                   return 1
                elif e and f: # +2 (+1 and)
                   return 1",
        "foo.py",
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
fn python_sequence_same_booleans() {
    let a = analyze(
        "def f(a, b):
                if a and b and True:  # +2 (+1 sequence of and)
                   return 1",
        "foo.py",
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
fn python_sequence_different_booleans() {
    let a = analyze(
        "def f(a, b):
                if a and b or True:  # +3 (+1 and, +1 or)
                   return 1",
        "foo.py",
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
fn python_formatted_sequence_different_booleans() {
    let a = analyze(
        "def f(a, b):
                if (  # +1
                    a and b and  # +1
                    (c or d)  # +1
                ):
                   return 1",
        "foo.py",
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
fn python_1_level_nesting() {
    let a = analyze(
        "def f(a, b):
                if a:  # +1
                    for i in range(b):  # +2
                        return 1",
        "foo.py",
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
fn python_2_level_nesting() {
    let a = analyze(
        "def f(a, b):
                if a:  # +1
                    for i in range(b):  # +2
                        if b:  # +3
                            return 1",
        "foo.py",
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
fn python_try_construct() {
    let a = analyze(
        "def f(a, b):
                try:                 # +1
                    for foo in bar:  # +2 (nesting = 1)
                        return a
                except Exception:    # +2 (nesting = 1)
                    if a < 0:        # +3 (nesting = 2)
                        return a",
        "foo.py",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 8.0,
      "average": 8.0,
      "min": 0.0,
      "max": 8.0
    }"###
    );
}

#[test]
fn python_ternary_operator() {
    let a = analyze(
        "def f(a, b):
                 if a % 2:  # +1
                     return 'c' if a else 'd'  # +2
                 return 'a' if a else 'b'  # +1",
        "foo.py",
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
fn python_nested_functions_lambdas() {
    let a = analyze(
        "def f(a, b):
                 def foo(a):
                     if a:  # +2 (+1 nesting)
                         return 1
                 # +3 (+1 for boolean sequence +2 for lambda nesting)
                 bar = lambda a: lambda b: b or True or True
                 return bar(foo(a))(a)",
        "foo.py",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    // 2 functions + 2 lambdas = 4
    insta::assert_json_snapshot!(
        cog,
        @r###"
    {
      "sum": 5.0,
      "average": 1.25,
      "min": 0.0,
      "max": 3.0
    }"###
    );
}

/// Ruff vs tree-sitter-python: this fixture has invalid indentation
/// (the inner `if (` is more indented than its sibling `word = ...`).
/// Tree-sitter-python's lossy CST silently treats it as a nested
/// statement, so the legacy walker still produced sum=9. Ruff
/// (correctly) rejects the input as a `SyntaxError` and the analyzer
/// returns an empty MetricSpace plus a `python.parse_error`
/// diagnostic. CPython's own `ast.parse` agrees with Ruff
/// (`unexpected indent`). This is a parser improvement, not a metric
/// regression — the legacy snapshot was based on garbage AST input.
#[test]
fn python_real_function() {
    let a = analyze(
        "def process_raw_constant(constant, min_word_length):
                 processed_words = []
                 raw_camelcase_words = []
                 for raw_word in re.findall(r'[a-z]+', constant):  # +1
                     word = raw_word.strip()
                         if (  # +2 (+1 if and +1 nesting)
                             len(word) >= min_word_length
                             and not (word.startswith('-') or word.endswith('-')) # +2 operators
                         ):
                             if is_camel_case_word(word):  # +3 (+1 if and +2 nesting)
                                 raw_camelcase_words.append(word)
                             else: # +1 else
                                 processed_words.append(word.lower())
                 return processed_words, raw_camelcase_words",
        "foo.py",
    );
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
