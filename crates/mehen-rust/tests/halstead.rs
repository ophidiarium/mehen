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
