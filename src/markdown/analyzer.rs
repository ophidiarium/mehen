//! Top-level Markdown analysis entry point.
//!
//! Parses a Markdown source buffer with the tree-sitter-markdown-text grammar
//! and produces a [`MarkdownMetrics`] record covering:
//!
//! - §5 LOC family,
//! - §4 word count `W`,
//! - §3.4 section tree,
//! - §6 Effective Content Units,
//! - §7 Markdown Reading Path Complexity (weighted + raw),
//! - §8 Markdown Cognitive Complexity,
//! - §9 Markdown Halstead + §9.4 embedded-code adjustment,
//! - §10 Documentation Maintainability Index core (V/M/R terms),
//! - §§29–38 language-aware prose layer.
//!
//! Phases C/D append link, table, visual, grounding, evidence, filler, and
//! review surfaces; they must not rewrite existing fields. The prose layer
//! is kept strictly separate per §29.1 — it never modifies any structural
//! score.

use std::path::Path;

use crate::markdown::dmi::{DmiInputs, compute_dmi};
use crate::markdown::ecu::{compute_ecu_inputs, effective_content_units};
use crate::markdown::embedded_code::embedded_volume;
use crate::markdown::halstead::compute_halstead;
use crate::markdown::loc::{LineClasses, derive_ratios, physical_line_count};
use crate::markdown::mcc::compute_mcc;
use crate::markdown::mrpc::compute_mrpc;
use crate::markdown::prose::analyze_prose;
use crate::markdown::sections::collect_sections;
use crate::markdown::types::{Complexity, Maintainability, MarkdownMetrics, Size};
use crate::markdown::words::count_words;
use tree_sitter::Parser as TsParser;

/// Parses `source` as Markdown and returns a metric record covering
/// Phase A, Phase B, and Phase E.
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

    // §3.4: the derived section tree has one section per heading. No
    // synthetic root is exported, so `sections.len()` is the heading count.
    let heading_sections = sections.len() as u64;
    let headings = heading_sections;

    let ecu_inputs = compute_ecu_inputs(&root, &classes);
    let ecu = effective_content_units(&loc, words, &ecu_inputs);

    // Phase-B complexity surface.
    let mrpc = compute_mrpc(&root, source);
    let mcc = compute_mcc(&root, source);
    let mut halstead = compute_halstead(&root, source);
    let emb = embedded_volume(&root, source);
    halstead.embedded_volume = emb;
    halstead.total_volume = halstead.volume + emb;

    let dmi = compute_dmi(DmiInputs {
        mrpc: mrpc.weighted,
        mcc: mcc.mcc,
        total_volume: halstead.total_volume,
    });

    // §§29–38 Prose layer. Kept strictly separate per §29.1 — it never
    // modifies any structural score.
    let prose = analyze_prose(&root, source.as_bytes());

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
        complexity: Complexity {
            reading_path_complexity: mrpc.weighted,
            reading_path_complexity_raw: mrpc.raw,
            cognitive_complexity: mcc.mcc,
            halstead,
        },
        maintainability: Maintainability {
            documentation_maintainability_index: dmi,
        },
        prose,
    }
}
