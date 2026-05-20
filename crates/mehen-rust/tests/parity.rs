//! Parity / improvement snapshots for the ra_ap_syntax-backed Rust
//! analyzer. Each test reproduces a Rust syntactic construct where
//! ra_ap_syntax's typed AST gives a strictly better answer than the
//! legacy tree-sitter walker — proven from the metric definition, not
//! from a desire to mirror legacy behavior.
//!
//! See `docs/rust-ra-ap-syntax-spec.md` for the full design rationale.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_rust::RustAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = RustAnalyzer::new();
    let file = SourceFile::new("foo.rs".into(), Language::Rust, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

/// `let-else` (RFC 3137, stable in Rust 1.65) is still a `LET_STMT` in
/// ra_ap_syntax. The legacy tree-sitter grammar (0.24.x) parses it,
/// but its `let_declaration` arm only checks `is_child(EQ)` for the
/// "is an assignment?" test. ra_ap_syntax exposes the diverging else
/// branch directly via `LetStmt::let_else()`, and the divergent
/// branch's body becomes a new cognitive nesting frame the moment it
/// contains a control-flow expression. This test confirms the walker
/// emits +1 ABC.assignments for the bind and that the control-flow
/// inside the `else` branch participates normally.
#[test]
fn rust_let_else_is_assignment_with_diverging_else() {
    let a = analyze(
        "fn parse(input: &str) -> Result<u32, ()> {
             let Some(stripped) = input.strip_prefix(\"#\") else {
                 return Err(());
             };
             stripped.parse().map_err(|_| ())
         }",
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    assert!(
        abc.assignments >= 1.0,
        "let-else binding must count as an assignment, got {}",
        serde_json::to_string(&abc).unwrap()
    );
    let nx = mehen_report::metrics_json::nexits(&a.root.metrics);
    // The `return Err(())` inside the let-else's diverging branch is a
    // real exit point.
    assert!(
        nx.sum >= 1.0,
        "let-else diverging branch's `return` must count as exit, got {}",
        serde_json::to_string(&nx).unwrap()
    );
}

/// `if let` chains (RFC 2497, stable in Rust 1.88). Multiple `if let
/// PAT = expr && let PAT = expr` are flattened into a single `IF_EXPR`
/// with a chain of `LetExpr` operands joined by `&&`. The legacy
/// tree-sitter walker exposed `let_chain` as a distinct named node.
/// ra_ap_syntax: each `&&` shows as a `BinExpr` with `LogicOp::And`,
/// and the boolean-sequence collapser in `mehen-metrics::cognitive`
/// folds the same-op run into a single +1.
#[test]
fn rust_if_let_chain_collapses_to_single_cognitive_bump() {
    let a = analyze(
        "fn f(a: Option<i32>, b: Option<i32>) -> Option<i32> {
             if let Some(x) = a && let Some(y) = b && x > y {
                 Some(x + y)
             } else {
                 None
             }
         }",
    );
    let cog = mehen_report::metrics_json::cognitive(&a.root.metrics);
    // +1 for the `if`, +1 for the same-op `&&` run (collapsed), +1 for
    // the `else` keyword. Cognitive sum must be exactly 3.
    assert_eq!(
        cog.sum,
        3.0,
        "if-let chain with `&&` must collapse to a single +1 + else, got {}",
        serde_json::to_string(&cog).unwrap()
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    // 1 (unit) + 1 (function baseline) + 1 (if) + 2 (two `&&`) = 5
    assert_eq!(
        cy.sum,
        5.0,
        "expected 5 cyclomatic decisions, got {}",
        serde_json::to_string(&cy).unwrap()
    );
}

/// `?` operator (try-expression) inside a deeply-nested expression.
/// The legacy walker captured it via `try_expression` — ra_ap_syntax
/// surfaces the same node as `TRY_EXPR`. Two `?` in one expression
/// must each contribute +1 cyclomatic / +1 cognitive (no nesting) /
/// +1 nexit / +1 ABC.condition.
#[test]
fn rust_question_marks_in_chain_each_count() {
    let a = analyze(
        "fn f() -> Result<u32, ()> {
             let r = compute()?.lookup(key)?.parse::<u32>()?;
             Ok(r)
         }",
    );
    let nx = mehen_report::metrics_json::nexits(&a.root.metrics);
    assert!(
        nx.sum >= 3.0,
        "three `?` operators must yield 3 exits, got {}",
        serde_json::to_string(&nx).unwrap()
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    // 1 (unit) + 1 (fn baseline) + 3 (three `?`) = 5
    assert!(
        cy.sum >= 5.0,
        "three `?` must each be a decision, got {}",
        serde_json::to_string(&cy).unwrap()
    );
}

/// Closures with explicit type annotations: `|x: i32, y: i32| -> i32`.
/// Legacy walker counted parameter parens / commas via the
/// `closure_parameters` named node. ra_ap_syntax exposes
/// `ClosureExpr::param_list()` directly with one `Param` per declared
/// parameter — argc is the AST-level parameter count, not a token
/// count. This test confirms the closure-arg count matches the AST.
#[test]
fn rust_typed_closure_records_correct_arg_count() {
    let a = analyze(
        "fn make() -> impl Fn(i32, i32, i32) -> i32 {
             |x: i32, y: i32, z: i32| -> i32 { x + y + z }
         }",
    );
    let nargs = mehen_report::metrics_json::nargs(&a.root.metrics);
    assert_eq!(
        nargs.total_closures,
        3.0,
        "closure with 3 typed params must report 3 closure args, got {}",
        serde_json::to_string(&nargs).unwrap()
    );
    assert_eq!(
        nargs.closures_max,
        3.0,
        "closures_max must reflect the AST-level param count, got {}",
        serde_json::to_string(&nargs).unwrap()
    );
}

