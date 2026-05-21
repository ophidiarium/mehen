// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! PowerShell LOC tests, ported from
//! `src/metrics/loc.rs::tests` per rewrite plan §8.2.
//!
//! Snapshots are byte-identical to the pre-1.0 `metric.loc` strings.

use mehen_core::{
    AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, MetricKey, MetricValue,
    SourceFile,
};
use mehen_powershell::PowerShellAnalyzer;

fn analyze(source: &str) -> LanguageAnalysis {
    let analyzer = PowerShellAnalyzer::new();
    // Match the pre-1.0 `check_metrics` test helper: trim trailing
    // whitespace/newlines and re-append a single `\n`. This is the
    // shape the legacy parser sees, so SLOC line counting compares
    // directly.
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let file = SourceFile::new("foo.ps1".into(), Language::PowerShell, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

fn lloc(report: &LanguageAnalysis) -> f64 {
    match report
        .root
        .metrics
        .get(&MetricKey::new("loc.lloc"))
        .unwrap()
    {
        MetricValue::Int(i) => i as f64,
        MetricValue::Float(f) => f,
    }
}

#[test]
fn powershell_simple_loc() {
    let a = analyze("# header\nfunction Greet($name) {\n    Write-Host \"hi, $name\"\n}");
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::loc(&a.root.metrics),
        @r###"
    {
      "sloc": 4.0,
      "ploc": 3.0,
      "lloc": 2.0,
      "cloc": 1.0,
      "blank": 0.0,
      "sloc_average": 2.0,
      "ploc_average": 1.5,
      "lloc_average": 1.0,
      "cloc_average": 0.5,
      "blank_average": 0.0,
      "sloc_min": 3.0,
      "sloc_max": 3.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 3.0,
      "ploc_max": 3.0,
      "lloc_min": 2.0,
      "lloc_max": 2.0,
      "blank_min": 0.0,
      "blank_max": 0.0
    }
    "###
    );
}

#[test]
fn powershell_comment_and_block_comment_are_counted_as_cloc() {
    // `#` line comments and `<# ... #>` block comments both surface as
    // the named `comment` node in tree-sitter-pwsh.
    let a = analyze(
        "<#\n             Doc comment\n             #>\n             # inline comment\n             $x = 1 # trailing comment",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::loc(&a.root.metrics),
        @r###"
    {
      "sloc": 5.0,
      "ploc": 1.0,
      "lloc": 1.0,
      "cloc": 5.0,
      "blank": 0.0,
      "sloc_average": 5.0,
      "ploc_average": 1.0,
      "lloc_average": 1.0,
      "cloc_average": 5.0,
      "blank_average": 0.0,
      "sloc_min": 5.0,
      "sloc_max": 5.0,
      "cloc_min": 5.0,
      "cloc_max": 5.0,
      "ploc_min": 1.0,
      "ploc_max": 1.0,
      "lloc_min": 1.0,
      "lloc_max": 1.0,
      "blank_min": 0.0,
      "blank_max": 0.0
    }
    "###
    );
}

#[test]
fn powershell_assignment_counts_as_one_lloc() {
    let a = analyze("$x = 1\n$y = 2\n$z = 3");
    assert_eq!(lloc(&a), 3.0);
}
