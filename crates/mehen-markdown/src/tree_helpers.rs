// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Tree-sitter walking helpers shared across the per-metric analyzers.
//!
//! The Markdown crate has many sibling analyzers (`mcc`, `mrpc`,
//! `halstead`, `embedded_code`, …) that all walk the same tree-sitter
//! CST and need the same low-level extraction primitives: line spans,
//! fence-content line counts, fenced-code language tags, link
//! destinations, table-cell counts, etc. Without a shared module each
//! analyzer was carrying byte-identical helper copies; CPD flagged
//! ~155 lines of duplication between `mcc.rs` and `mrpc.rs` alone.
//!
//! This module is the consolidation point. Helpers here are
//! deliberately small, return owned values, and avoid borrowing from
//! the cursor — that keeps each call site free to use the helper
//! without holding a `TreeCursor` open across other work.

use crate::grammar::Markdown;
use crate::legacy_node::Node;

/// Number of source lines a node covers, with the same end-of-line
/// convention every Markdown analyzer wants:
///
/// - When the node ends at column 0 of a row past its start, the row
///   it ends *on* belongs to the next block — strip it.
/// - Otherwise the end row is the last row containing node content.
///
/// Returns at least `1` for any non-empty node.
pub(crate) fn node_line_span(node: &Node<'_>) -> usize {
    let start = node.start_row();
    let (end_row, end_col) = node.end_position();
    let mut end = end_row;
    if end > start && end_col == 0 {
        end -= 1;
    }
    end.saturating_sub(start) + 1
}

