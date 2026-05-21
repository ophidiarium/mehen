// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! NPA tests for the tree-sitter-kotlin walker.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_kotlin::KotlinAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = KotlinAnalyzer::new();
    let file = SourceFile::new("foo.kt".into(), Language::Kotlin, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn kotlin_npa_counts_class_properties() {
    let a = analyze(
        "class C {
             val a: Int = 1
             private val b: Int = 2
             protected val c: Int = 3
             internal val d: Int = 4
         }",
    );
    let npa = mehen_report::metrics_json::npa(&a.root.metrics);
    // public: a. non-public: b, c, d.
    insta::assert_json_snapshot!(
        npa,
        @r###"
    {
      "classes": 1.0,
      "interfaces": 0.0,
      "class_attributes": 4.0,
      "interface_attributes": 0.0,
      "classes_average": 0.25,
      "interfaces_average": null,
      "total": 1.0,
      "total_attributes": 4.0,
      "average": 0.25
    }"###
    );
}

#[test]
fn kotlin_npa_counts_constructor_properties() {
    // Constructor parameters with `val`/`var` are class attributes.
    // Plain parameters (no val/var) are NOT attributes.
    let a = analyze(
        "class C(val a: Int, private var b: String, internal val c: Long, d: Double) {
             protected val e: Int = 0
         }",
    );
    let npa = mehen_report::metrics_json::npa(&a.root.metrics);
    insta::assert_json_snapshot!(
        npa,
        @r###"
    {
      "classes": 1.0,
      "interfaces": 0.0,
      "class_attributes": 4.0,
      "interface_attributes": 0.0,
      "classes_average": 0.25,
      "interfaces_average": null,
      "total": 1.0,
      "total_attributes": 4.0,
      "average": 0.25
    }"###
    );
}

#[test]
fn kotlin_npa_routes_interface_properties_to_interface_counters() {
    // Same class-vs-interface routing concern as NPM: tree-sitter-kotlin
    // uses `class_declaration` for both classes and interfaces, so the
    // container must be decided by the declaration's leading keyword.
    let a = analyze(
        "interface Foo {
             val a: Int
             val b: Int
         }

         class Bar {
             val c: Int = 1
             private val d: Int = 2
         }",
    );
    let npa = mehen_report::metrics_json::npa(&a.root.metrics);
    insta::assert_json_snapshot!(
        npa,
        @r###"
    {
      "classes": 1.0,
      "interfaces": 2.0,
      "class_attributes": 2.0,
      "interface_attributes": 2.0,
      "classes_average": 0.5,
      "interfaces_average": 1.0,
      "total": 3.0,
      "total_attributes": 4.0,
      "average": 0.75
    }"###
    );
}
