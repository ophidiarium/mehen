//! Per-fence code burden per §14.1.
//!
//! For each `fenced_code_block`, we compute:
//!
//! ```text
//! CodeFenceBurden(c) =
//!     1.0
//!   + 0.08 * max(0, LOC_c - 12)
//!   + 0.50 * sat(LOC_c; 40, 120)
//!   + 0.40 * sat(line_length_p95_c; 100, 180)
//!   + 1.50 * missing_language_tag
//!   + 0.00 * parser_error_if_language_supported    // Phase B wires this
//!   + 0.20 * code_cognitive_c                      // Phase B wires this
//!   + 0.05 * sqrt(code_halstead_volume_c)          // Phase B wires this
//! ```
//!
//! Phase C ships the shape-only terms (LOC, line length, missing tag). The
//! parser-error / cognitive / halstead terms stay zero until Phase B lands.

use crate::grammar::Markdown;
use crate::legacy_node::Node;
use crate::mathops::sat;
use crate::nearby::{BlockSpan, has_prose_within};

/// Per-fence summary used to populate ArtifactRecord rows and Phase D's
/// filler / grounding pipelines.
#[derive(Debug, Clone)]
pub(crate) struct CodeFence {
    pub(crate) start_line: u64,
    pub(crate) end_line: u64,
    pub(crate) language: Option<String>,
    pub(crate) loc: u64,
    pub(crate) has_language_tag: bool,
    pub(crate) has_nearby_prose: bool,
    pub(crate) burden: f64,
}

/// Walks the tree, records every `fenced_code_block`, and skips diagram
/// fences (they are owned by `visuals.rs`). Returns a deterministic list
/// sorted by start_line.
pub(crate) fn analyze_code_fences(
    root: &Node<'_>,
    source: &str,
    blocks: &[BlockSpan],
) -> Vec<CodeFence> {
    let mut out: Vec<CodeFence> = Vec::new();
    walk(root, source, blocks, &mut out);
    // Sort for determinism.
    out.sort_by_key(|a| a.start_line);
    out
}

fn walk(node: &Node<'_>, source: &str, blocks: &[BlockSpan], out: &mut Vec<CodeFence>) {
    let kind: Markdown = node.kind_id().into();
    if matches!(kind, Markdown::FencedCodeBlock)
        && let Some(rec) = build(node, source, blocks)
    {
        out.push(rec);
        return;
    }
    let mut cursor = node.cursor();
    if cursor.goto_first_child() {
        loop {
            walk(&cursor.node(), source, blocks, out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn build(node: &Node<'_>, source: &str, blocks: &[BlockSpan]) -> Option<CodeFence> {
    let start_line = (node.start_row() as u64) + 1;
    let (end_row, end_col) = node.end_position();
    let mut end = end_row;
    if end > node.start_row() && end_col == 0 {
        end -= 1;
    }
    let end_line = (end as u64) + 1;

    let language = find_first(node, Markdown::InfoString)
        .and_then(|n| find_first(&n, Markdown::Language))
        .map(|n| node_text(&n, source).trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty());
    let has_language_tag = language.is_some();

    // Diagrams are handled in `visuals.rs`; skip them here so the burden
    // score isn't counted twice. We still leave the record in the artifact
    // list (see `analyzer.rs`), but filter at the §14.1 site.
    if let Some(lang) = language.as_deref()
        && matches!(
            lang,
            "mermaid"
                | "plantuml"
                | "puml"
                | "dot"
                | "graphviz"
                | "d2"
                | "vega-lite"
                | "vegalite"
                | "vl"
                | "vega"
        )
    {
        return None;
    }

    // Body LOC (excludes fence delimiters): use CodeFenceContent span.
    let (body_loc, line_len_p95) = code_body_stats(node, source);

    let has_nearby_prose = has_prose_within(blocks, start_line, end_line, 2);

    let missing_language_tag = if has_language_tag { 0.0 } else { 1.0 };
    let burden = 1.0
        + 0.08 * (body_loc as f64 - 12.0).max(0.0)
        + 0.50 * sat(body_loc as f64, 40.0, 120.0)
        + 0.40 * sat(line_len_p95, 100.0, 180.0)
        + 1.50 * missing_language_tag;

    Some(CodeFence {
        start_line,
        end_line,
        language,
        loc: body_loc,
        has_language_tag,
        has_nearby_prose,
        burden,
    })
}

fn code_body_stats(node: &Node<'_>, source: &str) -> (u64, f64) {
    let Some(content) = find_first(node, Markdown::CodeFenceContent) else {
        return (0, 0.0);
    };
    let start = content.start_byte();
    let end = content.end_byte();
    let body = &source[start..end];
    let mut line_lengths: Vec<usize> = Vec::new();
    for line in body.lines() {
        line_lengths.push(line.chars().count());
    }
    let loc = line_lengths.len() as u64;
    if line_lengths.is_empty() {
        return (loc, 0.0);
    }
    // p95 = length at index ceil(0.95 * N) - 1.
    line_lengths.sort_unstable();
    let idx = ((0.95 * line_lengths.len() as f64).ceil() as usize)
        .saturating_sub(1)
        .min(line_lengths.len() - 1);
    (loc, line_lengths[idx] as f64)
}

fn find_first<'a>(node: &Node<'a>, target: Markdown) -> Option<Node<'a>> {
    let mut cursor = node.cursor();
    if !cursor.goto_first_child() {
        return None;
    }
    loop {
        let child = cursor.node();
        let kind: Markdown = child.kind_id().into();
        if kind == target {
            return Some(child);
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    None
}

fn node_text(node: &Node<'_>, source: &str) -> String {
    let start = node.start_byte();
    let end = node.end_byte();
    String::from_utf8_lossy(&source.as_bytes()[start..end]).into_owned()
}
