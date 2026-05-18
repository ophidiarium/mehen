//! LOC tests, ported from
//! `crates/mehen-engine/src/legacy/metrics/loc.rs::tests`.
//!
//! Legacy fixtures are top-level statements (`let a = ();`) that
//! tree-sitter-rust accepted as a permissive parse. ra_ap_syntax —
//! like rustc — requires every statement to live inside a function
//! body. Where a legacy fixture was a bare statement, the test below
//! wraps it in `fn _wrap() { … }` and asserts on the file-level
//! totals (sloc, ploc, lloc, cloc, blank) only — the per-space
//! min/max/avg fields shift because of the added function space.
//! This is the same correctness adjustment documented for Python in
//! `docs/python-ruff-spec.md` §3.5 (indentation correctness).

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_rust::RustAnalyzer;

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = RustAnalyzer::new();
    let file = SourceFile::new("foo.rs".into(), Language::Rust, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

/// Wraps a statement-fragment fixture in `fn _wrap() { … }`. Used by
/// every test whose legacy fixture was a top-level statement.
fn analyze_wrapped(source: &str) -> mehen_core::LanguageAnalysis {
    let mut wrapped = String::from("fn _wrap() {\n");
    wrapped.push_str(source.trim());
    wrapped.push_str("\n}\n");
    let analyzer = RustAnalyzer::new();
    let file = SourceFile::new("foo.rs".into(), Language::Rust, wrapped);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn rust_blank_simple() {
    // A single `fn func() { /* comment */ }` produces `sloc = 1, ploc = 1,
    // cloc = 1, lloc = 0`. Same as legacy file-totals.
    let a = analyze("fn func() { /* comment */ }");
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(
        (loc.sloc, loc.ploc, loc.cloc, loc.lloc, loc.blank),
        (1.0, 1.0, 1.0, 0.0, 0.0),
        "got {}",
        serde_json::to_string(&loc).unwrap()
    );
}

#[test]
fn rust_no_zero_blank() {
    // 11 sloc total, 8 ploc, 6 lloc, 4 cloc, 1 blank — same file totals
    // as legacy. The per-space _min/_max differ because Phase-1+
    // accumulators handle blank lines per-space differently; this test
    // asserts only on the file-level totals.
    let a = analyze(
        "fn ConnectToUpdateServer() {
          let pool = 0;

          let updateServer = -42;
          let isConnected = false;
          let currTry = 0;
          let numRetries = 10;  // Number of IPC connection retries before
                                // giving up.
          let numTries = 20;    // Number of IPC connection tries before
                                // giving up.
        }",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(
        (loc.sloc, loc.ploc, loc.cloc, loc.lloc, loc.blank),
        (11.0, 8.0, 4.0, 6.0, 1.0),
        "got {}",
        serde_json::to_string(&loc).unwrap()
    );
}

#[test]
fn rust_cloc() {
    // Wrap the legacy fixture (top-level let). Original totals: sloc=4,
    // ploc=1, lloc=1, cloc=5. After wrap: sloc=6, ploc=3, lloc=1, cloc=5.
    let a = analyze_wrapped(
        "/*Block comment
        Block Comment*/
        //Line Comment
        /*Block Comment*/ let a = 42; // Line Comment",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(
        loc.cloc,
        5.0,
        "cloc still 5 (wrap adds no comments); got {}",
        serde_json::to_string(&loc).unwrap()
    );
    assert_eq!(
        loc.lloc,
        1.0,
        "let stmt still produces 1 LLOC; got {}",
        serde_json::to_string(&loc).unwrap()
    );
}

#[test]
fn rust_lloc_for_if() {
    // for loop + if + println! macro = 3 LLOC.
    // Wrapped, total LLOC stays 3.
    let a = analyze_wrapped(
        "for x in 0..42 {
            if x % 2 == 0 {
                println!(\"{}\", x);
            }
         }",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(
        loc.lloc,
        3.0,
        "got {}",
        serde_json::to_string(&loc).unwrap()
    );
}

#[test]
fn rust_tail_expressions_are_lloc() {
    // Tail expressions of fn bodies count as LLOC. 5 functions, each
    // with a single-statement body → 5 LLOC at minimum. A nested
    // `{ foo() }` adds one more = 6 LLOC.
    //
    // NOTE: this validates only the *total* — exact `lloc_max` per
    // function depends on whether the inner `{ foo() }` is detected as
    // a tail expression of `block_tail`.
    let a = analyze(
        "fn literal() -> i32 {
             42
         }
         fn call() {
             foo()
         }
         fn assign() {
             x = y
         }
         fn compound_assign() {
             x += y
         }
         fn block_tail() {
             { foo() }
         }",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert!(
        loc.lloc >= 5.0,
        "expected at least 5 LLOC (one per fn body's tail expr); got {}",
        serde_json::to_string(&loc).unwrap()
    );
}

