//! Halstead tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/halstead.rs::tests`.

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
fn rust_operators_and_operands() {
    // Drift from pre-1.0: the legacy walker classified tokens via
    // tree-sitter-rust's `Op` table; the Phase-9 ra_ap_syntax walker
    // uses ra_ap_syntax's `T!` macro mapping. The token stream produced
    // is similar but not identical at the boundary (e.g. `!` from a
    // `println!` macro is its own token rather than fused into the
    // path). We assert only on a *lower bound* of unique operators
    // and operands plus the reported volume's order of magnitude — the
    // exact `n1`/`n2` numbers depend on the lexer mapping and are
    // documented in `docs/rust-ra-ap-syntax-spec.md` §4.
    let a = analyze(
        "fn main() {
              let a = 5; let b = 5; let c = 5;
              let avg = (a + b + c) / 3;
              println!(\"{}\", avg);
            }",
    );
    let h = mehen_report::metrics_json::halstead(&a.root.metrics);
    // Operators: `fn`, `let` (×4), `=` (×4), `+` (×2), `/`, `(`, ...
    // Operands: `main`, `a`, `b`, `c`, `avg`, `5` (×3), `3`, `println`,
    // `"{}"`. The unique-operand count must include at least the
    // distinct identifiers and literal strings.
    assert!(
        h.n2 >= 7.0,
        "expected n2 >= 7 (main, a, b, c, avg, number, string), got {}",
        serde_json::to_string(&h).unwrap()
    );
    assert!(
        h.n1 >= 5.0,
        "expected n1 >= 5 (fn, let, =, /, +, ...), got {}",
        serde_json::to_string(&h).unwrap()
    );
    assert!(
        h.volume > 100.0,
        "expected non-trivial volume, got {}",
        serde_json::to_string(&h).unwrap()
    );
}

/// Regression: nested function spaces must carry their own Halstead
/// counts in the per-space JSON. PR #95 discussion_r3265658502 flagged
/// this on the Python walker; the Rust walker had the same bug —
/// `observe_token` recorded every event onto `stack[0]` so inner
/// functions ended up with `halstead.N1 == halstead.N2 == 0`.
#[test]
fn rust_nested_function_halstead_is_non_zero() {
    let a = analyze(
        "fn outer() {
    fn inner() {
        let x = 1 + 2;
    }
    inner();
}",
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
        "inner fn must record `let`, `=`, `+` operators, got {}",
        serde_json::to_string(&inner_h).unwrap()
    );
    assert!(
        inner_h.big_n2 > 0.0,
        "inner fn must record `x`, `1`, `2` operands, got {}",
        serde_json::to_string(&inner_h).unwrap()
    );
    assert!(
        inner_h.volume > 0.0,
        "inner fn volume must be > 0, got {}",
        serde_json::to_string(&inner_h).unwrap()
    );

    let outer_h = mehen_report::metrics_json::halstead(&outer.metrics);
    assert!(
        outer_h.big_n1 >= inner_h.big_n1,
        "outer fn N1 must roll up inner: outer={} inner={}",
        serde_json::to_string(&outer_h).unwrap(),
        serde_json::to_string(&inner_h).unwrap()
    );
    assert!(
        outer_h.big_n2 >= inner_h.big_n2,
        "outer fn N2 must roll up inner: outer={} inner={}",
        serde_json::to_string(&outer_h).unwrap(),
        serde_json::to_string(&inner_h).unwrap()
    );
}
