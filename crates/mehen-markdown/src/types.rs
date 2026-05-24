// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Public-shaped metric types exported by the Markdown analyzer.
//!
//! The field layout mirrors §23 of
//! `docs/mehen_markdown_metrics_research_foundation.md`. Phase A produced the
//! LOC family, word count, section count, heading count, and Effective
//! Content Units. Phase B adds `complexity` (MRPC, MCC, Halstead) and
//! `maintainability.documentation_maintainability_index` (DMI core). Phase C
//! adds `links`, `visuals`, `tables`, `maintainability.artifact_debt_score`,
//! and a per-artifact detail list. Phase E adds the language-aware prose
//! metric surface (§§29–38) as a separate top-level `prose` key. Later phases
//! append more fields; no field ever shrinks.

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
pub struct LocFamily {
    pub dloc: u64,
    pub ploc: u64,
    pub cloc: u64,
    pub tloc: u64,
    pub mloc: u64,
    pub bloc: u64,
    pub aloc: u64,
}

/// Derived LOC ratios per §5.1. Each is clamped to `[0.0, 1.0]`.
#[derive(Debug, Default, Clone, Serialize)]
pub struct LocRatios {
    pub artifact_line_ratio: f64,
    pub code_line_ratio: f64,
    pub table_line_ratio: f64,
    pub math_line_ratio: f64,
    pub blank_line_ratio: f64,
}

/// High-level size metrics described in §4, §6, and §23.
///
/// `words` is the narrative word count `W` (§4, anti-gaming §37.5 applied).
/// `effective_content_units` is ECU per §6.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Size {
    pub words: u64,
    pub effective_content_units: f64,
    pub sections: u64,
    pub headings: u64,
}

/// One derived section per §3.4.
///
/// Parent / child IDs use indices into the surrounding `Vec<Section>`;
/// top-level sections have `parent_section_id = None`.
#[derive(Debug, Clone, Serialize)]
pub struct Section {
    pub section_id: usize,
    pub heading_level: Option<u8>,
    pub heading_text: Option<String>,
    pub start_line: u64,
    pub end_line: u64,
    pub parent_section_id: Option<usize>,
    pub child_section_ids: Vec<usize>,
    pub word_count: u64,
    pub block_count: u64,
}

/// Counts that feed ECU per §6. Kept separate from `Size` to keep the public
/// surface close to §23 while letting the aggregator pick only what it needs.
#[derive(Debug, Default, Clone, Serialize)]
pub struct EcuInputs {
    pub table_cells: u64,
    pub diagram_nodes: u64,
    pub diagram_edges: u64,
    pub math_tokens: u64,
    pub raw_html_or_mdx_lines: u64,
}

/// Markdown Halstead sub-metrics per §9.
///
/// `operators_*` / `operands_*` match the text's `n1`, `N1`, `n2`, `N2`.
/// `vocabulary` = n1 + n2, `length` = N1 + N2. `volume`, `difficulty`, and
/// `effort` are derived per §9.3. `embedded_volume` is the §9.4 sum over
/// supported code fences; `total_volume` = `volume + embedded_volume`.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Halstead {
    pub operators_distinct: u64,
    pub operators_total: u64,
    pub operands_distinct: u64,
    pub operands_total: u64,
    pub vocabulary: u64,
    pub length: u64,
    pub volume: f64,
    pub difficulty: f64,
    pub effort: f64,
    pub embedded_volume: f64,
    pub total_volume: f64,
}

/// Complexity aggregate exported under the §23 `complexity` key.
///
/// `reading_path_complexity` is the §7.3 weighted MRPC (with weights from the
/// edge-type table); `reading_path_complexity_raw` is the §7.2 unweighted
/// graph form `|E| - |N| + 2P`, surfaced for auditability.
/// `cognitive_complexity` is §8's final MCC after scaffold credit is applied.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Complexity {
    pub reading_path_complexity: f64,
    pub reading_path_complexity_raw: f64,
    pub cognitive_complexity: f64,
    pub halstead: Halstead,
}

