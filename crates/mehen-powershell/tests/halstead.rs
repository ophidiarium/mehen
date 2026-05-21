// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! PowerShell Halstead tests, ported from
//! `src/metrics/halstead.rs::tests` per rewrite plan §8.2.

use mehen_core::{
    AnalysisConfig, Language, LanguageAnalysis, LanguageAnalyzer, MetricKey, MetricValue,
    SourceFile,
};
use mehen_powershell::PowerShellAnalyzer;

fn analyze(source: &str) -> LanguageAnalysis {
    let analyzer = PowerShellAnalyzer::new();
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let file = SourceFile::new("foo.ps1".into(), Language::PowerShell, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

fn metric(report: &LanguageAnalysis, key: &str) -> f64 {
    match report.root.metrics.get(&MetricKey::new(key)).unwrap() {
        MetricValue::Int(i) => i as f64,
        MetricValue::Float(f) => f,
    }
}

#[test]
fn powershell_operator_wrappers_do_not_double_count() {
    // Regression: tree-sitter-pwsh nests every operator leaf token (e.g.
    // `-eq`, `-f`, `=`, `2>`) inside a named wrapper rule
    // (`comparison_operator`, `format_operator`, `assignment_operator`,
    // `file_redirection_operator`, `merging_redirection_operator`). The
    // walker visits both wrapper and leaf — classifying both as
    // operators would double-count. The classifier matches only the
    // leaves.
    let a = analyze("$x = $a -eq $b");
    // Operators: `=` and `-eq` → 2 distinct, 2 total.
    // Operands: `$x`, `$a`, `$b` → 3 distinct, 3 total.
    assert_eq!(metric(&a, "halstead.n1"), 2.0);
    assert_eq!(metric(&a, "halstead.N1"), 2.0);
    assert_eq!(metric(&a, "halstead.n2"), 3.0);
    assert_eq!(metric(&a, "halstead.N2"), 3.0);

    // Same invariant for the format operator `-f`.
    let b = analyze("$s = \"{0}\" -f $a");
    // Operators: `=` and `-f` → 2 distinct, 2 total.
    assert_eq!(metric(&b, "halstead.n1"), 2.0);
    assert_eq!(metric(&b, "halstead.N1"), 2.0);
}

#[test]
fn powershell_function_and_command_names_count_as_operands() {
    // The PowerShell operand set must include the identifier leaves
    // that drive function declarations (`function_name`) and command
    // invocations (`command_name`, `path_command_name_token`). Without
    // them, Halstead N2 and volume are suppressed for cmdlet-heavy
    // scripts.

    // Simple cmdlet call: `Get-Item /tmp` → operands are `Get-Item`
    // and `/tmp` (a generic_token argument).
    let a = analyze("Get-Item /tmp");
    assert_eq!(metric(&a, "halstead.n2"), 2.0);
    assert_eq!(metric(&a, "halstead.N2"), 2.0);

    // Path-style command: `./build.sh arg1` → operands are
    // `./build.sh` (a `path_command_name_token` leaf) and `arg1`. Must
    // not double-count the `path_command_name` wrapper.
    let b = analyze("./build.sh arg1");
    assert_eq!(metric(&b, "halstead.n2"), 2.0);
    assert_eq!(metric(&b, "halstead.N2"), 2.0);

    // Function declaration: the `function_name` leaf counts once.
    let c = analyze("function Greet { }");
    assert_eq!(metric(&c, "halstead.n2"), 1.0);
    assert_eq!(metric(&c, "halstead.N2"), 1.0);
}

#[test]
fn powershell_string_literals_count_as_operands() {
    // Double-quoted ("expandable") and here-string double-quoted
    // literals have no content-leaf node (their text lives inside the
    // wrapper's byte range directly), so the `expandable_string_literal`
    // / `expandable_here_string_literal` *wrapper* kinds themselves are
    // classified as operands — matching the verbatim
    // (single-quoted) branch.
    //
    // 4 distinct strings (`''`, `""`, `'hello'`, `"world"`) plus 4
    // distinct `$` variables (`$a..$d`) → n2 = 8.
    let a = analyze(
        "$a = ''
             $b = \"\"
             $c = 'hello'
             $d = \"world\"",
    );
    assert_eq!(metric(&a, "halstead.n2"), 8.0);
    assert_eq!(metric(&a, "halstead.N2"), 8.0);

    // Empty expandable `""` on its own — n2 = 2 (the empty string + `$x`).
    let b = analyze("$x = \"\"");
    // Operators: `=` → n1=1, N1=1.
    // Operands: `$x`, `""` → n2=2, N2=2.
    assert_eq!(metric(&b, "halstead.n1"), 1.0);
    assert_eq!(metric(&b, "halstead.N1"), 1.0);
    assert_eq!(metric(&b, "halstead.n2"), 2.0);
    assert_eq!(metric(&b, "halstead.N2"), 2.0);
}
