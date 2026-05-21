// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Derived section tree per §3.4.
//!
//! The tree-sitter-markdown-text grammar produces a natively nested `section`
//! AST: each heading opens a `section` that contains all downstream blocks
//! until the next same-or-higher-level heading. Heading skips (e.g. H1 → H3)
//! keep the intervening depth collapsed — the grammar does *not* synthesize
//! virtual sections. This module flattens that tree into the
//! [`crate::types::Section`] list consumed by the exported schema.
//!
//! Parent/child relationships are preserved by walking in pre-order and
//! emitting the parent section before its children. This matches §3.4
//! which requires a `parent_section_id` pointing to the enclosing heading's
//! section and a `child_section_ids` list of directly-nested subsections.

use crate::grammar::Markdown;
use crate::legacy_node::Node;
use crate::types::Section;
use crate::words::count_words;

/// Collects sections (one per heading) in document order.
///
/// §3.4 defines the derived section tree as *one section per heading*. A
/// document with no headings returns an empty list. Pre-heading content
/// is accounted for in `size.words` but has no section of its own.
///
/// Internally we keep a synthetic "file" placeholder so the tree walk can
/// attribute pre-heading content and preserve parent/child ids during
/// construction; that placeholder is dropped and the remaining sections
/// are renumbered before returning to the caller.
pub(crate) fn collect_sections(root: &Node<'_>) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();

    // Synthetic root used only during walk. Dropped before return.
    sections.push(Section {
        section_id: 0,
        heading_level: None,
        heading_text: None,
        start_line: (root.start_row() as u64) + 1,
        end_line: section_end_line(root),
        parent_section_id: None,
        child_section_ids: Vec::new(),
        word_count: 0,
        block_count: 0,
    });

    walk(root, 0, &mut sections);

    populate_word_and_block_counts(root, &mut sections);

    // Strip the synthetic root and renumber remaining sections so the
    // exported `sections` array reflects only heading-rooted sections with
    // contiguous ids starting at 0.
    sections.remove(0);
    renumber_sections(&mut sections);

    sections
}

/// Renumbers `sections` so `section_id` is the array index and every
/// `parent_section_id` / `child_section_ids` entry refers to the renumbered
/// ids. Sections whose parent was the dropped synthetic root become
/// top-level (`parent_section_id = None`).
fn renumber_sections(sections: &mut [Section]) {
    // Map old section_id -> new index. Since the synthetic root lived at
    // id 0, every remaining section's old id is >= 1. The new order is
    // the current vector order.
    let old_to_new: std::collections::HashMap<usize, usize> = sections
        .iter()
        .enumerate()
        .map(|(new_idx, s)| (s.section_id, new_idx))
        .collect();

    for (new_idx, section) in sections.iter_mut().enumerate() {
        section.section_id = new_idx;
        section.parent_section_id = match section.parent_section_id {
            Some(0) | None => None,
            Some(old_parent) => old_to_new.get(&old_parent).copied(),
        };
        section.child_section_ids = section
            .child_section_ids
            .iter()
            .filter_map(|old_id| old_to_new.get(old_id).copied())
            .collect();
    }
}

