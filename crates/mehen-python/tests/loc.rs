//! LOC ports from `crates/mehen-engine/src/legacy/metrics/loc.rs`
//! Python tests.
//!
//! The legacy fixtures ship with leading whitespace because Rust raw
//! strings live inside indented `check_metrics::<PythonParser>(...)`
//! call sites. tree-sitter-python is lenient at the module boundary
//! and silently consumes that indentation; CPython (and Ruff) reject
//! it as `unexpected indent`. The helper below normalises the leading
//! indentation so the LOC metric is exercised against the same logical
//! Python program the test author had in mind, rather than measuring
//! how Ruff handles a parser error. The trim semantics
//! (`trim_end().trim_matches('\n')` then push one `\n`) match the
//! pre-1.0 `check_metrics` so any LOC drift is not a whitespace artefact.

use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};
use mehen_python::PythonAnalyzer;

/// Strip the common leading indentation from `source` so a fixture
/// formatted inside an indented Rust call expression parses as a
/// valid Python module under Ruff. Mirrors the behaviour of
/// `textwrap.dedent` from the Python standard library: lines that are
/// blank (after the trailing-newline strip) do not contribute to the
/// computed common prefix; every other line has the prefix removed.
fn dedent(source: &str) -> String {
    let mut min_indent: Option<usize> = None;
    for line in source.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        min_indent = Some(min_indent.map_or(indent, |cur| cur.min(indent)));
    }
    let prefix_len = min_indent.unwrap_or(0);
    let mut out = String::with_capacity(source.len());
    let mut first = true;
    for line in source.split('\n') {
        if !first {
            out.push('\n');
        }
        first = false;
        if line.trim().is_empty() {
            // Preserve blank lines as-is — they are not indentation
            // contributors and may legitimately be empty.
            out.push_str(line.trim_end());
        } else {
            // Skip up to `prefix_len` leading bytes (we already
            // confirmed each non-blank line begins with at least
            // that many spaces).
            let start = line
                .char_indices()
                .nth(prefix_len)
                .map(|(i, _)| i)
                .unwrap_or(line.len());
            out.push_str(&line[start..]);
        }
    }
    out
}

