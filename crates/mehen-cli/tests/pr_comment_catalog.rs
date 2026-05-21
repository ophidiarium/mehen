// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Template-catalog linter covering §39.5.2 / §39.5.3 at test time.
//!
//! Complements `scripts/check_pr_template_catalog.sh`: both run the same
//! inspection against `src/diff_markdown.rs`. This test variant keeps the
//! safety-net inside the workspace's regular test suite so pushes that
//! slip past CI scripts are still rejected.

use std::path::PathBuf;

fn diff_markdown_source() -> String {
    // Walk up from `crates/mehen-cli/` to the workspace root.
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf();
    let path = workspace_root.join("crates/mehen-report/src/github_markdown_docs.rs");
    std::fs::read_to_string(&path)
        .expect("failed to read crates/mehen-report/src/github_markdown_docs.rs")
}

/// Each phrase from §39.5.3. Exact, verbatim match with the spec.
const FORBIDDEN_PHRASES: &[&str] = &[
    "because",
    "due to",
    "caused by",
    "following",
    "since",
    "likely",
    "probably",
    "appears to",
    "seems",
    "may indicate",
    "suggests",
    "possibly",
];

fn line_is_comment(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*')
}

/// Returns `Some(name)` if `line` declares a function, either at column 0
/// or inside an `impl` block (leading whitespace OK). Accepts `fn`, `pub fn`,
/// `pub(crate) fn`, `pub(super) fn`, and `async fn`.
fn parse_top_level_fn(line: &str) -> Option<String> {
    let mut rest = line.trim_start();
    for prefix in ["pub(crate) ", "pub(super) ", "pub ", ""] {
        if let Some(r) = rest.strip_prefix(prefix) {
            rest = r;
            break;
        }
    }
    if let Some(r) = rest.strip_prefix("async ") {
        rest = r;
    }
    rest = rest.strip_prefix("fn ")?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() { None } else { Some(name) }
}

#[test]
fn no_forbidden_phrases_in_emitter() {
    let src = diff_markdown_source();
    let mut hits: Vec<(usize, String, String)> = Vec::new();
    for (idx, line) in src.lines().enumerate() {
        if line_is_comment(line) {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        for phrase in FORBIDDEN_PHRASES {
            if lower.contains(phrase) {
                hits.push((idx + 1, phrase.to_string(), line.to_string()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "§39.5.3 forbidden phrases in emitter:\n{}",
        hits.iter()
            .map(|(n, p, l)| format!("  L{n}: `{p}` :: {l}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Allow-listed mechanical rendering helpers that legitimately call
/// `format!` / `write!`. Keep in sync with
/// `scripts/check_pr_template_catalog.sh`.
const ALLOWED_FUNCS: &[&str] = &[
    "render_doc_section",
    "render_drill_down",
    "render_drill_structural",
    "render_drill_en_wording",
    "render_drill_en_lexical",
    "render_drill_ja",
    "render_filler_contributors",
    "write_headline_table",
    "heading_scope",
    "format_int_thousands",
    "format_link_list",
    "format_surface_list_without_line",
    "format_value",
    "build_file_link",
    "render",
];

/// Strip the trailing content of a `//` line comment. String literals in
/// emitter code may contain `//` (URLs), but the linter's `line_is_comment`
/// already skips whole-line comments, and brace-depth tracking only needs
/// approximate accuracy.
fn strip_line_comment(line: &str) -> &str {
    if let Some(idx) = line.find("//") {
        &line[..idx]
    } else {
        line
    }
}

/// Count `{` minus `}` tokens on a single line (approximate, line-comment
/// stripped). Good enough for the emitter file's structure; the linter
/// doesn't need lexical-perfection, only "did we leave the `fn` body yet".
fn net_braces(line: &str) -> i32 {
    let trimmed = strip_line_comment(line);
    let open = trimmed.matches('{').count() as i32;
    let close = trimmed.matches('}').count() as i32;
    open - close
}

/// Shared per-line state update: advance brace depth, promote a function
/// signature into a body once the opening `{` appears, and clear
/// `current_fn` once the body closes.
fn advance_scope(
    line: &str,
    depth: &mut i32,
    current_fn: &mut Option<String>,
    fn_open_depth: &mut i32,
    inside_body: &mut bool,
) {
    *depth += net_braces(line);
    if current_fn.is_some() && !*inside_body && *depth > *fn_open_depth {
        *inside_body = true;
    }
    if current_fn.is_some() && *inside_body && *depth <= *fn_open_depth {
        *current_fn = None;
        *inside_body = false;
    }
}

#[test]
fn all_format_calls_live_inside_template_or_allow_list() {
    let src = diff_markdown_source();
    // Track the enclosing top-level function using brace depth so that code
    // between two `fn` definitions (e.g. a `const` initialized with
    // `format!`) is attributed to `(top-level)` rather than the preceding
    // function. `inside_body` flips to true only after we actually see the
    // opening `{`, so a multi-line signature doesn't clear the association
    // prematurely.
    let mut current_fn: Option<String> = None;
    let mut fn_open_depth: i32 = 0;
    let mut inside_body: bool = false;
    let mut depth: i32 = 0;
    let mut hits: Vec<(usize, String)> = Vec::new();

    for (idx, line) in src.lines().enumerate() {
        let line_no = idx + 1;
        // Top-level `fn` declarations start a new enclosing function.
        if let Some(name) = parse_top_level_fn(line) {
            current_fn = Some(name);
            fn_open_depth = depth;
            inside_body = false;
            advance_scope(
                line,
                &mut depth,
                &mut current_fn,
                &mut fn_open_depth,
                &mut inside_body,
            );
            continue;
        }
        if line_is_comment(line) {
            advance_scope(
                line,
                &mut depth,
                &mut current_fn,
                &mut fn_open_depth,
                &mut inside_body,
            );
            continue;
        }
        if line.contains("format!(") || line.contains("write!(") || line.contains("writeln!(") {
            let enclosing = current_fn.as_deref().unwrap_or("(top-level)");
            let permitted = enclosing.starts_with("tmpl_") || ALLOWED_FUNCS.contains(&enclosing);
            if !permitted {
                hits.push((line_no, format!("fn {enclosing}: {}", line.trim())));
            }
        }
        advance_scope(
            line,
            &mut depth,
            &mut current_fn,
            &mut fn_open_depth,
            &mut inside_body,
        );
    }
    assert!(
        hits.is_empty(),
        "format!/write! calls outside §39.5.2 catalog:\n{}",
        hits.iter()
            .map(|(n, l)| format!("  L{n}: {l}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
