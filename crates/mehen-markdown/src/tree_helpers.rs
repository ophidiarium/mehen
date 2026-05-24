// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Markdown tree walking helpers shared across the per-metric analyzers.
//!
//! The Markdown crate has many sibling analyzers (`mcc`, `mrpc`,
//! `halstead`, `embedded_code`, …) that all walk the same syntax tree
//! and need the same low-level extraction primitives: line spans,
//! link destinations, table-cell counts, prose-container checks, etc.
//! Without a shared module each analyzer was carrying byte-identical
//! helper copies; CPD flagged ~155 lines of duplication between `mcc.rs`
//! and `mrpc.rs` alone.
//!
//! This module is the consolidation point. Helpers here are
//! deliberately small, return owned values, and avoid borrowing from
//! the cursor — that keeps each call site free to use the helper
//! without holding a `TreeCursor` open across other work.

use crate::document::{MarkdownDocument, normalize_reference_label, unescape_markdown};
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

/// Find a link destination and resolve reference-style links through the
/// document's reference definitions. Inline links keep their literal payload;
/// `[text][id]`, `[id][]`, and shortcut `[id]` use the destination from
/// `[id]: ...` when one exists.
pub(crate) fn find_resolved_link_dest(
    node: &Node<'_>,
    source: &str,
    document: &MarkdownDocument,
) -> Option<String> {
    if let Some(dest) = find_link_dest(node, source) {
        if is_full_reference_link(node, source)
            && let Some(resolved) = reference_definition_destination(document, &dest)
        {
            return Some(resolved);
        }
        return Some(dest);
    }

    if is_inline_link(node, source) {
        return None;
    }

    find_link_label(node, source)
        .as_deref()
        .and_then(|label| reference_definition_destination(document, label))
}

fn reference_definition_destination(document: &MarkdownDocument, label: &str) -> Option<String> {
    let label = normalize_reference_label(&unescape_markdown(label));
    document
        .reference_definitions
        .iter()
        .find(|definition| definition.label == label)
        .map(|definition| definition.destination.clone())
}

fn is_inline_link(node: &Node<'_>, source: &str) -> bool {
    source
        .get(node.start_byte()..node.end_byte())
        .is_some_and(|text| text.contains("]("))
}

fn is_full_reference_link(node: &Node<'_>, source: &str) -> bool {
    let Some(text) = source.get(node.start_byte()..node.end_byte()) else {
        return false;
    };
    if text.contains("](") {
        return false;
    }
    let Some(close) = text.rfind(']') else {
        return false;
    };
    let before_close = &text[..close];
    let Some(open) = before_close.rfind('[') else {
        return false;
    };
    open > 0 && before_close[..open].ends_with(']')
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax_tree::{Node, Tree, parse_with_document};

    fn first_node<'a>(tree: &'a Tree, target: Markdown) -> Node<'a> {
        let mut stack = vec![tree.root()];
        while let Some(node) = stack.pop() {
            if node.kind_id() == target as u16 {
                return node;
            }
            let mut cursor = node.cursor();
            if cursor.goto_first_child() {
                loop {
                    stack.push(cursor.node());
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
        }
        panic!("missing {target:?} node");
    }

    #[test]
    fn escaped_reference_key_resolves_definition_destination() {
        let source = "[visible][ref\\]]\n\n[ref\\]]: https://example.com\n";
        let (tree, document) = parse_with_document(source);
        let link = first_node(&tree, Markdown::Link);

        assert_eq!(
            find_resolved_link_dest(&link, source, &document),
            Some("https://example.com".to_string())
        );
    }
}
