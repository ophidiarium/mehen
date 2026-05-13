# Markdown Metrics — Structural Layer

Markdown in a software repository is a mixed technical artifact: prose, code
fences, diagrams, tables, images, links, math, front matter, and repository
references. **mehen** parses each Markdown document into a block/inline AST and
computes a metric suite that treats every one of those constructs as a
first-class citizen, not something to be stripped before counting words.

This page is the reference for the **structural** layer, which is
language-opaque: it operates from the AST, token classes, and punctuation
classes only, and never looks inside prose for grammar or meaning. For
language-aware signals (readability, wording quality, Japanese style
conformance), see [Markdown Prose Metrics](./markdown-prose.md).

Every formula below cites `(§N.N)` in the research foundation document
`docs/mehen_markdown_metrics_research_foundation.md`, which is the normative
source for derivations, thresholds, and rationale.

## Markdown LOC family (§5)

Unlike a single SLOC count, the Markdown LOC family separates physical lines
by construct. A 1,000-line file with 700 lines of code fences is not the same
artifact as a 1,000-line prose file.

| Metric | Meaning |
|---|---|
| `MD.DLOC` | Physical Markdown lines (everything). |
| `MD.PLOC` | Prose lines (narrative paragraphs, headings, list text). |
| `MD.CLOC` | Code-fence and indented-code lines. |
| `MD.TLOC` | Table lines. |
| `MD.MLOC` | Math block lines. |
| `MD.BLOC` | Blank lines. |
| `MD.ALOC` | Artifact lines = `CLOC + TLOC + MLOC` + diagram/raw HTML/MDX lines. |

**Derived ratios (§5.1)** — `ArtifactLineRatio`, `CodeLineRatio`,
`TableLineRatio`, `MathLineRatio`, `BlankLineRatio` — are each computed as
`component / max(1, DLOC)` and support document classification and anomaly
detection.

### Example

```markdown
# Auth API

Brief intro paragraph.

```ts
export async function login(): Promise<Session> { /* … */ }
```

| Field | Required |
|---|---|
| `email` | yes |
```

Expected shape:

```yaml
loc:
  dloc: 11
  ploc: 4
  cloc: 3
  tloc: 3
  mloc: 0
  bloc: 2
  aloc: 6
```

## Section tree (§3.4)

Headings produce a derived section tree carrying `heading_level`, byte/line
ranges, parent/child IDs, per-section `word_count`, and per-section
`artifact_counts` / `link_counts`. Heading quality flags capture **heading
skips** (for example `H1 → H4` without an `H2`/`H3`), **chunking smell** for
oversized `H2` sections, and **fragmentation smell** when a document is
flooded with tiny `H5` nodes. Downstream metrics (§8 MCC, §17 filler risk,
§20 section balance) read from this tree.

## Effective Content Units (§6)

`ECU` normalizes review mass across prose and technical artifacts so that a
small-but-dense ADR is not overshadowed by a long linear README in volume
comparisons:

```text
ECU = W / 240
    + 0.35 * CLOC
    + 0.06 * table_cells
    + 0.40 * diagram_nodes
    + 0.25 * diagram_edges
    + 0.12 * math_tokens
    + 0.20 * raw_html_or_mdx_lines
```

The `W / 240` term anchors on the standard adult silent-reading-rate scale
(§6.1). Interpretation bands (§6.2): `< 5` = small, `5–20` = normal,
`20–60` = large, `> 60` = documentation subsystem.

## MRPC — Markdown Reading Path Complexity (§7)

`MRPC` is the cyclomatic-complexity analogue for a Markdown file. It builds a
document navigation graph `G_doc = (N, E)` where nodes are sections, large
code blocks, diagrams, footnotes, linked repository documents, and external
domains, and edges are sequential, parent-child, internal link, relative
link, external link, and artifact explanation relations (§7.1).

**Classical form (§7.2):** `MRPC_raw = |E| − |N| + 2P`, where `P` is the
number of connected components.

