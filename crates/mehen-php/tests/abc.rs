// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! ABC tests for the Phase 8 mago-syntax-backed walker.
//!
//! Legacy tree-sitter PHP carried no ABC snapshot test (the trait
//! impl in `legacy/metrics/abc.rs` was untested). These tests pin
//! the assignment / branch / condition triple against every PHP
//! construct the legacy classifier covered.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_php::PhpAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = PhpAnalyzer::new();
    let file = SourceFile::new("foo.php".into(), Language::Php, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn php_abc_assignments_cover_all_assignment_forms() {
    // ABC.A: plain assign (1), augmented +=  (2), null-coalesce ??= (3),
    // string concat .= (4), prefix ++ (5), prefix -- (6),
    // postfix ++ (7), postfix -- (8). 8 total.
    let a = analyze(
        "<?php
         function f() {
             $x = 1;
             $x += 1;
             $x ??= 1;
             $x .= '!';
             ++$x;
             --$x;
             $x++;
             $x--;
         }",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    assert_eq!(
        abc.assignments,
        8.0,
        "{}",
        serde_json::to_string(&abc).unwrap()
    );
}

#[test]
fn php_abc_branches_cover_calls_news_and_intrinsics() {
    // ABC.B: call (1), method call (2), static call (3),
    // null-safe method call (4), `new` (5), `include` (6),
    // `require_once` (7), `yield` (8). 8 total.
    let a = analyze(
        "<?php
         function f($obj) {
             foo();
             $obj->m();
             X::s();
             $obj?->n();
             new Foo();
             include 'a.php';
             require_once 'b.php';
             yield 1;
         }",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    assert_eq!(
        abc.branches,
        8.0,
        "{}",
        serde_json::to_string(&abc).unwrap()
    );
}

#[test]
fn php_abc_conditions_cover_control_flow_and_comparisons() {
    // ABC.C: if (1), `===` (2), `&&` (3), elseif (4), `<` (5),
    // `else` (6), foreach (7), `==` (8). 8 total.
    let a = analyze(
        "<?php
         function f($x, $xs) {
             if ($x === 1 && true) {
             } elseif ($x < 0) {
             } else {
             }
             foreach ($xs as $y) {
                 if ($y == 0) {}
             }
         }",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    // if + === + && + elseif + < + else + foreach + if(inside) + ==
    // = 9 conditions actually. Let me re-count by hand:
    //   if(outer) (1)
    //   === (2)
    //   && (3)
    //   elseif (4)
    //   < (5)
    //   else (6)
    //   foreach (7)
    //   if(inner) (8)
    //   == (9)
    assert_eq!(
        abc.conditions,
        9.0,
        "{}",
        serde_json::to_string(&abc).unwrap()
    );
}

#[test]
fn php_abc_xor_does_not_count() {
    // `xor` does not short-circuit, so it does not add a path —
    // legacy excluded it from ABC.C and from cyclomatic.
    let a = analyze(
        "<?php
         function f() {
             $r = true xor false;
         }",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    // ABC.A: one `=` assignment.
    // ABC.C: zero — `xor` is excluded.
    assert_eq!(
        abc.assignments,
        1.0,
        "{}",
        serde_json::to_string(&abc).unwrap()
    );
    assert_eq!(
        abc.conditions,
        0.0,
        "{}",
        serde_json::to_string(&abc).unwrap()
    );
}
