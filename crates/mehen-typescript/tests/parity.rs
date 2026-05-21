// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Parity snapshots for the Oxc-backed TS/JS/TSX/JSX analyzer.
//!
//! Each test reproduces a legacy `crates/mehen-engine/src/legacy/metrics/*.rs`
//! TypeScript / TSX assertion using the same fixture and the same
//! expected JSON. The snapshots come from the legacy
//! `check_metrics::<TypescriptParser>` body — every drift from the
//! pre-1.0 tree-sitter-typescript output must be classified per the
//! plan §12.3.1 (parity bug, intentional improvement, parser limitation,
//! or metric-definition fix).

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_typescript::{TsxAnalyzer, TypeScriptAnalyzer};

fn analyze_ts(source: &str, filename: &str) -> mehen_core::LanguageAnalysis {
    // The legacy `check_metrics` strips trailing newlines and pushes a
    // single one — match that precisely so any LOC line-count drift is
    // not just a whitespace artifact.
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = TypeScriptAnalyzer::new();
    let file = SourceFile::new(filename.into(), Language::TypeScript, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

fn analyze_tsx(source: &str, filename: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = TsxAnalyzer::new();
    let file = SourceFile::new(filename.into(), Language::Tsx, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn typescript_for_variants_count_once() {
    let a = analyze_ts(
        "function f(arr) { // +2 (+1 unit space)
             for (let i = 0; i < 3; i++) {}  // +1
             for (const k in arr) {}          // +1
             for (const v of arr) {}          // +1
         }",
        "foo.ts",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    insta::assert_json_snapshot!(
        cy,
        @r###"
    {
      "sum": 5.0,
      "average": 2.5,
      "min": 1.0,
      "max": 4.0
    }"###
    );
}

#[test]
fn typescript_do_while() {
    let a = analyze_ts(
        "function f() { // +2 (+1 unit space)
             do {
                 x++;
             } while (x < 10); // +1 loop
         }",
        "foo.ts",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    insta::assert_json_snapshot!(
        cy,
        @r###"
    {
      "sum": 3.0,
      "average": 1.5,
      "min": 1.0,
      "max": 2.0
    }"###
    );
}

#[test]
fn typescript_if_else_if_else() {
    let a = analyze_ts(
        "function foo() {
             if (this._closed) return Promise.resolve(); // +1
             if (this._tempDirectory) { // +1
                 this.kill();
             } else if (this.connection) { // +1
                 this.kill();
             } else { // +1
                 throw new Error(`Error`);
            }
            helper.removeEventListeners(this._listeners);
            return this._processClosing;
        }",
        "foo.ts",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(cog, @r###"
    {
      "sum": 4.0,
      "average": 4.0,
      "min": 0.0,
      "max": 4.0
    }"###);
}

#[test]
fn typescript_try_catch_nesting() {
    let a = analyze_ts(
        "function f() {
             try {                  // +1
                 if (a) {           // +2 (nesting = 1)
                     return 1;
                 }
             } catch (e) {          // +2 (nesting = 1)
                 if (b) {           // +3 (nesting = 2)
                     throw e;
                 }
             }
         }",
        "foo.ts",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    insta::assert_json_snapshot!(cog, @r###"
    {
      "sum": 8.0,
      "average": 8.0,
      "min": 0.0,
      "max": 8.0
    }"###);
}

#[test]
fn typescript_throw_counts_as_exit() {
    let a = analyze_ts(
        "function f(x: number) {
             if (x < 0) throw new Error('neg');
             return x;
         }",
        "foo.ts",
    );
    let nx = mehen_report::metrics_json::nexits(&a.root.metrics);
    insta::assert_json_snapshot!(nx, @r###"
    {
      "sum": 2.0,
      "average": 2.0,
      "min": 0.0,
      "max": 2.0
    }"###);
}

#[test]
fn typescript_abc_basic() {
    let a = analyze_ts(
        "function f(a: number, b: number): number {
             let x = a;          // +1 A (declarator with `=`)
             x += b;             // +1 A
             log(x);             // +1 B
             if (x > b) {        // +1 C (if) + +1 C (>)
                 return x;
             }
             return b;
         }",
        "foo.ts",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    insta::assert_json_snapshot!(abc, @r###"
    {
      "assignments": 2.0,
      "branches": 1.0,
      "conditions": 2.0,
      "magnitude": 3.0,
      "assignments_average": 1.0,
      "branches_average": 0.5,
      "conditions_average": 1.0,
      "assignments_min": 0.0,
      "assignments_max": 2.0,
      "branches_min": 0.0,
      "branches_max": 1.0,
      "conditions_min": 0.0,
      "conditions_max": 2.0
    }"###);
}

#[test]
fn typescript_operators_and_operands() {
    let a = analyze_ts(
        "function main() {
          var a, b, c, avg;
          a = 5; b = 5; c = 5;
          avg = (a + b + c) / 3;
          console.log(\"{}\", avg);
        }",
        "foo.ts",
    );
    let h = mehen_report::metrics_json::halstead(&a.root.metrics);
    insta::assert_json_snapshot!(h, @r###"
    {
      "n1": 10.0,
      "N1": 24.0,
      "n2": 11.0,
      "N2": 21.0,
      "length": 45.0,
      "estimated_program_length": 71.27302875388389,
      "purity_ratio": 1.583845083419642,
      "vocabulary": 21.0,
      "volume": 197.65428402504423,
      "difficulty": 9.545454545454545,
      "level": 0.10476190476190476,
      "effort": 1886.699983875422,
      "time": 104.81666577085679,
      "bugs": 0.05089564733125986
    }"###);
}

#[test]
fn tsx_operators_and_operands() {
    let a = analyze_tsx(
        "function main() {
          var a, b, c, avg;
          a = 5; b = 5; c = 5;
          avg = (a + b + c) / 3;
          console.log(\"{}\", avg);
        }",
        "foo.tsx",
    );
    let h = mehen_report::metrics_json::halstead(&a.root.metrics);
    insta::assert_json_snapshot!(h, @r###"
    {
      "n1": 10.0,
      "N1": 24.0,
      "n2": 11.0,
      "N2": 21.0,
      "length": 45.0,
      "estimated_program_length": 71.27302875388389,
      "purity_ratio": 1.583845083419642,
      "vocabulary": 21.0,
      "volume": 197.65428402504423,
      "difficulty": 9.545454545454545,
      "level": 0.10476190476190476,
      "effort": 1886.699983875422,
      "time": 104.81666577085679,
      "bugs": 0.05089564733125986
    }"###);
}

#[test]
fn typescript_npa_counts_public_fields() {
    let a = analyze_ts(
        "class C {
             a: number = 1;
             public b: number = 2;
             private c: number = 3;
             protected d: number = 4;
         }",
        "foo.ts",
    );
    let npa = mehen_report::metrics_json::npa(&a.root.metrics);
    insta::assert_json_snapshot!(npa, @r###"
    {
      "classes": 2.0,
      "interfaces": 0.0,
      "class_attributes": 4.0,
      "interface_attributes": 0.0,
      "classes_average": 0.5,
      "interfaces_average": null,
      "total": 2.0,
      "total_attributes": 4.0,
      "average": 0.5
    }"###);
}

#[test]
fn typescript_npa_counts_ecmascript_private_fields() {
    let a = analyze_ts(
        "class C {
             a: number = 1;
             #b: number = 2;
             #c: number = 3;
         }",
        "foo.ts",
    );
    let npa = mehen_report::metrics_json::npa(&a.root.metrics);
    insta::assert_json_snapshot!(npa, @r###"
    {
      "classes": 1.0,
      "interfaces": 0.0,
      "class_attributes": 3.0,
      "interface_attributes": 0.0,
      "classes_average": 0.3333333333333333,
      "interfaces_average": null,
      "total": 1.0,
      "total_attributes": 3.0,
      "average": 0.3333333333333333
    }"###);
}

#[test]
fn typescript_npm_counts_modifiers() {
    let a = analyze_ts(
        "class C {
             a() {}
             public b() {}
             private c() {}
             protected d() {}
         }",
        "foo.ts",
    );
    let npm = mehen_report::metrics_json::npm(&a.root.metrics);
    insta::assert_json_snapshot!(npm, @r###"
    {
      "classes": 2.0,
      "interfaces": 0.0,
      "class_methods": 4.0,
      "interface_methods": 0.0,
      "classes_average": 0.5,
      "interfaces_average": null,
      "total": 2.0,
      "total_methods": 4.0,
      "average": 0.5
    }"###);
}

#[test]
fn typescript_npm_counts_ecmascript_private_methods() {
    let a = analyze_ts(
        "class C {
             a() {}
             #b() {}
             #c() {}
         }",
        "foo.ts",
    );
    let npm = mehen_report::metrics_json::npm(&a.root.metrics);
    insta::assert_json_snapshot!(npm, @r###"
    {
      "classes": 1.0,
      "interfaces": 0.0,
      "class_methods": 3.0,
      "interface_methods": 0.0,
      "classes_average": 0.3333333333333333,
      "interfaces_average": null,
      "total": 1.0,
      "total_methods": 3.0,
      "average": 0.3333333333333333
    }"###);
}

#[test]
fn tsx_npm_counts_ecmascript_private_methods() {
    let a = analyze_tsx(
        "class C {
             a() {}
             #b() {}
         }",
        "foo.tsx",
    );
    let npm = mehen_report::metrics_json::npm(&a.root.metrics);
    insta::assert_json_snapshot!(npm, @r###"
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
    }"###);
}

/// Locks in the principled TypeScript Halstead spec
/// (`docs/typescript-halstead-spec.md`): pure type metadata
/// (annotations, interface bodies, `implements` clauses, type
/// parameters, `TS*Keyword` predefined types) does NOT contribute to
/// either operators or operands. The fixture is the same as the
/// embedded-code-large markdown fence — if this test ever drifts,
/// re-run `cargo insta accept` for the markdown snapshot too.
#[test]
fn typescript_halstead_excludes_type_only_tokens() {
    let a = analyze_ts(
        "interface Shape {
    area(): number;
}

class Circle implements Shape {
    constructor(private radius: number) {}
    area(): number {
        return Math.PI * this.radius * this.radius;
    }
}

class Rectangle implements Shape {
    constructor(private w: number, private h: number) {}
    area(): number {
        return this.w * this.h;
    }
}

function totalArea(shapes: Shape[]): number {
    return shapes.reduce((t, s) => t + s.area(), 0);
}",
        "fence.ts",
    );
    let h = mehen_report::metrics_json::halstead(&a.root.metrics);
    insta::assert_json_snapshot!(h);
}

#[test]
fn typescript_wmc_class_sums_method_cyclomatics() {
    let a = analyze_ts(
        "class C {
             a(x: number) {
                 if (x) { return 1; }
                 return 0;
             }
             b() { return 1; }
         }",
        "foo.ts",
    );
    let wmc = mehen_report::metrics_json::wmc(&a.root.metrics);
    insta::assert_json_snapshot!(wmc, @r###"
    {
      "classes": 3.0,
      "interfaces": 0.0,
      "total": 3.0
    }"###);
}

/// Regression: nested function spaces must carry their own Halstead
/// counts in the per-space JSON. PR #95 discussion_r3265658502 flagged
/// the bug on the Python walker; TS had the same `stack[0]`-only
/// behaviour (with an explicit comment acknowledging it).
#[test]
fn typescript_nested_function_halstead_is_non_zero() {
    let a = analyze_ts(
        "function outer() {
  function inner() {
    const x = 1 + 2;
    return x;
  }
  inner();
}",
        "nested.ts",
    );
    assert_eq!(a.root.spaces.len(), 1, "expected outer fn");
    let outer = &a.root.spaces[0];
    assert_eq!(outer.name.as_deref(), Some("outer"));
    assert_eq!(outer.spaces.len(), 1, "expected nested inner fn");
    let inner = &outer.spaces[0];
    assert_eq!(inner.name.as_deref(), Some("inner"));

    let inner_h = mehen_report::metrics_json::halstead(&inner.metrics);
    assert!(
        inner_h.big_n1 > 0.0,
        "inner fn must record `const`, `=`, `+`, `return` operators, got {}",
        serde_json::to_string(&inner_h).unwrap()
    );
    assert!(
        inner_h.big_n2 > 0.0,
        "inner fn must record `x`, `1`, `2` operands, got {}",
        serde_json::to_string(&inner_h).unwrap()
    );

    let outer_h = mehen_report::metrics_json::halstead(&outer.metrics);
    assert!(
        outer_h.big_n1 >= inner_h.big_n1,
        "outer N1 must roll up inner: outer={} inner={}",
        serde_json::to_string(&outer_h).unwrap(),
        serde_json::to_string(&inner_h).unwrap()
    );
    assert!(
        outer_h.big_n2 >= inner_h.big_n2,
        "outer N2 must roll up inner: outer={} inner={}",
        serde_json::to_string(&outer_h).unwrap(),
        serde_json::to_string(&inner_h).unwrap()
    );
}
