// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! ABC metric tests for the Go walker.
//!
//! Every legacy `check_metrics::<GoParser>` ABC test from
//! `crates/mehen-engine/src/legacy/metrics/abc.rs` is ported here
//! byte-identical so the parity contract (plan §12.3.1) is visibly
//! maintained.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_go::GoAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = GoAnalyzer::new();
    let file = SourceFile::new("foo.go".into(), Language::Go, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn go_abc_basic() {
    let a = analyze(
        "package main

             func f(a, b int) int {
                 x, y := a, b
                 log(x)
                 if x > y {
                     return x
                 }
                 return y
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
fn go_abc_comments_in_targets_and_logical_conditions() {
    let a = analyze(
        "package main

             func f(a, b int) {
                 x, /* target comment */ y := a, b
                 _ = !((x > 0 && y > 0) || x == y)
             }",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    insta::assert_json_snapshot!(
        abc,
        @r###"
    {
      "assignments": 3.0,
      "branches": 0.0,
      "conditions": 6.0,
      "magnitude": 6.708203932499369,
      "assignments_average": 1.5,
      "branches_average": 0.0,
      "conditions_average": 3.0,
      "assignments_min": 0.0,
      "assignments_max": 3.0,
      "branches_min": 0.0,
      "branches_max": 0.0,
      "conditions_min": 0.0,
      "conditions_max": 6.0
    }"###
    );
}

#[test]
fn go_abc_receive_assignments() {
    let a = analyze(
        "package main

             func f(ch chan int) {
                 x := <-ch
                 y, ok := <-ch
                 <-ch
             }",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    insta::assert_json_snapshot!(
        abc,
        @r###"
    {
      "assignments": 3.0,
      "branches": 0.0,
      "conditions": 0.0,
      "magnitude": 3.0,
      "assignments_average": 1.5,
      "branches_average": 0.0,
      "conditions_average": 0.0,
      "assignments_min": 0.0,
      "assignments_max": 3.0,
      "branches_min": 0.0,
      "branches_max": 0.0,
      "conditions_min": 0.0,
      "conditions_max": 0.0
    }"###
    );
}

#[test]
fn go_abc_range_clause_assignments() {
    let a = analyze(
        "package main

             func f(m map[string]int) {
                 for k, v := range m {
                 }
                 for k = range m {
                 }
                 for range m {
                 }
             }",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    insta::assert_json_snapshot!(
        abc,
        @r###"
    {
      "assignments": 3.0,
      "branches": 0.0,
      "conditions": 3.0,
      "magnitude": 4.242640687119285,
      "assignments_average": 1.5,
      "branches_average": 0.0,
      "conditions_average": 1.5,
      "assignments_min": 0.0,
      "assignments_max": 3.0,
      "branches_min": 0.0,
      "branches_max": 0.0,
      "conditions_min": 0.0,
      "conditions_max": 3.0
    }"###
    );
}

#[test]
fn go_abc_default_cases() {
    let a = analyze(
        "package main

             func f(x int) int {
                 switch x {
                 case 1:
                     return 1
                 default:
                     return 0
                 }
             }",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    insta::assert_json_snapshot!(
        abc,
        @r###"
    {
      "assignments": 0.0,
      "branches": 0.0,
      "conditions": 2.0,
      "magnitude": 2.0,
      "assignments_average": 0.0,
      "branches_average": 0.0,
      "conditions_average": 1.0,
      "assignments_min": 0.0,
      "assignments_max": 0.0,
      "branches_min": 0.0,
      "branches_max": 0.0,
      "conditions_min": 0.0,
      "conditions_max": 2.0
    }"###
    );
}
