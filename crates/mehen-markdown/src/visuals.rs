// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Visual and diagram metrics per §12.
//!
//! Visuals are either images (`image` or `image_block`) or diagrams (fenced
//! code blocks with a recognized diagram language tag: `mermaid`,
//! `plantuml`, `dot`, `graphviz`, `d2`, `vega-lite`). For each, we compute
//! alt-text/caption presence, nearby-reference presence, bounded-size
//! credit, and repo-resolution. The aggregate `VisualScaffoldScore` and
//! `VisualNetEffect` emit under `visuals.*` in the exported schema.

use std::path::{Path, PathBuf};

use crate::diagrams::{DiagramSignal, parse_diagram};
use crate::grammar::Markdown;
use crate::mathops::{clamp01, normalize_zero, sat};
use crate::nearby::{BlockSpan, has_prose_within};
use crate::syntax_tree::Node;
use crate::tree_helpers::{fence_content_text, find_first, node_text};
use crate::types::{DiagramRecord, ImageRecord, Visuals};

/// Combined visual analysis output.
pub(crate) struct VisualAnalysis {
    pub(crate) images: Vec<ImageRecord>,
    pub(crate) diagrams: Vec<DiagramRecord>,
    pub(crate) aggregate: Visuals,
}

/// Walks the tree, produces per-image and per-diagram records, and
/// computes the aggregate visual metrics.
pub(crate) fn analyze_visuals(
    root: &Node<'_>,
    source: &str,
    file_path: &Path,
    words: u64,
    blocks: &[BlockSpan],
) -> VisualAnalysis {
    let base_dir = file_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let mut images: Vec<ImageRecord> = Vec::new();
    let mut diagrams: Vec<DiagramRecord> = Vec::new();

    walk(root, source, &base_dir, blocks, &mut images, &mut diagrams);

    // Sort for determinism.
    images.sort_by(|a, b| a.line.cmp(&b.line).then(a.destination.cmp(&b.destination)));
    diagrams.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then(a.language.cmp(&b.language))
    });

    let aggregate = aggregate_visuals(&images, &diagrams, words);

    VisualAnalysis {
        images,
        diagrams,
        aggregate,
    }
}

