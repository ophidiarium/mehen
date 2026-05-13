//! Template-catalog linter covering §39.5.2 / §39.5.3 at test time.
//!
//! Complements `scripts/check_pr_template_catalog.sh`: both run the same
//! inspection against `src/diff_markdown.rs`. This test variant keeps the
//! safety-net inside the workspace's regular test suite so pushes that
//! slip past CI scripts are still rejected.

use std::path::PathBuf;

fn diff_markdown_source() -> String {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = repo_root.join("src").join("diff_markdown.rs");
    std::fs::read_to_string(&path).expect("failed to read src/diff_markdown.rs")
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
    "print_doc_json",
    "evaluate_fail_on",
];

#[test]
fn all_format_calls_live_inside_template_or_allow_list() {
    let src = diff_markdown_source();
    let mut current_fn: Option<String> = None;
    let mut hits: Vec<(usize, String)> = Vec::new();

    for (idx, line) in src.lines().enumerate() {
        let line_no = idx + 1;
        // Track enclosing fn by string search. Accept both `fn NAME(` and
        // any of the visibility-prefixed forms (`pub fn`, `pub(crate) fn`,
        // etc.). Only top-level (column 0) declarations count as enclosing.
        if let Some(name) = parse_top_level_fn(line) {
            current_fn = Some(name);
            continue;
        }
        if line_is_comment(line) {
            continue;
        }
        if line.contains("format!(") || line.contains("write!(") || line.contains("writeln!(") {
            let enclosing = current_fn.as_deref().unwrap_or("(top-level)");
            if enclosing.starts_with("tmpl_") {
                continue;
            }
            if ALLOWED_FUNCS.contains(&enclosing) {
                continue;
            }
            hits.push((line_no, format!("fn {enclosing}: {}", line.trim())));
        }
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
