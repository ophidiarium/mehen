//! PowerShell NPM tests, ported from
//! `src/metrics/npm.rs::tests` per rewrite plan §8.2.

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
fn powershell_npm_counts_all_methods_as_public_including_hidden() {
    // Same convention as NPA: PowerShell has no `private` /
    // `protected`; `hidden` only suppresses Get-Member output. NPM
    // counts every method as public.
    let a = analyze(
        "class C {
                 [void] A() { }
                 hidden [void] B() { }
                 [void] Cm() { }
                 hidden [void] D() { }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::npm(&a.root.metrics),
        @r###"
    {
      "classes": 4.0,
      "interfaces": 0.0,
      "class_methods": 4.0,
      "interface_methods": 0.0,
      "classes_average": 1.0,
      "interfaces_average": null,
      "total": 4.0,
      "total_methods": 4.0,
      "average": 1.0
    }
    "###
    );
}
