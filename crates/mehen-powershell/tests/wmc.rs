//! PowerShell WMC tests, ported from
//! `src/metrics/wmc.rs::tests` per rewrite plan §8.2.

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
fn powershell_wmc_class_sums_method_cyclomatics() {
    // class C: A cyc = 2 (if), B cyc = 1 → classes = 3.
    let a = analyze(
        "class C {
                 [int] A([int]$x) {
                     if ($x -gt 0) {
                         return 1
                     }
                     return 0
                 }
                 [int] B() { return 1 }
             }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::wmc(&a.root.metrics),
        @r###"
    {
      "classes": 3.0,
      "interfaces": 0.0,
      "total": 3.0
    }
    "###
    );
}
