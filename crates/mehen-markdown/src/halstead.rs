//! Markdown Halstead metrics per §9.
//!
//! Walks the AST once and classifies each leaf / inline / block node as
//! operator or operand following §§9.1-9.2, using the tree-sitter-markdown-text
//! grammar's node kinds. Operators are identified by kind (so all `##` H2
//! markers share one operator class); operands are identified by their byte
//! text (so two occurrences of the same word count once in n2 but twice in
//! N2, matching classical Halstead).
//!
//! §9.3 formulas:
//!
//! ```text
//! vocab   = n1 + n2
//! length  = N1 + N2
//! volume  = length * log2(max(2, vocab))
//! diff    = (n1 / 2) * (N2 / max(1, n2))
//! effort  = volume * diff
//! ```
//!
//! §9.4 embedded-code adjustment happens outside this module in
//! `embedded_code.rs` and is composed into the final `Halstead` record by
//! the analyzer.

use std::collections::BTreeMap;

use crate::grammar::Markdown;
use crate::legacy_node::Node;
use crate::tree_helpers::fence_language_tag;
use crate::types::Halstead;

/// Distinct operator classes (rich enough that MCC and Halstead use the same
/// shape). Each variant represents one row in §9.1.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum OperatorKind {
    HeadingMarkerH1,
    HeadingMarkerH2,
    HeadingMarkerH3,
    HeadingMarkerH4,
    HeadingMarkerH5,
    HeadingMarkerH6,
    SetextMarkerH1,
    SetextMarkerH2,
    ListMarker,
    TaskMarkerChecked,
    TaskMarkerUnchecked,
    TableDelim,
    TableAlignLeft,
    TableAlignRight,
    LinkOp,
    ImageOp,
    /// Code fence by language tag (empty for unlabelled). Stored
    /// separately as a string so each tag is its own operator class.
    FenceTag(String),
    InlineCodeOp,
    BlockquoteMarker,
    CalloutOp,
    MathDelimiter,
    MathInlineDelimiter,
    EmphasisDelim,
    StrongDelim,
    StrikethroughDelim,
    FootnoteMarker,
    RawHtmlOp,
    MdxJsxOp,
    DirectiveOp,
    /// Terminator (`. ? ! 。 …`).
    Terminator,
    /// Separator (`, ; :`).
    Separator,
    /// Bracket (`() [] {} <>`).
    Bracket,
    /// Operator-like (`= + - * / | & :: -> =>`).
    OperatorLike,
}

/// Core public entry point. Walks the AST and emits the §9.3 derived values.
pub(crate) fn compute_halstead(root: &Node<'_>, source: &str) -> Halstead {
    let mut operator_counts: BTreeMap<OperatorKind, u64> = BTreeMap::new();
    let mut operand_counts: BTreeMap<String, u64> = BTreeMap::new();

    let mut state = Ctx {
        operator_counts: &mut operator_counts,
        operand_counts: &mut operand_counts,
        source,
    };
    state.walk(root);

    // §9.3.
    let n1 = operator_counts.len() as u64;
    let big_n1: u64 = operator_counts.values().sum();
    let n2 = operand_counts.len() as u64;
    let big_n2: u64 = operand_counts.values().sum();

    let vocabulary = n1 + n2;
    let length = big_n1 + big_n2;
    let vocab_f = vocabulary.max(2) as f64;
    let volume = length as f64 * vocab_f.log2();
    let difficulty = if n2 > 0 {
        (n1 as f64 / 2.0) * (big_n2 as f64 / n2 as f64)
    } else {
        0.0
    };
    let effort = volume * difficulty;

    Halstead {
        operators_distinct: n1,
        operators_total: big_n1,
        operands_distinct: n2,
        operands_total: big_n2,
        vocabulary,
        length,
        volume,
        difficulty,
        effort,
        // embedded_volume + total_volume are filled in by the analyzer after
        // embedded-code dispatch.
        embedded_volume: 0.0,
        total_volume: volume,
    }
}

