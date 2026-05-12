//! Top-level Markdown analysis entry point.
//!
//! Parses a Markdown source buffer with the tree-sitter-markdown-text grammar
//! and produces a [`MarkdownMetrics`] record covering:
//! - §5 LOC family
//! - §4 word count
//! - §3.4 section tree
//! - §6 Effective Content Units
//! - §11 link classification + debt + scent + review burden
//! - §12 visuals (images + diagrams)
//! - §13 tables
//! - §14.1 per-code-fence burden (stored in `artifacts`)
//! - §14.3 per-math-block burden (stored in `artifacts`)
//! - §19 artifact debt score
//!
//! Phase B (MRPC, MCC, Halstead, DMI) and Phase D (grounding, filler, RCI)
//! append more fields; no field ever shrinks.

use std::path::Path;

use crate::markdown::artifact_debt::{DebtInputs, artifact_debt_score};
use crate::markdown::code_burden::{CodeFence, analyze_code_fences};
use crate::markdown::ecu::{compute_ecu_inputs, effective_content_units};
use crate::markdown::links::analyze_links;
use crate::markdown::loc::{LineClasses, derive_ratios, physical_line_count};
use crate::markdown::math_burden::{MathBlock, analyze_math_blocks};
use crate::markdown::nearby::{BlockSpan, collect_blocks, has_prose_within};
use crate::markdown::sections::collect_sections;
use crate::markdown::tables::{aggregate_tables, analyze_tables};
use crate::markdown::types::{
    ArtifactKind, ArtifactRecord, DiagramRecord, ImageRecord, Maintainability, MarkdownMetrics,
    Size, TableRecord,
};
use crate::markdown::visuals::analyze_visuals;
use crate::markdown::words::count_words;
use crate::node::Node;
use tree_sitter::Parser as TsParser;

/// Parses `source` as Markdown and returns a metric record covering all of
/// Phase A and Phase C. `path` is recorded verbatim into the output's
/// `path` field; the caller controls whether it is absolute or relative.
pub(crate) fn analyze_markdown(source: &str, path: &Path) -> MarkdownMetrics {
    let mut parser = TsParser::new();
    parser
        .set_language(&tree_sitter_markdown_text::LANGUAGE.into())
        .expect("tree-sitter-markdown-text set_language failed");
    let tree = parser
        .parse(source.as_bytes(), None)
        .expect("tree-sitter-markdown-text parse failed");
    let ts_root = tree.root_node();
    let root = crate::node::Node(ts_root);

    let total_lines = physical_line_count(source);
    let classes = LineClasses::build(&root, total_lines);
    let loc = classes.loc_family();
    let loc_ratios = derive_ratios(&loc);

    let words = count_words(&root);
    let sections = collect_sections(&root);
    let heading_sections = sections.len() as u64;
    let headings = heading_sections;

    let ecu_inputs = compute_ecu_inputs(&root, &classes);
    let ecu = effective_content_units(&loc, words, &ecu_inputs);

    // Phase C: block index for nearby-prose queries.
    let blocks: Vec<BlockSpan> = collect_blocks(&root);

    // Phase C: links.
    // TODO(link-check): the `resolved = None` external links become a CLI
    // flag so `--link-check` probes external URLs. For now they stay
    // unchecked to keep analysis offline and deterministic.
    let (link_records, link_agg) = analyze_links(&root, source, path, &sections, &[]);

    // Phase C: visuals (images + diagrams).
    let visual_analysis = analyze_visuals(&root, source, path, words, &blocks);

    // Phase C: tables. Patch has_local_explanation afterwards via the
    // nearby-block index.
    let mut table_records = analyze_tables(&root, source);
    for t in &mut table_records {
        t.has_local_explanation = has_prose_within(&blocks, t.start_line, t.end_line, 2);
        // Recompute scaffold now that the explanation flag is known. The
        // per-table formula is a product of size_credit × has_header ×
        // has_local_explanation.
        if !t.has_local_explanation {
            t.scaffold = 0.0;
        }
    }
    let tables_agg = aggregate_tables(&table_records);

    // Phase C: code fences (skipping diagram-tagged fences which are owned
    // by visuals.rs).
    let code_fences: Vec<CodeFence> = analyze_code_fences(&root, source, &blocks);

    // Phase C: math blocks.
    let math_blocks: Vec<MathBlock> = analyze_math_blocks(&root, source, &blocks);

    // Diagram ECU contribution: feed diagram_nodes / diagram_edges now that
    // we have real counts. This fixes up §6 ECU without changing existing
    // code in `ecu.rs` (Phase A leaves zeros).
    let mut ecu_inputs_final = ecu_inputs.clone();
    ecu_inputs_final.diagram_nodes = visual_analysis.diagrams.iter().map(|d| d.nodes).sum();
    ecu_inputs_final.diagram_edges = visual_analysis.diagrams.iter().map(|d| d.edges).sum();
    let ecu_final = if ecu_inputs_final.diagram_nodes > 0 || ecu_inputs_final.diagram_edges > 0 {
        effective_content_units(&loc, words, &ecu_inputs_final)
    } else {
        ecu
    };

    // Phase C: unified artifact list — used by §19 and later by Phase D.
    let html_records = collect_html_blocks(&root);
    let artifacts = build_artifact_list(
        &code_fences,
        &table_records,
        &visual_analysis.diagrams,
        &visual_analysis.images,
        &math_blocks,
        &html_records,
        &blocks,
    );

    // Phase C: artifact debt score.
    let debt_inputs = DebtInputs {
        artifacts: &artifacts,
        links: &link_records,
        loc: &loc,
        raw_html_or_mdx_lines: ecu_inputs_final.raw_html_or_mdx_lines,
        diagram_parse_errors: visual_analysis
            .diagrams
            .iter()
            .filter(|d| d.parse_error)
            .count() as u64,
    };
    let artifact_debt = artifact_debt_score(&debt_inputs);

    MarkdownMetrics {
        path: path.to_string_lossy().to_string(),
        loc,
        loc_ratios,
        size: Size {
            words,
            effective_content_units: ecu_final,
            sections: heading_sections,
            headings,
        },
        ecu_inputs: ecu_inputs_final,
        sections,
        links: link_agg,
        visuals: visual_analysis.aggregate,
        tables: tables_agg,
        maintainability: Maintainability {
            documentation_maintainability_index: 0.0,
            section_balance_score: 0.0,
            artifact_debt_score: artifact_debt,
        },
        artifacts,
    }
}

