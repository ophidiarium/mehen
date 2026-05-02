# Science-Backed Heuristic Metrics for Markdown Documents in Software Repositories

**Project context:** [`ophidiarium/mehen`](https://github.com/ophidiarium/mehen/)  
**Document purpose:** Research foundation for extending `mehen` from source-code metrics into Markdown technical-document metrics.  
**Primary use case:** CI/CLI analysis of Markdown files in software projects, including READMEs, ADRs, runbooks, architecture documents, tutorials, API references, generated docs, and AI-assisted documentation.  
**Version:** Draft 1.0  
**Date:** 2026-05-02

---

## Executive Summary

Markdown in software repositories is not merely prose. It is a mixed technical artifact containing prose, code, diagrams, math, tables, images, links, references, configuration snippets, commands, repository paths, issue references, and generated or AI-assisted content.

Therefore, Markdown metrics should not strip technical containers. Code fences, Mermaid diagrams, math blocks, tables, links, images, raw HTML, MDX, and repository references should be parsed and scored as first-class document structures.

This document proposes a defensible metric suite for Markdown files that mirrors the spirit of source-code metrics such as cyclomatic complexity, cognitive complexity, Halstead metrics, and maintainability index, while adapting them to documentation-specific constructs.

The core proposed metrics are:

| Metric | Purpose |
|---|---|
| Markdown LOC Family | Separates prose, code, table, math, artifact, and blank lines. |
| Effective Content Units | Normalized document size across prose and technical artifacts. |
| Markdown Reading Path Complexity | Cyclomatic-complexity analogue for document navigation paths. |
| Markdown Cognitive Complexity | Cognitive-complexity analogue for local reading burden. |
| Markdown Halstead Metrics | Token/operator/operand complexity for Markdown structure and content. |
| Documentation Maintainability Index | Overall maintainability of the document as a repository artifact. |
| Link Debt Score | Fragility and maintenance cost of links and references. |
| Information Scent Score | Quality of navigational and evidential cues. |
| Visual Scaffolding Score | Degree to which images and diagrams help rather than hurt comprehension. |
| Table Burden Score | Maintenance and scanning cost of tables. |
| Repository Grounding Score | How strongly the document is anchored in repository reality. |
| Evidence Coverage Score | How well sections are supported by links, code, diagrams, tables, references, or repo artifacts. |
| Filler / Lazy Structure Risk | AI-era metric for detecting large, generic, weakly grounded documentation. |
| Review Criticality Index | Whether a document deserves careful human review even if it is short. |

The intended output should allow judgments such as:

```text
This document is very long, but easy to maintain: linear structure, low link debt, few volatile artifacts.
```

or:

```text
This document is small, but dense: many code examples, diagrams, relative links, and API references. Review carefully.
```

or:

```text
This document is large but likely low-value filler: weak structure, no references, low repository grounding, little specificity.
```

The system should avoid claiming that a document is AI-generated. Instead, it should report structural evidence: filler risk, lazy sectioning, weak grounding, low evidence coverage, and repetition.

---

## 1. Research Foundation

### 1.1 Source-code metric analogues

Classical source-code metrics provide a useful design pattern:

| Source-code metric | Markdown analogue | Shared concept |
|---|---|---|
| Lines of Code | Markdown LOC Family | Size and surface area. |
| Cyclomatic Complexity | Markdown Reading Path Complexity | Independent paths through a graph. |
| Cognitive Complexity | Markdown Cognitive Complexity | Human mental effort caused by nesting and flow breaks. |
| Halstead Metrics | Markdown Halstead Metrics | Operators, operands, vocabulary, volume, difficulty, effort. |
| Maintainability Index | Documentation Maintainability Index | Combined maintainability risk score. |

McCabe cyclomatic complexity defines complexity from graph structure using edges, nodes, and connected components. Markdown can be represented similarly as a navigation graph: sections, links, footnotes, artifacts, and repository references form paths a reader may follow.

Halstead metrics count operators and operands in source code. Markdown can likewise define structural operators such as heading markers, list markers, table delimiters, link syntax, image syntax, code fences, inline code delimiters, math delimiters, emphasis markers, and blockquote markers. Operands can include word-like tokens, identifiers, paths, URLs, table headers, diagram labels, math symbols, and embedded code identifiers.

Cognitive complexity penalizes structures that interrupt linear reasoning. Markdown has analogous costs: heading skips, long sections, deep nested lists, link clusters, tables, diagrams, code blocks, math, blockquotes, admonitions, and raw HTML/MDX.

### 1.2 Cognitive load

Cognitive Load Theory distinguishes between intrinsic load, extraneous load, and germane load. Technical documents naturally contain intrinsic complexity. Bad structure adds extraneous complexity. Good diagrams, examples, and tables can reduce burden by acting as scaffolding, but poorly integrated artifacts can increase load.

Therefore, metrics should charge complexity for technical artifacts while also allowing a bounded scaffolding credit when the artifact is well-labelled, locally explained, and appropriately sized.

### 1.3 Hypertext and links

Links are a central documentation feature. They provide information scent and traceability, but they also introduce branch points, maintenance burden, and link-rot risk.

Internal anchors, relative repository links, external links, issue references, scholarly references, and image links should be classified separately. A relative link to `../src/lib.rs` is not equivalent to an external blog link. A broken internal anchor is an objective defect. A dense cluster of external links may be useful in a bibliography but burdensome in a tutorial paragraph.

### 1.4 Visuals and diagrams

Images and diagrams often reduce comprehension burden by showing relationships that prose would otherwise describe poorly. However, their benefit is conditional. A diagram with no caption, no nearby explanation, parse errors, excessive graph density, or many cycles can become a liability.

For Markdown in software repositories, visual analysis should include:

- Markdown images.
- Local image paths.
- External image URLs.
- Mermaid diagrams.
- PlantUML diagrams.
- Graphviz / DOT diagrams.
- D2 diagrams.
- Vega / Vega-Lite diagrams.
- Raw SVG or embedded HTML diagrams where detectable.

### 1.5 Tables

Tables are valuable up to a point. They support comparison, compact reference, and structured scanning. However, large or wide tables are hard to read, hard to diff, and hard to maintain.

A table with 6-60 cells often improves comprehension. A table with 300+ cells is usually a maintenance artifact that may belong in generated output or structured data.

### 1.6 AI-era documentation problem

Modern software documentation is increasingly written or expanded by AI agents. The tool should not attempt unreliable AI-authorship detection. Instead, it should detect observable structural smells:

- Large prose mass without references or repository anchors.
- Shallow or lazy sectioning.
- Low specificity.
- Low artifact density.
- Repetition and near-duplicate paragraphs.
- Hollow references.
- Placeholder-heavy sections.
- Low evidence coverage.

The proposed metric is named **Filler / Lazy Structure Risk**, not AI Probability.

---

## 2. Design Principles

1. **Do not strip technical containers.**  
   Code fences, diagrams, math, raw HTML, MDX, images, tables, links, and footnotes are part of the document and should contribute to metrics.

2. **Keep linguistics opaque.**  
   The core system should not rely on grammar quality, sentiment, topic models, syllables, or language-specific readability formulas. It may count word-like tokens, numeric tokens, identifier-like tokens, path-like tokens, and punctuation classes.

3. **Use AST-driven analysis.**  
   The system should rely on a Markdown AST from Tree-sitter or a compatible parser, not regex-only scanning.

4. **Separate metrics by construct.**  
   Maintainability, cognitive load, review criticality, evidence coverage, and filler risk are different. A single score should not hide those distinctions.

5. **Report contributors, not just scores.**  
   Each high score should explain which sections, links, tables, diagrams, or code blocks caused it.

6. **Use profile-aware thresholds.**  
   READMEs, ADRs, runbooks, tutorials, and generated references have different expected structures.

7. **Make CI output actionable.**  
   Scores should be accompanied by line ranges and suggested fixes.

---

## 3. Required Markdown AST Model

The Markdown grammar should expose block-level and inline-level syntax deeply enough for metrics while avoiding full natural-language parsing.

### 3.1 Block nodes

Minimum block node types:

```text
document
section
heading
paragraph
blockquote
callout/admonition
list
list_item
task_list_item
table
table_header
table_row
table_cell
fenced_code_block
indented_code_block
math_block
html_block
mdx_jsx_block
directive_block
image_block
thematic_break
footnote_definition
footnote_reference
link_reference_definition
front_matter
```

### 3.2 Inline nodes

Minimum inline node types:

```text
text_span
word_token
numeric_token
identifier_like_token
path_like_token
punctuation_class
inline_code
emphasis
strong
strikethrough
link
image
autolink
html_inline
mdx_jsx_inline
math_inline
```

### 3.3 Punctuation classes

The grammar does not need one semantic node for every punctuation character. Punctuation classes are sufficient:

```text
terminator:      . ? ! 。 …
separator:       , ; :
bracket:         () [] {} <>
operator_like:   = + - * / | & :: -> =>
```

### 3.4 Derived section tree

Headings should produce a derived section tree:

```text
section_id
heading_level
heading_text_hash
start_byte
end_byte
start_line
end_line
parent_section_id
child_section_ids
block_count
word_count
artifact_counts
link_counts
local_metrics
```

Heading quality should be measured:

```text
H1 -> H2 -> H3       normal
H1 -> H4             heading skip
large H2 section     chunking smell
many tiny H5 nodes    fragmentation smell
```

### 3.5 Embedded-language injections

Code fences should be classified by info string:

```text
programming/config: rust, ts, tsx, javascript, python, go, ruby, shell, json, yaml, toml, sql
diagram DSL:         mermaid, plantuml, dot, graphviz, d2, vega-lite
math/proof:          latex, tex, math
opaque literal:      log, text, console, stacktrace, diff, patch
unknown/unlabelled:  missing or unsupported language
```

For supported programming languages, reuse existing `mehen` analyzers. For diagrams, extract graph nodes, edges, connected components, and cycles where possible. For unknown or unlabelled code fences, compute fallback token, line, and language-tag metrics.

---

## 4. Notation

Let:

```text
W          visible narrative word-like tokens outside code fences
W_s        visible narrative tokens in section s
DLOC       physical Markdown lines
PLOC       prose physical lines
CLOC       code-fence and indented-code lines
TLOC       table lines
MLOC       math lines
ALOC       artifact lines = CLOC + TLOC + MLOC + diagram/raw HTML/MDX lines

H          number of headings
S          number of derived sections
A          non-prose artifact count

L_int      internal same-document links
L_rel      repository-relative links
L_ext      external links
L_broken   broken links, depending on validation mode

T          number of tables
I          number of images
G          number of diagrams
M          number of math blocks/spans
```

Helper functions:

```text
clamp01(x) = min(1, max(0, x))
sat(x; lo, hi) = clamp01((x - lo) / (hi - lo))
log1p(x) = ln(1 + x)
```

Use `sat` instead of hard thresholds so documents do not jump sharply between categories.

---

## 5. Markdown LOC Family

Markdown LOC should be reported as a family rather than a single value.

```text
MD.DLOC     physical Markdown lines
MD.PLOC     prose lines
MD.CLOC     code-fence and indented-code lines
MD.TLOC     table lines
MD.MLOC     math lines
MD.BLOC     blank lines
MD.ALOC     artifact lines
```

A 1,000-line Markdown file with 700 lines of code examples is not equivalent to a 1,000-line prose-only document.

### 5.1 Derived ratios

```text
ArtifactLineRatio = ALOC / max(1, DLOC)
CodeLineRatio     = CLOC / max(1, DLOC)
TableLineRatio    = TLOC / max(1, DLOC)
MathLineRatio     = MLOC / max(1, DLOC)
BlankLineRatio    = BLOC / max(1, DLOC)
```

These ratios support document classification and anomaly detection.

---

## 6. Effective Content Units

Effective Content Units estimate normalized content mass across prose and technical artifacts.

```text
ECU = W / 240
    + 0.35 * CLOC
    + 0.06 * table_cells
    + 0.40 * diagram_nodes
    + 0.25 * diagram_edges
    + 0.12 * math_tokens
    + 0.20 * raw_html_or_mdx_lines
```

### 6.1 Motivation

The `W / 240` term uses a common adult silent-reading-speed scale anchor. The remaining coefficients estimate additional review mass contributed by artifacts. They intentionally do not dominate final scoring because code, diagrams, tables, and math are also scored separately.

### 6.2 Interpretation

```text
ECU < 5       small document
5-20          normal technical document
20-60         large document
> 60          documentation subsystem or reference-scale file
```

---

## 7. Markdown Reading Path Complexity

Markdown Reading Path Complexity, or MRPC, is the cyclomatic-complexity analogue for documents.

### 7.1 Document navigation graph

Build a graph:

```text
G_doc = (N, E)
```

Nodes:

```text
sections
large code blocks
tables with >= 12 cells
diagrams
footnotes/reference definitions
linked repository documents
external domains, optionally collapsed
```

Edges:

```text
sequential section edge
parent-child heading edge
internal link edge
relative repository link edge
external link edge
artifact explanation edge
footnote/reference edge
```

### 7.2 Classical form

```text
MRPC_raw = |E| - |N| + 2P
```

where `P` is the number of connected components.

### 7.3 Weighted form

```text
edge_weight(e) =
  0.15  hierarchy edge
  0.20  sequential edge
  0.50  internal same-document anchor
  0.65  footnote/reference edge
  0.80  relative repository link
  1.00  external link
  1.20  unresolved/broken link
  0.40  artifact explanation edge
```

```text
MRPC = max(1, sum(edge_weight(e) for e in E) - |N| + 2P)
```

### 7.4 Interpretation

| MRPC | Meaning |
|---:|---|
| 1-5 | Mostly linear, low navigation complexity. |
| 6-15 | Normal non-linearity for many technical docs. |
| 16-35 | High navigation complexity. |
| >35 | Documentation hub/subsystem; consider split or profile-specific threshold. |

High MRPC is not automatically bad. A documentation index or API reference is expected to be path-rich. A tutorial is not.

---

## 8. Markdown Cognitive Complexity

Markdown Cognitive Complexity, or MCC, estimates local reading effort caused by flow breaks, nesting, context switches, dense links, and technical artifacts.

### 8.1 Base weights

| Element | Base weight | Rationale |
|---|---:|---|
| Heading level change by +1 | 0.20 | Normal hierarchy helps. |
| Heading skip, e.g. H2 -> H4 | 1.00 | Missing hierarchy. |
| Section > 800 words without subheading | 2.00 | Large chunk. |
| Paragraph > 160 words | 1.25 | Long local block. |
| List | 0.40 | Scannable but still a structure. |
| Nested list level | `0.50 * depth` | Tracking burden. |
| Task list item | 0.35 | Adds status semantics. |
| Blockquote | 0.50 | Context switch. |
| Callout/admonition | 0.75 | Attention switch. |
| Inline link | 0.25 | Branch point. |
| Dense link cluster | 1.50 | Disorientation risk. |
| Footnote reference | 0.60 | Deferred context. |
| Image | 0.50 | Visual switch before credit. |
| Diagram block | 1.50 | Relation decoding. |
| Code fence <= 12 LOC | 1.00 | Local technical example. |
| Code fence > 12 LOC | `1 + 0.08 * (CLOC - 12)` | Longer scan. |
| Unlabelled code fence | +1.50 | Missing parser/highlight. |
| Table <= 60 cells | 0.75 | Compact comparison. |
| Table > 60 cells | `0.75 + 0.03 * (cells - 60)^0.85` | Non-linear burden. |
| Display math block | 1.50 | Symbolic switch. |
| Raw HTML/MDX block | `0.30 * lines`, cap 8 | Rendering ambiguity. |
| Diagram parse error | +3.00 | Cannot trust artifact. |
| Broken internal/relative link | +3.00 | Objective defect. |
| External link unchecked | +0.30 | Maintenance uncertainty. |
| External link broken | +4.00 | Evidence failure. |

### 8.2 Nesting multiplier

```text
nest(n) = list_depth + blockquote_depth + callout_depth + details_depth
nest_multiplier(n) = 1 + 0.18 * nest(n)
```

Markdown nesting is usually less costly than nested control flow in code, so the multiplier is intentionally smaller than a code-oriented nesting penalty.

### 8.3 Artifact clustering multiplier

For each 20-rendered-line window:

```text
artifact_density_window = artifacts_in_window / 20
cluster_multiplier = 1 + sat(artifact_density_window; 0.15, 0.45) * 0.35
```

Dense clusters of artifacts increase local mental switching.

### 8.4 Scaffolding credit

Artifacts can reduce complexity when labelled, bounded, and locally explained.

```text
local_explanation(a) =
  1 if explanatory prose/caption exists within +/- 2 blocks else 0

has_label(a) =
  1 if code has language, image has alt/caption, table has header,
  or diagram has title/caption else 0

bounded(a) =
  1 - sat(size(a); useful_hi, severe_hi)
```

```text
scaffold_credit(a) =
  base_credit(type(a)) * local_explanation(a) * has_label(a) * bounded(a)
```

Base credits:

| Artifact | Credit |
|---|---:|
| Code example with language tag and <= 30 LOC | 0.75 |
| Diagram with <= 20 nodes and caption/nearby explanation | 1.25 |
| Image with alt/caption and nearby explanation | 0.80 |
| Table with header and 6-60 cells | 1.00 |
| Math block with nearby explanation | 0.50 |

Cap total credit:

```text
MCC_credit_cap = 0.25 * MCC_positive
```

Final formula:

```text
MCC_positive =
  sum(base_weight(n) * nest_multiplier(n) * cluster_multiplier(n)
      for all flow-break nodes n)

MCC =
  max(0, MCC_positive - min(sum(scaffold_credit(a)), MCC_credit_cap))
```

### 8.5 Interpretation

| MCC | Meaning |
|---:|---|
| 0-10 | Easy local reading. |
| 11-25 | Normal technical documentation. |
| 26-50 | Dense or artifact-heavy. |
| 51-100 | High burden; split or scaffold. |
| >100 | Documentation subsystem, not one maintainable page. |

---

## 9. Markdown Halstead Metrics

Markdown Halstead metrics define operators and operands for Markdown syntax and technical content.

### 9.1 Operators

Operators include:

```text
heading markers by level
list markers
task item markers
table delimiters/alignment
link operators
image operators
code fence operators by language
inline code operator
blockquote/callout operators
math delimiters
emphasis/strong/strikethrough
footnote operators
raw HTML/MDX/directive operators
punctuation classes
diagram DSL statement classes
embedded code operators, scaled
```

Let:

```text
n1 = distinct operator types
N1 = total operator occurrences
```

### 9.2 Operands

Operands include:

```text
word-like tokens
numeric/version tokens
identifier-like tokens
path-like tokens
link destinations
table headers
image destinations/alt hashes
code identifiers from embedded analyzers
diagram node labels and edge labels
math symbols/commands
```

Let:

```text
n2 = distinct operand types
N2 = total operand occurrences
```

### 9.3 Formulas

```text
MDH_vocab   = n1 + n2
MDH_length  = N1 + N2
MDH_volume  = MDH_length * log2(max(2, MDH_vocab))
MDH_diff    = (n1 / 2) * (N2 / max(1, n2))
MDH_effort  = MDH_volume * MDH_diff
```

### 9.4 Embedded code adjustment

```text
embedded_volume =
  sum(0.20 * sqrt(code_halstead_volume_c)
    + 0.50 * code_cognitive_c
    + 0.10 * code_loc_c
    for code block c)
```

```text
MDH_volume_total = MDH_volume + embedded_volume
```

Raw embedded-code Halstead volume can dwarf document-level Markdown signals, so code volume is square-rooted. Code cognitive complexity remains linear because a cognitively complex embedded example genuinely requires review.

---

## 10. Documentation Maintainability Index

Documentation Maintainability Index, or DMI, estimates how maintainable a Markdown file is as a repository artifact.

### 10.1 Normalized components

```text
V_norm  = sat(ln(1 + MDH_volume_total); 8, 15)
M_norm  = sat(MCC; 15, 80)
R_norm  = sat(MRPC; 8, 40)
L_norm  = LinkDebtScore
T_norm  = TableBurdenScore
A_norm  = ArtifactDebtScore
S_norm  = 1 - SectionBalanceScore
F_norm  = FillerLazyRisk
G_norm  = GoodScaffoldScore
```

### 10.2 Formula

```text
DMI = clamp01(
      1
    - 0.18 * V_norm
    - 0.18 * M_norm
    - 0.10 * R_norm
    - 0.16 * L_norm
    - 0.10 * T_norm
    - 0.10 * A_norm
    - 0.10 * S_norm
    - 0.12 * F_norm
    + 0.10 * G_norm
) * 100
```

### 10.3 Weight motivation

| Component | Weight | Reason |
|---|---:|---|
| Halstead volume | 0.18 | Construct/content variety affects update burden. |
| Cognitive complexity | 0.18 | Local reading effort is central. |
| Reading path complexity | 0.10 | Non-linearity matters but can be intentional. |
| Link debt | 0.16 | Links are volatile documentation dependencies. |
| Table burden | 0.10 | Large tables are hard to diff/update. |
| Artifact debt | 0.10 | Code/diagram/math/raw blocks add maintenance surface. |
| Poor section balance | 0.10 | Chunking and structure affect editability. |
| Filler risk | 0.12 | Low-value bulk still costs review/maintenance. |
| Good scaffolding | +0.10 | Useful examples, diagrams, and tables should be rewarded. |

### 10.4 Interpretation

| DMI | Meaning |
|---:|---|
| 85-100 | Highly maintainable. |
| 70-84 | Good. |
| 50-69 | Maintainable with attention. |
| 30-49 | Hard to maintain. |
| 0-29 | Documentation debt. |

DMI is not usefulness. A long, linear filler document can have acceptable DMI but high filler risk. A short, dense architecture note can have low DMI but high value.

---

## 11. Link and Reference Metrics

Links are crucial in software docs. Internal, relative, and external links should be treated differently.

### 11.1 Link classification

| Class | Examples | Meaning |
|---|---|---|
| Internal anchor | `#install` | Local navigation, low external dependency. |
| Relative repository file | `../src/lib.rs`, `docs/api.md#auth` | Strong repository grounding. |
| Absolute same-repo URL | GitHub/GitLab URL | Often should be relative. |
| External vendor/API doc | AWS, Rust docs, MDN | Evidence but rot risk. |
| Scholarly/reference URL | DOI, arXiv, RFC, W3C | Strong evidence if valid. |
| Issue/PR link | GitHub/GitLab/Jira | Traceability. |
| Bare URL | Raw URL text | Weak readability. |
| Image target | Local or external image | Visual dependency. |
| Broken/unresolved | Malformed/missing | Defect. |

### 11.2 Link Debt Score

```text
broken_rate      = L_broken / max(1, L_total)
external_rate    = L_ext / max(1, L_total)
bare_rate        = L_bare / max(1, L_total)
anchor_miss_rate = missing_internal_anchors / max(1, L_int)
```

```text
LinkDebtScore = clamp01(
    0.45 * sat(broken_rate; 0.00, 0.10)
  + 0.20 * sat(anchor_miss_rate; 0.00, 0.10)
  + 0.15 * sat(bare_rate; 0.05, 0.30)
  + 0.10 * sat(external_rate; 0.60, 0.90)
  + 0.10 * sat(link_density_per_100w; 6, 14)
)
```

Broken links dominate because they are objective defects. External links are not bad by default, but too many external links make a document more fragile and branchy.

### 11.3 Information Scent Score

```text
descriptive_link_text_rate =
  links with non-empty, non-generic text / max(1, links)

resolved_relative_rate =
  resolved relative links / max(1, relative links)

anchor_success_rate =
  resolved internal anchors / max(1, internal links)

reference_section_present =
  1 if bibliography/reference section exists for citation-heavy docs else 0
```

```text
InformationScentScore = clamp01(
    0.30 * descriptive_link_text_rate
  + 0.30 * resolved_relative_rate
  + 0.20 * anchor_success_rate
  + 0.20 * reference_section_present
)
```

Generic link text detection can remain language-light: empty text, raw URL labels, repeated identical labels, and a configurable small set such as `here`, `link`, and `click here`.

### 11.4 Link Review Burden

```text
LinkReviewBurden =
    0.3 * L_int
  + 0.8 * L_rel
  + 1.0 * L_ext
  + 2.5 * L_broken
  + 0.5 * L_footnote
```

Use this especially in PR deltas.

---

## 12. Visual and Diagram Metrics

### 12.1 Visual Scaffolding Score

For each visual artifact `v`:

```text
alt_or_caption(v) =
  1 if alt text, title, caption, or explicit nearby label exists else 0

nearby_reference(v) =
  1 if previous/next 2 blocks introduce or explain the visual else 0

bounded_size(v) =
  1 - sat(visual_complexity(v); 20, 80)

repo_resolved(v) =
  1 if target resolves locally or external target passes link check else 0
```

```text
V_scaffold(v) =
  alt_or_caption(v) * nearby_reference(v) * bounded_size(v) * repo_resolved(v)
```

Aggregate with diminishing returns:

```text
VisualScaffoldScore =
  clamp01(sum(V_scaffold(v)) / max(1, sqrt(W / 500 + 1)))
```

### 12.2 Diagram Complexity

For parsable diagrams:

```text
diagram_nodes      = graph nodes/entities/classes/states
diagram_edges      = arrows/relations/transitions
diagram_components = connected components
diagram_cycles     = max(0, diagram_edges - diagram_nodes + diagram_components)
```

```text
DiagramComplexity =
    0.40 * diagram_nodes
  + 0.55 * diagram_edges
  + 1.50 * diagram_cycles
  + 2.00 * parse_error
  + 1.00 * missing_title_or_caption
```

Rationale:

- Nodes are concepts.
- Edges are relations.
- Cycles require mental simulation.
- Parse errors and missing captions are maintenance defects.

### 12.3 Visual Net Effect

```text
VisualNetEffect =
  sum(DiagramComplexity + image_complexity)
  - 2.0 * sum(V_scaffold(v))
```

Interpretation:

```text
VisualNetEffect < 0     visuals probably help more than hurt
VisualNetEffect ≈ 0     neutral
VisualNetEffect > 0     visuals likely under-explained or too complex
```

---

## 13. Table Metrics

### 13.1 Table Burden Score

For each table `t`:

```text
rows_t, cols_t, cells_t = dimensions
has_header_t = 1/0
empty_rate_t = empty cells / cells_t

wide_penalty_t          = sat(cols_t; 5, 12)
long_penalty_t          = sat(rows_t; 20, 100)
cell_penalty_t          = sat(cells_t; 60, 300)
missing_header_penalty  = 1 - has_header_t
alignment_complexity_t  = distinct alignments / max(1, cols_t)
```

```text
T_burden(t) = clamp01(
    0.25 * wide_penalty_t
  + 0.25 * long_penalty_t
  + 0.25 * cell_penalty_t
  + 0.15 * missing_header_penalty
  + 0.05 * sat(empty_rate_t; 0.10, 0.50)
  + 0.05 * alignment_complexity_t
)
```

Aggregate:

```text
TableBurdenScore =
  0.5 * mean(T_burden(t)) + 0.5 * max(T_burden(t))
```

### 13.2 Table Scaffolding Score

Piecewise size credit:

| Cells | Assumption |
|---:|---|
| 1-5 | Too small to matter much. |
| 6-60 | Useful comparison scaffold. |
| 61-150 | Mixed. |
| >150 | More burden than scaffold. |

Formula:

```text
size_credit_t =
  0.2                                  if cells_t < 6
  1.0                                  if 6 <= cells_t <= 60
  max(0, 1 - (cells_t - 60) / 120)     if cells_t > 60

local_explanation_t =
  1 if nearby block introduces or summarizes table else 0

TableScaffold(t) =
  size_credit_t * has_header_t * local_explanation_t
```

Aggregate:

```text
TableScaffoldScore =
  clamp01(sum(TableScaffold(t)) / max(1, sqrt(T)))
```

Hard warning:

```text
cells_t > 300 OR cols_t > 12 OR rows_t > 100
```

Suggested remediation:

```text
Split table, generate it from structured data, move source to YAML/JSON/CSV,
or group rows into separate sections.
```

---

## 14. Embedded Code, Config, Logs, and Math

### 14.1 Code fence burden

```text
CodeFenceBurden(c) =
    1.0
  + 0.08 * max(0, LOC_c - 12)
  + 0.50 * sat(LOC_c; 40, 120)
  + 0.40 * sat(line_length_p95_c; 100, 180)
  + 1.50 * missing_language_tag
  + 1.00 * parser_error_if_language_supported
  + 0.20 * code_cognitive_c
  + 0.05 * sqrt(code_halstead_volume_c)
```

Use existing `mehen` analyzers when the language is supported. For unsupported languages, keep fallback metrics: LOC, token variety, line length, and language tag presence.

### 14.2 Executable Example Credit

```text
ExecutableExampleCredit(c) =
  1.0 if language tag exists
  * 1.0 if LOC_c <= 40
  * 1.0 if nearby prose introduces purpose/result
  * 1.0 if block contains command/test/example markers or repo-local symbols
```

This should reduce filler risk and increase repository grounding, but should not erase cognitive complexity.

### 14.3 Math burden

```text
MathBurden(m) =
    1.0
  + 0.10 * math_tokens
  + 0.25 * distinct_math_commands
  + 1.00 * no_nearby_explanation
```

No symbolic reasoning is required. Count tokens, commands, lines, and local explanation.

---

## 15. Repository Grounding Score

Repository Grounding Score is one of the most important AI-era metrics.

A Markdown file in a software project should often connect to repository reality: files, commands, packages, APIs, configs, tests, examples, issue references, diagrams, and versioned facts.

### 15.1 Signals

```text
resolved relative links
resolved internal anchors
path-like tokens that resolve to repo files/directories
labelled code fences
command blocks
package/API/config identifiers
version-like tokens
issue/PR references
diagrams whose nodes match components/files
tables containing paths/config/API names
references to tests/examples
```

### 15.2 Formula

```text
repo_link_density =
  resolved_relative_links / max(1, W / 500)

path_resolution_rate =
  resolved_path_like_tokens / max(1, path_like_tokens)

code_example_density =
  labelled_code_fences / max(1, W / 800)

identifier_density =
  identifier_like_tokens / max(1, W)

version_fact_density =
  numeric_or_version_tokens / max(1, W)
```

```text
RepositoryGroundingScore = clamp01(
    0.25 * sat(repo_link_density; 0.5, 4.0)
  + 0.25 * path_resolution_rate
  + 0.20 * sat(code_example_density; 0.5, 3.0)
  + 0.15 * sat(identifier_density; 0.02, 0.12)
  + 0.15 * sat(version_fact_density; 0.01, 0.08)
)
```

### 15.3 Interpretation

| Score | Meaning |
|---:|---|
| 0.00-0.20 | Almost no repository grounding. |
| 0.21-0.50 | Weak grounding. |
| 0.51-0.80 | Useful technical grounding. |
| 0.81-1.00 | Very grounded; maybe dense. |

---

## 16. Evidence Coverage Score

Evidence Coverage Score measures structural support, not truth.

### 16.1 Evidence anchors

Evidence anchors include:

```text
resolved relative link
external link
internal link to non-trivial section
labelled code fence
table with header
parseable diagram
image with alt/caption
math block with explanation
issue/PR/reference link
DOI/arXiv/RFC/W3C/vendor-doc style reference
path-like token resolved to repository
```

### 16.2 Per-section formula

```text
anchor_density_s =
  evidence_anchors_s / max(1, W_s / 250)

section_evidence_s =
  sat(anchor_density_s; 0.2, 1.5)
```

### 16.3 Aggregate formula

```text
EvidenceCoverageScore =
  0.5 * mean(section_evidence_s)
+ 0.5 * p25(section_evidence_s)
```

The 25th percentile prevents one well-linked section from hiding many unsupported sections.

### 16.4 Interpretation

| Score | Meaning |
|---:|---|
| 0.00-0.25 | Mostly unsupported prose. |
| 0.26-0.50 | Some anchors, many weak sections. |
| 0.51-0.75 | Reasonably evidenced. |
| 0.76-1.00 | Strongly anchored. |

---

## 17. Filler / Lazy Structure Risk

This metric addresses the AI-era problem:

```text
The document is just filler: structure is lazy, there are no references,
it is large but useless.
```

The metric should not be presented as AI-authorship detection. It measures observable structural weaknesses.

### 17.1 Unanchored Prose Mass

```text
anchored_words =
  words in sections containing at least one evidence anchor

unanchored_words =
  W - anchored_words

UnanchoredProseMass =
  sat(unanchored_words / max(1, W); 0.35, 0.85)
```

### 17.2 Low Artifact Density

```text
artifact_density =
  A / max(1, W / 800)

LowArtifactDensity =
  1 - sat(artifact_density; 0.5, 2.0)
```

A 4,000-word technical document with zero code, zero links, zero diagrams, zero tables, and zero references is structurally suspicious for most software-document profiles.

### 17.3 Low Repository Grounding

```text
LowRepoGrounding =
  1 - RepositoryGroundingScore
```

### 17.4 Lazy Sectioning

```text
heading_density =
  H / max(1, W / 700)

long_section_rate =
  sections with W_s > 1200 / max(1, S)

shallow_large_doc =
  1 if W > 2500 and max_heading_depth <= 2 else 0
```

```text
LazySectioning = clamp01(
    0.35 * (1 - sat(heading_density; 0.6, 2.0))
  + 0.35 * sat(long_section_rate; 0.10, 0.60)
  + 0.30 * shallow_large_doc
)
```

### 17.5 Repetition / boilerplate density

Use token shingles, not semantic analysis.

```text
paragraph_shingles =
  normalized 5-token shingles per paragraph

near_duplicate_paragraph_rate =
  paragraphs with Jaccard similarity > 0.82 to another paragraph / paragraphs

repeated_heading_rate =
  duplicate normalized headings / headings
```

```text
RepetitionDensity = clamp01(
    0.75 * sat(near_duplicate_paragraph_rate; 0.02, 0.20)
  + 0.25 * sat(repeated_heading_rate; 0.02, 0.15)
)
```

### 17.6 Specificity Scarcity

Software docs usually contain concrete tokens.

```text
specific_tokens =
  identifier_like_tokens
+ path_like_tokens
+ numeric_or_version_tokens
+ inline_code_tokens

specificity_density =
  specific_tokens / max(1, W)

SpecificityScarcity =
  1 - sat(specificity_density; 0.03, 0.15)
```

This remains language-opaque.

### 17.7 Reference Hollowness

```text
reference_like_items =
  bibliography entries + footnote definitions + external citations

verifiable_reference_items =
  entries with DOI/arXiv/RFC/URL or successful link check

ReferenceHollowness =
  1 - verifiable_reference_items / max(1, reference_like_items)
```

### 17.8 Placeholder Density

```text
placeholder_tokens =
  TODO + TBD + FIXME + XXX + lorem + placeholder markers + empty links/images

PlaceholderDensity =
  sat(placeholder_tokens / max(1, W / 1000); 0.5, 4.0)
```

### 17.9 Final formula

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

### 17.10 Interpretation

| FillerLazyRisk | Meaning |
|---:|---|
| 0.00-0.20 | Low filler risk. |
| 0.21-0.40 | Mild risk; normal for conceptual docs. |
| 0.41-0.60 | Review for generic expansion. |
| 0.61-0.80 | High risk: likely low-value bulk. |
| 0.81-1.00 | Severe risk: verbose, lazy, weakly grounded. |

### 17.11 Diagnostic labels

```text
large-unanchored-prose
low-repository-grounding
lazy-sectioning
low-artifact-density
near-duplicate-paragraphs
specificity-scarcity
hollow-references
placeholder-heavy
```

### 17.12 Example output

```text
Filler / Lazy Structure Risk: 0.73 HIGH

Top contributors:
  - 71% of prose is in sections without evidence anchors
  - 3,420 words, only 1 relative link and 0 code examples
  - max heading depth = 2 with 4 sections > 1,200 words
  - specificity density = 1.8% (threshold: 3%-15%)

Interpretation:
  This is not an AI-detection result.
  The document is structurally easy but weakly grounded and likely contains low-value filler.
```

---

## 18. Review Criticality Index

Review Criticality Index, or RCI, answers:

```text
Should I review this document carefully?
```

A small document can be review-critical if it is dense with technical anchors.

### 18.1 Formula

```text
DensityScore = clamp01(
    0.25 * sat(MCC / max(1, W / 500); 4, 18)
  + 0.20 * sat(MDH_volume_total / max(1, W); 20, 120)
  + 0.20 * RepositoryGroundingScore
  + 0.15 * EvidenceCoverageScore
  + 0.10 * sat(LinkReviewBurden / max(1, W / 500); 2, 10)
  + 0.10 * sat(embedded_code_complexity / max(1, W / 500); 2, 12)
)
```

```text
RCI = clamp01(
    0.65 * DensityScore
  + 0.20 * sat(abs(metric_delta_percent); 10, 60)
  + 0.15 * sat(changed_links_or_artifacts; 2, 20)
) * 100
```

### 18.2 Interpretation

| RCI | Meaning |
|---:|---|
| 0-25 | Low review criticality. |
| 26-50 | Normal review. |
| 51-75 | Careful review recommended. |
| 76-100 | High-risk documentation change. |

### 18.3 Combined interpretation matrix

| DMI | RCI | Filler risk | Interpretation |
|---|---|---|---|
| High | Low | Low | Long but easy and probably healthy. |
| High | Low | High | Easy to maintain but likely low-value filler. |
| Low | High | Low | Dense valuable doc; review carefully. |
| Low | High | High | Dangerous: hard to maintain and weakly grounded. |
| Medium | High | Medium | Normal technical reference or architecture doc. |

---

## 19. Artifact Debt Score

Artifact Debt Score captures maintainability risk from code, diagrams, images, tables, math, raw HTML, and MDX.

```text
ArtifactDebtScore = clamp01(
    0.25 * sat(unlabelled_code_fences / max(1, code_fences); 0.05, 0.50)
  + 0.20 * sat(artifact_parse_errors / max(1, artifacts); 0.00, 0.20)
  + 0.15 * sat(oversized_artifacts / max(1, artifacts); 0.05, 0.30)
  + 0.15 * sat(unexplained_artifacts / max(1, artifacts); 0.10, 0.60)
  + 0.15 * sat(raw_html_or_mdx_lines / max(1, DLOC); 0.05, 0.25)
  + 0.10 * sat(external_artifact_links / max(1, artifacts); 0.10, 0.60)
)
```

Artifacts are not bad. Artifact debt is high when artifacts are unlabelled, unparsable, oversized, unexplained, or externally fragile.

---

## 20. Section Balance Score

Section balance measures whether the document is chunked in a maintainable way.

```text
section_word_counts = [W_s for each section s]
median_section_words = median(section_word_counts)
p95_section_words = percentile(section_word_counts, 95)
large_section_rate = count(W_s > 1200) / max(1, S)
tiny_section_rate = count(W_s < 40) / max(1, S)
heading_skip_rate = heading_skips / max(1, H)
```

```text
SectionBalanceScore = clamp01(
    1
  - 0.30 * sat(p95_section_words; 900, 2000)
  - 0.25 * sat(large_section_rate; 0.05, 0.40)
  - 0.15 * sat(tiny_section_rate; 0.20, 0.70)
  - 0.20 * sat(heading_skip_rate; 0.02, 0.20)
  - 0.10 * sat(abs(max_heading_depth - expected_depth); 2, 5)
)
```

Expected depth should be profile-specific.

---

## 21. Good Scaffold Score

Good Scaffold Score rewards helpful technical structure.

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

This score offsets maintainability penalties only modestly. It should never erase objective defects like broken links or parse failures.

---

## 22. Document-Type Profiles

Thresholds should be profile-aware.

### 22.1 README / landing page

Expected:

```text
moderate links
install/use examples
relative links to docs/source
low-to-medium MRPC
low filler risk
```

Warnings:

```text
huge shallow README
many external links but no local docs
no install/use example
no relative links in mature repo
```

### 22.2 ADR / decision record

Expected:

```text
explicit context links
links to code/issues/previous ADRs
alternatives section/table
medium evidence coverage
```

Warnings:

```text
no linked context
no alternatives
high filler risk
diagrams without decision trace
```

### 22.3 Runbook

Expected:

```text
commands
code fences
callouts/warnings
configs/service links
strong specificity
```

Warnings:

```text
unlabelled code fences
stale links
low specificity
giant tables
```

### 22.4 API/reference doc

Expected:

```text
higher table density
higher link density
higher Halstead volume
possibly high MRPC
```

Warnings:

```text
broken links
missing examples
large undiffable tables
poor section balance
```

### 22.5 Tutorial / guide

Expected:

```text
linear path
bounded examples
low MRPC
some visual/code scaffolding
low link density
```

Warnings:

```text
too many branch links
missing expected outputs
long sections without checkpoints
high early cognitive complexity
```

### 22.6 Generated documentation

Expected:

```text
high size
possibly high table/link density
regular structure
many references to APIs or generated symbols
```

Warnings:

```text
generated bulk with low repository grounding
large tables with no source indication
missing generation metadata
stale symbols
```

---

## 23. Proposed Exported Schema

```yaml
markdown:
  loc:
    dloc: 0
    ploc: 0
    cloc: 0
    tloc: 0
    mloc: 0
    aloc: 0

  size:
    words: 0
    effective_content_units: 0.0
    sections: 0
    headings: 0

  complexity:
    reading_path_complexity: 0.0
    cognitive_complexity: 0.0
    halstead:
      operators_distinct: 0
      operators_total: 0
      operands_distinct: 0
      operands_total: 0
      vocabulary: 0
      length: 0
      volume: 0.0
      difficulty: 0.0
      effort: 0.0
      embedded_volume: 0.0
      total_volume: 0.0

  maintainability:
    documentation_maintainability_index: 0.0
    section_balance_score: 0.0
    artifact_debt_score: 0.0

  links:
    total: 0
    internal: 0
    relative: 0
    external: 0
    broken: 0
    link_debt_score: 0.0
    information_scent_score: 0.0
    review_burden: 0.0

  visuals:
    images: 0
    diagrams: 0
    visual_scaffold_score: 0.0
    visual_net_effect: 0.0

  tables:
    count: 0
    max_cells: 0
    table_burden_score: 0.0
    table_scaffold_score: 0.0

  grounding:
    repository_grounding_score: 0.0
    evidence_coverage_score: 0.0

  ai_era:
    filler_lazy_structure_risk: 0.0
    labels: []

  review:
    review_criticality_index: 0.0

  diagnostics:
    top_contributors: []
```

---

## 24. Example Interpretations

### 24.1 Long, linear, easy to maintain, but maybe filler

```text
W = 8,000
MRPC = 4
MCC = 18
DMI = 82
RepositoryGroundingScore = 0.22
EvidenceCoverageScore = 0.30
FillerLazyRisk = 0.58
RCI = 28
```

Interpretation:

```text
Structurally simple and probably easy to edit.
However, weak grounding and evidence coverage mean it may be low-value filler.
Review for usefulness, not complexity.
```

### 24.2 Small but dense

```text
W = 650
MRPC = 18
MCC = 42
DMI = 47
RepositoryGroundingScore = 0.91
EvidenceCoverageScore = 0.83
FillerLazyRisk = 0.08
RCI = 87
```

Interpretation:

```text
Short but dense and important.
Contains many links/artifacts/code references.
Review carefully.
```

### 24.3 Diagram-heavy but well scaffolded

```text
W = 2,400
Diagrams = 5
VisualScaffoldScore = 0.86
VisualNetEffect = -3.2
MCC = 31
DMI = 74
```

Interpretation:

```text
Diagrams add complexity but likely help comprehension.
```

### 24.4 Giant table debt

```text
Table max cells = 840
TableBurdenScore = 0.93
DMI = 41
RCI = 72
```

Interpretation:

```text
Move table source to structured data or split it.
The table is hard to diff and maintain.
```

---

## 25. Recommended PR Report Format

### 25.1 Dense technical document

```text
Markdown Metrics: docs/architecture/runtime.md

Summary:
  DMI: 58  (medium maintainability risk)
  MCC: 47  (high cognitive complexity)
  MRPC: 12 (moderate navigation complexity)
  Filler/Lazy Risk: 0.11 (low)
  Review Criticality: 78 (high)

Why:
  + Added 4 Mermaid diagrams, 2 with >25 nodes
  + Added 18 relative links, all resolved
  + Added 3 code fences; 1 missing language tag
  + Largest section: 1,640 words without subheading
  + Evidence coverage improved from 0.42 -> 0.76

Suggested fixes:
  - Split section "Runtime topology" after ~800 words
  - Add language tag to code fence at line 214
  - Add caption/nearby explanation for diagram at line 171
```

### 25.2 Filler-like document

```text
Markdown Metrics: docs/generated/overview.md

Summary:
  DMI: 76  (easy to maintain)
  MCC: 12  (low complexity)
  MRPC: 2  (linear)
  Filler/Lazy Risk: 0.79 (high)
  Review Criticality: 22 (low)

Why:
  + 4,900 words, 0 code fences, 1 relative link, 0 tables/diagrams
  + 82% of prose appears in sections without evidence anchors
  + max heading depth = 2; 3 sections > 1,200 words
  + specificity density = 1.5%

Interpretation:
  Not an AI-detection result.
  The document is structurally easy but weakly grounded; review for generic filler before relying on it.
```

---

## 26. Validation Plan

The formulas above should be treated as seed heuristics and validated empirically.

### 26.1 Construct validity

| Metric | Claimed construct |
|---|---|
| MRPC | Non-linear navigation paths. |
| MCC | Local cognitive burden. |
| MDH | Structural/content vocabulary and volume. |
| DMI | Maintainability risk. |
| LinkDebtScore | Link fragility. |
| VisualScaffoldScore | Visual support quality. |
| TableBurdenScore | Table scanning/diff burden. |
| RepositoryGroundingScore | Repo-specific usefulness. |
| EvidenceCoverageScore | Structural support for claims/instructions. |
| FillerLazyRisk | Large low-grounding documentation smell. |
| RCI | Review attention required. |

### 26.2 Criterion validity

Compare metrics against:

```text
human understandability ratings
human maintainability ratings
Markdown PR review time
number of doc-related PR comments
broken link counts over time
stale code example reports
follow-up fixes after merge
task success in doc usability tests
```

### 26.3 Reliability tests

Metrics should be stable under:

```text
line wrapping changes
whitespace-only changes
reference definition reordering
equivalent inline/reference links
heading anchor normalization
language aliases like js/javascript
```

### 26.4 Anti-gaming tests

The metric should not be trivially improved by:

```text
decorative images without captions
empty links
splitting every paragraph into tiny headings
fake references
unlabelled code fences
duplicate examples
giant tables hidden in raw HTML
```

---

## 27. Implementation Roadmap

### Phase 1: Structural Markdown analysis

Implement:

```text
Markdown LOC family
section tree
heading quality metrics
paragraph/list/table/code/image/link extraction
basic inline token classes
MRPC
MCC without embedded code integration
LinkDebtScore for internal and relative links
```

### Phase 2: Technical artifact analysis

Implement:

```text
code fence classification
embedded code metrics via existing mehen analyzers
diagram parsing for Mermaid and PlantUML if feasible
table burden/scaffold metrics
math token counting
raw HTML/MDX burden
ArtifactDebtScore
```

### Phase 3: Repository intelligence

Implement:

```text
relative link resolution
path-like token resolution
same-repo absolute URL normalization
issue/PR reference classification
repository grounding score
evidence coverage score
```

### Phase 4: AI-era and review metrics

Implement:

```text
Filler/Lazy Structure Risk
near-duplicate paragraph detection
specificity scarcity
review criticality index
PR delta reporting
profile-aware thresholds
```

### Phase 5: Calibration

Implement:

```text
configuration profiles
baseline collection across repositories
percentile-based threshold tuning
human review feedback collection
metric drift tracking
```

---

## 28. Final Recommendation

Implement this as a metric suite, not one mega-score.

The minimum strong first version should include:

```text
MD LOC family
MRPC
MCC
Markdown Halstead
DMI
LinkDebtScore
RepositoryGroundingScore
EvidenceCoverageScore
FillerLazyRisk
ReviewCriticalityIndex
TableBurdenScore
VisualScaffoldScore
```

The most important conceptual separation is:

```text
DMI  = how maintainable is it?
RCI  = how carefully should I review it?
FLR  = is it likely low-value filler?
MCC  = how hard is it to read locally?
MRPC = how non-linear is its navigation?
RGS  = how grounded is it in the repository?
ECS  = how structurally evidenced is it?
```

This gives the desired judgments:

```text
Long but easy:
  high DMI, low MCC, low MRPC

Small but dense:
  high RCI, high MCC/MDH per word, high grounding

Large but useless filler:
  high FillerLazyRisk, low RepositoryGroundingScore, low EvidenceCoverageScore,
  low artifact density, lazy sectioning

Useful but hard:
  low DMI, high RCI, low filler risk
```

This design keeps the Markdown extension compatible with `mehen`'s code-metric culture while adding what Markdown needs: links, visuals, tables, embedded code, repository grounding, and AI-era filler detection without pretending to solve AI authorship attribution.

---

## References

1. McCabe, T. J. "A Complexity Measure." *IEEE Transactions on Software Engineering*, 1976. https://doi.org/10.1109/TSE.1976.233837
2. Halstead, M. H. *Elements of Software Science.* Elsevier, 1977.
3. CommonMark Specification. https://spec.commonmark.org/current/
4. GitHub Flavored Markdown Specification. https://github.github.com/gfm/
5. Sweller, J. "Cognitive Load During Problem Solving: Effects on Learning." *Cognitive Science*, 1988. https://doi.org/10.1207/s15516709cog1202_4
6. Pirolli, P., and Card, S. "Information Foraging." *Psychological Review*, 1999. https://doi.org/10.1037/0033-295X.106.4.643
7. Mayer, R. E. "Multimedia Learning." Cambridge University Press, 2001.
8. Winn, W. "The Role of Graphics in Training Documents." In *The Technology of Text*, 1982.
9. Gelman, A., Pasarica, C., and Dodhia, R. "Let's Practice What We Preach: Turning Tables into Graphs." *The American Statistician*, 2002.
10. WCAG 2.2, W3C Recommendation. https://www.w3.org/TR/WCAG22/
11. SonarSource Cognitive Complexity white paper. https://www.sonarsource.com/resources/cognitive-complexity/
12. Tang, X. et al. "An Empirical Study of Documentation Issues in Software Projects." MSR-related documentation quality research. https://sanadlab.org/assets/pdf/TangMSR23.pdf
13. Liang, W. et al. "GPT detectors are biased against non-native English writers." *Patterns*, 2023. https://doi.org/10.1016/j.patter.2023.100779
14. Walters, W. H., and Wilder, E. I. "Fabrication and errors in the bibliographic citations generated by ChatGPT." *Scientific Reports*, 2023. https://www.nature.com/articles/s41598-023-41032-5
15. Brysbaert, M. "How many words do we read per minute? A review and meta-analysis of reading rate." *Journal of Memory and Language*, 2019. https://doi.org/10.1016/j.jml.2019.104047