fn walk(
    node: &Node<'_>,
    source: &str,
    base_dir: &Path,
    blocks: &[BlockSpan],
    images: &mut Vec<ImageRecord>,
    diagrams: &mut Vec<DiagramRecord>,
) {
    let kind: Markdown = node.kind_id().into();
    match kind {
        Markdown::Image => {
            if let Some(rec) = record_image(node, source, base_dir, blocks) {
                images.push(rec);
            }
            return;
        }
        Markdown::ImageBlock => {
            // Image-block is a stand-alone image — its parent is the
            // document, not a paragraph. Treat it identically.
            if let Some(rec) = record_image(node, source, base_dir, blocks) {
                images.push(rec);
            }
            return;
        }
        Markdown::FencedCodeBlock => {
            if let Some(rec) = record_diagram(node, source, blocks) {
                diagrams.push(rec);
                return;
            }
        }
        _ => {}
    }
    let mut cursor = node.cursor();
    if cursor.goto_first_child() {
        loop {
            walk(&cursor.node(), source, base_dir, blocks, images, diagrams);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn record_image(
    node: &Node<'_>,
    source: &str,
    base_dir: &Path,
    blocks: &[BlockSpan],
) -> Option<ImageRecord> {
    let line = (node.start_row() as u64) + 1;
    let destination = find_first(node, Markdown::LinkDestination).map(|n| node_text(&n, source))?;
    let alt_text = find_first(node, Markdown::LinkLabel)
        .map(|n| {
            let s = node_text(&n, source);
            s.trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .to_string()
        })
        .unwrap_or_default();
    let has_title = find_first(node, Markdown::LinkTitle).is_some();
    let has_alt_or_caption = !alt_text.trim().is_empty() || has_title;

    let end_line = line;
    let has_nearby_reference = has_prose_within(blocks, line, end_line, 2);

    let repo_resolved = if is_absolute_url(&destination) {
        // Per spec: external stays unchecked. We still treat it as resolved
        // for the V_scaffold multiplier because the network check is opt-in.
        true
    } else {
        base_dir.join(&destination).exists()
    };

    // image_complexity is a 0..1 proxy for visual weight. In lieu of a size
    // check, use 1 - bounded_size(v): small alt-less images get a higher
    // complexity penalty. Because we have no size signal we default to 1.0
    // for the "unknown visual complexity" case. The bounded_size credit
    // multiplier used in V_scaffold captures the good-case reward.
    let image_complexity = 1.0;
    let bounded_size = 1.0 - sat(image_complexity, 20.0, 80.0);

    let alt_c = if has_alt_or_caption { 1.0 } else { 0.0 };
    let near_c = if has_nearby_reference { 1.0 } else { 0.0 };
    let repo_c = if repo_resolved { 1.0 } else { 0.0 };
    let scaffold = alt_c * near_c * bounded_size * repo_c;

    Some(ImageRecord {
        line,
        destination,
        alt_text,
        has_alt_or_caption,
        has_nearby_reference,
        bounded_size,
        repo_resolved,
        image_complexity,
        scaffold,
    })
}

fn record_diagram(node: &Node<'_>, source: &str, blocks: &[BlockSpan]) -> Option<DiagramRecord> {
    let info = find_first(node, Markdown::InfoString)
        .and_then(|n| find_first(&n, Markdown::Language))
        .map(|n| node_text(&n, source))
        .unwrap_or_default();
    let language = info.trim().to_ascii_lowercase();
    if !is_diagram_language(&language) {
        return None;
    }
    let body = fence_content_text(node, source).unwrap_or_default();

    let start_line = (node.start_row() as u64) + 1;
    let (end_row, end_col) = node.end_position();
    let mut end = end_row;
    if end > node.start_row() && end_col == 0 {
        end -= 1;
    }
    let end_line = (end as u64) + 1;

    let signal: DiagramSignal = parse_diagram(&language, &body);
    let has_nearby = has_prose_within(blocks, start_line, end_line, 2);
    let has_title_or_caption = signal.has_title || has_nearby;

    let missing_title = if has_title_or_caption { 0.0 } else { 1.0 };
    let parse_error = if signal.parse_error { 1.0 } else { 0.0 };
    let complexity = 0.40 * signal.nodes as f64
        + 0.55 * signal.edges as f64
        + 1.50 * signal.cycles as f64
        + 2.00 * parse_error
        + 1.00 * missing_title;

    Some(DiagramRecord {
        start_line,
        end_line,
        language,
        nodes: signal.nodes,
        edges: signal.edges,
        components: signal.components,
        cycles: signal.cycles,
        parse_error: signal.parse_error,
        has_title_or_caption,
        complexity,
    })
}

fn is_diagram_language(lang: &str) -> bool {
    matches!(
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
}

fn aggregate_visuals(images: &[ImageRecord], diagrams: &[DiagramRecord], words: u64) -> Visuals {
    let mut v = Visuals {
        images: images.len() as u64,
        diagrams: diagrams.len() as u64,
        diagram_nodes_total: diagrams.iter().map(|d| d.nodes).sum(),
        diagram_edges_total: diagrams.iter().map(|d| d.edges).sum(),
        diagram_cycles_total: diagrams.iter().map(|d| d.cycles).sum(),
        diagram_parse_error_count: diagrams.iter().filter(|d| d.parse_error).count() as u64,
        ..Visuals::default()
    };

    let diagram_scaffold: f64 = diagrams
        .iter()
        .map(|d| {
            let alt = if d.has_title_or_caption { 1.0 } else { 0.0 };
            let bounded = 1.0 - sat(d.complexity, 20.0, 80.0);
            // Repo-resolved for diagrams means "parsed successfully".
            let repo = if d.parse_error { 0.0 } else { 1.0 };
            // Diagrams are always considered to have a nearby reference if
            // their body contains a title.
            let near = alt;
            alt * near * bounded * repo
        })
        .sum();
    let image_scaffold: f64 = images.iter().map(|i| i.scaffold).sum();
    let scaffold_sum = diagram_scaffold + image_scaffold;
    let divisor = ((words as f64 / 500.0) + 1.0).sqrt().max(1.0);
    v.visual_scaffold_score = normalize_zero(clamp01(scaffold_sum / divisor));

    let diagram_total: f64 = diagrams.iter().map(|d| d.complexity).sum();
    let image_total: f64 = images.iter().map(|i| i.image_complexity).sum();
    v.visual_net_effect = normalize_zero(diagram_total + image_total - 2.0 * scaffold_sum);
    v
}

fn is_absolute_url(s: &str) -> bool {
    s.starts_with("http://")
        || s.starts_with("https://")
        || s.starts_with("data:")
        || s.starts_with("ftp://")
        || s.starts_with("mailto:")
}
