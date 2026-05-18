//! LOC tests for the Phase 8 mago-syntax-backed walker.
//!
//! Legacy tree-sitter PHP carried no LOC snapshot test (LOC was
//! implemented in `legacy/metrics/loc.rs` but no PHP fixture exercised
//! it). These tests are new — they pin the LLOC accounting against
//! every statement-shaped node enumerated in `legacy/metrics/loc.rs`'s
//! PHP arm so future regressions surface immediately.

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
fn php_lloc_counts_simple_function_body() {
    // function (1) + if (2) + return (3) + return (4) = 4 LLOC.
    let a = analyze(
        "<?php
         function f($x) {
             if ($x > 0) {
                 return 1;
             }
             return 0;
         }",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(loc.lloc, 4.0, "{}", serde_json::to_string(&loc).unwrap());
}

#[test]
fn php_lloc_counts_namespace_use_const() {
    // namespace (1) + use (2) + const (3) = 3 LLOC.
    let a = analyze(
        "<?php
         namespace App;
         use Foo;
         const C = 1;",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(loc.lloc, 3.0, "{}", serde_json::to_string(&loc).unwrap());
}

#[test]
fn php_lloc_counts_class_members() {
    // class (1) + class-const (2) + property (3) + method (4) +
    // return (5) inside method = 5 LLOC.
    let a = analyze(
        "<?php
         class C {
             const X = 1;
             public $a;
             public function m() {
                 return 1;
             }
         }",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(loc.lloc, 5.0, "{}", serde_json::to_string(&loc).unwrap());
}

#[test]
fn php_lloc_counts_loops_and_switch() {
    // function (1) + while (2) + break (3) + foreach (4) +
    // continue (5) + for (6) + return (7) + switch (8) +
    // case (9) + return (10) + default (11) + return (12) = 12 LLOC.
    let a = analyze(
        "<?php
         function f($xs) {
             while (true) { break; }
             foreach ($xs as $x) { continue; }
             for ($i = 0; $i < 10; $i++) { return; }
             switch ($xs) {
                 case 1: return 1;
                 default: return 0;
             }
         }",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(loc.lloc, 12.0, "{}", serde_json::to_string(&loc).unwrap());
}

#[test]
fn php_lloc_counts_try_throw_echo_unset() {
    // function (1) + try (2) + throw-as-expr-statement (3) +
    // echo (4) + unset (5) = 5 LLOC.
    // (The catch clause statement counts inside the try body's
    // statements; it is not its own LLOC.)
    let a = analyze(
        "<?php
         function f() {
             try { throw new \\Exception('e'); } catch (\\Exception $e) {}
             echo 'hi';
             unset($x);
         }",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(loc.lloc, 5.0, "{}", serde_json::to_string(&loc).unwrap());
}

#[test]
fn php_lloc_does_not_count_else_clauses_separately() {
    // `if … else` is one logical statement, not two. Legacy did not
    // bump LLOC for `else_clause`; we mirror that.
    // function (1) + if (2) + return (3) + return (4) = 4 LLOC.
    let a = analyze(
        "<?php
         function f($x) {
             if ($x) {
                 return 1;
             } else {
                 return 0;
             }
         }",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(loc.lloc, 4.0, "{}", serde_json::to_string(&loc).unwrap());
}