/// §11.1 link classifications. Each link is assigned exactly one primary
/// class; `is_image` and `is_bare_url` ride along on [`LinkRecord`] so the
/// aggregate breakdown in `links.*` does not double-count.
///
/// Serialized as snake_case to align with the §23 schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkClass {
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
    /// Reference-style link/image use whose label did not resolve.
    UnresolvedReferenceUse,
    /// A `link_reference_definition` block — the `[abc]: url` anchor entry.
    ReferenceDefinition,
}

/// Details about a single link / image / autolink / footnote reference /
/// reference definition encountered in the doc. Used to compute aggregate
/// link scores and to seed per-artifact detail rows later.
#[derive(Debug, Clone, Serialize)]
pub struct LinkRecord {
    /// One-based start row of the link span in the source.
    pub line: u64,
    /// Primary classification per §11.1.
    pub class: LinkClass,
    /// The raw link destination / URI.
    pub destination: String,
    /// Visible link text (empty for reference definitions / footnote refs).
    pub text: String,
    /// True when the link wraps an image (`![alt](url)`).
    pub is_image: bool,
    /// True when the visible link text is a bare raw URL.
    pub is_bare_url: bool,
    /// Resolution status:
    /// - `Some(true)`  — relative or internal target that resolves.
    /// - `Some(false)` — relative or internal target that does NOT resolve.
    /// - `None`        — external link, never checked by default.
    pub resolved: Option<bool>,
}

/// Aggregate link metrics (§11.1–§11.4, §23).
#[derive(Debug, Default, Clone, Serialize)]
pub struct Links {
    pub total: u64,
    pub internal: u64,
    pub relative: u64,
    pub external: u64,
    pub external_vendor: u64,
    pub scholarly: u64,
    pub issue_pr: u64,
    pub absolute_same_repo: u64,
    pub image: u64,
    pub footnote: u64,
    pub bare_url: u64,
    pub broken: u64,
    pub link_debt_score: f64,
    pub information_scent_score: f64,
    pub review_burden: f64,
}

/// Per-table record used for §13 metrics.
#[derive(Debug, Clone, Serialize)]
pub struct TableRecord {
    pub start_line: u64,
    pub end_line: u64,
    pub rows: u64,
    pub cols: u64,
    pub cells: u64,
    pub has_header: bool,
    pub empty_rate: f64,
    pub distinct_alignments: u64,
    pub has_local_explanation: bool,
    pub burden: f64,
    pub scaffold: f64,
    pub hard_warning: bool,
}

/// Aggregate table metrics (§13, §23).
#[derive(Debug, Default, Clone, Serialize)]
pub struct Tables {
    pub count: u64,
    pub max_cells: u64,
    pub table_burden_score: f64,
    pub table_scaffold_score: f64,
    pub hard_warnings: u64,
}

/// Per-diagram record used for §12.2.
#[derive(Debug, Clone, Serialize)]
pub struct DiagramRecord {
    pub start_line: u64,
    pub end_line: u64,
    pub language: String,
    pub nodes: u64,
    pub edges: u64,
    pub components: u64,
    pub cycles: u64,
    pub parse_error: bool,
    pub has_title_or_caption: bool,
    pub complexity: f64,
}

/// Per-image record used for §12.1 / §12.3.
#[derive(Debug, Clone, Serialize)]
pub struct ImageRecord {
    pub line: u64,
    pub destination: String,
    pub alt_text: String,
    pub has_alt_or_caption: bool,
    pub has_nearby_reference: bool,
    pub bounded_size: f64,
    pub repo_resolved: bool,
    pub image_complexity: f64,
    pub scaffold: f64,
}

/// Aggregate visual metrics (§12, §23).
#[derive(Debug, Default, Clone, Serialize)]
pub struct Visuals {
    pub images: u64,
    pub diagrams: u64,
    pub diagram_nodes_total: u64,
    pub diagram_edges_total: u64,
    pub diagram_cycles_total: u64,
    /// Count of diagrams whose parser reported `parse_error = true`. Phase F
    /// uses this as an aggregate signal for the `diagram_parse_error_added`
    /// callout. TODO(phase-next): expose parse-error flag per-artifact so the
    /// diff emitter can point at the specific diagram instead of relying on a
    /// file-level count.
    #[serde(skip_serializing_if = "is_zero_u64", default)]
    pub diagram_parse_error_count: u64,
    pub visual_scaffold_score: f64,
    pub visual_net_effect: f64,
}

