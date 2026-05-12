//! Public-shaped metric types exported by the Markdown analyzer.
//!
//! The field layout mirrors §23 of
//! `docs/mehen_markdown_metrics_research_foundation.md`. Phase A (LOC family,
//! word count, section count, heading count, Effective Content Units) is
//! stable; Phase C (this module) adds `links`, `visuals`, `tables`,
//! `maintainability.artifact_debt_score`, and a per-artifact detail list.
//! Later phases append more fields; no field ever shrinks.

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

/// §11.1 link classifications. Each link is assigned exactly one primary
/// class; `is_image` and `is_bare_url` ride along on [`LinkRecord`] so the
/// aggregate breakdown in `links.*` does not double-count.
///
/// Serialized as snake_case to align with the §23 schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LinkClass {
    /// `#anchor` into the current document.
    Internal,
    /// Relative filesystem path (possibly with a fragment).
    Relative,
    /// Absolute URL pointing at the same repo on the same host.
    AbsoluteSameRepo,
    /// Any other absolute URL not captured by more specific classes.
    External,
    /// Subset of external targeting well-known vendor/API docs.
    ExternalVendor,
    /// DOI / arXiv / RFC / W3C / PubMed — scholarly/standards refs.
    Scholarly,
    /// GitHub/GitLab issues / pulls, Jira, Linear — traceability links.
    IssuePr,
    /// Reference to a footnote definition in the same doc.
    Footnote,
    /// A `link_reference_definition` block — the `[abc]: url` anchor entry.
    ReferenceDefinition,
}

/// Details about a single link / image / autolink / footnote reference /
/// reference definition encountered in the doc. Used to compute aggregate
/// link scores and to seed per-artifact detail rows later.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct LinkRecord {
    /// One-based start row of the link span in the source.
    pub(crate) line: u64,
    /// Primary classification per §11.1.
    pub(crate) class: LinkClass,
    /// The raw link destination / URI.
    pub(crate) destination: String,
    /// Visible link text (empty for reference definitions / footnote refs).
    pub(crate) text: String,
    /// True when the link wraps an image (`![alt](url)`).
    pub(crate) is_image: bool,
    /// True when the visible link text is a bare raw URL.
    pub(crate) is_bare_url: bool,
    /// Resolution status:
    /// - `Some(true)`  — relative or internal target that resolves.
    /// - `Some(false)` — relative or internal target that does NOT resolve.
    /// - `None`        — external link, never checked by default.
    pub(crate) resolved: Option<bool>,
}

/// Aggregate link metrics (§11.1–§11.4, §23).
#[derive(Debug, Default, Clone, Serialize)]
pub(crate) struct Links {
    pub(crate) total: u64,
    pub(crate) internal: u64,
    pub(crate) relative: u64,
    pub(crate) external: u64,
    pub(crate) external_vendor: u64,
    pub(crate) scholarly: u64,
    pub(crate) issue_pr: u64,
    pub(crate) absolute_same_repo: u64,
    pub(crate) image: u64,
    pub(crate) footnote: u64,
    pub(crate) bare_url: u64,
    pub(crate) broken: u64,
    pub(crate) link_debt_score: f64,
    pub(crate) information_scent_score: f64,
    pub(crate) review_burden: f64,
}

/// Per-table record used for §13 metrics.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct TableRecord {
    pub(crate) start_line: u64,
    pub(crate) end_line: u64,
    pub(crate) rows: u64,
    pub(crate) cols: u64,
    pub(crate) cells: u64,
    pub(crate) has_header: bool,
    pub(crate) empty_rate: f64,
    pub(crate) distinct_alignments: u64,
    pub(crate) has_local_explanation: bool,
    pub(crate) burden: f64,
    pub(crate) scaffold: f64,
    pub(crate) hard_warning: bool,
}

/// Aggregate table metrics (§13, §23).
#[derive(Debug, Default, Clone, Serialize)]
pub(crate) struct Tables {
    pub(crate) count: u64,
    pub(crate) max_cells: u64,
    pub(crate) table_burden_score: f64,
    pub(crate) table_scaffold_score: f64,
    pub(crate) hard_warnings: u64,
}