/// `async fn` is a `Fn` AST node with `async_token()` set. It still
/// opens a function space; nesting / depth follows the same rules.
#[test]
fn rust_async_fn_opens_function_space() {
    let a = analyze(
        "async fn fetch(url: &str) -> String {
             let res = client.get(url).await;
             res.text().await
         }",
    );
    let nargs = mehen_report::metrics_json::nargs(&a.root.metrics);
    assert_eq!(
        nargs.total_functions,
        1.0,
        "async fn must count as a function, got {}",
        serde_json::to_string(&nargs).unwrap()
    );
    assert_eq!(
        nargs.functions_min,
        1.0,
        "async fn `fetch` has 1 param, functions_min must be 1, got {}",
        serde_json::to_string(&nargs).unwrap()
    );
}

/// Trait associated types (`type Item;`) and constants (`const N: u32;`)
/// must NOT count as methods. Only `Fn` items inside a Trait body
/// contribute to NPM.
#[test]
fn rust_trait_associated_types_and_consts_are_not_methods() {
    let a = analyze(
        "trait Iterator2 {
             type Item;
             const SIZE: usize = 4;
             fn next(&mut self) -> Option<Self::Item>;
             fn count(&self) -> usize { 0 }
         }",
    );
    let npm = mehen_report::metrics_json::npm(&a.root.metrics);
    // 2 fns (next, count). type/const are not methods.
    assert_eq!(
        npm.interface_methods,
        2.0,
        "associated type/const must not count as methods, got {}",
        serde_json::to_string(&npm).unwrap()
    );
}

/// Macro-call body opacity: `vec![1, if x { 2 } else { 3 }]` contains
/// an `if` inside macro tokens. Legacy walker correctly excluded
/// macro-internal control flow from cyclomatic / cognitive. The
/// ra_ap_syntax-backed walker preserves that — the `if` lives inside
/// a `MacroCall`'s token tree, which our walker marks opaque.
#[test]
fn rust_macro_body_control_flow_is_opaque() {
    let a = analyze(
        "fn f() {
             let v = vec![1, if x { 2 } else { 3 }];
         }",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    // Unit (1) + fn baseline (1) = 2. Macro tokens add 0.
    assert_eq!(
        cy.sum,
        2.0,
        "macro body's `if` must not contribute to cyclomatic, got {}",
        serde_json::to_string(&cy).unwrap()
    );
}

/// `match` arm guards (`Some(x) if x > 0 => …`) — the guard is a
/// boolean expression that runs after pattern matching succeeds. The
/// match arm itself contributes +1 cyclomatic; the guard's `>` adds
/// +1 ABC.condition (a comparison) but not extra cyclomatic in our
/// walker. Legacy did the same.
#[test]
fn rust_match_arm_guard_adds_condition_not_cyclomatic() {
    let a = analyze(
        "fn classify(x: Option<i32>) -> &'static str {
             match x {
                 Some(n) if n > 0 => \"positive\",
                 Some(n) if n < 0 => \"negative\",
                 _ => \"zero or none\",
             }
         }",
    );
    let cy = mehen_report::metrics_json::cyclomatic(&a.root.metrics);
    // Unit (1) + fn baseline (1) + 3 match arms = 5
    assert_eq!(
        cy.sum,
        5.0,
        "3 match arms must give 3 cyclomatic decisions, got {}",
        serde_json::to_string(&cy).unwrap()
    );
    let abc = mehen_report::metrics_json::abc(&a.root.metrics);
    // 3 match arms (each is +1 condition) + match expr itself (+1) +
    // 2 comparison `>`/`<` (each +1) = 6 conditions minimum.
    assert!(
        abc.conditions >= 6.0,
        "guards' comparisons must add ABC conditions, got {}",
        serde_json::to_string(&abc).unwrap()
    );
}

/// `pub(crate) fn` and `pub(super) fn` are both *non-default*
/// visibility modifiers. ra_ap_syntax's `HasVisibility::visibility()`
/// returns `Some(_)` for any `pub`/`pub(...)` form. Our walker
/// classifies any non-None visibility as public for NPM purposes —
/// which is what the legacy walker's "child is `visibility_modifier`"
/// check did too.
#[test]
fn rust_pub_crate_and_pub_super_count_as_public() {
    let a = analyze(
        "struct S;
         impl S {
             pub fn a(&self) {}
             pub(crate) fn b(&self) {}
             pub(super) fn c(&self) {}
             fn d(&self) {}
         }",
    );
    let npm = mehen_report::metrics_json::npm(&a.root.metrics);
    // 4 methods total. Public: a, b, c. Non-public: d.
    assert_eq!(
        npm.class_methods,
        4.0,
        "got {}",
        serde_json::to_string(&npm).unwrap()
    );
    assert_eq!(
        npm.classes,
        3.0,
        "pub/pub(crate)/pub(super) all count as public, got {}",
        serde_json::to_string(&npm).unwrap()
    );
}