/// Line count of the *content* inside a fenced code block — i.e. the
/// span of the inner `code_fence_content` child, ignoring the opening
/// and closing fence markers.
///
/// `indented_code_block` has no delimiters, so the whole node IS
/// content; we fall back to [`node_line_span`] for that case.
pub(crate) fn fence_content_line_count(node: &Node<'_>) -> usize {
    let kind: Markdown = node.kind_id().into();
    if matches!(kind, Markdown::IndentedCodeBlock) {
        return node_line_span(node);
    }
    let mut cursor = node.cursor();
    if !cursor.goto_first_child() {
        return 0;
    }
    loop {
        let child = cursor.node();
        if matches!(child.kind_id().into(), Markdown::CodeFenceContent) {
            return node_line_span(&child);
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    0
}

/// Read the language tag from a fenced code block's `info_string`
/// (e.g. ```` ```rust ```` → `Some("rust")`). Returns `None` for an
/// indented code block, an empty info string, or any structural
/// failure walking into the `info_string > language` chain.
///
/// `lowercase` controls case folding. Most analyzers normalize to
/// lowercase so they can match `Rust`, `rust`, `RUST` interchangeably;
/// the embedded-code dispatcher keeps the original case so extension-
/// based `FenceLanguage` resolution still has the original text.
pub(crate) fn fence_language_tag(node: &Node<'_>, source: &str, lowercase: bool) -> Option<String> {
    let mut cursor = node.cursor();
    if !cursor.goto_first_child() {
        return None;
    }
    loop {
        let child = cursor.node();
        if matches!(child.kind_id().into(), Markdown::InfoString) {
            let mut c2 = child.cursor();
            if c2.goto_first_child() {
                loop {
                    let inner = c2.node();
                    if matches!(inner.kind_id().into(), Markdown::Language) {
                        let bytes = source.as_bytes();
                        let start = inner.start_byte();
                        let end = inner.end_byte();
                        if end <= bytes.len() && start < end {
                            let raw = std::str::from_utf8(&bytes[start..end]).ok()?.trim();
                            if !raw.is_empty() {
                                return Some(if lowercase {
                                    raw.to_ascii_lowercase()
                                } else {
                                    raw.to_string()
                                });
                            }
                        }
                    }
                    if !c2.goto_next_sibling() {
                        break;
                    }
                }
            }
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    None
}

/// Total number of table cells (header + body rows) below `node`.
/// Walks the immediate children for `pipe_table_header` /
/// `pipe_table_row` and counts each `pipe_table_cell` inside.
pub(crate) fn count_table_cells(node: &Node<'_>) -> usize {
    let mut total = 0usize;
    let mut cursor = node.cursor();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if matches!(
                child.kind_id().into(),
                Markdown::PipeTableHeader | Markdown::PipeTableRow
            ) {
                let mut c2 = child.cursor();
                if c2.goto_first_child() {
                    loop {
                        if matches!(c2.node().kind_id().into(), Markdown::PipeTableCell) {
                            total += 1;
                        }
                        if !c2.goto_next_sibling() {
                            break;
                        }
                    }
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    total
}

/// Find a `link_destination` (or parenthesized variant) inside `node`
/// and return the URL with the surrounding `<>` stripped. The walk is
/// a stack-based DFS — depth-first finding the first matching node so
/// nested brackets in destinations don't confuse the search.
pub(crate) fn find_link_dest(node: &Node<'_>, source: &str) -> Option<String> {
    let mut stack = vec![*node];
    while let Some(n) = stack.pop() {
        if matches!(
            n.kind_id().into(),
            Markdown::LinkDestination | Markdown::LinkDestinationParenthesis
        ) {
            let bytes = source.as_bytes();
            let start = n.start_byte();
            let end = n.end_byte();
            if end <= bytes.len() && start < end {
                let text = std::str::from_utf8(&bytes[start..end]).ok()?.trim();
                return Some(
                    text.trim_start_matches('<')
                        .trim_end_matches('>')
                        .to_string(),
                );
            }
        }
        let mut cursor = n.cursor();
        if cursor.goto_first_child() {
            loop {
                stack.push(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
    None
}

/// Find a `link_label` inside `node` and return its raw text.
pub(crate) fn find_link_label(node: &Node<'_>, source: &str) -> Option<String> {
    let mut stack = vec![*node];
    while let Some(n) = stack.pop() {
        if matches!(n.kind_id().into(), Markdown::LinkLabel) {
            let bytes = source.as_bytes();
            let start = n.start_byte();
            let end = n.end_byte();
            if end <= bytes.len() && start < end {
                return std::str::from_utf8(&bytes[start..end])
                    .ok()
                    .map(|s| s.to_string());
            }
        }
        let mut cursor = n.cursor();
        if cursor.goto_first_child() {
            loop {
                stack.push(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
    None
}

/// Find the first direct child whose kind matches `target`. Returns
/// `None` when the node has no children or no matching child. Common
/// shape across `code_burden`, `visuals`, etc. for plucking a single
/// known child (e.g. the `link_destination` inside an `inline_link`).
pub(crate) fn find_first<'a>(node: &Node<'a>, target: Markdown) -> Option<Node<'a>> {
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

/// Slice the source bytes covered by `node` as a UTF-8 lossy `String`.
/// Lossy conversion (rather than `from_utf8`) keeps every analyzer
/// resilient to a tree-sitter cursor stepping into an invalid byte
/// run inside an HTML-block / MDX raw segment.
pub(crate) fn node_text(node: &Node<'_>, source: &str) -> String {
    let start = node.start_byte();
    let end = node.end_byte();
    String::from_utf8_lossy(&source.as_bytes()[start..end]).into_owned()
}

/// Whether a link destination has an explicit URL scheme.
///
/// Matches RFC 3986 scheme grammar: an ALPHA followed by zero or more
/// of ALPHA / DIGIT / `+` / `-` / `.`, terminated by `:`. Returns
/// `false` for relative paths, anchors, fragments, and bare paths.
pub(crate) fn has_scheme(dest: &str) -> bool {
    if let Some(colon) = dest.find(':') {
        let scheme = &dest[..colon];
        let chars: Vec<char> = scheme.chars().collect();
        if chars.is_empty() {
            return false;
        }
        if !chars[0].is_ascii_alphabetic() {
            return false;
        }
        for c in &chars[1..] {
            if !(c.is_ascii_alphanumeric() || *c == '+' || *c == '-' || *c == '.') {
                return false;
            }
        }
        return true;
    }
    false
}