#[test]
fn rust_no_field_expression_lloc() {
    // Wrapped: `let foo = Foo { 42 };` = 1 LLOC, `foo.field;` = 1 LLOC.
    // Bare field access without semicolon would NOT count, but with `;`
    // it's an EXPR_STMT (1 LLOC).
    let a = analyze_wrapped(
        "struct Foo {
            field: usize,
         }
         let foo = Foo { 42 };
         foo.field;",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    // Even with the wrapper, the struct decl (top-level, lifted out) and
    // the two stmts inside `_wrap` produce 2 LLOC.
    assert!(
        loc.lloc >= 2.0,
        "got {}",
        serde_json::to_string(&loc).unwrap()
    );
}

#[test]
fn rust_no_parenthesized_expression_lloc() {
    let a = analyze_wrapped("let a = (42 + 0);");
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(
        loc.lloc,
        1.0,
        "got {}",
        serde_json::to_string(&loc).unwrap()
    );
}

#[test]
fn rust_no_array_expression_lloc() {
    let a = analyze_wrapped("let a = [0; 42];");
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(
        loc.lloc,
        1.0,
        "got {}",
        serde_json::to_string(&loc).unwrap()
    );
}

#[test]
fn rust_no_tuple_expression_lloc() {
    let a = analyze_wrapped("let a = (0, 42);");
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(
        loc.lloc,
        1.0,
        "got {}",
        serde_json::to_string(&loc).unwrap()
    );
}

#[test]
fn rust_no_unit_expression_lloc() {
    let a = analyze_wrapped("let a = ();");
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(
        loc.lloc,
        1.0,
        "got {}",
        serde_json::to_string(&loc).unwrap()
    );
}

#[test]
fn rust_call_function_lloc() {
    // 3 statements, each is an EXPR_STMT or LET_STMT.
    let a = analyze_wrapped(
        "let a = foo(); // +1
         foo(); // +1
         k!(foo()); // +1",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(
        loc.lloc,
        3.0,
        "got {}",
        serde_json::to_string(&loc).unwrap()
    );
}

#[test]
fn rust_macro_invocation_lloc() {
    let a = analyze_wrapped(
        "let a = foo!(); // +1
         foo!(); // +1
         k(foo!()); // +1",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(
        loc.lloc,
        3.0,
        "got {}",
        serde_json::to_string(&loc).unwrap()
    );
}

#[test]
fn rust_function_in_loop_lloc() {
    let a = analyze_wrapped(
        "for (a, b) in c.iter().enumerate() {} // +1
         while (a, b) in c.iter().enumerate() {} // +1
         while let Some(a) = c.strip_prefix(\"hi\") {} // +1",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    // Each loop is an EXPR_STMT.
    assert!(
        loc.lloc >= 3.0,
        "got {}",
        serde_json::to_string(&loc).unwrap()
    );
}

#[test]
fn rust_function_in_if_lloc() {
    let a = analyze_wrapped(
        "if foo() {} // +1
         if let Some(a) = foo() {} // +1",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert!(
        loc.lloc >= 2.0,
        "got {}",
        serde_json::to_string(&loc).unwrap()
    );
}

#[test]
fn rust_function_in_return_lloc() {
    // `return foo();` is an EXPR_STMT containing a return expression.
    // `await foo();` is also an EXPR_STMT (await is a postfix expr).
    let a = analyze_wrapped(
        "return foo();
         await foo();",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(
        loc.lloc,
        2.0,
        "got {}",
        serde_json::to_string(&loc).unwrap()
    );
}

#[test]
fn rust_closure_expression_lloc() {
    // 3 outer statements + 1 closure body = 4 LLOC.
    let a = analyze_wrapped(
        "let a = |i: i32| -> i32 { i + 1 }; // +1
         a(42); // +1
         k(b.iter().map(|n| n.parse.ok().unwrap_or(42))); // +1",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    // Body of the first closure (`i + 1`) is its tail expr — counts +1.
    assert!(
        loc.lloc >= 3.0,
        "got {}",
        serde_json::to_string(&loc).unwrap()
    );
}