**Weighted form (§7.3):** edges carry weights — `0.15` hierarchy, `0.20`
sequential, `0.50` internal anchor, `0.80` relative repo link, `1.00`
external, `1.20` broken — so `MRPC = max(1, Σ weight(e) − |N| + 2P)`.

Interpretation (§7.4): `1–5` is mostly linear, `16–35` is a documentation
hub, `> 35` is a documentation subsystem that probably wants a split or a
profile-specific threshold. A tutorial with `MRPC = 20` is suspect; an API
index with `MRPC = 20` is normal.

## MCC — Markdown Cognitive Complexity (§8)

`MCC` estimates local reading burden caused by flow breaks, nesting, context
switches, and dense artifact clusters. Base weights (§8.1) range from `0.20`
for a normal heading-level increment up to `+4.00` for a verified broken
external link.

The formula has three multiplicative layers:

- **Nesting multiplier (§8.2):** `1 + 0.18 * nest(n)` — intentionally smaller
  than code-oriented nesting penalties because Markdown nesting is cheaper.
- **Artifact clustering multiplier (§8.3):** dense clusters of artifacts in
  a 20-rendered-line window increase local switching cost.
- **Scaffolding credit (§8.4):** well-labelled, bounded, locally explained
  artifacts earn credit capped at `0.25 * MCC_positive`. Credit applies only
  when label, nearby explanation, and bounded size are all present.

Final: `MCC = max(0, MCC_positive − min(Σ scaffold_credit(a), MCC_credit_cap))`.

Interpretation (§8.5): `0–10` easy, `26–50` dense, `> 100` documentation
subsystem rather than one page.

## Markdown Halstead (§9)

Markdown Halstead measures token vocabulary and volume using Markdown-native
operators and operands instead of code operators.

**Operators (§9.1):** heading markers by level, list markers, table
delimiters, link/image operators, code fence openers by language, inline
code, blockquote, math delimiters, emphasis markers, footnote operators,
raw-HTML/MDX/directive operators, punctuation classes, diagram DSL statement
classes, and embedded-code operators (scaled).

**Operands (§9.2):** word-like tokens, numeric/version tokens,
identifier-like tokens, path-like tokens, link destinations, table headers,
image destinations/alt hashes, code identifiers from embedded analyzers,
diagram node/edge labels, and math symbols/commands.

**Formulas (§9.3):** same shape as source-code Halstead — `MDH_vocab = n1 +
n2`, `MDH_length = N1 + N2`, `MDH_volume = MDH_length * log2(max(2,
MDH_vocab))`, `MDH_diff = (n1/2) * (N2/max(1, n2))`, `MDH_effort = MDH_volume
* MDH_diff`.

**Embedded-code adjustment (§9.4):** embedded analyzers contribute
`0.20 * sqrt(code_halstead_volume) + 0.50 * code_cognitive + 0.10 * code_loc`
per code block. Raw embedded volume is square-rooted because it can otherwise
dwarf document-level signals, while cognitive complexity stays linear because
a hard example genuinely requires review.

## DMI — Documentation Maintainability Index (§10)

`DMI` summarises how maintainable the file is as a repository artifact.
Components are first normalized to `[0, 1]` (§10.1), then combined:

```text
DMI = clamp01(
      1
    − 0.18 * V_norm       (Markdown Halstead volume)
    − 0.18 * M_norm       (MCC)
    − 0.10 * R_norm       (MRPC)
    − 0.16 * L_norm       (Link Debt)
    − 0.10 * T_norm       (Table Burden)
    − 0.10 * A_norm       (Artifact Debt)
    − 0.10 * S_norm       (Poor Section Balance)
    − 0.12 * F_norm       (Filler/Lazy Risk)
    + 0.10 * G_norm       (Good Scaffold)
) * 100
```

Bands (§10.4): `85–100` highly maintainable, `70–84` good, `50–69` needs
attention, `30–49` hard, `0–29` documentation debt.

DMI is not **usefulness**. A long, linear filler document can score
respectable DMI while scoring high on filler risk (§17). A short dense
architecture note can score low DMI but be extremely high-value.

