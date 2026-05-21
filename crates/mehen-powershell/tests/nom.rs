// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! PowerShell NOM tests, ported from
//! `src/metrics/nom.rs::tests` per rewrite plan §8.2.
//!
//! Snapshots are byte-identical to the pre-1.0 `metric.nom` strings.

use mehen_core::{AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, SourceFile};
use mehen_powershell::PowerShellAnalyzer;

fn analyze(source: &str) -> LanguageAnalysis {
    let analyzer = PowerShellAnalyzer::new();
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let file = SourceFile::new("foo.ps1".into(), Language::PowerShell, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn powershell_nom_counts_functions_methods_and_script_block_closures() {
    // 3 functions (f1, f2, M) + 1 closure (the `{ ... }` scriptblock).
    let a = analyze(
        "function f1 { }
             function f2 { }
             class C {
                 [void] M() { }
             }
             $sb = { param($x) $x + 1 }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::nom(&a.root.metrics),
        @r###"
    {
      "functions": 3.0,
      "closures": 1.0,
      "functions_average": 0.5,
      "closures_average": 0.16666666666666666,
      "total": 4.0,
      "average": 0.6666666666666666,
      "functions_min": 0.0,
      "functions_max": 1.0,
      "closures_min": 0.0,
      "closures_max": 1.0
    }
    "###
    );
}
