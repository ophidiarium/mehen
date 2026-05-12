//! Top-level Markdown analysis entry point.
//!
//! Parses a Markdown source buffer with the tree-sitter-markdown-text grammar
//! and produces a [`MarkdownMetrics`] record covering the §5 LOC family, §4
//! word count, §3.4 section tree, and §6 Effective Content Units. Later
//! phases extend this module (links, tables, visuals, MRPC, MCC, DMI, prose
//! layer) but must not rewrite Phase-A fields.

use std::path::Path;

use crate::markdown::ecu::{compute_ecu_inputs, effective_content_units};
use crate::markdown::loc::{LineClasses, derive_ratios, physical_line_count};
use crate::markdown::sections::collect_sections;
use crate::markdown::types::{MarkdownMetrics, Size};
use crate::markdown::words::count_words;
use tree_sitter::Parser as TsParser;

/// Parses `source` as Markdown and returns the Phase-A metric record.
///
/// `path` is recorded verbatim into the output's `path` field; the caller
/// controls whether it is absolute or relative.
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

    // `sections[0]` is the synthetic root; subtract it from the counted
    // sections so the `sections` total reflects *heading-rooted* sections
    // only (§3.4 uses "derived section tree" to mean heading sections).
    let heading_sections = sections
        .iter()
        .filter(|s| s.heading_level.is_some())
        .count() as u64;
    let headings = heading_sections; // one heading per section in this grammar.

    let ecu_inputs = compute_ecu_inputs(&root, &classes);
    let ecu = effective_content_units(&loc, words, &ecu_inputs);

    MarkdownMetrics {
        path: path.to_string_lossy().to_string(),
        loc,
        loc_ratios,
        size: Size {
            words,
            effective_content_units: ecu,
            sections: heading_sections,
            headings,
        },
        ecu_inputs,
        sections,
    }
}