/// Per-diagram record used for §12.2.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct DiagramRecord {
    pub(crate) start_line: u64,
    pub(crate) end_line: u64,
    pub(crate) language: String,
    pub(crate) nodes: u64,
    pub(crate) edges: u64,
    pub(crate) components: u64,
    pub(crate) cycles: u64,
    pub(crate) parse_error: bool,
    pub(crate) has_title_or_caption: bool,
    pub(crate) complexity: f64,
}

/// Per-image record used for §12.1 / §12.3.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ImageRecord {
    pub(crate) line: u64,
    pub(crate) destination: String,
    pub(crate) alt_text: String,
    pub(crate) has_alt_or_caption: bool,
    pub(crate) has_nearby_reference: bool,
    pub(crate) bounded_size: f64,
    pub(crate) repo_resolved: bool,
    pub(crate) image_complexity: f64,
    pub(crate) scaffold: f64,
}

/// Aggregate visual metrics (§12, §23).
#[derive(Debug, Default, Clone, Serialize)]
pub(crate) struct Visuals {
    pub(crate) images: u64,
    pub(crate) diagrams: u64,
    pub(crate) diagram_nodes_total: u64,
    pub(crate) diagram_edges_total: u64,
    pub(crate) diagram_cycles_total: u64,
    pub(crate) visual_scaffold_score: f64,
    pub(crate) visual_net_effect: f64,
}

/// Maintainability aggregate per §23.
///
/// Phase A / Phase C only fills `artifact_debt_score` (§19). The other two
/// fields are reserved for forward compatibility with Phase B's DMI wiring
/// and Phase E's section balance score so the on-disk YAML / JSON shape
/// never needs to rename keys.
#[derive(Debug, Default, Clone, Serialize)]
pub(crate) struct Maintainability {
    pub(crate) documentation_maintainability_index: f64,
    pub(crate) section_balance_score: f64,
    pub(crate) artifact_debt_score: f64,
}

/// Which kind of artifact this detail row represents. Serialized as
/// snake_case to align with §23.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ArtifactKind {
    Code,
    Table,
    Diagram,
    Image,
    Math,
    Html,
}

/// Per-artifact row used by Phase D metrics (filler risk, RCI) and the §23
/// exported schema.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct ArtifactRecord {
    pub(crate) id: u64,
    pub(crate) kind: ArtifactKind,
    pub(crate) start_line: u64,
    pub(crate) end_line: u64,
    pub(crate) language_tag: Option<String>,
    /// For code: LOC. For table: cells. For diagram: nodes. For math: tokens.
    /// For image / html: line count.
    pub(crate) size: u64,
    pub(crate) has_explanation: bool,
    pub(crate) has_label: bool,
    pub(crate) oversized: bool,
    /// Per-artifact cognitive cost metric (§14.1 code fence burden,
    /// §12.2 diagram complexity, §13.1 table burden, §14.3 math burden).
    /// For images we store the image_complexity; for html we store the
    /// raw line count as a weak proxy.
    pub(crate) burden: f64,
}

/// Phase-A + Phase-C Markdown metric output.
///
/// Emitted per file on the JSON / YAML / TOML path and under the `markdown`
/// key of the exported schema so later phases can add sibling keys like
/// `complexity`, `grounding`, etc., without renames.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MarkdownMetrics {
    /// The analyzed file's relative or absolute path, as seen by the CLI.
    pub(crate) path: String,
    pub(crate) loc: LocFamily,
    pub(crate) loc_ratios: LocRatios,
    pub(crate) size: Size,
    pub(crate) ecu_inputs: EcuInputs,
    pub(crate) sections: Vec<Section>,
    pub(crate) links: Links,
    pub(crate) visuals: Visuals,
    pub(crate) tables: Tables,
    pub(crate) maintainability: Maintainability,
    pub(crate) artifacts: Vec<ArtifactRecord>,
}