#[derive(Debug, Clone)]
struct HtmlBlockRecord {
    start_line: u64,
    end_line: u64,
}

fn collect_html_blocks(root: &Node<'_>) -> Vec<HtmlBlockRecord> {
    let mut out: Vec<HtmlBlockRecord> = Vec::new();
    walk_html(root, &mut out);
    out.sort_by_key(|a| a.start_line);
    out
}

fn walk_html(node: &Node<'_>, out: &mut Vec<HtmlBlockRecord>) {
    use crate::languages::Markdown::*;
    let kind: crate::languages::Markdown = node.kind_id().into();
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
    ) {
        let start_line = (node.start_row() as u64) + 1;
        let (end_row, end_col) = node.end_position();
        let mut end = end_row;
        if end > node.start_row() && end_col == 0 {
            end -= 1;
        }
        let end_line = (end as u64) + 1;
        out.push(HtmlBlockRecord {
            start_line,
            end_line,
        });
        return;
    }
    let mut cursor = node.cursor();
    if cursor.goto_first_child() {
        loop {
            walk_html(&cursor.node(), out);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_artifact_list(
    code: &[CodeFence],
    tables: &[TableRecord],
    diagrams: &[DiagramRecord],
    images: &[ImageRecord],
    math: &[MathBlock],
    html: &[HtmlBlockRecord],
    blocks: &[BlockSpan],
) -> Vec<ArtifactRecord> {
    let mut out: Vec<ArtifactRecord> = Vec::new();

    for c in code {
        let oversized = c.loc > 120;
        out.push(ArtifactRecord {
            id: 0,
            kind: ArtifactKind::Code,
            start_line: c.start_line,
            end_line: c.end_line,
            language_tag: c.language.clone(),
            size: c.loc,
            has_explanation: c.has_nearby_prose,
            has_label: c.has_language_tag,
            oversized,
            burden: c.burden,
        });
    }

    for t in tables {
        let oversized = t.cells > 300;
        out.push(ArtifactRecord {
            id: 0,
            kind: ArtifactKind::Table,
            start_line: t.start_line,
            end_line: t.end_line,
            language_tag: None,
            size: t.cells,
            has_explanation: t.has_local_explanation,
            has_label: t.has_header,
            oversized,
            burden: t.burden,
        });
    }

    for d in diagrams {
        let oversized = d.nodes > 80;
        out.push(ArtifactRecord {
            id: 0,
            kind: ArtifactKind::Diagram,
            start_line: d.start_line,
            end_line: d.end_line,
            language_tag: Some(d.language.clone()),
            size: d.nodes,
            has_explanation: d.has_title_or_caption,
            has_label: d.has_title_or_caption,
            oversized,
            burden: d.complexity,
        });
    }

    for img in images {
        out.push(ArtifactRecord {
            id: 0,
            kind: ArtifactKind::Image,
            start_line: img.line,
            end_line: img.line,
            language_tag: None,
            size: 1,
            has_explanation: img.has_nearby_reference,
            has_label: img.has_alt_or_caption,
            oversized: false,
            burden: img.image_complexity,
        });
    }

    for m in math {
        let oversized = m.tokens > 50;
        out.push(ArtifactRecord {
            id: 0,
            kind: ArtifactKind::Math,
            start_line: m.start_line,
            end_line: m.end_line,
            language_tag: None,
            size: m.tokens,
            has_explanation: m.has_nearby_prose,
            has_label: false,
            oversized,
            burden: m.burden,
        });
    }

    for h in html {
        let size = h.end_line.saturating_sub(h.start_line) + 1;
        let has_explanation = has_prose_within(blocks, h.start_line, h.end_line, 2);
        out.push(ArtifactRecord {
            id: 0,
            kind: ArtifactKind::Html,
            start_line: h.start_line,
            end_line: h.end_line,
            language_tag: None,
            size,
            has_explanation,
            has_label: false,
            oversized: false,
            burden: size as f64,
        });
    }

    // Sort: start_line, then kind (lexicographic) for determinism, then
    // assign sequential ids.
    out.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then((a.kind as u8).cmp(&(b.kind as u8)))
            .then(a.end_line.cmp(&b.end_line))
    });
    for (i, rec) in out.iter_mut().enumerate() {
        rec.id = i as u64;
    }
    out
}
