//! Markdown Cognitive Complexity (MCC) per §8.
//!
//! Walks the AST, accumulates per-element base weights from §8.1, applies the
//! §8.2 nesting multiplier (`1 + 0.18 * nest(n)`), and the §8.3 cluster
//! multiplier computed from a rolling 20-line window of artifact density,
//! then subtracts scaffold credit per §8.4 (capped at `0.25 * MCC_positive`).
//!
//! Phase-B stubs:
//! - Broken internal/relative link (+3.00) → 0.00 until Phase C link
//!   validator lands.
//! - External link unchecked (+0.30) → applied (external link always adds a
//!   small penalty pending validation).
//! - External link broken (+4.00) → 0.00 until Phase C.
//! - Diagram parse error (+3.00) → 0.00 until Phase C diagram parser lands.

use crate::grammar::Markdown;
use crate::legacy_node::Node;

/// §8 aggregate: positive weight before credit, credit amount used, final
/// MCC. Only `mcc` is exported to the public record; `positive` and
/// `credit_used` stay accessible to in-crate tests so we can assert the
/// intermediate arithmetic.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct MccResult {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) positive: f64,
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) credit_used: f64,
    pub(crate) mcc: f64,
}

/// Public entry point.
pub(crate) fn compute_mcc(root: &Node<'_>, source: &str) -> MccResult {
    let mut ctx = Walker::new(source);
    // Pass 1: collect artifact lines for the 20-line-window cluster density
    // and record each block's sequence index for §8.4 locality lookup.
    ctx.scan_blocks(root);
    // Pass 2: accumulate weights and queue scaffold-credit candidates.
    ctx.walk(root);

    let credit_raw: f64 = ctx.pending_credits.iter().sum();
    let credit = credit_raw.min(0.25 * ctx.positive);
    let mcc = (ctx.positive - credit).max(0.0);
    MccResult {
        positive: ctx.positive,
        credit_used: credit,
        mcc,
    }
}

struct Walker<'a> {
    source: &'a str,
    positive: f64,
    /// Individual scaffold-credit contributions queued during the walk.
    /// They are summed and capped at `0.25 * positive` after the walk.
    pending_credits: Vec<f64>,
    last_heading_level: Option<u8>,
    /// Each physical line has `1` if an artifact block touches it, else `0`.
    /// Used for the §8.3 cluster multiplier.
    artifact_line: Vec<bool>,
    /// The ordered list of block-level node starts keyed by `BlockKind`.
    /// Used to check "prose / heading within ±2 blocks" for §8.4.
    blocks: Vec<(BlockKind, u32)>,
    // Nesting depths tracked during recursive walk.
    list_depth: u32,
    blockquote_depth: u32,
    callout_depth: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Paragraph,
    Code,
    Table,
    Math,
    Image,
    RawHtml,
    Heading,
    Other,
}

impl<'a> Walker<'a> {
    fn new(source: &'a str) -> Self {
        let mut lines = 1usize;
        for b in source.bytes() {
            if b == b'\n' {
                lines += 1;
            }
        }
        if source.ends_with('\n') {
            lines = lines.saturating_sub(1);
        }
        Self {
            source,
            positive: 0.0,
            pending_credits: Vec::new(),
            last_heading_level: None,
            artifact_line: vec![false; lines.max(1)],
            blocks: Vec::new(),
            list_depth: 0,
            blockquote_depth: 0,
            callout_depth: 0,
        }
    }

