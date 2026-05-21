// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! PowerShell NExit tests, ported from
//! `src/metrics/exit.rs::tests` per rewrite plan §8.2.
//!
//! NExit counts function exit points: `return`, `throw`, `exit`. In
//! tree-sitter-pwsh those are children of `flow_control_statement` —
//! the language analyzer inspects `child(0)`'s kind to disambiguate.
//! `break` / `continue` are loop-local control flow and are not exits
//! (mirrors the Ruby `break`/`next` vs. `return` convention).
//!
//! Snapshots are byte-identical to the pre-1.0 `metric.nexits` strings.

use mehen_core::{AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, SourceFile};
use mehen_powershell::PowerShellAnalyzer;

fn analyze(source: &str) -> LanguageAnalysis {
    let analyzer = PowerShellAnalyzer::new();
    let file = SourceFile::new("foo.ps1".into(), Language::PowerShell, source.to_string());
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn powershell_return_throw_and_exit_count_as_exits() {
    // 3 exits: throw + exit + return.
    let a = analyze(
        "function f($a) {
                 if ($a -lt 0) {
                     throw 'bad'
                 }
                 if ($a -gt 100) {
                     exit 1
                 }
                 return $a
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::nexits(&a.root.metrics),
        @r###"
    {
      "sum": 3.0,
      "average": 3.0,
      "min": 0.0,
      "max": 3.0
    }
    "###
    );
}

#[test]
fn powershell_break_and_continue_do_not_count_as_exits() {
    // Like other languages in mehen, `break` / `continue` are loop-local
    // control flow and must not count as function exits.
    let a = analyze(
        "function f {
                 foreach ($x in 1..10) {
                     if ($x -eq 5) { break }
                     if ($x -eq 3) { continue }
                 }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::nexits(&a.root.metrics),
        @r###"
    {
      "sum": 0.0,
      "average": 0.0,
      "min": 0.0,
      "max": 0.0
    }
    "###
    );
}
