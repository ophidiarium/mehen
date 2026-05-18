//! PowerShell NPA tests, ported from
//! `src/metrics/npa.rs::tests` per rewrite plan §8.2.

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
fn powershell_npa_counts_all_properties_as_public_including_hidden() {
    // PowerShell has no access-modifier equivalent to `private` /
    // `protected`. The `hidden` keyword only suppresses a property
    // from default Get-Member / IntelliSense; the property is still
    // publicly accessible. Per about_Hidden: "hidden members are
    // still public". NPA counts every property as public.
    let a = analyze(
        "class C {
                 [int]$a = 1
                 hidden [int]$b = 2
                 [int]$c = 3
                 hidden [int]$d = 4
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::npa(&a.root.metrics),
        @r###"
    {
      "classes": 4.0,
      "interfaces": 0.0,
      "class_attributes": 4.0,
      "interface_attributes": 0.0,
      "classes_average": 1.0,
      "interfaces_average": null,
      "total": 4.0,
      "total_attributes": 4.0,
      "average": 1.0
    }
    "###
    );
}
