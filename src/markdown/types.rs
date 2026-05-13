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

/// §11.1 link classifications. Each link is assigned exactly one primary
/// class; `is_image` and `is_bare_url` ride along on [`LinkRecord`] so the
/// aggregate breakdown in `links.*` does not double-count.
///
/// Serialized as snake_case to align with the §23 schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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
    /// Count of diagrams whose parser reported `parse_error = true`. Phase F
    /// uses this as an aggregate signal for the `diagram_parse_error_added`
    /// callout. TODO(phase-next): expose parse-error flag per-artifact so the
    /// diff emitter can point at the specific diagram instead of relying on a
    /// file-level count.
    #[serde(skip_serializing_if = "is_zero_u64", default)]
    pub(crate) diagram_parse_error_count: u64,
    pub(crate) visual_scaffold_score: f64,
    pub(crate) visual_net_effect: f64,
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
pub(crate) struct Maintainability {
    pub(crate) documentation_maintainability_index: f64,
    pub(crate) section_balance_score: f64,
    pub(crate) good_scaffold_score: f64,
    pub(crate) artifact_debt_score: f64,
}

/// §15 + §16 grounding metrics.
#[derive(Debug, Default, Clone, Serialize)]
pub(crate) struct Grounding {
    pub(crate) repository_grounding_score: f64,
    pub(crate) evidence_coverage_score: f64,
}

/// §17 AI-era filler/lazy structure risk with diagnostic labels and the top
/// contributing sub-score values.
#[derive(Debug, Default, Clone, Serialize)]
pub(crate) struct AiEra {
    pub(crate) filler_lazy_structure_risk: f64,
    /// Diagnostic labels per §17.11. Sorted alphabetically for determinism.
    pub(crate) labels: Vec<String>,
    /// Top-3 contributing sub-scores as `(label, score)` pairs. Sorted by
    /// `-score, label`. Capped at 3.
    pub(crate) top_contributors: Vec<(String, f64)>,
}

/// §18 Review Criticality Index surface.
#[derive(Debug, Default, Clone, Serialize)]
pub(crate) struct Review {
    pub(crate) review_criticality_index: f64,
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

/// Phase-A + Phase-B + Phase-C + Phase-D + Phase-E Markdown metric output.
///
/// Emitted per file on the JSON / YAML / TOML path and under the `markdown`
/// key of the exported schema so later phases can add sibling keys like
/// `grounding`, `evidence`, etc., without renames.
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
    pub(crate) links: Links,
    /// Per-link detail rows (§11.1). Phase F's `mehen diff` consumes these to
    /// detect newly added broken relative/anchor/external links per §39.4.
    /// Kept as an additive field so existing JSON consumers see an extra
    /// array; serializers never emit it when empty to keep snapshots stable.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub(crate) link_records: Vec<LinkRecord>,
    pub(crate) visuals: Visuals,
    pub(crate) tables: Tables,
    pub(crate) maintainability: Maintainability,
    /// §§15–16 grounding + evidence coverage scores (Phase D).
    pub(crate) grounding: Grounding,
    /// §17 AI-era filler / lazy structure risk (Phase D).
    pub(crate) ai_era: AiEra,
    /// §18 review criticality index (Phase D).
    pub(crate) review: Review,
    pub(crate) artifacts: Vec<ArtifactRecord>,
    /// §§29–38 Prose metric layer. Always emitted; its presence does NOT
    /// modify DMI / MCC / MRPC / FillerLazyRisk in later phases.
    pub(crate) prose: crate::markdown::prose::ProseReport,
}