fn is_zero_u64(v: &u64) -> bool {
    *v == 0
}

/// Maintainability aggregate per §23.
///
/// - `documentation_maintainability_index` is the §10.2 DMI on `[0, 100]`
///   scale. Phase B wires the V / M / R components; Phase C adds L / T / A;
///   Phase D wires S / F / G, completing the §10 formula.
/// - `artifact_debt_score` is the §19 per-artifact debt aggregate (Phase C).
/// - `section_balance_score` is §20's balance measure (Phase D).
/// - `good_scaffold_score` is §21's scaffold reward (Phase D).
#[derive(Debug, Default, Clone, Serialize)]
pub struct Maintainability {
    pub documentation_maintainability_index: f64,
    pub section_balance_score: f64,
    pub good_scaffold_score: f64,
    pub artifact_debt_score: f64,
}

/// §15 + §16 grounding metrics.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Grounding {
    pub repository_grounding_score: f64,
    pub evidence_coverage_score: f64,
}

/// §17 AI-era filler/lazy structure risk with diagnostic labels and the top
/// contributing sub-score values.
#[derive(Debug, Default, Clone, Serialize)]
pub struct AiEra {
    pub filler_lazy_structure_risk: f64,
    /// Diagnostic labels per §17.11. Sorted alphabetically for determinism.
    pub labels: Vec<String>,
    /// Top-3 contributing sub-scores as `(label, score)` pairs. Sorted by
    /// `-score, label`. Capped at 3.
    pub top_contributors: Vec<(String, f64)>,
}

/// §18 Review Criticality Index surface.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Review {
    pub review_criticality_index: f64,
}

/// Which kind of artifact this detail row represents. Serialized as
/// snake_case to align with §23.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
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
pub struct ArtifactRecord {
    pub id: u64,
    pub kind: ArtifactKind,
    pub start_line: u64,
    pub end_line: u64,
    pub language_tag: Option<String>,
    /// For code: LOC. For table: cells. For diagram: nodes. For math: tokens.
    /// For image / html: line count.
    pub size: u64,
    pub has_explanation: bool,
    pub has_label: bool,
    pub oversized: bool,
    /// Per-artifact cognitive cost metric (§14.1 code fence burden,
    /// §12.2 diagram complexity, §13.1 table burden, §14.3 math burden).
    /// For images we store the image_complexity; for html we store the
    /// raw line count as a weak proxy.
    pub burden: f64,
}

/// Phase-A + Phase-B + Phase-C + Phase-D + Phase-E Markdown metric output.
///
/// Emitted per file on the JSON / YAML / TOML path and under the `markdown`
/// key of the exported schema so later phases can add sibling keys like
/// `grounding`, `evidence`, etc., without renames.
#[derive(Debug, Clone, Serialize)]
pub struct MarkdownMetrics {
    /// The analyzed file's relative or absolute path, as seen by the CLI.
    pub path: String,
    pub loc: LocFamily,
    pub loc_ratios: LocRatios,
    pub size: Size,
    pub ecu_inputs: EcuInputs,
    pub sections: Vec<Section>,
    pub complexity: Complexity,
    pub links: Links,
    /// Per-link detail rows (§11.1). Phase F's `mehen diff` consumes these to
    /// detect newly added broken relative/anchor/external links per §39.4.
    /// Kept as an additive field so existing JSON consumers see an extra
    /// array; serializers never emit it when empty to keep snapshots stable.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub link_records: Vec<LinkRecord>,
    pub visuals: Visuals,
    pub tables: Tables,
    pub maintainability: Maintainability,
    /// §§15–16 grounding + evidence coverage scores (Phase D).
    pub grounding: Grounding,
    /// §17 AI-era filler / lazy structure risk (Phase D).
    pub ai_era: AiEra,
    /// §18 review criticality index (Phase D).
    pub review: Review,
    pub artifacts: Vec<ArtifactRecord>,
    /// §§29–38 Prose metric layer. Always emitted; its presence does NOT
    /// modify DMI / MCC / MRPC / FillerLazyRisk in later phases.
    pub prose: crate::prose::ProseReport,
}
