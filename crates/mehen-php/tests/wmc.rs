//! WMC tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/wmc.rs::tests`.

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
fn php_wmc_class_sums_method_cyclomatics() {
    // class C: a cyc=2 (if), b cyc=1 -> classes = 3
    let a = analyze(
        "<?php
         class C {
             public function a(int $x): int {
                 if ($x > 0) {
                     return 1;
                 }
                 return 0;
             }
             public function b(): int { return 1; }
         }",
    );
    let wmc = mehen_report::metrics_json::wmc(&a.root.metrics);
    insta::assert_json_snapshot!(
        wmc,
        @r###"
    {
      "classes": 3.0,
      "interfaces": 0.0,
      "total": 3.0
    }"###
    );
}