struct Ctx<'a, 'b> {
    operator_counts: &'a mut BTreeMap<OperatorKind, u64>,
    operand_counts: &'a mut BTreeMap<String, u64>,
    source: &'b str,
}

impl Ctx<'_, '_> {
    fn bump_op(&mut self, k: OperatorKind) {
        *self.operator_counts.entry(k).or_insert(0) += 1;
    }

    fn bump_operand(&mut self, key: String) {
        if key.is_empty() {
            return;
        }
        *self.operand_counts.entry(key).or_insert(0) += 1;
    }

    fn walk(&mut self, node: &Node<'_>) {
        use Markdown::*;

        let kind: Markdown = node.kind_id().into();

        // Stop containers for operands that are URL / code text — still
        // classify them as operators at the wrapper level.
        let mut descend = true;

        match kind {
            // Heading markers — operators.
            AtxH1Marker => self.bump_op(OperatorKind::HeadingMarkerH1),
            AtxH2Marker => self.bump_op(OperatorKind::HeadingMarkerH2),
            AtxH3Marker => self.bump_op(OperatorKind::HeadingMarkerH3),
            AtxH4Marker => self.bump_op(OperatorKind::HeadingMarkerH4),
            AtxH5Marker => self.bump_op(OperatorKind::HeadingMarkerH5),
            AtxH6Marker => self.bump_op(OperatorKind::HeadingMarkerH6),
            SetextH1Underline => self.bump_op(OperatorKind::SetextMarkerH1),
            SetextH2Underline => self.bump_op(OperatorKind::SetextMarkerH2),

            // List markers.
            ListMarkerMinus
            | ListMarkerMinus2
            | ListMarkerMinusDontInterrupt
            | ListMarkerPlus
            | ListMarkerPlus2
            | ListMarkerPlusDontInterrupt
            | ListMarkerStar
            | ListMarkerStar2
            | ListMarkerStarDontInterrupt
            | ListMarkerDot
            | ListMarkerDot2
            | ListMarkerDotDontInterrupt
            | ListMarkerParenthesis
            | ListMarkerParenthesis2
            | ListMarkerParenthesisDontInterrupt => self.bump_op(OperatorKind::ListMarker),

            // Task list markers.
            TaskListMarkerChecked => self.bump_op(OperatorKind::TaskMarkerChecked),
            TaskListMarkerUnchecked => self.bump_op(OperatorKind::TaskMarkerUnchecked),

            // Table operators.
            PipeTableDelimiterRow | PipeTableDelimiterCell | PipeTableStart => {
                self.bump_op(OperatorKind::TableDelim)
            }
            PipeTableAlignLeft => self.bump_op(OperatorKind::TableAlignLeft),
            PipeTableAlignRight => self.bump_op(OperatorKind::TableAlignRight),

            // Link / image wrappers — each whole `Link` is one operator
            // occurrence for `[…](…)`. Inner LinkLabel prose descends as
            // operand text; the LinkDestination descends as an operand path.
            Link => {
                self.bump_op(OperatorKind::LinkOp);
            }
            Image => {
                self.bump_op(OperatorKind::ImageOp);
            }

            // Code fences: record the language tag as the operator's
            // discriminator so e.g. `rust` and `python` are distinct
            // operators.
            FencedCodeBlock => {
                let tag = fence_info_tag(node, self.source).unwrap_or_default();
                self.bump_op(OperatorKind::FenceTag(tag));
                // Embedded content handled as a single identifier-like
                // operand (the `code_fence_content` string). We do not
                // descend into its leaves so §9.4's scaling applies cleanly.
                if let Some(content) = fenced_code_content(node, self.source) {
                    self.bump_operand(format!("code:{}", sha_hex(content.as_bytes())));
                }
                descend = false;
            }
            IndentedCodeBlock => {
                self.bump_op(OperatorKind::FenceTag(String::new()));
                // Treat the entire indented content as one operand hash.
                let bytes = self.source.as_bytes();
                let start = node.start_byte();
                let end = node.end_byte();
                if end <= bytes.len() && start < end {
                    self.bump_operand(format!("indent_code:{}", sha_hex(&bytes[start..end])));
                }
                descend = false;
            }

            InlineCode => {
                self.bump_op(OperatorKind::InlineCodeOp);
                // The inline-code content is opaque; hash it so two identical
                // `` `foo` `` references count as one operand.
                if let Some(text) = inline_code_text(node, self.source) {
                    self.bump_operand(format!("inline:{}", sha_hex(text.as_bytes())));
                }
                descend = false;
            }

            // Blockquote and callout markers.
            BlockQuoteMarker => self.bump_op(OperatorKind::BlockquoteMarker),
            CalloutMarkerOpen | CalloutMarkerClose | CalloutType => {
                self.bump_op(OperatorKind::CalloutOp)
            }

            // Math delimiters.
            MathBlockDelimiter => self.bump_op(OperatorKind::MathDelimiter),
            MathInlineDelimiter | MathInlineDelimiter2 => {
                self.bump_op(OperatorKind::MathInlineDelimiter)
            }

            // Emphasis / strong / strikethrough.
            EmphasisDelimiter | EmphasisDelimiter2 => self.bump_op(OperatorKind::EmphasisDelim),
            StrongDelimiter | StrongDelimiter2 => self.bump_op(OperatorKind::StrongDelim),
            StrikethroughDelimiter => self.bump_op(OperatorKind::StrikethroughDelim),

            // Footnote operators.
            FootnoteReferenceOpen | FootnoteLabelOpen | FootnoteDefinitionStart => {
                self.bump_op(OperatorKind::FootnoteMarker)
            }

            // Raw HTML / MDX / directive operators.
            HtmlOpenTag
            | HtmlCloseTag
            | HtmlComment
            | HtmlCdata
            | HtmlDeclaration
            | HtmlProcessingInstruction
            | HtmlBlock1Start
            | HtmlBlock1End
            | HtmlBlock2Start
            | HtmlBlock3Start
            | HtmlBlock4Start
            | HtmlBlock5Start
            | HtmlBlock6Start
            | HtmlBlock7Start => self.bump_op(OperatorKind::RawHtmlOp),
            MdxJsxOpenTag | MdxJsxCloseTag | MdxJsxOpenTag2 | MdxJsxCloseTag2
            | MdxJsxExpression => self.bump_op(OperatorKind::MdxJsxOp),
            DirectiveBlockDelimiter | DirectiveName => self.bump_op(OperatorKind::DirectiveOp),

            // Punctuation classes per §3.3.
            Terminator => self.bump_op(OperatorKind::Terminator),
            Separator => self.bump_op(OperatorKind::Separator),
            Bracket => self.bump_op(OperatorKind::Bracket),
            OperatorLike => self.bump_op(OperatorKind::OperatorLike),

            // Operand leaves.
            WordToken | WordToken1 | WordToken2 | WordToken3 => {
                self.push_text_operand(node);
            }
            NumericToken => {
                self.push_text_operand(node);
            }
            IdentifierLikeToken => {
                self.push_text_operand(node);
            }
            PathLikeToken => {
                self.push_text_operand(node);
            }

            // Link destinations — operand (URL).
            LinkDestination | LinkDestinationParenthesis | Uri => {
                self.push_text_operand(node);
                // Don't descend further into URL text.
                descend = false;
            }

            // Table headers (header row cells) — count each cell's text as
            // an operand in addition to any word-like tokens inside it.
            // §9.2 lists "table headers" as a distinct operand class.
            PipeTableHeader => {
                let mut c = node.cursor();
                if c.goto_first_child() {
                    loop {
                        let cell = c.node();
                        if matches!(cell.kind_id().into(), Markdown::PipeTableCell) {
                            let text = node_text(&cell, self.source).trim().to_string();
                            if !text.is_empty() {
                                self.bump_operand(format!("th:{}", text));
                            }
                        }
                        if !c.goto_next_sibling() {
                            break;
                        }
                    }
                }
                // Fall through: descend so each word token in the header
                // row also contributes as a regular word operand.
            }

            _ => {}
        }

        if !descend {
            return;
        }

        let mut cursor = node.cursor();
        if cursor.goto_first_child() {
            loop {
                self.walk(&cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn push_text_operand(&mut self, node: &Node<'_>) {
        let bytes = self.source.as_bytes();
        let start = node.start_byte();
        let end = node.end_byte();
        if end <= bytes.len() && start < end {
            let text = std::str::from_utf8(&bytes[start..end])
                .unwrap_or("")
                .trim()
                .to_string();
            if !text.is_empty() {
                self.bump_operand(text);
            }
        }
    }
}

fn fence_info_tag(node: &Node<'_>, source: &str) -> Option<String> {
    fence_language_tag(node, source, true)
}

fn fenced_code_content(node: &Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.cursor();
    if !cursor.goto_first_child() {
        return None;
    }
    loop {
        let child = cursor.node();
        if matches!(child.kind_id().into(), Markdown::CodeFenceContent) {
            let bytes = source.as_bytes();
            let start = child.start_byte();
            let end = child.end_byte();
            if end <= bytes.len() && start <= end {
                return std::str::from_utf8(&bytes[start..end])
                    .ok()
                    .map(str::to_string);
            }
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    None
}

fn inline_code_text(node: &Node<'_>, source: &str) -> Option<String> {
    let bytes = source.as_bytes();
    let start = node.start_byte();
    let end = node.end_byte();
    if end <= bytes.len() && start < end {
        return std::str::from_utf8(&bytes[start..end])
            .ok()
            .map(|s| s.trim().trim_matches('`').to_string());
    }
    None
}

fn node_text(node: &Node<'_>, source: &str) -> String {
    let bytes = source.as_bytes();
    let start = node.start_byte();
    let end = node.end_byte();
    if end <= bytes.len() && start < end {
        std::str::from_utf8(&bytes[start..end])
            .unwrap_or("")
            .to_string()
    } else {
        String::new()
    }
}

/// Cheap deterministic hash → lowercase hex. We only need stable
/// equivalence, so the FNV-1a 64-bit variant is plenty.
fn sha_hex(bytes: &[u8]) -> String {
    // FNV-1a 64-bit.
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_markdown_text::LANGUAGE.into())
            .unwrap();
        parser.parse(src, None).unwrap()
    }

    #[test]
    fn empty_halstead_is_zero() {
        let tree = parse("");
        let root = crate::legacy_node::Node(tree.root_node());
        let h = compute_halstead(&root, "");
        assert_eq!(h.operators_distinct, 0);
        assert_eq!(h.operators_total, 0);
        assert_eq!(h.operands_distinct, 0);
        assert_eq!(h.operands_total, 0);
        assert_eq!(h.vocabulary, 0);
        assert_eq!(h.length, 0);
        assert_eq!(h.volume, 0.0);
        assert_eq!(h.total_volume, 0.0);
    }

    #[test]
    fn heading_plus_prose_counts() {
        let src = "# Hello world\n";
        let tree = parse(src);
        let root = crate::legacy_node::Node(tree.root_node());
        let h = compute_halstead(&root, src);
        // Operators: 1 H1 marker → n1=1, N1=1.
        assert_eq!(h.operators_distinct, 1);
        assert_eq!(h.operators_total, 1);
        // Operands: `Hello`, `world` → n2=2, N2=2.
        assert_eq!(h.operands_distinct, 2);
        assert_eq!(h.operands_total, 2);
        assert_eq!(h.vocabulary, 3);
        assert_eq!(h.length, 3);
        // Volume = 3 * log2(3) ≈ 4.754887.
        assert!((h.volume - 3.0 * (3.0_f64).log2()).abs() < 1e-6);
    }

    #[test]
    fn link_counts_as_operator_and_url_as_operand() {
        let src = "# H\n\nSee [here](https://example.com).\n";
        let tree = parse(src);
        let root = crate::legacy_node::Node(tree.root_node());
        let h = compute_halstead(&root, src);
        // Operators include: one H1 marker, one Link, one Terminator (`.`).
        assert!(h.operators_distinct >= 3);
        // The URL is one operand; `See` and `here` are word operands.
        assert!(h.operands_distinct >= 3);
    }
}
