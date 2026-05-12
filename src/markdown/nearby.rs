//! Block-level neighborhood lookups shared by tables, visuals, and code.
//!
//! Several §§11–14 formulas ask whether a prose block explains or
//! introduces a nearby artifact. "Nearby" here is the §19 convention:
//! within ±2 top-level blocks (i.e. two blocks before or after the artifact
//! in the document order). This module walks the AST once, collects a
//! flat list of block rows, and exposes a helper that tells you whether a
//! given artifact line range has a prose block ±2 positions away.

use crate::languages::Markdown;
use crate::node::Node;

/// A flattened block descriptor. One entry per top-level block in the
/// document. Lines are one-based and inclusive.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BlockSpan {
    pub(crate) start_line: u64,
    pub(crate) end_line: u64,
    pub(crate) is_prose: bool,
}

/// Collects every top-level block in the document, preserving order. A
/// "top-level block" is a direct child of a `section`, `document`, or
/// generic `block` container. We deliberately do not descend into
/// artifacts — they cover exactly the same line range and we need the
/// prose blocks around them, not inside them.
pub(crate) fn collect_blocks(root: &Node<'_>) -> Vec<BlockSpan> {
    let mut out: Vec<BlockSpan> = Vec::new();
    walk(root, &mut out);
    out
}

fn walk(node: &Node<'_>, out: &mut Vec<BlockSpan>) {
    let kind: Markdown = node.kind_id().into();

    if is_block_like(&kind) {
        let start = (node.start_row() as u64) + 1;
        let (end_row, end_col) = node.end_position();
        let mut end = end_row;
        if end > node.start_row() && end_col == 0 {
            end -= 1;
        }
        let end_line = (end as u64) + 1;
        out.push(BlockSpan {
            start_line: start,
            end_line,
            is_prose: is_prose_block(&kind),
        });
        return;
    }

    let mut cursor = node.cursor();
    if cursor.goto_first_child() {
        loop {
            walk(&cursor.node(), out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn is_block_like(kind: &Markdown) -> bool {
    matches!(
        kind,
        Markdown::Paragraph
            | Markdown::FencedCodeBlock
            | Markdown::IndentedCodeBlock
            | Markdown::HtmlBlock
            | Markdown::HtmlBlock1
            | Markdown::HtmlBlock3
            | Markdown::HtmlBlock4
            | Markdown::HtmlBlock5
            | Markdown::HtmlBlock6
            | Markdown::HtmlBlock7
            | Markdown::HtmlCommentBlock
            | Markdown::MdxJsxBlock
            | Markdown::MathBlock
            | Markdown::DirectiveBlock
            | Markdown::ImageBlock
            | Markdown::PipeTable
            | Markdown::BlockQuote
            | Markdown::PlainBlockQuote
            | Markdown::Callout
            | Markdown::List
            | Markdown::ThematicBreak
            | Markdown::ThematicBreak2
            | Markdown::FootnoteDefinition
            | Markdown::LinkReferenceDefinition
            | Markdown::AtxHeading
            | Markdown::AtxHeading2
            | Markdown::AtxHeading3
            | Markdown::AtxHeading4
            | Markdown::AtxHeading5
            | Markdown::AtxHeading6
            | Markdown::SetextHeading
            | Markdown::SetextHeading2
    )
}

fn is_prose_block(kind: &Markdown) -> bool {
    matches!(
        kind,
        Markdown::Paragraph
            | Markdown::BlockQuote
            | Markdown::PlainBlockQuote
            | Markdown::Callout
            | Markdown::List
            | Markdown::AtxHeading
            | Markdown::AtxHeading2
            | Markdown::AtxHeading3
            | Markdown::AtxHeading4
            | Markdown::AtxHeading5
            | Markdown::AtxHeading6
            | Markdown::SetextHeading
            | Markdown::SetextHeading2
    )
}

/// True when the artifact spanning `[start_line, end_line]` has a prose
/// block within ±`radius` positions in the block-order index.
pub(crate) fn has_prose_within(
    blocks: &[BlockSpan],
    start_line: u64,
    end_line: u64,
    radius: usize,
) -> bool {
    let Some(idx) = blocks
        .iter()
        .position(|b| b.start_line <= start_line && b.end_line >= end_line)
    else {
        return false;
    };
    let lo = idx.saturating_sub(radius);
    let hi = (idx + radius).min(blocks.len().saturating_sub(1));
    for (i, block) in blocks.iter().enumerate().take(hi + 1).skip(lo) {
        if i == idx {
            continue;
        }
        if block.is_prose {
            return true;
        }
    }
    false
}