## Link Debt (§11)

Links are classified by destination (§11.1): internal anchor, relative
repository file, absolute same-repo URL, external, issue/PR, bare URL, image
target, or broken/unresolved.

**Link Debt Score (§11.2):**

```text
broken_rate      = L_broken / max(1, L_total)
external_rate    = L_ext    / max(1, L_total)
bare_rate        = L_bare   / max(1, L_total)
anchor_miss_rate = missing_internal_anchors / max(1, L_int)

LinkDebtScore = clamp01(
    0.45 * sat(broken_rate;      0.00, 0.10)
  + 0.20 * sat(anchor_miss_rate; 0.00, 0.10)
  + 0.15 * sat(bare_rate;        0.05, 0.30)
  + 0.10 * sat(external_rate;    0.60, 0.90)
  + 0.10 * sat(link_density_per_100w; 6, 14)
)
```

Broken links dominate because they are objective defects. External links are
not bad by default, but too many make a document fragile and branchy.

Companion scores: **Information Scent Score (§11.3)** rewards descriptive
link text, resolved relative links, working anchors, and a reference section
when the doc is citation-heavy. **Link Review Burden (§11.4)** —
`0.3*L_int + 0.8*L_rel + 1.0*L_ext + 2.5*L_broken + 0.5*L_footnote` — is the
cost-per-PR signal used in diff reporting.

## Table Burden + Scaffold (§13)

Tables are valuable up to a point. A table with 6–60 cells usually improves
comprehension; a table with 300+ cells is usually a maintenance artifact that
belongs in generated output or structured data.

**Per-table burden (§13.1)** combines wide, long, and cell-count saturation
terms with missing-header, empty-cell, and alignment-complexity penalties,
then aggregates `TableBurdenScore = 0.5 * mean(T_burden) + 0.5 * max(T_burden)`.

**Table Scaffolding Score (§13.2)** uses a piecewise size credit:

| Cells | Credit |
|---|---|
| 1–5 | `0.2` — too small to matter. |
| 6–60 | `1.0` — useful comparison scaffold. |
| 61–150 | `max(0, 1 − (cells − 60) / 120)`. |
| > 150 | More burden than scaffold; credit is near zero. |

A **hard warning** fires when `cells > 300` OR `cols > 12` OR `rows > 100`,
with the suggested remediation: split the table, generate it from structured
data, or move the source to YAML/JSON/CSV.

## Visual Scaffold + Net Effect (§12)

A diagram or image helps comprehension only when it is labelled, bounded,
nearby-explained, and its target resolves.

**Per-visual scaffold (§12.1):**

```text
V_scaffold(v) =
  alt_or_caption(v)
  * nearby_reference(v)
  * bounded_size(v)
  * repo_resolved(v)
```

Aggregated with diminishing returns:

```text
VisualScaffoldScore = clamp01(sum(V_scaffold(v)) / max(1, sqrt(W/500 + 1)))
```

**Diagram Complexity (§12.2)** for parseable diagrams:

```text
DiagramComplexity =
    0.40 * diagram_nodes
  + 0.55 * diagram_edges
  + 1.50 * diagram_cycles
  + 2.00 * parse_error
  + 1.00 * missing_title_or_caption
```

Cycles require mental simulation; parse errors and missing captions are
maintenance defects.

**Visual Net Effect (§12.3)** is `Σ DiagramComplexity + Σ image_complexity −
2.0 * Σ V_scaffold(v)`. Negative values mean the visuals probably help more
than they hurt; positive values mean they are under-explained or too
complex.

## Artifact Debt (§19)

Artifacts are not bad; artifact **debt** is high when artifacts are
unlabelled, unparsable, oversized, unexplained, or externally fragile.