    fn scan_blocks(&mut self, node: &Node<'_>) {
        use Markdown::*;
        let kind: Markdown = node.kind_id().into();
        let bk = classify_block(&kind);
        let is_artifact = matches!(
            kind,
            FencedCodeBlock
                | IndentedCodeBlock
                | PipeTable
                | MathBlock
                | HtmlBlock
                | HtmlBlock1
                | HtmlBlock3
                | HtmlBlock4
                | HtmlBlock5
                | HtmlBlock6
                | HtmlBlock7
                | HtmlCommentBlock
                | MdxJsxBlock
                | ImageBlock
                | DirectiveBlock
        );
        if is_artifact {
            let start = node.start_row();
            let (end_row, end_col) = node.end_position();
            let mut end = end_row;
            if end > start && end_col == 0 {
                end -= 1;
            }
            for row in start..=end.min(self.artifact_line.len().saturating_sub(1)) {
                self.artifact_line[row] = true;
            }
        }
        if bk != BlockKind::Other {
            self.blocks.push((bk, node.start_row() as u32));
        }
        let mut cursor = node.cursor();
        if cursor.goto_first_child() {
            loop {
                self.scan_blocks(&cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn walk(&mut self, node: &Node<'_>) {
        use Markdown::*;

        let kind: Markdown = node.kind_id().into();

        // Headings.
        if is_atx_heading(&kind) || is_setext_heading(&kind) {
            let level = heading_level(node).unwrap_or(1);
            if let Some(prev) = self.last_heading_level
                && level > prev
            {
                // Deeper level. Penalize a heading skip (>= 2 steps)
                // with 1.00; a smooth +1 step earns the normal 0.20.
                let delta = level.saturating_sub(prev);
                if delta == 1 {
                    self.positive += 0.20 * self.current_nest_multiplier();
                } else {
                    self.positive += 1.00 * self.current_nest_multiplier();
                }
            }
            // First heading and going-shallower: no penalty.
            self.last_heading_level = Some(level);
        }

        // Section without subheading + > 800 words — checked only when the
        // node is a `section*` container.
        if is_section(&kind) && section_has_no_sub_heading(node) {
            let words = count_section_words(node);
            if words > 800 {
                self.positive += 2.00 * self.current_nest_multiplier();
            }
        }

        // Paragraph > 160 words → 1.25.
        if matches!(kind, Paragraph) {
            let words = count_word_tokens(node);
            if words > 160 {
                self.positive += 1.25 * self.current_nest_multiplier();
            }
            // Dense link cluster: > 4 inline links in a paragraph → 1.50.
            let links = count_inline_links(node);
            if links > 4 {
                self.positive +=
                    1.50 * self.cluster_multiplier(node) * self.current_nest_multiplier();
            }
        }

        // Lists and list structures.
        match kind {
            List | ListPlus | ListMinus | ListStar | ListDot | ListParenthesis => {
                self.positive += 0.40 * self.current_nest_multiplier();
                self.list_depth += 1;
                self.recurse(node);
                self.list_depth -= 1;
                return;
            }
            ListItem | ListItem2 | ListItem3 | ListItem4 | ListItem5 => {
                // Nested list level: charge 0.50 * depth per §8.1. `depth`
                // here is the current list-depth *before* the list-item
                // increments it further; using list_depth directly approximates
                // "level" since each outer list already incremented the depth.
                self.positive +=
                    0.50 * self.list_depth.max(1) as f64 * self.current_nest_multiplier();
            }
            TaskListItem | TaskListItem2 | TaskListItem3 | TaskListItem4 | TaskListItem5 => {
                self.positive += 0.35 * self.current_nest_multiplier();
            }
            BlockQuote | PlainBlockQuote => {
                self.positive += 0.50 * self.current_nest_multiplier();
                self.blockquote_depth += 1;
                self.recurse(node);
                self.blockquote_depth -= 1;
                return;
            }
            Callout => {
                self.positive += 0.75 * self.current_nest_multiplier();
                self.callout_depth += 1;
                self.recurse(node);
                self.callout_depth -= 1;
                return;
            }
            _ => {}
        }

        // Inline links / images (not the whole paragraph).
        if matches!(kind, Link) {
            self.positive += 0.25 * self.current_nest_multiplier();
            // External link unchecked → +0.30 per §8.1. Phase B applies this
            // universally until Phase C differentiates valid / broken.
            if let Some(dest) = find_link_dest(node, self.source)
                && is_external(&dest)
            {
                self.positive += 0.30 * self.current_nest_multiplier();
            }
            // TODO(Phase C): broken internal/relative link → +3.00;
            // external broken → +4.00. Left at 0.00 until the link
            // validator lands.
        }

        // Footnote reference.
        if matches!(kind, FootnoteReference) {
            self.positive += 0.60 * self.current_nest_multiplier();
        }

        // Images.
        if matches!(kind, Image) {
            self.positive += 0.50 * self.current_nest_multiplier();
            // §8.4 credit: image with alt/caption + nearby explanation,
            // bounded. We approximate `alt` as the non-empty link-label
            // text inside the Image node.
            let label = find_link_label(node, self.source).unwrap_or_default();
            let has_label = !label.trim().is_empty();
            if has_label {
                let start = node.start_row() as u32;
                let local = local_explanation(&self.blocks, start);
                // Base credit for image 0.80; bounded = 1 since we have no
                // size for the rendered image yet — Phase C can refine.
                let credit = 0.80 * (local as f64) * 1.0;
                if credit > 0.0 {
                    self.pending_credits.push(credit);
                }
            }
        }
        if matches!(kind, ImageBlock) {
            self.positive += 0.50 * self.current_nest_multiplier();
        }

        // Code fences.
        if matches!(kind, FencedCodeBlock | IndentedCodeBlock) {
            // LOC counts fence content only — fence markers would inflate
            // the size-based weighting by ~2 lines near the §8.1 `<=12`
            // cutoff (Codex P2).
            let loc = fence_content_line_count(node);
            let info = fence_info(node, self.source);
            let is_diagram = matches!(
                info.as_deref(),
                Some("mermaid") | Some("plantuml") | Some("dot") | Some("graphviz") | Some("d2")
            );
            if is_diagram {
                self.positive +=
                    1.50 * self.cluster_multiplier(node) * self.current_nest_multiplier();
                // §8.4 diagram credit: 1.25 * local_explanation * has_label *
                // bounded. Phase B doesn't have a caption detector yet — use
                // a conservative `has_label = 1` (the `mermaid`/etc. info
                // string already makes the diagram type clear) and local
                // explanation via ±2 blocks.
                let start = node.start_row() as u32;
                let local = local_explanation(&self.blocks, start);
                let credit = 1.25 * (local as f64) * 1.0;
                if credit > 0.0 {
                    self.pending_credits.push(credit);
                }
                // TODO(Phase C): diagram parse error → +3.00. Stub.
            } else {
                let base = if loc <= 12 {
                    1.00
                } else {
                    1.00 + 0.08 * (loc as f64 - 12.0)
                };
                let unlabelled = match kind {
                    FencedCodeBlock => info.is_none(),
                    IndentedCodeBlock => true,
                    _ => false,
                };
                let mut weight = base;
                if unlabelled {
                    weight += 1.50;
                }
                self.positive +=
                    weight * self.cluster_multiplier(node) * self.current_nest_multiplier();
                // §8.4 scaffold credit for code examples:
                //   0.75 * local_explanation * has_label * bounded
                // where has_label = language tag present, bounded = 1 if
                // loc <= 30 decaying to 0 at loc == 60.
                if !unlabelled {
                    let start = node.start_row() as u32;
                    let local = local_explanation(&self.blocks, start);
                    let bounded = bounded_size(loc as f64, 30.0, 60.0);
                    let credit = 0.75 * (local as f64) * bounded;
                    if credit > 0.0 {
                        self.pending_credits.push(credit);
                    }
                }
            }
        }

        // Pipe tables.
        if matches!(kind, PipeTable) {
            let cells = count_table_cells(node);
            let weight = if cells <= 60 {
                0.75
            } else {
                0.75 + 0.03 * (cells as f64 - 60.0).powf(0.85)
            };
            self.positive +=
                weight * self.cluster_multiplier(node) * self.current_nest_multiplier();
            // §8.4 table credit: 1.00 * local_explanation * has_header *
            // bounded. `bounded` fades from 1 at 60 cells to 0 at 150.
            let has_header = pipe_table_has_header(node);
            if has_header && cells > 0 {
                let start = node.start_row() as u32;
                let local = local_explanation(&self.blocks, start);
                let bounded = bounded_size(cells as f64, 60.0, 150.0);
                let credit = 1.00 * (local as f64) * bounded;
                if credit > 0.0 {
                    self.pending_credits.push(credit);
                }
            }
        }

        // Math blocks.
        if matches!(kind, MathBlock) {
            self.positive += 1.50 * self.cluster_multiplier(node) * self.current_nest_multiplier();
            // §8.4 math credit: 0.50 * local_explanation * bounded. Use
            // line span as the size proxy.
            let start = node.start_row() as u32;
            let local = local_explanation(&self.blocks, start);
            let lines = node_line_span(node) as f64;
            let bounded = bounded_size(lines, 6.0, 20.0);
            let credit = 0.50 * (local as f64) * bounded;
            if credit > 0.0 {
                self.pending_credits.push(credit);
            }
        }

        // Raw HTML / MDX blocks: 0.30 * lines, cap 8.
        if matches!(
            kind,
            HtmlBlock
                | HtmlBlock1
                | HtmlBlock3
                | HtmlBlock4
                | HtmlBlock5
                | HtmlBlock6
                | HtmlBlock7
                | HtmlCommentBlock
                | MdxJsxBlock
                | DirectiveBlock
        ) {
            let lines = node_line_span(node) as f64;
            let weight = (0.30 * lines).min(8.0);
            self.positive +=
                weight * self.cluster_multiplier(node) * self.current_nest_multiplier();
        }

        self.recurse(node);
    }

    fn recurse(&mut self, node: &Node<'_>) {
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

    fn current_nest_multiplier(&self) -> f64 {
        let nest = self.list_depth + self.blockquote_depth + self.callout_depth;
        1.0 + 0.18 * nest as f64
    }

    fn cluster_multiplier(&self, node: &Node<'_>) -> f64 {
        // 20-line window centered on the node's start row.
        let start = node.start_row();
        let lo = start.saturating_sub(10);
        let hi = (start + 10).min(self.artifact_line.len());
        let window = &self.artifact_line[lo..hi];
        if window.is_empty() {
            return 1.0;
        }
        let hits = window.iter().filter(|b| **b).count() as f64;
        let density = hits / window.len() as f64;
        1.0 + saturate(density, 0.15, 0.45) * 0.35
    }
}

/// `1` if a prose / heading block exists within ±2 blocks of the `start_row`.
///
/// `blocks` is the document-order block list. We find the nearest block
/// matching `start_row` and peek 2 neighbours to each side. A prose block
/// (Paragraph) or Heading counts as a local explanation.
fn local_explanation(blocks: &[(BlockKind, u32)], start_row: u32) -> u8 {
    let idx = match blocks.iter().position(|(_, row)| *row == start_row) {
        Some(i) => i,
        None => {
            // Fall back: closest block by absolute row distance.
            let mut best: Option<usize> = None;
            let mut best_d = u32::MAX;
            for (i, (_, r)) in blocks.iter().enumerate() {
                let d = r.abs_diff(start_row);
                if d < best_d {
                    best_d = d;
                    best = Some(i);
                }
            }
            match best {
                Some(i) => i,
                None => return 0,
            }
        }
    };
    let lo = idx.saturating_sub(2);
    let hi = (idx + 3).min(blocks.len());
    for (i, (bk, _)) in blocks[lo..hi].iter().enumerate() {
        let abs = lo + i;
        if abs == idx {
            continue;
        }
        if matches!(bk, BlockKind::Paragraph | BlockKind::Heading) {
            return 1;
        }
    }
    0
}

/// Returns `1 - sat(size; useful_hi, severe_hi)` per §8.4 `bounded(a)`.
fn bounded_size(size: f64, useful_hi: f64, severe_hi: f64) -> f64 {
    1.0 - saturate(size, useful_hi, severe_hi)
}

fn is_external(dest: &str) -> bool {
    // An external link has an explicit URL scheme (RFC 3986: ALPHA
    // followed by ALPHA / DIGIT / "+" / "-" / ".").
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

fn saturate(x: f64, lo: f64, hi: f64) -> f64 {
    if hi <= lo {
        return 0.0;
    }
    ((x - lo) / (hi - lo)).clamp(0.0, 1.0)
}

fn classify_block(kind: &Markdown) -> BlockKind {
    use Markdown::*;
    match kind {
        Paragraph => BlockKind::Paragraph,
        FencedCodeBlock | IndentedCodeBlock => BlockKind::Code,
        PipeTable => BlockKind::Table,
        MathBlock => BlockKind::Math,
        ImageBlock => BlockKind::Image,
        HtmlBlock | HtmlBlock1 | HtmlBlock3 | HtmlBlock4 | HtmlBlock5 | HtmlBlock6 | HtmlBlock7
        | HtmlCommentBlock | MdxJsxBlock => BlockKind::RawHtml,
        AtxHeading | AtxHeading2 | AtxHeading3 | AtxHeading4 | AtxHeading5 | AtxHeading6
        | SetextHeading | SetextHeading2 => BlockKind::Heading,
        _ => BlockKind::Other,
    }
}

fn is_atx_heading(kind: &Markdown) -> bool {
    matches!(
        kind,
        Markdown::AtxHeading
            | Markdown::AtxHeading2
            | Markdown::AtxHeading3
            | Markdown::AtxHeading4
            | Markdown::AtxHeading5
            | Markdown::AtxHeading6
    )
}

fn is_setext_heading(kind: &Markdown) -> bool {
    matches!(kind, Markdown::SetextHeading | Markdown::SetextHeading2)
}

fn is_section(kind: &Markdown) -> bool {
    matches!(
        kind,
        Markdown::Section
            | Markdown::Section1
            | Markdown::Section2
            | Markdown::Section3
            | Markdown::Section4
            | Markdown::Section5
            | Markdown::Section6
    )
}

fn heading_level(heading: &Node<'_>) -> Option<u8> {
    let level = heading.child_by_field_name("level")?;
    Some(match level.kind_id().into() {
        Markdown::AtxH1Marker | Markdown::SetextH1Underline => 1,
        Markdown::AtxH2Marker | Markdown::SetextH2Underline => 2,
        Markdown::AtxH3Marker => 3,
        Markdown::AtxH4Marker => 4,
        Markdown::AtxH5Marker => 5,
        Markdown::AtxH6Marker => 6,
        _ => return None,
    })
}

fn section_has_no_sub_heading(section: &Node<'_>) -> bool {
    let mut cursor = section.cursor();
    if !cursor.goto_first_child() {
        return true;
    }
    loop {
        let child = cursor.node();
        let child_kind: Markdown = child.kind_id().into();
        if is_section(&child_kind) {
            return false;
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    true
}

fn count_section_words(node: &Node<'_>) -> u64 {
    let mut total = 0u64;
    walk_words(node, &mut total);
    total
}

fn walk_words(node: &Node<'_>, total: &mut u64) {
    let kind: Markdown = node.kind_id().into();
    // Don't descend into stop-containers — mirrors `words.rs` rules.
    match kind {
        Markdown::FencedCodeBlock
        | Markdown::IndentedCodeBlock
        | Markdown::InlineCode
        | Markdown::CodeFenceContent
        | Markdown::InlineCodeContent
        | Markdown::InlineCodeContent2
        | Markdown::InfoString
        | Markdown::Language
        | Markdown::MathBlock
        | Markdown::MathInline
        | Markdown::MathBlockContent
        | Markdown::MathInlineContent
        | Markdown::HtmlBlock
        | Markdown::HtmlBlock1
        | Markdown::HtmlBlock3
        | Markdown::HtmlBlock4
        | Markdown::HtmlBlock5
        | Markdown::HtmlBlock6
        | Markdown::HtmlBlock7
        | Markdown::HtmlCommentBlock
        | Markdown::HtmlInline
        | Markdown::MdxJsxBlock
        | Markdown::MdxJsxInline
        | Markdown::Autolink
        | Markdown::Uri
        | Markdown::Email
        | Markdown::LinkDestination
        | Markdown::LinkDestinationParenthesis
        | Markdown::LinkTitle
        | Markdown::MinusMetadata
        | Markdown::PlusMetadata
        | Markdown::PipeTableDelimiterRow => {
            return;
        }
        _ => {}
    }
    if matches!(
        kind,
        Markdown::WordToken
            | Markdown::WordToken1
            | Markdown::WordToken2
            | Markdown::WordToken3
            | Markdown::NumericToken
            | Markdown::IdentifierLikeToken
            | Markdown::PathLikeToken
    ) {
        *total += 1;
    }
    let mut cursor = node.cursor();
    if cursor.goto_first_child() {
        loop {
            walk_words(&cursor.node(), total);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn count_word_tokens(node: &Node<'_>) -> u64 {
    let mut total = 0u64;
    walk_words(node, &mut total);
    total
}

fn count_inline_links(node: &Node<'_>) -> u64 {
    let mut total = 0u64;
    let mut stack = vec![*node];
    while let Some(n) = stack.pop() {
        if matches!(n.kind_id().into(), Markdown::Link) {
            total += 1;
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
    total
}

fn count_table_cells(node: &Node<'_>) -> usize {
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

fn pipe_table_has_header(node: &Node<'_>) -> bool {
    let mut cursor = node.cursor();
    if !cursor.goto_first_child() {
        return false;
    }
    loop {
        if matches!(cursor.node().kind_id().into(), Markdown::PipeTableHeader) {
            return true;
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    false
}

fn node_line_span(node: &Node<'_>) -> usize {
    let start = node.start_row();
    let (end_row, end_col) = node.end_position();
    let mut end = end_row;
    if end > start && end_col == 0 {
        end -= 1;
    }
    end.saturating_sub(start) + 1
}

/// Content-only line count inside a fenced code block. Indented code blocks
/// have no delimiters so their content equals their span.
fn fence_content_line_count(node: &Node<'_>) -> usize {
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

fn fence_info(node: &Node<'_>, source: &str) -> Option<String> {
    let mut cursor = node.cursor();
    if !cursor.goto_first_child() {
        return None;
    }
    loop {
        let child = cursor.node();
        let kind: Markdown = child.kind_id().into();
        if matches!(kind, Markdown::InfoString) {
            let mut c2 = child.cursor();
            if c2.goto_first_child() {
                loop {
                    let inner = c2.node();
                    if matches!(inner.kind_id().into(), Markdown::Language) {
                        let bytes = source.as_bytes();
                        let start = inner.start_byte();
                        let end = inner.end_byte();
                        if end <= bytes.len() && start < end {
                            let tag = std::str::from_utf8(&bytes[start..end])
                                .ok()?
                                .trim()
                                .to_ascii_lowercase();
                            if !tag.is_empty() {
                                return Some(tag);
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

fn find_link_dest(node: &Node<'_>, source: &str) -> Option<String> {
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

fn find_link_label(node: &Node<'_>, source: &str) -> Option<String> {
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
    fn empty_doc_mcc_zero() {
        let tree = parse("");
        let root = crate::legacy_node::Node(tree.root_node());
        let r = compute_mcc(&root, "");
        assert_eq!(r.mcc, 0.0);
        assert_eq!(r.positive, 0.0);
    }

    #[test]
    fn heading_skip_penalizes() {
        let src = "# Top\n\n### Skipped\n";
        let tree = parse(src);
        let root = crate::legacy_node::Node(tree.root_node());
        let r = compute_mcc(&root, src);
        // Heading skip H1→H3 contributes 1.00 with nest_multiplier=1.
        assert!(r.positive >= 1.0, "positive: {}", r.positive);
    }

    #[test]
    fn section_800_words_charges() {
        // Build a section with ≥ 801 words.
        let filler = "word ".repeat(801);
        let src = format!("# Title\n\n{}\n", filler);
        let tree = parse(&src);
        let root = crate::legacy_node::Node(tree.root_node());
        let r = compute_mcc(&root, &src);
        // §8.1 charges 2.00 per section-without-subheading > 800 words.
        assert!(r.positive >= 2.0, "positive: {}", r.positive);
    }

    #[test]
    fn fences_and_tables_adjust_cluster() {
        let src = "# T\n\n```\nfoo\n```\n\n```\nbar\n```\n";
        let tree = parse(src);
        let root = crate::legacy_node::Node(tree.root_node());
        let r = compute_mcc(&root, src);
        // Two unlabelled code fences: 1.00 + 1.50 penalty each. They sit in
        // an artifact-dense window, so cluster multiplier > 1.
        assert!(r.positive > 5.0, "positive: {}", r.positive);
    }

    #[test]
    fn unlabelled_code_fence_adds_1_5() {
        let labelled = "# H\n\nIntro prose.\n\n```rust\nlet x = 1;\n```\n\nExplanation.\n";
        let unlabelled = "# H\n\nIntro prose.\n\n```\nlet x = 1;\n```\n\nExplanation.\n";
        let t1 = parse(labelled);
        let t2 = parse(unlabelled);
        let r1 = compute_mcc(&crate::legacy_node::Node(t1.root_node()), labelled);
        let r2 = compute_mcc(&crate::legacy_node::Node(t2.root_node()), unlabelled);
        // The positive difference between unlabelled and labelled should be
        // at least 1.50 (after matching cluster multipliers). Allow for tiny
        // numeric drift due to cluster windows.
        assert!(
            r2.positive - r1.positive >= 1.49,
            "unlabelled delta: {:.4}",
            r2.positive - r1.positive
        );
    }

    #[test]
    fn scaffold_credit_subtracts_cap() {
        // A code example with language tag + adjacent prose → non-zero
        // credit. MCC should be lower than positive.
        let src = "# Example\n\nThis shows how to print:\n\n```rust\nfn main() { println!(\"hi\"); }\n```\n\nThat prints `hi`.\n";
        let tree = parse(src);
        let root = crate::legacy_node::Node(tree.root_node());
        let r = compute_mcc(&root, src);
        assert!(r.credit_used > 0.0, "credit should apply");
        assert!(r.mcc < r.positive, "{} !< {}", r.mcc, r.positive);
        assert!(r.credit_used <= 0.25 * r.positive + 1e-9);
    }
}
