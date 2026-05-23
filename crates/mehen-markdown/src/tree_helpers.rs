// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Markdown tree walking helpers shared across the per-metric analyzers.
//!
//! The Markdown crate has many sibling analyzers (`mcc`, `mrpc`,
//! `halstead`, `embedded_code`, …) that all walk the same syntax tree
//! and need the same low-level extraction primitives: line spans,
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
use crate::syntax_tree::Node;

#[derive(Clone, Copy, Debug)]
pub(crate) struct ProseContext {
    include_heading_content: bool,
    include_heading_blocks: bool,
    include_link_labels: bool,
    include_tables: bool,
}

impl ProseContext {
    pub(crate) const BODY: Self = Self {
        include_heading_content: false,
        include_heading_blocks: false,
        include_link_labels: true,
        include_tables: true,
    };

    pub(crate) const BODY_AND_HEADING_TEXT: Self = Self {
        include_heading_content: true,
        include_heading_blocks: false,
        include_link_labels: true,
        include_tables: true,
    };

    pub(crate) const PLACEHOLDER_TEXT: Self = Self {
        include_heading_content: true,
        include_heading_blocks: false,
        include_link_labels: false,
        include_tables: false,
    };

    pub(crate) const SECTION_TEXT: Self = Self {
        include_heading_content: true,
        include_heading_blocks: true,
        include_link_labels: true,
        include_tables: true,
    };
}

/// Containers whose descendants are structural or machine-readable rather than
/// narrative prose for Markdown metric walks.
pub(crate) fn is_non_prose_container(kind: Markdown) -> bool {
    use Markdown::*;
    matches!(
        kind,
        FencedCodeBlock
            | IndentedCodeBlock
            | InlineCode
            | CodeFenceContent
            | InlineCodeContent
            | InlineCodeContent2
            | InfoString
            | Language
            | MathBlock
            | MathInline
            | MathBlockContent
            | MathInlineContent
            | HtmlBlock
            | HtmlBlock1
            | HtmlBlock3
            | HtmlBlock4
            | HtmlBlock5
            | HtmlBlock6
            | HtmlBlock7
            | HtmlCommentBlock
            | HtmlInline
            | HtmlComment
            | HtmlCdata
            | HtmlDeclaration
            | HtmlProcessingInstruction
            | HtmlOpenTag
            | HtmlCloseTag
            | MdxJsxBlock
            | MdxJsxInline
            | MdxJsxOpenTag
            | MdxJsxOpenTag2
            | MdxJsxCloseTag
            | MdxJsxCloseTag2
            | MdxJsxExpression
            | DirectiveBlock
            | Autolink
            | Uri
            | Email
            | LinkDestination
            | LinkDestinationParenthesis
            | LinkTitle
            | TextNoAngle
            | MinusMetadata
            | PlusMetadata
            | PipeTableDelimiterRow
            | PipeTableDelimiterCell
            | PipeTableAlignLeft
            | PipeTableAlignRight
    )
}

pub(crate) fn opens_prose_context(kind: Markdown, context: ProseContext) -> bool {
    use Markdown::*;
    matches!(
        kind,
        Paragraph
            | BlockQuote
            | PlainBlockQuote
            | Callout
            | CalloutHeaderParagraph
            | ListItemContent
            | TaskListItemContent
    ) || (context.include_heading_content && matches!(kind, AtxHeadingContent))
        || (context.include_heading_blocks
            && matches!(
                kind,
                AtxHeading
                    | AtxHeading2
                    | AtxHeading3
                    | AtxHeading4
                    | AtxHeading5
                    | AtxHeading6
                    | SetextHeading
                    | SetextHeading2
            ))
        || (context.include_link_labels && matches!(kind, LinkLabel | FootnoteLabel))
        || (context.include_tables
            && matches!(kind, PipeTableCell | PipeTableHeader | PipeTableRow))
}

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

#[derive(Clone, Copy, Debug)]
struct ContentBounds {
    start_byte: usize,
    end_byte: usize,
    start_row: usize,
    end_row: usize,
    end_col: usize,
}

fn code_content_bounds(node: &Node<'_>) -> Option<ContentBounds> {
    let target = match node.kind_id().into() {
        Markdown::FencedCodeBlock => Markdown::CodeFenceContent,
        Markdown::IndentedCodeBlock => Markdown::IndentedChunk,
        _ => return None,
    };

    let mut cursor = node.cursor();
    if !cursor.goto_first_child() {
        return None;
    }

    let mut bounds: Option<ContentBounds> = None;
    loop {
        let child = cursor.node();
        let child_kind: Markdown = child.kind_id().into();
        if child_kind == target {
            let (end_row, end_col) = child.end_position();
            bounds = Some(match bounds {
                Some(mut existing) => {
                    existing.end_byte = child.end_byte();
                    existing.end_row = end_row;
                    existing.end_col = end_col;
                    existing
                }
                None => ContentBounds {
                    start_byte: child.start_byte(),
                    end_byte: child.end_byte(),
                    start_row: child.start_row(),
                    end_row,
                    end_col,
                },
            });
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    bounds
}

/// Line count of the *content* inside a fenced code block — i.e. the
/// span of the inner `code_fence_content` child, ignoring the opening
/// and closing fence markers.
///
/// `indented_code_block` has no delimiters, so its `_indented_chunk`
/// children are treated as the same content span.
pub(crate) fn fence_content_line_count(node: &Node<'_>) -> usize {
    let Some(bounds) = code_content_bounds(node) else {
        return 0;
    };
    let mut end = bounds.end_row;
    if end > bounds.start_row && bounds.end_col == 0 {
        end -= 1;
    }
    end.saturating_sub(bounds.start_row) + 1
}

/// Source text inside a fenced or indented code block, with CRLF/LF normalized
/// to LF so semantic metrics do not drift by checkout platform.
pub(crate) fn fence_content_text(node: &Node<'_>, source: &str) -> Option<String> {
    let bounds = code_content_bounds(node)?;
    let bytes = source.as_bytes();
    if bounds.end_byte > bytes.len() || bounds.start_byte > bounds.end_byte {
        return None;
    }
    let text = String::from_utf8_lossy(&bytes[bounds.start_byte..bounds.end_byte]);
    Some(normalize_line_endings(&text))
}

fn normalize_line_endings(text: &str) -> String {
    if !text.as_bytes().contains(&b'\r') {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' {
            if matches!(chars.peek(), Some('\n')) {
                chars.next();
            }
            out.push('\n');
        } else {
            out.push(ch);
        }
    }
    out
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
            let bytes = source.as_bytes();
            let start = child.start_byte();
            let end = child.end_byte();
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
            return None;
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
/// resilient to a syntax node stepping into an invalid byte
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