```text
ArtifactDebtScore = clamp01(
    0.25 * sat(unlabelled_code_fences / max(1, code_fences); 0.05, 0.50)
  + 0.20 * sat(artifact_parse_errors  / max(1, artifacts);   0.00, 0.20)
  + 0.15 * sat(oversized_artifacts    / max(1, artifacts);   0.05, 0.30)
  + 0.15 * sat(unexplained_artifacts  / max(1, artifacts);   0.10, 0.60)
  + 0.15 * sat(raw_html_or_mdx_lines  / max(1, DLOC);        0.05, 0.25)
  + 0.10 * sat(external_artifact_links/ max(1, artifacts);   0.10, 0.60)
)
```

Feeds directly into DMI via its `A_norm` term.

## Repository Grounding (§15) + Evidence Coverage (§16)

A Markdown file in a software project should connect to repository reality
— files, commands, packages, APIs, configs, tests, and versioned facts — or
at least acknowledge that it doesn't.

**Repository Grounding (§15)** combines resolved relative links, path-like
token resolution, labelled code fence density, identifier density, and
version-fact density:

```text
RepositoryGroundingScore = clamp01(
    0.25 * sat(repo_link_density;    0.5, 4.0)
  + 0.25 * path_resolution_rate
  + 0.20 * sat(code_example_density; 0.5, 3.0)
  + 0.15 * sat(identifier_density;   0.02, 0.12)
  + 0.15 * sat(version_fact_density; 0.01, 0.08)
)
```

Bands (§15.3): `0.00–0.20` almost none, `0.21–0.50` weak, `0.51–0.80`
useful, `0.81–1.00` very grounded.

**Evidence Coverage (§16)** measures structural support for each section:

```text
anchor_density_s  = evidence_anchors_s / max(1, W_s / 250)
section_evidence_s = sat(anchor_density_s; 0.2, 1.5)

EvidenceCoverageScore =
    0.5 * mean(section_evidence_s)
  + 0.5 * p25(section_evidence_s)
```

The 25th-percentile term prevents one well-linked section from hiding many
unsupported sections.

## Filler / Lazy Structure Risk (§17)

This metric addresses the AI-era documentation problem:

> The document is just filler: structure is lazy, there are no references,
> it is large but useless.

**This is not AI-authorship detection.** It reports structural evidence —
unanchored prose, low artifact density, weak repository grounding, lazy
sectioning, repetition, specificity scarcity, hollow references, and
placeholder density — without making any claim about how the text was
written.

Sub-scores (§§17.1–17.8):

- **UnanchoredProseMass** — fraction of words living in sections with no
  evidence anchors.
- **LowArtifactDensity** — `1 − sat(A / (W/800); 0.5, 2.0)`.
- **LowRepoGrounding** — `1 − RepositoryGroundingScore`.
- **LazySectioning** — combines heading density, large-section rate, and
  the "shallow big doc" flag (`W > 2,500` AND max heading depth ≤ 2).
- **RepetitionDensity** — token-shingle Jaccard > 0.82 detects
  near-duplicate paragraphs.
- **SpecificityScarcity** — identifiers + paths + version tokens + inline
  code tokens relative to `W`.
- **ReferenceHollowness** — bibliography entries without verifiable
  DOI/arXiv/RFC/URL anchors.
- **PlaceholderDensity** — TODO/TBD/FIXME/XXX/lorem and empty links per
  1,000 words.

**Final formula (§17.9):**

```text
FillerLazyRisk = clamp01(
    0.20 * UnanchoredProseMass
  + 0.15 * LowArtifactDensity
  + 0.20 * LowRepoGrounding
  + 0.15 * LazySectioning
  + 0.12 * RepetitionDensity
  + 0.12 * SpecificityScarcity
  + 0.04 * ReferenceHollowness
  + 0.02 * PlaceholderDensity
)
```

Bands (§17.10): `0.00–0.20` low, `0.21–0.40` mild, `0.41–0.60` review,
`0.61–0.80` high, `0.81–1.00` severe.

**Diagnostic labels (§17.11)** attached to high scores are stable strings:
`large-unanchored-prose`, `low-repository-grounding`, `lazy-sectioning`,
`low-artifact-density`, `near-duplicate-paragraphs`, `specificity-scarcity`,
`hollow-references`, `placeholder-heavy`. PR reporting (§39) quotes these
labels verbatim rather than paraphrasing.

