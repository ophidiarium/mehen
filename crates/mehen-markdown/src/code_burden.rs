// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Per-fence code burden per §14.1.
//!
//! For each fenced code block, we compute:
//!
//! ```text
//! CodeFenceBurden(c) =
//!     1.0
//!   + 0.08 * max(0, LOC_c - 12)
//!   + 0.50 * sat(LOC_c; 40, 120)
//!   + 0.40 * sat(line_length_p95_c; 100, 180)
//!   + 1.50 * missing_language_tag
//!   + 0.00 * parser_error_if_language_supported    // Phase B wires this
//!   + 0.20 * code_cognitive_c                      // Phase B wires this
//!   + 0.05 * sqrt(code_halstead_volume_c)          // Phase B wires this
//! ```
//!
//! Phase C ships the shape-only terms (LOC, line length, missing tag). The
//! parser-error / cognitive / halstead terms stay zero until Phase B lands.

use crate::document::{CodeBlock, MarkdownDocument, is_diagram_language};
use crate::mathops::sat;
use crate::nearby::{BlockSpan, has_prose_within};

/// Per-fence summary used to populate ArtifactRecord rows and Phase D's
/// filler / grounding pipelines.
#[derive(Debug, Clone)]
pub(crate) struct CodeFence {
    pub(crate) start_line: u64,
    pub(crate) end_line: u64,
    pub(crate) language: Option<String>,
    pub(crate) loc: u64,
    pub(crate) has_language_tag: bool,
    pub(crate) has_nearby_prose: bool,
    pub(crate) burden: f64,
}

/// Records every fenced code block and skips diagram
/// fences (they are owned by `visuals.rs`). Returns a deterministic list
/// sorted by start_line.
pub(crate) fn analyze_code_fences(
    document: &MarkdownDocument,
    blocks: &[BlockSpan],
) -> Vec<CodeFence> {
    let mut out = document
        .code_blocks
        .iter()
        .filter_map(|block| build(block, blocks))
        .collect::<Vec<_>>();
    // Sort for determinism.
    out.sort_by_key(|a| a.start_line);
    out
}

fn build(block: &CodeBlock, blocks: &[BlockSpan]) -> Option<CodeFence> {
    if !block.is_fenced() {
        return None;
    }

    // Diagrams are handled in `visuals.rs`; skip them here so the burden
    // score isn't counted twice. We still leave the record in the artifact
    // list (see `analyzer.rs`), but filter at the §14.1 site.
    if let Some(lang) = block.language.as_deref()
        && is_diagram_language(lang)
    {
        return None;
    }

    // Body LOC excludes fence delimiters because the document fact stores
    // only the pulldown code-block text.
    let (body_loc, line_len_p95) = code_body_stats(&block.content);

    let has_nearby_prose = has_prose_within(blocks, block.start_line, block.end_line, 2);

    let has_language_tag = block.language.is_some();
    let missing_language_tag = if has_language_tag { 0.0 } else { 1.0 };
    let burden = 1.0
        + 0.08 * (body_loc as f64 - 12.0).max(0.0)
        + 0.50 * sat(body_loc as f64, 40.0, 120.0)
        + 0.40 * sat(line_len_p95, 100.0, 180.0)
        + 1.50 * missing_language_tag;

    Some(CodeFence {
        start_line: block.start_line,
        end_line: block.end_line,
        language: block.language.clone(),
        loc: body_loc,
        has_language_tag,
        has_nearby_prose,
        burden,
    })
}

fn code_body_stats(body: &str) -> (u64, f64) {
    let mut line_lengths: Vec<usize> = Vec::new();
    for line in body.lines() {
        line_lengths.push(line.chars().count());
    }
    let loc = line_lengths.len() as u64;
    if line_lengths.is_empty() {
        return (loc, 0.0);
    }
    // p95 = length at index ceil(0.95 * N) - 1.
    line_lengths.sort_unstable();
    let idx = ((0.95 * line_lengths.len() as f64).ceil() as usize)
        .saturating_sub(1)
        .min(line_lengths.len() - 1);
    (loc, line_lengths[idx] as f64)
}

#[cfg(test)]
mod tests {
    use crate::document::parse_document;

    use super::analyze_code_fences;

    #[test]
    fn fenced_code_burden_uses_pulldown_body_without_delimiters() {
        let document =
            parse_document("# T\n\nIntro.\n\n```Rust,no_run {#sample}\nfn main() {}\n```\n");

        let fences = analyze_code_fences(&document, &[]);

        assert_eq!(fences.len(), 1);
        assert_eq!(fences[0].start_line, 5);
        assert_eq!(fences[0].end_line, 7);
        assert_eq!(fences[0].language.as_deref(), Some("rust"));
        assert_eq!(fences[0].loc, 1);
        assert!(fences[0].has_language_tag);
    }

    #[test]
    fn diagram_fences_are_owned_by_visuals() {
        let document = parse_document("```mermaid\ngraph TD\n  A --> B\n```\n");

        let fences = analyze_code_fences(&document, &[]);

        assert!(fences.is_empty());
    }
}
