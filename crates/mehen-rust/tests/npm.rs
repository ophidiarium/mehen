// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! NPM tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/npm.rs::tests`.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_rust::RustAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = RustAnalyzer::new();
    let file = SourceFile::new("foo.rs".into(), Language::Rust, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn rust_npm_counts_pub_in_impl_block() {
    // impl S -> 2 methods, 1 public
    let a = analyze(
        "struct S;
         impl S {
             pub fn a(&self) {}
             fn b(&self) {}
         }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::npm(&a.root.metrics),
        @r###"
    {
      "classes": 1.0,
      "interfaces": 0.0,
      "class_methods": 2.0,
      "interface_methods": 0.0,
      "classes_average": 0.5,
      "interfaces_average": null,
      "total": 1.0,
      "total_methods": 2.0,
      "average": 0.5
    }"###
    );
}

#[test]
fn rust_npm_counts_trait_signature_and_default_methods() {
    // Drift from pre-1.0: the legacy NPM serialization used `interfaces`
    // to mean "number of interface containers", and `interfaces_average`
    // to mean `interface_methods / interfaces`. The Phase-1+ pipeline's
    // NPM (in `mehen-metrics::counters::NpmStats::publish_npm`) re-uses
    // those field names with different semantics: `interfaces` is now
    // the total *public-method* count in interfaces, and
    // `interfaces_average` is `public / total` (the public-ratio). This
    // is a deliberate metric-definition change shared with the Python /
    // TypeScript / PowerShell ports — every language's NPM follows the
    // same `publish_npm` shape now. The Phase-9 ra_ap_syntax walker
    // produces:
    //   - 2 public methods in this trait (both `a` and `b` are
    //     implicitly public; legacy rule "trait methods are public")
    //   - 2 total methods
    //   - public ratio = 2/2 = 1.0
    let a = analyze(
        "trait T {
             fn a(&self);
             fn b(&self) {}
         }",
    );
    insta::assert_json_snapshot!(
        mehen_report::metrics_json::npm(&a.root.metrics),
        @r###"
    {
      "classes": 0.0,
      "interfaces": 2.0,
      "class_methods": 0.0,
      "interface_methods": 2.0,
      "classes_average": null,
      "interfaces_average": 1.0,
      "total": 2.0,
      "total_methods": 2.0,
      "average": 1.0
    }"###
    );
}