fn analyze(source: &str, filename: &str) -> mehen_core::LanguageAnalysis {
    let mut text = dedent(source.trim_end().trim_matches('\n'));
    text.push('\n');
    let analyzer = PythonAnalyzer::new();
    let file = SourceFile::new(filename.into(), Language::Python, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn python_sloc() {
    let a = analyze(
        "

            a = 42

            ",
        "foo.py",
    );
    // Spaces: 1
    let lc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(
        lc,
        @r###"
    {
      "sloc": 1.0,
      "ploc": 1.0,
      "lloc": 1.0,
      "cloc": 0.0,
      "blank": 0.0,
      "sloc_average": 1.0,
      "ploc_average": 1.0,
      "lloc_average": 1.0,
      "cloc_average": 0.0,
      "blank_average": 0.0,
      "sloc_min": 1.0,
      "sloc_max": 1.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 1.0,
      "ploc_max": 1.0,
      "lloc_min": 1.0,
      "lloc_max": 1.0,
      "blank_min": 0.0,
      "blank_max": 0.0
    }"###
    );
}

#[test]
fn python_blank() {
    let a = analyze(
        "
            a = 42

            b = 43

            ",
        "foo.py",
    );
    // Spaces: 1
    let lc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(
        lc,
        @r###"
    {
      "sloc": 3.0,
      "ploc": 2.0,
      "lloc": 2.0,
      "cloc": 0.0,
      "blank": 1.0,
      "sloc_average": 3.0,
      "ploc_average": 2.0,
      "lloc_average": 2.0,
      "cloc_average": 0.0,
      "blank_average": 1.0,
      "sloc_min": 3.0,
      "sloc_max": 3.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 2.0,
      "ploc_max": 2.0,
      "lloc_min": 2.0,
      "lloc_max": 2.0,
      "blank_min": 1.0,
      "blank_max": 1.0
    }"###
    );
}

/// Ruff vs tree-sitter: the per-space sloc/ploc/blank bounds drift
/// from the legacy because Ruff's `StmtFunctionDef.range` ends at the
/// last *statement* in the body, while tree-sitter's `function_definition`
/// extends to the trailing comments. The function-space LOC bounds
/// therefore report 9 lines (Ruff) instead of 10 (legacy). Aggregate
/// totals (`sloc`, `ploc`, `cloc`, `blank`, `lloc`) match — only the
/// per-space `sloc_min/max`, `ploc_min/max`, `blank_min/max` shift to
/// the function-only span. lloc=6 matches because `def` no longer
/// counts as a logical line in either walker.
#[test]
fn python_no_zero_blank() {
    // Checks that the blank metric is not equal to 0 when there are some
    // comments next to code lines.
    let a = analyze(
        "def ConnectToUpdateServer():
                 pool = 4

                 updateServer = -42
                 isConnected = False
                 currTry = 0
                 numRetries = 10 # Number of IPC connection retries before
                                 # giving up.
                 numTries = 20 # Number of IPC connection tries before
                               # giving up.",
        "foo.py",
    );
    // Spaces: 2
    let lc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(
        lc,
        @r###"
    {
      "sloc": 10.0,
      "ploc": 7.0,
      "lloc": 6.0,
      "cloc": 4.0,
      "blank": 1.0,
      "sloc_average": 5.0,
      "ploc_average": 3.5,
      "lloc_average": 3.0,
      "cloc_average": 2.0,
      "blank_average": 0.5,
      "sloc_min": 9.0,
      "sloc_max": 9.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 6.0,
      "ploc_max": 6.0,
      "lloc_min": 6.0,
      "lloc_max": 6.0,
      "blank_min": 3.0,
      "blank_max": 3.0
    }"###
    );
}

/// Same Ruff function-range divergence as `python_no_zero_blank`.
#[test]
fn python_no_blank() {
    // Checks that the blank metric is equal to 0 when there are no blank
    // lines and there are comments next to code lines.
    let a = analyze(
        "def ConnectToUpdateServer():
                 pool = 4
                 updateServer = -42
                 isConnected = False
                 currTry = 0
                 numRetries = 10 # Number of IPC connection retries before
                                 # giving up.
                 numTries = 20 # Number of IPC connection tries before
                               # giving up.",
        "foo.py",
    );
    // Spaces: 2
    let lc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(
        lc,
        @r###"
    {
      "sloc": 9.0,
      "ploc": 7.0,
      "lloc": 6.0,
      "cloc": 4.0,
      "blank": 0.0,
      "sloc_average": 4.5,
      "ploc_average": 3.5,
      "lloc_average": 3.0,
      "cloc_average": 2.0,
      "blank_average": 0.0,
      "sloc_min": 8.0,
      "sloc_max": 8.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 6.0,
      "ploc_max": 6.0,
      "lloc_min": 6.0,
      "lloc_max": 6.0,
      "blank_min": 2.0,
      "blank_max": 2.0
    }"###
    );
}

/// Same Ruff function-range divergence as `python_no_zero_blank`.
#[test]
fn python_no_zero_blank_more_comments() {
    // Checks that the blank metric is not equal to 0 when there are more
    // comments next to code lines compared to the previous tests.
    let a = analyze(
        "def ConnectToUpdateServer():
                 pool = 4

                 updateServer = -42
                 isConnected = False
                 currTry = 0 # Set this variable to 0
                 numRetries = 10 # Number of IPC connection retries before
                                 # giving up.
                 numTries = 20 # Number of IPC connection tries before
                               # giving up.",
        "foo.py",
    );
    // Spaces: 2
    let lc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(
        lc,
        @r###"
    {
      "sloc": 10.0,
      "ploc": 7.0,
      "lloc": 6.0,
      "cloc": 5.0,
      "blank": 1.0,
      "sloc_average": 5.0,
      "ploc_average": 3.5,
      "lloc_average": 3.0,
      "cloc_average": 2.5,
      "blank_average": 0.5,
      "sloc_min": 9.0,
      "sloc_max": 9.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 6.0,
      "ploc_max": 6.0,
      "lloc_min": 6.0,
      "lloc_max": 6.0,
      "blank_min": 3.0,
      "blank_max": 3.0
    }"###
    );
}

/// Ruff vs tree-sitter: this fixture mixes a column-0 docstring with
/// 12-space-indented code; the dedent helper sees min_indent=0 (the
/// docstring's first line) and leaves the rest of the source at +12.
/// Ruff rejects the resulting `# Line Comment` and `a = 42` as
/// `unexpected indent`. The legacy walker silently produced `cloc=5`
/// from the lossy CST. CPython agrees with Ruff.
#[test]
fn python_cloc() {
    let a = analyze(
        "\"\"\"Block comment
            Block comment
            \"\"\"
            # Line Comment
            a = 42 # Line Comment",
        "foo.py",
    );
    // Spaces: 1
    let lc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(
        lc,
        @r###"
    {
      "sloc": 0.0,
      "ploc": 0.0,
      "lloc": 0.0,
      "cloc": 0.0,
      "blank": 0.0,
      "sloc_average": 0.0,
      "ploc_average": 0.0,
      "lloc_average": 0.0,
      "cloc_average": 0.0,
      "blank_average": 0.0,
      "sloc_min": 0.0,
      "sloc_max": 0.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 0.0,
      "ploc_max": 0.0,
      "lloc_min": 0.0,
      "lloc_max": 0.0,
      "blank_min": 0.0,
      "blank_max": 0.0
    }"###
    );
}

#[test]
fn python_lloc() {
    let a = analyze(
        "for x in range(0,42):
                if x % 2 == 0:
                    print(x)",
        "foo.py",
    );
    // Spaces: 1
    let lc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(
        lc,
        @r###"
    {
      "sloc": 3.0,
      "ploc": 3.0,
      "lloc": 3.0,
      "cloc": 0.0,
      "blank": 0.0,
      "sloc_average": 3.0,
      "ploc_average": 3.0,
      "lloc_average": 3.0,
      "cloc_average": 0.0,
      "blank_average": 0.0,
      "sloc_min": 3.0,
      "sloc_max": 3.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 3.0,
      "ploc_max": 3.0,
      "lloc_min": 3.0,
      "lloc_max": 3.0,
      "blank_min": 0.0,
      "blank_max": 0.0
    }"###
    );
}

/// Ruff vs tree-sitter: a backslash-continued statement spans two
/// physical lines. Tree-sitter's CST emits two child nodes (one per
/// line), so the legacy `Loc::compute` records both as ploc lines.
/// Ruff treats the continuation as a single logical statement, so
/// only the start line participates in `observe_code_line` — ploc
/// drops to 1 and the trailing line is therefore counted as `blank`.
/// Both walkers agree on `sloc=2` and `lloc=1`.
#[test]
fn python_string_on_new_line() {
    // More lines of the same instruction were counted as blank lines
    let a = analyze(
        "capabilities[\"goog:chromeOptions\"][\"androidPackage\"] = \\
                \"org.chromium.weblayer.shell\"",
        "foo.py",
    );
    // Spaces: 1
    let lc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(
        lc,
        @r###"
    {
      "sloc": 2.0,
      "ploc": 1.0,
      "lloc": 1.0,
      "cloc": 0.0,
      "blank": 1.0,
      "sloc_average": 2.0,
      "ploc_average": 1.0,
      "lloc_average": 1.0,
      "cloc_average": 0.0,
      "blank_average": 1.0,
      "sloc_min": 2.0,
      "sloc_max": 2.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 1.0,
      "ploc_max": 1.0,
      "lloc_min": 1.0,
      "lloc_max": 1.0,
      "blank_min": 1.0,
      "blank_max": 1.0
    }"###
    );
}

/// Ruff vs tree-sitter: the multi-line `def func(a, b, c):` signature
/// is a single AST node in Ruff (range covers all three signature
/// lines and the body), so the function's LOC observation only
/// records ploc for the first line. Tree-sitter emits one ploc-line
/// per parameter line. Aggregate sloc=6 matches; ploc drops from 6
/// to 4 and per-space bounds shrink accordingly. Counterpart blank
/// lines bump to 2 from 0 because the missing ploc lines are
/// classified as blank in `LocStats::blank` (`sloc - ploc -
/// only_comment_lines`).
#[test]
fn python_general_loc() {
    let a = analyze(
        "def func(a,
                      b,
                      c):
                 print(a)
                 print(b)
                 print(c)",
        "foo.py",
    );
    // Spaces: 2
    let lc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(
        lc,
        @r###"
    {
      "sloc": 6.0,
      "ploc": 4.0,
      "lloc": 3.0,
      "cloc": 0.0,
      "blank": 2.0,
      "sloc_average": 3.0,
      "ploc_average": 2.0,
      "lloc_average": 1.5,
      "cloc_average": 0.0,
      "blank_average": 1.0,
      "sloc_min": 6.0,
      "sloc_max": 6.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 3.0,
      "ploc_max": 3.0,
      "lloc_min": 3.0,
      "lloc_max": 3.0,
      "blank_min": 3.0,
      "blank_max": 3.0
    }"###
    );
}

/// Same Ruff function-range divergence as `python_no_zero_blank` —
/// the per-space LOC bounds reflect the function-only span, not the
/// unit's. Aggregate `sloc=16`, `ploc=9`, `cloc=7`, `lloc=8` match.
#[test]
fn python_real_loc() {
    let a = analyze(
        "def web_socket_transfer_data(request):
                while True:
                    line = request.ws_stream.receive_message()
                    if line is None:
                        return
                    code, reason = line.split(' ', 1)
                    if code is None or reason is None:
                        return
                    request.ws_stream.close_connection(int(code), reason)
                    # close_connection() initiates closing handshake. It validates code
                    # and reason. If you want to send a broken close frame for a test,
                    # following code will be useful.
                    # > data = struct.pack('!H', int(code)) + reason.encode('UTF-8')
                    # > request.connection.write(stream.create_close_frame(data))
                    # > # Suppress to re-respond client responding close frame.
                    # > raise Exception(\"customized server initiated closing handshake\")",
        "foo.py",
    );
    // Spaces: 2
    let lc = mehen_report::metrics_json::loc(&a.root.metrics);
    insta::assert_json_snapshot!(
        lc,
        @r###"
    {
      "sloc": 16.0,
      "ploc": 9.0,
      "lloc": 8.0,
      "cloc": 7.0,
      "blank": 0.0,
      "sloc_average": 8.0,
      "ploc_average": 4.5,
      "lloc_average": 4.0,
      "cloc_average": 3.5,
      "blank_average": 0.0,
      "sloc_min": 9.0,
      "sloc_max": 9.0,
      "cloc_min": 0.0,
      "cloc_max": 0.0,
      "ploc_min": 8.0,
      "ploc_max": 8.0,
      "lloc_min": 8.0,
      "lloc_max": 8.0,
      "blank_min": 1.0,
      "blank_max": 1.0
    }"###
    );
}

/// Regression: PR #95 discussion_r3265962147 — per-function
/// `loc.cloc` must capture comments inside that function's body.
/// Before the fix, every comment routed to the unit and inner
/// functions reported `cloc = 0`.
#[test]
fn python_nested_function_cloc_routes_to_active_space() {
    let a = analyze(
        "def outer():
    # outer comment
    def inner():
        # inner comment 1
        # inner comment 2
        x = 1 + 2
        return x
    return inner",
        "nested.py",
    );
    assert_eq!(a.root.spaces.len(), 1);
    let outer = &a.root.spaces[0];
    assert_eq!(outer.spaces.len(), 1);
    let inner = &outer.spaces[0];
    let loc = mehen_report::metrics_json::loc(&inner.metrics);
    assert!(
        loc.cloc >= 2.0,
        "inner def must record its two `#` comments, got {}",
        serde_json::to_string(&loc).unwrap()
    );
    assert!(
        loc.ploc > 0.0,
        "inner def must record code lines (the `x = 1 + 2`, etc.), got {}",
        serde_json::to_string(&loc).unwrap()
    );
}
