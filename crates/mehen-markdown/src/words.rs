//! Narrative word counter `W` (§4) with anti-gaming defenses (§37.5).
//!
//! Counts `word_token`, `numeric_token`, `identifier_like_token`, and
//! `path_like_token` nodes *only when they appear inside prose contexts*.
//! A prose context is a `paragraph`, `atx_heading_content`, `setext_heading`,
//! `block_quote`, `callout`, or a list item's inline content. Inline code,
//! link destinations, image destinations, raw HTML, MDX, math, autolinks,
//! front-matter, and pipe-table delimiter characters are NOT prose and do
//! not contribute.
//!
//! Alt-text inside an image or link *label* IS prose per §37.5 item 3, so
//! traversal into `link_label` is allowed — only the destination is skipped.

use crate::grammar::Markdown;
use crate::legacy_node::Node;

/// Counts narrative word-like tokens across the document.
pub(crate) fn count_words(root: &Node<'_>) -> u64 {
    let mut total: u64 = 0;
    visit(root, &mut total, false);
    total
}

/// Walks the tree, accumulating word counts in prose-eligible contexts.
///
/// `inside_prose` tracks whether the current walk is below a prose-shaped
/// ancestor. Technical containers flip the flag off for their subtree.
fn visit(node: &Node<'_>, total: &mut u64, inside_prose: bool) {
    use Markdown::*;

    let kind: Markdown = node.kind_id().into();

    // First handle "stop" containers — never descend into these. They shadow
    // any outer prose context.
    match kind {
        // Code.
        FencedCodeBlock
        | IndentedCodeBlock
        | InlineCode
        | CodeFenceContent
        | InlineCodeContent
        | InlineCodeContent2
        | InfoString
        | Language
        // Math.
        | MathBlock
        | MathInline
        | MathBlockContent
        | MathInlineContent
        // Raw HTML / MDX.
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
        // Autolinks are URI / email spans, not prose.
        | Autolink
        | Uri
        | Email
        // Link / image destinations are URLs, not prose.
        | LinkDestination
        | LinkDestinationParenthesis
        | LinkTitle
        | TextNoAngle
        // Front-matter is structured YAML/TOML, not prose.
        | MinusMetadata
        | PlusMetadata
        // Pipe-table delimiter characters are pure structure.
        | PipeTableDelimiterRow
        | PipeTableDelimiterCell
        | PipeTableAlignLeft
        | PipeTableAlignRight => {
            return;
        }
        _ => {}
    }

    // A prose-eligible container opens a prose context for its descendants.
    let opens_prose = matches!(
        kind,
        Paragraph
            | AtxHeadingContent
            | SetextHeading
            | SetextHeading2
            | BlockQuote
            | PlainBlockQuote
            | Callout
            | CalloutHeaderParagraph
            | ListItemContent
            | TaskListItemContent
            | LinkLabel
            | FootnoteLabel
            | PipeTableCell
            | PipeTableHeader
            | PipeTableRow
    );

    let next_inside = inside_prose || opens_prose;

    if next_inside && is_word_token(kind) {
        *total += 1;
    }

    // Recurse.
    let mut cursor = node.cursor();
    if cursor.goto_first_child() {
        loop {
            visit(&cursor.node(), total, next_inside);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn is_word_token(kind: Markdown) -> bool {
    matches!(
        kind,
        Markdown::WordToken
            | Markdown::WordToken1
            | Markdown::WordToken2
            | Markdown::WordToken3
            | Markdown::NumericToken
            | Markdown::IdentifierLikeToken
            | Markdown::PathLikeToken
    )
}
