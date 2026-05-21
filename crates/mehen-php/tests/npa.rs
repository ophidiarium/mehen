// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! NPA tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/npa.rs::tests`.

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
fn php_npa_counts_each_property_in_grouped_declaration() {
    // `public $a, $b;` is one property declaration with two property
    // items — count both. class_attributes: 3 (a, b, c). class_npa: 2 (a, b).
    let a = analyze(
        "<?php
         class C {
             public $a, $b;
             private $c;
         }",
    );
    let npa = mehen_report::metrics_json::npa(&a.root.metrics);
    insta::assert_json_snapshot!(
        npa,
        @r#"
    {
      "classes": 2.0,
      "interfaces": 0.0,
      "class_attributes": 3.0,
      "interface_attributes": 0.0,
      "classes_average": 0.6666666666666666,
      "interfaces_average": null,
      "total": 2.0,
      "total_attributes": 3.0,
      "average": 0.6666666666666666
    }
    "#
    );
}

#[test]
fn php_npa_counts_promoted_constructor_properties() {
    // PHP 8 constructor property promotion: `public int $id` in the
    // ctor declares a real property and must contribute to NPA.
    // 2 attributes total (id, name); 1 public (id).
    let a = analyze(
        "<?php
         class C {
             public function __construct(
                 public int $id,
                 private string $name
             ) {}
         }",
    );
    let npa = mehen_report::metrics_json::npa(&a.root.metrics);
    insta::assert_json_snapshot!(
        npa,
        @r#"
    {
      "classes": 1.0,
      "interfaces": 0.0,
      "class_attributes": 2.0,
      "interface_attributes": 0.0,
      "classes_average": 0.5,
      "interfaces_average": null,
      "total": 1.0,
      "total_attributes": 2.0,
      "average": 0.5
    }
    "#
    );
}

#[test]
fn php_npa_does_not_count_promoted_params_outside_constructor() {
    // The PHP grammar accepts promoted-style parameters syntactically
    // inside any method, even though PHP rejects them at runtime
    // outside `__construct`. We must not attribute these as class
    // properties — the regular property declarations are the only
    // real attributes here.
    //
    // Only `$real` is an attribute. `$bogus` is a promoted-style
    // parameter on a non-constructor method and must not be counted.
    let a = analyze(
        "<?php
         class C {
             public $real;
             public function notConstructor(public int $bogus) {}
         }",
    );
    let npa = mehen_report::metrics_json::npa(&a.root.metrics);
    insta::assert_json_snapshot!(
        npa,
        @r#"
    {
      "classes": 1.0,
      "interfaces": 0.0,
      "class_attributes": 1.0,
      "interface_attributes": 0.0,
      "classes_average": 1.0,
      "interfaces_average": null,
      "total": 1.0,
      "total_attributes": 1.0,
      "average": 1.0
    }
    "#
    );
}
