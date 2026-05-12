//! Public-shaped metric types exported by the Markdown analyzer.
//!
//! The field layout mirrors §23 of
//! `docs/mehen_markdown_metrics_research_foundation.md`. Phase A produced the
//! LOC family, word count, section count, heading count, and Effective
//! Content Units. Phase B adds `complexity` (MRPC, MCC, Halstead) and
//! `maintainability` (DMI core). Phase E adds the language-aware prose metric
//! surface (§§29–38) as a separate top-level `prose` key. Later phases append
//! more fields; no field ever shrinks.

use serde::Serialize;

/// The LOC family described in §5.
///
/// - `dloc`: total physical lines in the file.
/// - `ploc`: prose physical lines (paragraph / heading / blockquote content).
/// - `cloc`: code-fence and indented-code lines, including fence markers.
/// - `tloc`: pipe-table lines (header, delimiter, rows).
/// - `mloc`: math-block lines (including `$$` delimiters).
/// - `bloc`: blank lines.
/// - `aloc`: artifact lines = cloc + tloc + mloc + raw HTML / MDX / directive
///   lines + front-matter lines. A line is counted in exactly one bucket.
#[derive(Debug, Default, Clone, Serialize)]
pub(crate) struct LocFamily {
    pub(crate) dloc: u64,
    pub(crate) ploc: u64,
    pub(crate) cloc: u64,
    pub(crate) tloc: u64,
    pub(crate) mloc: u64,
    pub(crate) bloc: u64,
    pub(crate) aloc: u64,
}

/// Derived LOC ratios per §5.1. Each is clamped to `[0.0, 1.0]`.
#[derive(Debug, Default, Clone, Serialize)]
pub(crate) struct LocRatios {
    pub(crate) artifact_line_ratio: f64,
    pub(crate) code_line_ratio: f64,
    pub(crate) table_line_ratio: f64,
    pub(crate) math_line_ratio: f64,
    pub(crate) blank_line_ratio: f64,
}

/// High-level size metrics described in §4, §6, and §23.
///
/// `words` is the narrative word count `W` (§4, anti-gaming §37.5 applied).
/// `effective_content_units` is ECU per §6.
#[derive(Debug, Default, Clone, Serialize)]
pub(crate) struct Size {
    pub(crate) words: u64,
    pub(crate) effective_content_units: f64,
    pub(crate) sections: u64,
    pub(crate) headings: u64,
}

/// One derived section per §3.4.
///
/// Parent / child IDs use indices into the surrounding `Vec<Section>`;
/// top-level sections have `parent_section_id = None`.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct Section {
    pub(crate) section_id: usize,
    pub(crate) heading_level: Option<u8>,
    pub(crate) heading_text: Option<String>,
    pub(crate) start_line: u64,
    pub(crate) end_line: u64,
    pub(crate) parent_section_id: Option<usize>,
    pub(crate) child_section_ids: Vec<usize>,
    pub(crate) word_count: u64,
    pub(crate) block_count: u64,
}

/// Counts that feed ECU per §6. Kept separate from `Size` to keep the public
/// surface close to §23 while letting the aggregator pick only what it needs.
#[derive(Debug, Default, Clone, Serialize)]
pub(crate) struct EcuInputs {
    pub(crate) table_cells: u64,
    pub(crate) diagram_nodes: u64,
    pub(crate) diagram_edges: u64,
    pub(crate) math_tokens: u64,
    pub(crate) raw_html_or_mdx_lines: u64,
}

/// Markdown Halstead sub-metrics per §9.
///
/// `operators_*` / `operands_*` match the text's `n1`, `N1`, `n2`, `N2`.
/// `vocabulary` = n1 + n2, `length` = N1 + N2. `volume`, `difficulty`, and
/// `effort` are derived per §9.3. `embedded_volume` is the §9.4 sum over
/// supported code fences; `total_volume` = `volume + embedded_volume`.
#[derive(Debug, Default, Clone, Serialize)]
pub(crate) struct Halstead {
    pub(crate) operators_distinct: u64,
    pub(crate) operators_total: u64,
    pub(crate) operands_distinct: u64,
    pub(crate) operands_total: u64,
    pub(crate) vocabulary: u64,
    pub(crate) length: u64,
    pub(crate) volume: f64,
    pub(crate) difficulty: f64,
    pub(crate) effort: f64,
    pub(crate) embedded_volume: f64,
    pub(crate) total_volume: f64,
}

/// Complexity aggregate exported under the §23 `complexity` key.
///
/// `reading_path_complexity` is the §7.3 weighted MRPC (with weights from the
/// edge-type table); `reading_path_complexity_raw` is the §7.2 unweighted
/// graph form `|E| - |N| + 2P`, surfaced for auditability.
/// `cognitive_complexity` is §8's final MCC after scaffold credit is applied.
#[derive(Debug, Default, Clone, Serialize)]
pub(crate) struct Complexity {
    pub(crate) reading_path_complexity: f64,
    pub(crate) reading_path_complexity_raw: f64,
    pub(crate) cognitive_complexity: f64,
    pub(crate) halstead: Halstead,
}

/// Maintainability aggregate per §23.
///
/// `documentation_maintainability_index` is the §10.2 DMI on `[0, 100]`
/// scale. Phase B wires the V / M / R components; L / T / A / S / F / G
/// terms stay at zero until Phases C and D land — see the TODO comments in
/// `dmi.rs`.
#[derive(Debug, Default, Clone, Serialize)]
pub(crate) struct Maintainability {
    pub(crate) documentation_maintainability_index: f64,
}

/// Phase-A + Phase-B + Phase-E Markdown metric output.
///
/// Emitted per file on the JSON / YAML / TOML path and under the `markdown`
/// key of the exported schema so later phases can add sibling keys like
/// `links`, `grounding`, etc., without renames.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MarkdownMetrics {
    /// The analyzed file's relative or absolute path, as seen by the CLI.
    pub(crate) path: String,
    pub(crate) loc: LocFamily,
    pub(crate) loc_ratios: LocRatios,
    pub(crate) size: Size,
    pub(crate) ecu_inputs: EcuInputs,
    pub(crate) sections: Vec<Section>,
    pub(crate) complexity: Complexity,
    pub(crate) maintainability: Maintainability,
    /// §§29–38 Prose metric layer. Always emitted; its presence does NOT
    /// modify DMI / MCC / MRPC / FillerLazyRisk in later phases.
    pub(crate) prose: crate::markdown::prose::ProseReport,
}