fn walk(node: &Node<'_>, parent_id: usize, sections: &mut Vec<Section>) {
    let mut cursor = node.cursor();
    if !cursor.goto_first_child() {
        return;
    }
    loop {
        let child = cursor.node();
        let kind: Markdown = child.kind_id().into();
        if is_section_node(&kind) {
            if let Some(heading) = find_heading_in_section(&child) {
                let (level, heading_text) = {
                    let (lvl, txt) = describe_heading(&heading);
                    (Some(lvl), txt)
                };
                let section_id = sections.len();
                // The grammar already parents H1 → H2 → H3 correctly. Heading
                // skips (H1 → H3) keep the H3 under whichever section wraps
                // it — we do not fabricate virtual sections.
                sections[parent_id].child_section_ids.push(section_id);
                sections.push(Section {
                    section_id,
                    heading_level: level,
                    heading_text,
                    start_line: (child.start_row() as u64) + 1,
                    end_line: section_end_line(&child),
                    parent_section_id: Some(parent_id),
                    child_section_ids: Vec::new(),
                    word_count: 0,
                    block_count: 0,
                });
                walk(&child, section_id, sections);
            } else {
                // A `section` node without a heading is a grammar artifact
                // (empty or pre-heading wrapper). Recurse into it but treat
                // its content as belonging to the enclosing section.
                walk(&child, parent_id, sections);
            }
        } else {
            // Non-section nodes can still contain sections (e.g. when a
            // block is between sections), so recurse.
            walk(&child, parent_id, sections);
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

fn is_section_node(kind: &Markdown) -> bool {
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

fn find_heading_in_section<'a>(section: &Node<'a>) -> Option<Node<'a>> {
    let mut cursor = section.cursor();
    if !cursor.goto_first_child() {
        return None;
    }
    loop {
        let child = cursor.node();
        let kind: Markdown = child.kind_id().into();
        if is_atx_heading(&kind) || is_setext_heading(&kind) {
            return Some(child);
        }
        if !cursor.goto_next_sibling() {
            break;
        }
    }
    None
}

fn describe_heading(heading: &Node<'_>) -> (u8, Option<String>) {
    let kind: Markdown = heading.kind_id().into();
    let level = if is_atx_heading(&kind) {
        atx_level(heading).unwrap_or(1)
    } else if is_setext_heading(&kind) {
        setext_level(heading).unwrap_or(1)
    } else {
        1
    };
    let text = heading.child_by_field_name("heading_content").map(|node| {
        let start = node.start_byte();
        let end = node.end_byte();
        let _ = (start, end);
        // Heading text extraction from source bytes is Phase-B territory
        // (needed for information-scent / RCI). Phase A leaves it as `None`
        // until the source-bytes-aware constructor lands.
        String::new()
    });
    // Drop the empty string — return `None` to preserve semantic meaning.
    let text = text.filter(|s| !s.is_empty());
    (level, text)
}

fn atx_level(heading: &Node<'_>) -> Option<u8> {
    let level = heading.child_by_field_name("level")?;
    Some(match level.kind_id().into() {
        Markdown::AtxH1Marker => 1,
        Markdown::AtxH2Marker => 2,
        Markdown::AtxH3Marker => 3,
        Markdown::AtxH4Marker => 4,
        Markdown::AtxH5Marker => 5,
        Markdown::AtxH6Marker => 6,
        _ => return None,
    })
}

fn setext_level(heading: &Node<'_>) -> Option<u8> {
    let level = heading.child_by_field_name("level")?;
    Some(match level.kind_id().into() {
        Markdown::SetextH1Underline => 1,
        Markdown::SetextH2Underline => 2,
        _ => return None,
    })
}

fn section_end_line(section: &Node<'_>) -> u64 {
    let (end_row, end_col) = section.end_position();
    let end = if end_col == 0 && end_row > section.start_row() {
        end_row - 1
    } else {
        end_row
    };
    (end as u64) + 1
}

fn populate_word_and_block_counts(root: &Node<'_>, sections: &mut [Section]) {
    if sections.is_empty() {
        return;
    }

    // Block counts: count paragraph / list / table / code / html / math /
    // callout / thematic-break / image-block blocks per section range. Since
    // the grammar already nests blocks inside the correct section, walking
    // each section's subtree yields the right count.
    //
    // Word counts: each section's subtree minus nested sub-section subtrees
    // to avoid double-counting. This is achieved by computing the subtree
    // word count, then subtracting the children's subtree counts.

    // Root section: every block and every word in the document.
    // We compute the root's subtree first, then per-sub-section.
    let mut subtree_words: Vec<u64> = vec![0; sections.len()];
    let mut subtree_blocks: Vec<u64> = vec![0; sections.len()];

    // For the root "document" section (id 0), traverse the whole tree.
    subtree_words[0] = count_words(root);
    subtree_blocks[0] = count_blocks(root);

    // For every other section, find its subtree by matching its start/end
    // line range against the tree.
    for s in sections.iter().skip(1) {
        if let Some(node) = find_section_node(root, s.start_line, s.end_line) {
            subtree_words[s.section_id] = count_words(&node);
            subtree_blocks[s.section_id] = count_blocks(&node);
        }
    }

    // Convert subtree counts → own counts (subtree minus children).
    let child_ids: Vec<Vec<usize>> = sections
        .iter()
        .map(|s| s.child_section_ids.clone())
        .collect();
    for (i, section) in sections.iter_mut().enumerate() {
        let mut words_own = subtree_words[i];
        let mut blocks_own = subtree_blocks[i];
        for &c in &child_ids[i] {
            words_own = words_own.saturating_sub(subtree_words[c]);
            blocks_own = blocks_own.saturating_sub(subtree_blocks[c]);
        }
        section.word_count = words_own;
        section.block_count = blocks_own;
    }
}

fn count_blocks(node: &Node<'_>) -> u64 {
    let mut total: u64 = 0;
    visit_blocks(node, &mut total);
    total
}

fn visit_blocks(node: &Node<'_>, total: &mut u64) {
    let kind: Markdown = node.kind_id().into();
    if is_block(&kind) {
        *total += 1;
    }
    let mut cursor = node.cursor();
    if cursor.goto_first_child() {
        loop {
            visit_blocks(&cursor.node(), total);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn is_block(kind: &Markdown) -> bool {
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
    )
}

/// Locates the AST node whose start/end lines match a section's recorded
/// range. The section walk is small so a linear search is fine.
fn find_section_node<'a>(root: &Node<'a>, start_line: u64, end_line: u64) -> Option<Node<'a>> {
    let mut stack = vec![*root];
    while let Some(node) = stack.pop() {
        let (s_row, _) = node.start_position();
        let (e_row, e_col) = node.end_position();
        let s = (s_row as u64) + 1;
        let mut e = (e_row as u64) + 1;
        if e_col == 0 && e > s {
            e -= 1;
        }
        let kind: Markdown = node.kind_id().into();
        if is_section_node(&kind) && s == start_line && e == end_line {
            return Some(node);
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
    None
}