### Example output (§17.12)

```text
Filler / Lazy Structure Risk: 0.73 HIGH

Top contributors:
  - 71% of prose is in sections without evidence anchors
  - 3,420 words, only 1 relative link and 0 code examples
  - max heading depth = 2 with 4 sections > 1,200 words
  - specificity density = 1.8% (threshold: 3%-15%)
```

## Review Criticality Index (§18)

`RCI` answers "Should I review this document carefully?". A small document
can be review-critical if it is dense with technical anchors. It combines a
per-word `DensityScore` (MCC-per-word, MDH-volume-per-word, grounding,
evidence coverage, review burden, embedded code complexity) with a delta
term and a changed-links/artifacts term:

```text
RCI = clamp01(
    0.65 * DensityScore
  + 0.20 * sat(abs(metric_delta_percent); 10, 60)
  + 0.15 * sat(changed_links_or_artifacts; 2, 20)
) * 100
```

Bands (§18.2): `0–25` low, `51–75` careful review, `76–100` high-risk
change.

The combined **DMI × RCI × FillerLazyRisk** interpretation matrix (§18.3) is
the canonical way to summarise a file:

| DMI | RCI | Filler | Meaning |
|---|---|---|---|
| High | Low | Low | Long but easy and probably healthy. |
| High | Low | High | Easy to maintain but likely low-value filler. |
| Low | High | Low | Dense valuable doc; review carefully. |
| Low | High | High | Dangerous: hard to maintain and weakly grounded. |

## Section Balance (§20) + Good Scaffold (§21)

**Section Balance (§20)** checks whether the document is chunked in a
maintainable way — it penalizes oversized sections at the 95th percentile,
a high rate of very large sections, a high rate of tiny sections, heading
skips, and heading depth deviation from a profile-specific expectation.

**Good Scaffold (§21)** rewards helpful technical structure:

```text
GoodScaffoldScore = clamp01(
    0.25 * VisualScaffoldScore
  + 0.20 * TableScaffoldScore
  + 0.20 * bounded_labelled_code_example_score
  + 0.15 * InformationScentScore
  + 0.10 * section_summary_score
  + 0.10 * successful_internal_navigation_score
)
```

It offsets maintainability penalties **modestly**. It never erases objective
defects like broken links, parse failures, or inclusive-language flags.

## Worked example — exported shape

For a Markdown file the exported schema (§23) looks like this (abbreviated):

```yaml
markdown:
  loc: {dloc: 412, ploc: 220, cloc: 148, tloc: 22, mloc: 0, bloc: 22, aloc: 170}
  size: {words: 1670, effective_content_units: 12.4, sections: 9, headings: 14}
  complexity:
    reading_path_complexity: 14
    cognitive_complexity: 38
    halstead: {vocabulary: 312, length: 2840, volume: 22571.4, total_volume: 24118.9}
  maintainability:
    documentation_maintainability_index: 62
    section_balance_score: 0.74
    artifact_debt_score: 0.28
  links: {total: 34, broken: 2, link_debt_score: 0.22, review_burden: 29.4}
  visuals: {images: 0, diagrams: 1, visual_scaffold_score: 1.00, visual_net_effect: -2.0}
  tables: {count: 3, max_cells: 48, table_burden_score: 0.31, table_scaffold_score: 0.84}
  grounding: {repository_grounding_score: 0.72, evidence_coverage_score: 0.58}
  ai_era:
    filler_lazy_structure_risk: 0.14
    labels: []
  review: {review_criticality_index: 71}
```

This corresponds to a "medium DMI, high RCI, low filler risk" document —
the "dense, valuable, review carefully" row of §18.3.

## See also

- [Markdown Prose Metrics](./markdown-prose.md) — readability, lexical
  diversity, and wording quality for English and Japanese.
- [`mehen diff` PR comment](../commands/pr-comment.md) — how structural
  deltas surface in the sticky GitHub comment.
- `docs/mehen_markdown_metrics_research_foundation.md` — normative derivations
  and citation graph.
