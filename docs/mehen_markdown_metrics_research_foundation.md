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
| Prose Readability Suite (EN) | Language-aware readability ensemble (FKGL, Gunning Fog, ARI, Coleman-Liau, SMOG, New Dale-Chall, FORCAST) with a short-document guard. |
| Prose Readability Suite (JA) | Tateishi (1988) simplified RS, Jōyō-grade proxy, optional Lee–Hasebe jReadability replica, sentence-length in characters. |
| Lexical Diversity | MATTR, MTLD, HD-D, Yule's K, hapax ratio, lexical density. |
| Wording Quality Score | Passive, hedge, weasel, wordy-phrase, adverb, nominalization, expletive, cliché, and repetition detectors. |
| Inclusive Language Score | alex / retext-equality-style checks for gendered, ableist, and exclusionary terms. |
| Japanese Style Conformance | JTF-rule and textlint-preset-ja-technical-writing heuristics: fullwidth/halfwidth consistency, mixed keitai/jōtai, over-kanji runs, doubled joshi, weak phrases. |

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

2. **Keep the core language-agnostic; add language-aware metrics as a dedicated layer.**  
   The core structural layer (LOC family, MRPC, MCC, Halstead, DMI, link debt, table burden, grounding, evidence coverage, filler risk, RCI) must remain language-opaque and compute from AST structure, token classes, and punctuation classes only. Grammar quality, sentiment, topic models, and deep NLP remain out of scope.
   However, a separate, optional **language-aware prose layer** (§§29–38) MAY compute readability formulas, lexical diversity, and wording heuristics for English and Japanese. Language-aware metrics are reported as clearly labelled sub-scores, never folded silently into structural scores, and never block CI unless the user explicitly opts in.

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

> **Note:** The GitHub Action PR-comment surface is specified normatively in §39, which supersedes this section and forbids suggested-fix or interpretation prose. The samples below remain as a conceptual sketch for future interactive/CLI output modes (e.g., a `mehen doc report` long-form view).

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

## 29. Language-Aware Prose Metric Layer

The structural metrics above (§§5–28) are intentionally language-opaque: they depend on the AST, token classes, and punctuation classes, not on grammar or lexicon. That design survives scrutiny and should remain the core.

However, software repositories contain real prose — READMEs, ADRs, tutorials, runbooks — written primarily in English and, for a large share of the ecosystem (JP-based vendors, OSS with Japanese localization, Nikkei-backed projects), Japanese. A prose metric layer adds value that structural metrics cannot:

- Sentence-length and word-length signals that correlate with reader effort at the paragraph level.
- Style-guide enforcement (passive voice, weasel words, hedges) that catches real quality regressions.
- Locale-specific conformance (JTF rules, Jōyō kanji grade, keitai/jōtai consistency) that catches defects invisible to English-trained tools.

### 29.1 Architectural constraints

1. **Layered, not folded.** Prose metrics are reported as a *separate* top-level section in the output schema. They never modify DMI, MCC, MRPC, or FillerLazyRisk weights silently. A user reading a DMI score must be able to reproduce it without running the prose layer.
2. **Per-block language tag.** Language is detected per Markdown block (paragraph, heading, list item, blockquote), not per document. A Japanese blog post that cites an English RFC paragraph should score each block under the appropriate locale.
3. **Structural artifacts stay excluded.** Code fences, inline code, link destinations, image alt-text, YAML/TOML/JSON front-matter, HTML blocks, MDX, table delimiters, and autolinks are stripped before any readability or wording calculation. This mirrors the retext/remark AST approach.
4. **Short-text refusal.** Grade-level formulas are suppressed when (post-strip word count < 100) or (sentence count < 5); the tool reports raw counts and a `short_doc_warning` instead of a meaningless grade.
5. **Feature-gated dictionaries.** Dictionary-dependent features (Lindera for Japanese morphology, CMU for English syllables, Dale-Chall/NGSL familiar-word lists) ship behind Cargo `--features` flags so the default binary stays small.
6. **Deterministic and reproducible.** No network access, no cloud services, no nondeterministic sampling. Embedded data is versioned alongside the binary.

### 29.2 Output shape

Prose metrics extend the YAML schema from §23:

```yaml
markdown:
  prose:
    language_detection:
      dominant_language: en | ja | other | mixed
      blocks:
        - {range: [start, end], language: en, confidence: 0.97}
    english:
      readability:
        flesch_reading_ease: 0.0
        flesch_kincaid_grade: 0.0
        gunning_fog: 0.0
        smog: 0.0         # null if sentences < 30
        ari: 0.0
        coleman_liau: 0.0
        dale_chall_new: 0.0
        forcast: 0.0
        ensemble_grade_band: [low, high]
      lexical:
        mattr_50: 0.0
        mtld: 0.0
        hdd_42: 0.0
        yule_k: 0.0
        hapax_ratio: 0.0
        lexical_density: 0.0
        avg_sentence_words: 0.0
        p90_sentence_words: 0
        avg_word_chars: 0.0
      wording:
        passive_ratio: 0.0
        hedge_density: 0.0
        weasel_density: 0.0
        wordy_density: 0.0
        adverb_density: 0.0
        nominalization_density: 0.0
        expletive_count: 0
        lexical_illusions: 0
        cliche_density: 0.0
        nonword_count: 0
        long_sentence_count: 0
      inclusive_language:
        flags: []
    japanese:
      script_composition:
        kanji_ratio: 0.0
        hiragana_ratio: 0.0
        katakana_ratio: 0.0
        latin_ratio: 0.0
        digit_ratio: 0.0
        script_entropy: 0.0
      readability:
        tateishi_rs: 0.0
        jouyou_grade_mean: 0.0
        hyougai_ratio: 0.0
        jreadability: 0.0         # null without UniDic
        shibasaki_grade: 0.0      # null without tokenizer
      lexical:
        avg_sentence_chars: 0.0
        p90_sentence_chars: 0
        comma_period_ratio: 0.0
        jukugo_density: 0.0
      wording:
        politeness_dominant: desumasu | dearu | mixed
        keitai_jotai_mix_count: 0
        weak_phrase_count: 0
        redundant_expression_count: 0
        doubled_joshi_count: 0
        long_kanji_run_count: 0
      style_conformance:
        jtf_violations: []
  meta:
    short_doc_warning: false
    words_counted: 0
    sentences_counted: 0
    blocks_stripped: [code, frontmatter, html, alt_text]
```

### 29.3 Interaction with existing scores

- **DMI (§10):** unaffected by default; when the user opts in with `--with-prose-penalty`, a bounded term `0.05 * (1 − WordingQualityScore)` may be subtracted. The default weights in §10.2 are unchanged.
- **FillerLazyRisk (§17):** can optionally consume a `specificity_density_en` sub-score that uses stopword ratios in place of the purely character-class `specificity_density`. This remains opt-in behind a Cargo feature so the base metric stays reproducible.
- **Review Criticality Index (§18):** the ensemble readability grade of changed paragraphs contributes as an additive, explicit sub-term when the prose layer is enabled; never when it is disabled.

---

## 30. Language Detection for Markdown Blocks

Language identification happens once per Markdown block so that metric dispatch can choose the correct locale pipeline. The requirements are narrow: distinguish English, Japanese, and "other" with high precision on paragraph-sized inputs, no network access, and minimal binary cost.

### 30.1 Zero-dependency Unicode-block heuristic (default)

For the English-vs-Japanese split, Unicode-block ratios outperform trigram language models on short inputs. Chinese has no hiragana/katakana, so any non-zero kana presence is a strong positive for Japanese. The rule:

```text
let total = non_whitespace_non_punct_chars
let kana = hiragana_chars + katakana_chars
let cjk  = kana + han_chars
let latin = ascii_letter_chars + fullwidth_latin_letter_chars

if kana / total >= 0.15:                language = ja
elif cjk / total >= 0.40 and kana == 0: language = zh (treat as "other")
elif latin / total >= 0.80:             language = en
else:                                   language = other
```

False positives on Japanese↔English are essentially zero because Chinese has zero kana, and pure Japanese blocks cannot have 80% Latin letters. Mixed Latin-code + Japanese prose (common in software docs) is assigned Japanese as long as kana ≥ 15%, which is the behavior we want for running prose with embedded identifiers.

This path uses only the `unicode-script` crate (UAX #24 script lookups) and adds on the order of 100 KB to the binary. See https://crates.io/crates/unicode-script and https://www.unicode.org/reports/tr24/.

### 30.2 Opt-in trigram classifier

When the user needs finer granularity (e.g., distinguishing French from Spanish), enable one of two Cargo features:

- `whatlang` — pure Rust, 70 languages, trigram, MIT, ~80 KB crate (https://crates.io/crates/whatlang). Reliable above ~120 characters; returns a reliability score.
- `lingua` — Rust port of Lingua; highest accuracy in published benchmarks (https://github.com/pemistahl/lingua-rs). Supports a low-accuracy trigram-only mode. Full model is multi-megabyte; restricting to `[English, Japanese]` reduces it to a few MB. Apache-2.0.

`whichlang` (Quickwit, https://github.com/quickwit-oss/whichlang, MIT) is a third option; 16 languages, competitive with whatlang on short text.

`cld3` is not recommended — it requires a C++ toolchain and protobuf, defeating the offline/pure-Rust story.

### 30.3 Block-level tagging rules

- A block inherits its parent heading's language if its own kana/Latin signal is inconclusive (below 15 characters, for instance).
- Code fences, inline code, link targets, image targets, YAML/TOML front-matter, and HTML blocks are tagged `none` and excluded from prose metrics.
- A document with both English and Japanese blocks is labelled `mixed` at the document level; each block still gets its own tag for metric routing.

---

## 31. English Prose Readability Suite

The English readability layer implements a peer-reviewed ensemble of classical formulas, reported side-by-side. Grade-level scales are not interchangeable (§31.8), so the tool emits every number and a median band rather than averaging.

### 31.1 Flesch Reading Ease (Flesch 1948)

```text
FRES = 206.835 − 1.015 * (words / sentences) − 84.6 * (syllables / words)
```

Output: 0–100, higher = easier. Bands: 90–100 = 5th grade, 60–70 ≈ 8th–9th grade, 0–30 = college / academic. Original: Flesch (1948), *Journal of Applied Psychology* 32(3):221–233.

### 31.2 Flesch-Kincaid Grade Level (Kincaid et al. 1975)

```text
FKGL = 0.39 * (words / sentences) + 11.8 * (syllables / words) − 15.59
```

Output: U.S. grade level. Calibrated on Navy technical manuals; adopted via MIL-M-38784A. Primary source: Kincaid, Fishburne, Rogers & Chissom, *Derivation of New Readability Formulas for Navy Enlisted Personnel*, Research Branch Report 8-75 (1975), DTIC ADA006655.

### 31.3 Gunning Fog Index (Gunning 1952)

```text
Fog = 0.4 * [(words / sentences) + 100 * (complex_words / words)]
```

`complex_word` = 3+ syllables, excluding proper nouns, familiar compound words, and words made 3-syllable only by inflectional suffixes `-es`, `-ed`, `-ing`. Practical approximation: strip those suffixes before counting syllables; capitalize-mid-sentence filter for proper nouns. Target for business writing: grade 7–12.

### 31.4 SMOG Index (McLaughlin 1969)

```text
SMOG = 1.0430 * sqrt(polysyllables * 30 / sentences) + 3.1291
```

`polysyllable` = any word with ≥3 syllables, counted with repetition. Recommended minimum: 30 sentences; mehen returns `null` below that. SMOG targets 100% comprehension, so its grade runs 2–3 levels higher than FKGL on the same text.

### 31.5 Automated Readability Index (Smith & Senter 1967)

```text
ARI = 4.71 * (characters / words) + 0.5 * (words / sentences) − 21.43
```

`character` = ASCII letter or digit. Syllable-free, deterministic. Known pathology: long CamelCase or snake_case identifiers inflate `characters/words`; keep them stripped.

### 31.6 Coleman-Liau Index (Coleman & Liau 1975)

```text
L   = 100 * letters / words
S   = 100 * sentences / words
CLI = 0.0588 * L − 0.296 * S − 15.8
```

Also syllable-free. Same identifier-length caveat as ARI.

### 31.7 New Dale-Chall (Chall & Dale 1995)

```text
PDW = 100 * difficult_words / words
ASL = words / sentences
Raw = 0.1579 * PDW + 0.0496 * ASL  (+ 3.6365 if PDW > 5%)
```

`difficult_word` = a word not on the 3000-item Dale-Chall familiar list, after inflectional stripping. Because Dale-Chall's list is not openly licensed, mehen defaults to the **NGSL 1.2** (New General Service List, Browne et al. 2013, http://www.newgeneralservicelist.com/, CC BY) with 2,800 headwords as the familiar baseline. The tool notes which list was used in its output provenance.

### 31.8 FORCAST (Caylor et al. 1973)

```text
FORCAST = 20 − (N / 10)
```

where N = single-syllable words in a 150-word sample. Unique among classical formulas in targeting **non-narrative** text (manuals, specs, forms). Does not use sentence length — robust when lists, tables, or headings violate running-prose assumptions. For API-reference pages and parameter tables, FORCAST is often the only defensible classical score. Primary: Caylor, Sticht, Fox & Ford, HumRRO TR 73-5 (1973).

### 31.9 LIX and RIX (Björnsson 1968; Anderson 1983)

```text
LIX = (words / sentences) + 100 * (long_words / words)   // long_word = ≥ 7 letters
RIX = long_words / sentences
```

Language-agnostic, syllable-free. Useful as a sanity check; over-penalizes common 7+-letter English words like "business".

### 31.10 Ensemble reporting and interpretation

Two formulas on the same text routinely disagree by 2–4 grade levels; averaging them is statistically wrong because they target different comprehension thresholds (SMOG ≈ 100% comprehension; FKGL ≈ 75%; Dale-Chall in between) (see Schriver, "Readability Formulas in the New Millennium", 2000, https://www.karenschriverassociates.com/wp-content/uploads/2020/03/8-Schriver-Readability-formulas-whats-the-use.pdf).

Mehen therefore:

1. Emits every formula's raw score with provenance.
2. Computes an **ensemble grade band** as `[min(FKGL, Fog, ARI, CLI), max(FKGL, Fog, ARI, CLI)]` — the interval where those four "running-prose" formulas agree.
3. Emits FORCAST separately as the preferred single score for non-narrative docs.
4. Reports SMOG only when `sentences >= 30`.
5. Reports Dale-Chall only with an explicit `list: ngsl-1.2` or `list: dale-chall-new-1995` provenance tag.

Retext's majority-vote rule (flag a sentence when ≥4 of 7 formulas agree it is above the target grade) is used internally for per-sentence highlighting, following `retext-readability` (https://github.com/retextjs/retext-readability).

### 31.11 Syllable counting

A pure-heuristic vowel-group counter is the Tier-0 default:

```rust
fn count_syllables(word: &str) -> usize {
    let w: String = word.to_ascii_lowercase()
        .chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if w.is_empty() { return 0; }
    let vowels = ['a','e','i','o','u','y'];
    let mut count = 0usize;
    let mut prev_vowel = false;
    for c in w.chars() {
        let is_v = vowels.contains(&c);
        if is_v && !prev_vowel { count += 1; }
        prev_vowel = is_v;
    }
    if w.ends_with('e') && !w.ends_with("le") && count > 1 { count -= 1; }
    if w.ends_with("ed") && count > 1 {
        let second_last = w.chars().rev().nth(2);
        if !matches!(second_last, Some('t') | Some('d')) { count -= 1; }
    }
    count.max(1)
}
```

Expected agreement with CMU-backed counters: ~85% on open-domain text. Behind `--features syllables-cmu`, mehen links the CMU Pronouncing Dictionary (via `syllarust`, https://lib.rs/crates/syllarust) for exact counts on ~134k in-vocabulary words with the heuristic as OOV fallback. The `hyphenation` crate (Knuth-Liang patterns, https://crates.io/crates/hyphenation) is a third option but is biased on silent-`e` words.

### 31.12 Sentence segmentation for English

UAX #29 (via `unicode-segmentation`, https://crates.io/crates/unicode-segmentation) is the Tier-0 default; known weaknesses on `Mr.`, `Dr.`, `e.g.`, `i.e.`, `U.S.`, `v1.2.3`, `file.ext`, and URLs are mitigated by:

- A bundled ~150-entry English abbreviation list (non-breaking period contexts).
- Never splitting when the period is followed by a lowercase letter, a digit, or `<space><digit>`.
- Treating Markdown block boundaries (blank line, heading line, fence open/close, list-item start) as **hard** sentence terminators regardless of punctuation.
- Stripping inline code (`` ` ` ``), fenced blocks, URLs, image alt-text, YAML/TOML/JSON front-matter, and HTML before segmentation.

Palmer & Hearst (1997) report ~0.2% error on declarative sentences for careful classification trees versus 3–10% for naive period-space-capital (https://people.ischool.berkeley.edu/~hearst/papers/cl-palmer.pdf).

### 31.13 Thresholds by document type

| Doc type | FKGL target | Fog target | Passive max | Max sentence words |
|---|---:|---:|---:|---:|
| README / overview | ≤ 10 | ≤ 12 | 15 % | 30 |
| Tutorial / how-to | ≤ 9 | ≤ 11 | 10 % | 25 |
| API reference | ≤ 12 | ≤ 14 | 20 % | 35 |
| ADR / design doc | ≤ 12 | ≤ 14 | 25 % | 40 |
| Error messages | ≤ 7 | ≤ 9 | 5 % | 15 |
| Release notes / CHANGELOG | ≤ 11 | ≤ 13 | 15 % | 30 |

These synthesize Google's and Microsoft's developer-style guides, 18F Content Guide, and Hemingway defaults. They are conventions, not peer-reviewed numbers, and are exposed as tunable profile defaults.

---

## 32. English Lexical Diversity and Density

These metrics are formula-independent indicators of vocabulary richness and content-word saturation. They do not depend on syllable counts and are robust across document types.

### 32.1 Lexical density (Ure 1971; Halliday 1985)

```text
LD = content_words / total_words
```

Without POS tagging, approximate as `LD ≈ 1 − stopwords / tokens`, using the 175-entry NLTK English stopword list. Typical ranges: spoken English ~0.40, written ~0.52, academic ~0.60. High LD is legitimate in technical prose; very low LD flags conversational or templatic text.

### 32.2 Moving-Average Type-Token Ratio (MATTR)

```text
MATTR(w) = mean of TTR over sliding windows of size w tokens
```

Window size `w = 50` is standard (Covington & McFall 2010). Length-invariant by construction and cheap to compute. MTLD (McCarthy 2005) and HD-D (McCarthy & Jarvis 2010) are reported as alternative diversity measures behind `--features lexical-diversity`. Primary reference: McCarthy & Jarvis, "MTLD, vocd-D, and HD-D: A validation study", *Behavior Research Methods* 42(2):381–392 (2010), https://pmc.ncbi.nlm.nih.gov/articles/PMC3813439/.

### 32.3 Yule's K (Yule 1944)

```text
K = 10000 * (sum over i of i^2 * V(i, N) − N) / N^2
```

where V(i, N) = types occurring exactly i times in a text of N tokens. Length-invariant in the strict sense demonstrated by Tanaka-Ishii & Aihara (2015, https://direct.mit.edu/coli/article/41/3/481/1519/). Optional; MATTR is usually sufficient.

### 32.4 Hapax and dis-legomena ratios

```text
HapaxRatio = V_1 / V         // types occurring exactly once
DisRatio   = V_2 / V         // types occurring exactly twice
```

Zipf's law predicts HapaxRatio ≈ 0.5 on natural prose. Extremely high values (> 0.6) flag "laundry-list" reference dumps; very low values flag repetitive template content.

### 32.5 Sentence-length and word-length moments

Report `avg_sentence_words`, `p90_sentence_words`, `max_sentence_words`, `stddev_sentence_words`, `avg_word_chars`, `p90_word_chars`. These drive the §31 formulas but are reported individually so writers can see the levers directly.

---

## 33. English Wording and Style Quality

### 33.1 Passive voice (write-good / retext-passive pattern)

Regex approximation:

```text
\b(am|are|were|being|is|been|was|be)\b\s*(\w+ed|<irregular_past_participle>)\b
```

where `<irregular_past_participle>` is a ~175-entry list from `passive-voice@0.1.0` (https://github.com/btford/write-good). Per-paragraph `passive_ratio = passive_sentences / total_sentences`. Threshold by doc type from §31.13. False positives on predicative adjectives ("was happy") are the main error mode; scoring uses ratios, not hard counts.

### 33.2 Hedge words (Hyland 2005; proselint)

Static list of ~165 hedges: `appears`, `approximately`, `could`, `essentially`, `generally`, `likely`, `may`, `maybe`, `might`, `often`, `perhaps`, `possibly`, `probably`, `rather`, `seems`, `some`, `somewhat`, `suggest`, `tend`, `typically`, `usually`, etc. (canonical list at https://github.com/words/hedges).

```text
hedge_density = hedge_matches / words
```

Threshold: flag when > 3% in non-narrative docs. API docs legitimately hedge around `null` returns; suppress in sentences containing backtick-wrapped identifiers or `returns` keywords.

### 33.3 Weasel words (write-good)

Static list: `are a number`, `clearly`, `completely`, `exceedingly`, `excellent`, `extremely`, `fairly`, `few`, `huge`, `interestingly`, `largely`, `many`, `mostly`, `obviously`, `quite`, `relatively`, `remarkably`, `several`, `significantly`, `substantially`, `surprisingly`, `tiny`, `various`, `vast`, `very` (with `too many`/`too few` exceptions).

### 33.4 Wordy phrases (too-wordy, retext-simplify)

Dictionary of ~240 verbose phrases with simpler alternatives: `in order to → to`, `due to the fact that → because`, `at this point in time → now`, `utilize → use`, `facilitate → help`, `in the event that → if`, `a number of → many`, `commence → start`, `terminate → end`. Canonical list at https://github.com/retextjs/retext-simplify/blob/main/lib/patterns.js. Per-match count normalized per 100 words.

### 33.5 Adverb density (-ly endings)

```text
adverb_density = (ly_words − non_adverb_ly_exceptions) / words
```

Exception list: `only`, `reply`, `apply`, `supply`, `family`, `early`, `likely`, `lovely`, `silly`, `holy`, `daily`, `weekly`, `monthly`, `yearly`, etc. Hemingway's informal budget: ≤ 1 per 100 words.

### 33.6 Nominalizations

Suffix-pattern matcher flags content-word tokens ending in `-tion`, `-sion`, `-ment`, `-ence`, `-ance`, `-ity`, `-ness`, `-ism`. Threshold: flag paragraph when > 10% of content words. Based on Williams, *Style: Toward Clarity and Grace* (1990).

### 33.7 Expletive constructions

```text
/^\s*(there|it)\s+(is|are|was|were)\b/i
```

applied to each sentence start after segmentation. Count per 100 sentences; informational for reference docs, flagged for tutorials.

### 33.8 Lexical illusions (doubled words)

Token pairs `(t[i-1], t[i])` where `lower(t[i-1]) == lower(t[i])` and the token matches `\w+`. Canonical from Matt Might's shell script (http://matt.might.net/articles/shell-scripts-for-passive-voice-weasel-words-duplicates/). Zero-tolerance defect.

### 33.9 Cliché and non-word lists (proselint)

- `no-cliches` list (~700 entries, https://github.com/words/no-cliches) → `cliche_density` per 1000 words.
- `nonwords` list (~32 entries: `irregardless→regardless`, `thusly→thus`, `analyzation→analysis`, etc.) → `nonword_count`, flagged at error level.

### 33.10 Long-sentence flagging

Default: flag sentences with > 30 words (warning) or > 40 words (error). Hemingway's app uses 14 / 21; Microsoft Writing Style Guide recommends ≤ 25; Google Developer Documentation Style Guide recommends ≤ 20. Profile-specific defaults match §31.13.

### 33.11 Wording Quality Score composition

```text
WordingQualityScore = clamp01(
    1
  - 0.18 * sat(passive_ratio; 0.25, 0.60)
  - 0.15 * sat(hedge_density; 0.02, 0.08)
  - 0.12 * sat(weasel_density; 0.01, 0.05)
  - 0.12 * sat(wordy_density; 0.01, 0.05)
  - 0.10 * sat(adverb_density; 0.02, 0.06)
  - 0.08 * sat(nominalization_density; 0.08, 0.20)
  - 0.08 * sat(long_sentence_rate; 0.05, 0.30)
  - 0.07 * sat(cliche_density; 0.002, 0.02)
  - 0.05 * (lexical_illusions > 0 ? 1 : 0)
  - 0.05 * (nonword_count > 0 ? 1 : 0)
)
```

Reported as a 0–1 score with sub-score breakdown. Deliberately orthogonal to FillerLazyRisk (§17), which covers repetition and specificity; WordingQualityScore covers style/register.

### 33.12 Inclusive Language Score

alex / retext-equality-style (https://github.com/retextjs/retext-equality) checks against a bundled list covering:

- Gendered defaults: `mankind→humanity`, `manhole→maintenance-hole`, `fireman→firefighter`.
- Ableist idioms: `crazy`, `insane`, `lame`, `dumb`, `blind to`, `tone deaf`.
- Exclusionary tech terms: `master/slave → primary/replica`, `whitelist/blacklist → allowlist/denylist`, `grandfather clause → legacy exception`, `sanity check → spot check`.
- Condescending: `obviously`, `just`, `simply`, `easy`, `of course`.

Representative exclusionary-tech-term substitutions:

| Don't use | Use instead |
|---|---|
| master | primary, main, leader, controller |
| slave | replica, secondary, follower, responder |
| whitelist | allowlist, approved list, inclusion list |
| blacklist | denylist, blocklist, exclusion list |

Output is a per-document `InclusiveLanguageScore` and a list of flags with source spans.

### 33.13 Known tools and references

- vale (https://vale.sh) — Go prose linter with 12 extension points; mehen's rule schema is designed for selective compatibility (see §37.4).
- write-good (https://github.com/btford/write-good) — passive, weasel, too-wordy, so, there-is, cliches, adverbs.
- proselint (https://github.com/amperser/proselint) — hedges, cliches, jargon, redundancy, sexism, nonwords.
- retext family (https://github.com/retextjs/retext/blob/master/doc/plugins.md) — retext-readability, retext-simplify, retext-passive, retext-equality, retext-repeated-words, retext-indefinite-article, retext-contractions, retext-intensify, retext-profanities.
- alex (https://alexjs.com) — inclusive-language wrapper.
- Hemingway Editor algorithm analysis — https://medium.com/free-code-camp/deconstructing-the-hemingway-app-8098e22d878d.
- harper-core (https://github.com/Automattic/harper) — Rust grammar checker with 200+ lint modules including many prose-quality checks; useful reference for idiomatic Rust implementations.
- cargo-spellcheck + nlprule (https://crates.io/crates/cargo-spellcheck) — Rust spell + LanguageTool-derived grammar rules.

---

## 34. Japanese Prose: Character-Composition Metrics (Tier 0)

Japanese is unusual among major languages: script-composition alone carries enough information to produce defensible readability scores without a tokenizer. This is the foundational insight of Tateishi, Ono & Yamada (1988) and remains the basis for mehen's Tier-0 Japanese layer.

### 34.1 Unicode script classification

Each grapheme cluster is classified into one of:

| Class | Primary block | Notes |
|---|---|---|
| Hiragana | U+3040–U+309F | + Small Kana Extension U+1B130–U+1B16F |
| Katakana | U+30A0–U+30FF | + Phonetic Extensions U+31F0–U+31FF, Halfwidth U+FF65–U+FF9F |
| Kanji (Han) | U+4E00–U+9FFF | + Ext A U+3400–U+4DBF, Ext B U+20000–U+2A6DF, Compatibility U+F900–U+FAFF |
| CJK punctuation | U+3000–U+303F | Includes `。` U+3002, `、` U+3001, `「」` U+300C–U+300D |
| Latin | U+0020–U+007E | + Fullwidth U+FF01–U+FF5E |
| Digit | 0–9, U+FF10–U+FF19 | |

### 34.2 Primary ratios

```text
kanji_ratio      = kanji_chars    / visible_chars
hiragana_ratio   = hiragana_chars / visible_chars
katakana_ratio   = katakana_chars / visible_chars
latin_ratio      = latin_chars    / visible_chars
digit_ratio      = digit_chars    / visible_chars
script_entropy   = Shannon entropy over the five classes above
```

`visible_chars` excludes whitespace and CJK/ASCII punctuation.

### 34.3 Register bands

Corpus evidence (https://www.japanesestudies.org.uk/ejcjs/vol12/iss3/premaratne.html, https://www.kanshudo.com/grammar/sentence_length, https://scriptin.github.io/kanji-frequency/):

| Kanji ratio | Likely register |
|---|---|
| < 20 % | Children's writing, conversation, elementary textbook |
| 20–30 % | Casual prose, novels, blogs, user-facing content |
| 30–40 % | Newspaper, business writing, standard non-fiction |
| 40–50 % | Technical, legal, academic, whitepaper |
| > 50 % | Classical/literary, dense Sino-Japanese specialist text |

Katakana ratio > 15 % typically signals technical software documentation (loanwords like `データベース`, `インターフェース`) or marketing copy. Hiragana ratio > 75 % indicates text aimed at small children or machine-translated output.

### 34.4 Script-run features (Tateishi inputs)

A "run" is a maximal substring of same-script characters. Per document:

```text
la = mean chars per alphabet run
lh = mean chars per hiragana run
lc = mean chars per kanji run
lk = mean chars per katakana run
pa, ph, pc, pk = percentage of each run type among all runs
ls = mean chars per sentence
cp = ten (、) per kuten (。)
```

These are the exact inputs the Tateishi formula needs. Script-run segmentation also approximates bunsetsu boundaries: in `設定ファイルを編集します`, the five runs (`設定 | ファイル | を | 編集 | します`) align closely with UniDic SUW morphemes, giving a usable tokenizer-free word-count proxy.

### 34.5 Sentence segmentation for Japanese

Primary terminators: `。` (U+3002), `！`, `？`, and their half-width equivalents `.!?` when the surrounding context is Japanese. Splitting rules:

- Do not split when the terminator is inside `「…」`, `『…』`, `（…）`, `(...)` (track bracket depth).
- Treat `\n\n` (blank-line paragraph boundary) as a sentence boundary even without a terminator.
- Treat Markdown block boundaries (list items, headings, fence open/close) as hard terminators.
- Ellipsis `…` / `‥` / `...` is not a terminator.
- Mid-sentence Latin prose does not re-enable `.?!` splitting for surrounding Japanese.

See W3C JLREQ for normative bracket classes (https://www.w3.org/TR/jlreq/?lang=en).

### 34.6 Sentence-length thresholds

Corpus-cited norms (https://wordrabbit.jp/blog/102, https://www.kanshudo.com/grammar/sentence_length, https://daib-log.com/character-length/, https://www.w3.org/TR/jlreq/):

| Source | Recommended chars/sentence |
|---|---:|
| Hirosaki University やさしい日本語 | ≤ 24 |
| Kanshudo Wikipedia+Tatoeba avg | ~18 |
| Nakamura Akira, 名文作法 | ~30 avg |
| Tatsuno Kazuo, 文章の書き方 | 30–35 |
| Yasumoto Biten, 説得の文章術 | ≤ 40–50 |
| Arase Yasuji, 科学論文作成上のルール | 50–60 |
| 公用文作成の要領 | 50–60 |
| Wasabi JPN style guide | ~40 |

Mehen defaults: flag sentences > 60 characters (warning), > 90 (hard-to-read), > 120 (error). Warn when mean sentence length > 60.

---

## 35. Japanese Prose: Readability Formulas

### 35.1 Tateishi, Ono & Yamada (1988) — canonical Japanese formula

Published at COLING 1988 (https://aclanthology.org/C88-2135/). Principal-component analysis on 77 adult-technical-text samples. The full 10-variable form:

```text
RS = 0.06 * pa + 0.25 * ph − 0.19 * pc − 0.61 * pk
   − 1.34 * ls − 1.35 * la + 7.52 * lh − 22.1 * lc − 5.3 * lk
   − 3.87 * cp − 109.1
```

Simplified 6-variable form (Tateisi, IPSJ SIG Note DPHI 18(4), 1988; reproduced in Sato, Matsuyoshi & Kondoh, LREC 2008, https://www.cs.brandeis.edu/~marc/misc/proceedings/lrec-2008/pdf/165_paper.pdf):

```text
RS = −0.12 * ls − 1.37 * la + 7.4 * lh − 23.18 * lc − 5.4 * lk − 4.67 * cp + 115.79
```

Calibrated so mean = 50, SD = 10, **higher = easier**. Mehen emits the simplified form as `tateishi_rs` with sanity guards: refuse when `hiragana_ratio > 0.90` (formula is gamed upward) or when character count < 300.

Tateishi's formula is the best fit for mehen's Tier-0 Japanese layer because it needs sentence boundaries and script runs — both computable without a tokenizer.

### 35.2 Jōyō grade proxy

The Ministry of Education's 2010 **Jōyō kanji** list contains 2,136 characters (https://migaku.com/blog/japanese/joyo-kanji-complete-guide). Of those, 1,026 are **Kyōiku kanji** with assigned elementary-school grades 1–6; the remaining 1,110 are "secondary" (junior high + high school). Non-Jōyō kanji are 表外漢字 (hyōgai), immediate difficulty flags.

Mehen ships a static `jouyou_grades.rs` table mapping each char to a grade 1–8 (1–6 elementary, 7 = secondary Jōyō, 8 = non-Jōyō). Size: ~6 KB uncompressed.

```text
jouyou_grade_mean = mean(grade(c) for each kanji c in text)
hyougai_ratio     = non_jouyou_kanji_chars / kanji_chars
```

`jouyou_grade_mean` is a direct Japanese-school-grade analogue to Flesch-Kincaid. Values < 3 indicate elementary-school reading; > 6 indicates high-school+ technical prose.

### 35.3 JLPT word/kanji bands (optional)

Behind `--features japanese-jlpt`, mehen loads bundled JLPT N5–N1 word and kanji lists (~12k word entries, ~300 KB; sources https://www17408ui.sakura.ne.jp/tatsum/J-LEX/, https://www.kanshudo.com/collections/wikipedia_jlpt). Each token is tagged with its JLPT band (5 = easiest N5, 1 = hardest N1, 0 = outside N1). Reports `jlpt_band_distribution` and `above_n1_ratio`.

### 35.4 Shibasaki & Hara (2010) — school-grade predictor

KAKENHI-19300277 multiple-linear regression:

```text
Grade = −0.148 * Hp + 1.585 * Pc − 0.117 * Cs − 0.126 * Bs + 15.581
```

`Hp` = % hiragana chars, `Pc` = mean predicates per sentence, `Cs` = mean chars per sentence, `Bs` = mean bunsetsu boundaries per sentence. Output: Japanese school grade 1–9. `Pc` and `Bs` require a morphological analyzer. Emitted when `--features japanese-morph` is enabled.

Reference: KAKENHI project 19300277, https://kaken.nii.ac.jp/en/grant/KAKENHI-PROJECT-19300277/.

### 35.5 Lee & Hasebe jReadability (2015, 2020) — current state of the art

Stepwise linear regression on 958 1,000-character passages from 100 JFL textbooks, R² = 0.896. Formula (Lee & Hasebe 2020, http://jhlee.sakura.ne.jp/papers/lee-et-al2016rb.pdf):

```text
Readability = 11.724
            − 0.056 * mean_sentence_words
            − 0.126 * percent_kango
            − 0.042 * percent_wago
            − 0.145 * percent_verbs
            − 0.044 * percent_aux_particles
```

Higher = easier. Six-level mapping (https://github.com/joshdavham/jreadability):

| Score range | Level (approximately JLPT-aligned) |
|---|---|
| [5.5, 6.5) | Lower-elementary (easiest) |
| [4.5, 5.5) | Upper-elementary |
| [3.5, 4.5) | Lower-intermediate |
| [2.5, 3.5) | Upper-intermediate |
| [1.5, 2.5) | Lower-advanced |
| [0.5, 1.5) | Upper-advanced (hardest) |

Requires UniDic's `goshu` (word-origin) attribute to split 漢語 (Sino-Japanese) from 和語 (native Japanese) and 外来語 (loanwords). Proper nouns and gairaigo are excluded. Accordingly, jReadability is a Tier-2 feature behind `--features japanese-unidic` (Vibrato + UniDic, or Lindera-UniDic).

### 35.6 Obi / Obi2 (Sato et al. 2008, 2014)

Character-n-gram language-model-based readability trained on a 1,478-passage grade-labeled corpus spanning elementary G1 through college (https://www.cs.brandeis.edu/~marc/misc/proceedings/lrec-2008/pdf/165_paper.pdf, http://www.lrec-conf.org/proceedings/lrec2014/pdf/633_Paper.pdf). Mehen does not reimplement Obi directly — the LM requires a training corpus mehen cannot redistribute — but Obi's T13 grade scale is the reference interpretation for `jouyou_grade_mean`.

### 35.7 Mizuno/Goda JLPT-grounded sentence difficulty (optional)

```text
S = (w1 * vg + w2 * kg) / sl
```

`vg` = sentence's JLPT vocabulary level (0 = unlisted, 1–4 = bands), `kg` = JLPT kanji level, `sl` = sentence chars. Higher = easier. Useful when JLPT features are enabled; available at the sentence level for per-paragraph highlighting.

---

## 36. Japanese Prose: Wording, Style, and JTF Conformance

### 36.1 Comma/period ratio (tōten/kuten)

```text
cp = count(、) / count(。)
```

Already an input to the Tateishi formula; reported as a standalone clause-complexity signal. Higher `cp` = longer, more subordinated sentences.

### 36.2 Jukugo density

Jukugo (熟語) are kanji compounds ≥ 2 characters. Without a tokenizer:

```text
jukugo_density = kanji_runs_with_len_ge_2 / total_kanji_runs
```

High jukugo density indicates formal / technical / Sino-Japanese register. With a tokenizer, refine to noun morphemes whose surface is ≥ 2 kanji.

### 36.3 Kango / wago / gairaigo split

Available only with UniDic's `goshu` attribute (`--features japanese-unidic`):

- High `percent_gairaigo` (外来語, katakana loanwords) → trendy / IT / marketing register.
- High `percent_kango` (漢語, Sino-Japanese) → formal / technical / bureaucratic.
- High `percent_wago` (和語, native Japanese) → conversational / literary.

Imbalance (e.g., documented policy says `kango ≤ 40 %` but the text is 60 %) is a style flag.

### 36.4 Politeness level (keitai/jōtai)

Simple suffix matching on sentence-final morphemes classifies each sentence without a tokenizer:

- **です・ます (keitai / polite)**: `です`, `ます`, `でした`, `ました`, `ません`, `でしょう`, `ましょう`.
- **だ・である (jōtai / plain)**: `だ`, `である`, `だった`, `であった`, sentence-final i-adjective.
- **Honorific (sonkeigo/kenjōgo)**: `いらっしゃる`, `召し上がる`, `おります`, `ございます`, `お〜ください`, `ご〜いただく`.

```text
politeness_dominant          = majority class
keitai_jotai_mix_count       = sentences of the non-majority class
```

JTF rule #1 requires consistency; mehen flags mixed-register documents unless the user explicitly opts in (e.g., textlint preset splits headers, body, and lists into different register zones).

### 36.5 JTF Japanese Style Guide violations

The Japan Translation Federation's 12 rules (https://www.jtf.jp/tips/styleguide, English https://www.jtf.jp/pdf/jtf_style_guide_e.pdf) are mechanically checkable:

| Rule | Check | Severity |
|---|---|---|
| 1 | keitai/jōtai consistency | warn |
| 2 | `、` / `。` used as punctuation | info |
| 3 | Stick to Jōyō kanji (flag hyōgai) | warn |
| 4 | Okurigana per official rules | info |
| 5 | Trailing long-vowel mark on katakana compound endings (`コンピューター` not `コンピュータ`) | warn |
| 6 | Long katakana compounds broken with `・` or half-width space | info |
| 7 | Kanji / hiragana / katakana full-width | error |
| 8 | Digits and Latin alphabet half-width | warn |
| 9 | Symbols full-width | info |
| 10 | No space between full-width and half-width | info |
| 11 | `.`, `,`, spaces half-width | info |
| 12 | Standardize unit notation | info |

### 36.6 textlint preset-ja-technical-writing heuristics

Mehen ports a subset of `textlint-rule-preset-ja-technical-writing` (https://github.com/textlint-ja/textlint-rule-preset-ja-technical-writing) with their documented defaults:

| Rule | Default | What it checks |
|---|---|---|
| `sentence-length` | ≤ 100 chars | Long-sentence flag |
| `max-comma` | ≤ 3 `,` / sentence | Over-comma'd sentences |
| `max-ten` | ≤ 3 `、` / sentence | Over-reading-marked sentences |
| `max-kanji-continuous-len` | ≤ 6 consecutive kanji | Hard-to-read kanji runs |
| `no-mix-dearu-desumasu` | zone-aware | JTF rule 1 |
| `ja-no-mixed-period` | `。` | Sentence terminator consistency |
| `no-double-negative-ja` | — | `ないではない` |
| `no-doubled-joshi` | min_interval: 1 | Repeated particles (`を…を`) |
| `no-doubled-conjunctive-particle-ga` | — | Repeated `が` |
| `no-doubled-conjunction` | — | `しかし…しかし` |
| `no-dropping-the-ra` | — | Colloquial `見れる` for `見られる` |
| `no-hankaku-kana` | — | Halfwidth kana forbidden |
| `no-exclamation-question-mark` | — | `!` / `?` in technical docs |
| `ja-no-weak-phrase` | — | `かもしれない`, `と思います` |
| `ja-no-successive-word` | — | Repeated words |
| `ja-no-abusage` | — | Misused kanji |
| `ja-no-redundant-expression` | — | `することができる` → `できる` |
| `ja-unnatural-alphabet` | — | IME miscarriages |

All thresholds are user-tunable via profile configuration.

### 36.7 Japanese Wording Quality Score

```text
WordingQualityScore_ja = clamp01(
    1
  - 0.15 * sat(long_sentence_rate;          0.05, 0.30)
  - 0.12 * sat(weak_phrase_density;         0.01, 0.05)
  - 0.12 * sat(redundant_expression_rate;   0.01, 0.05)
  - 0.10 * sat(doubled_joshi_count / sentences;  0.02, 0.10)
  - 0.10 * sat(long_kanji_run_rate;         0.05, 0.25)
  - 0.10 * (keitai_jotai_mix_count > 0 ? sat(mix_ratio; 0.02, 0.20) : 0)
  - 0.08 * sat(max_comma_violation_rate;    0.02, 0.15)
  - 0.08 * sat(hyougai_ratio;               0.02, 0.10)
  - 0.07 * sat(jtf_violation_density;       0.5,  5.0)   // per 1000 chars
  - 0.08 * sat(gairaigo_excess;             0.30, 0.60)
)
```

Reported as a 0–1 score with sub-score breakdown. The coefficients are seed heuristics and will be re-tuned against real corpora per §26.

### 36.8 Integration with existing mehen metric families

| Family | Japanese extension |
|---|---|
| **MCC** (§8) | Contributes `keitai_jotai_mix_count` and over-kanji-run penalties. |
| **DMI** (§10) | `WordingQualityScore_ja` replaces the English `WordingQualityScore` when the document is JA-dominant. |
| **FillerLazyRisk** (§17) | Adds JA filler/weak-phrase patterns (`とても`, `すごく`, `なんとなく`, `〜的な`, `感じ`, `させていただく`, `〜のほう`, `〜という形で`). |
| **Review Criticality Index** (§18) | Uses `jouyou_grade_mean` as a density contributor when JA. |

---

## 37. Implementation and Integration

### 37.1 Tiered feature strategy

| Tier | Cargo features | What you get | Binary cost |
|---|---|---|---|
| 0 (default) | none | Unicode-block language detection; UAX #29 sentence/word segmentation; vowel-group English syllables; Tateishi simplified RS; all wording heuristics that run off static lists; all JTF mechanical checks | ~100–300 KB |
| 1a | `syllables-cmu` | CMU Pronouncing Dictionary for English syllables | +1–2 MB |
| 1b | `japanese-jouyou` | Jōyō grade proxy, hyōgai ratio | +10 KB |
| 1c | `japanese-jlpt` | JLPT N5–N1 word and kanji bands | +300 KB |
| 1d | `lingua` | High-accuracy trigram language detection (EN + JA only) | +2–5 MB |
| 2a | `japanese-morph` (Lindera + embedded IPADIC) | Bunsetsu counts, POS tags, Shibasaki grade, Jukugo morphological refinement | +50 MB |
| 2b | `japanese-unidic` (Vibrato + external UniDic) | jReadability replica, kango/wago/gairaigo split | external dict |
| 2c | `lexical-diversity` | MTLD, HD-D, Yule's K | +50 KB |
| 2d | `vale-rules` | Parser for vale-compatible YAML rule packs (existence, substitution, occurrence, repetition, capitalization primitives) | +200 KB |

### 37.2 Recommended crate dependencies

- `unicode-script` (MIT/Apache-2.0, https://crates.io/crates/unicode-script) — UAX #24 script lookups. Tier 0.
- `unicode-segmentation` (MIT/Apache-2.0, https://crates.io/crates/unicode-segmentation) — UAX #29 boundaries. Tier 0.
- `unicode-properties` (https://docs.rs/unicode-properties) — general category & emoji props. Tier 0.
- `regex` (Apache-2.0, Unicode-aware) — for passive-voice, hedge-word, abbreviation-trap handling. Tier 0.
- `syllarust` (MIT, https://lib.rs/crates/syllarust) — CMU-backed English syllables. Tier 1a.
- `whatlang` (MIT, https://crates.io/crates/whatlang) — lightweight trigram language detection. Optional tier 1d alternative.
- `lingua` (Apache-2.0, https://github.com/pemistahl/lingua-rs) — high-accuracy trigram. Tier 1d.
- `lindera` (MIT, https://github.com/lindera/lindera) with `embed-ipadic` + `compress` features. Tier 2a.
- `vibrato` (MIT/Apache-2.0, https://github.com/daac-tools/vibrato) with external UniDic. Tier 2b.
- `icu_segmenter` (https://crates.io/crates/icu_segmenter) — alternative sentence segmentation with locale tailoring, if the UAX #29 defaults prove insufficient.

`cld3` is not used (C++ toolchain dependency). `yoin` is not used (unmaintained). `franc-rs` is not used (lags canonical franc).

### 37.3 Dictionary licensing

- **NGSL 1.2** (Browne et al. 2013, http://www.newgeneralservicelist.com/) — CC BY; embed freely with attribution.
- **Dale-Chall 3000-word list** — copyright Jeanne Chall and Edgar Dale's heirs; do not bundle. Use NGSL as the default familiar-word source; allow the user to point to a locally obtained Dale-Chall list via `--dale-chall-list <path>`.
- **Jōyō kanji list** — Japanese Ministry of Education public-domain policy document; bundle freely.
- **JLPT word/kanji lists** — no official release from JEES/JF; community lists under various licenses. Mehen bundles J-LEX-derived lists with attribution.
- **IPADIC** — IPA/IPAdic license; bundled via Lindera's `embed-ipadic` feature; NOTICE file propagation required.
- **UniDic** — BSD-like with NINJAL credit; external dict via Vibrato.
- **CMU Pronouncing Dictionary** — public domain; bundled via `syllarust`.

A `LICENSE-THIRD-PARTY` file must accompany any binary with Tier-1 or Tier-2 features enabled.

### 37.4 vale-rule compatibility surface

Vale's 12 extension points (https://vale.sh/docs/checks/existence) cleanly map onto mehen's wording layer:

| Vale extension | Mehen equivalent |
|---|---|
| `existence` | Direct regex/token flag |
| `substitution` | Preferred-form map (already the model for retext-simplify) |
| `occurrence` | Min/max pattern count per scope |
| `repetition` | Already used for lexical illusions |
| `consistency` | Oxford comma, contraction, quote style |
| `conditional` | Used for jargon-introduction rules |
| `capitalization` | Heading / term case |
| `metric` | User-defined formulas over counts |
| `readability` | Subset of §31 formulas |
| `spelling` | Tier-2; opt-in Hunspell |
| `sequence` | Tier-2; requires POS tagging |
| `script` | Explicitly excluded (no Tengo / embedded scripting) |

Behind `--features vale-rules`, mehen parses vale YAML for the first nine primitives and emits flags through the same reporting pipeline. `sequence` and `script` are rejected with a clear error. This lets users adopt the Microsoft, Google, proselint, and alex vale packs without running the vale binary.

### 37.5 Anti-gaming defenses

Carrying over from §26.4, the prose layer must resist:

- **Code-block exfiltration of metrics**: prose heuristics never count content of `fenced_code_block`, `inline_code`, `link destinations`, `image_block` targets, `html_block`, `mdx_jsx_block`, `front_matter`, or `table_cell` delimiters.
- **Identifier inflation**: a long CamelCase or snake_case identifier is one word; its character contribution to ARI / Coleman-Liau is capped (`min(identifier_len, 20)`) when it appears in running prose.
- **Citation padding**: quoted-literal blockquotes count toward structural metrics but not toward weasel/hedge detection.
- **Short-doc gaming**: grade-level scores are suppressed when `words < 100` or `sentences < 5`; a `short_doc_warning` is emitted instead of a (manipulable) grade.
- **Abbreviation splitting**: the bundled abbreviation list suppresses sentence breaks after `Mr.`, `e.g.`, `i.e.`, `U.S.`, `vs.`, `approx.`, `fig.`, `ver.`, `ch.`, etc.

### 37.6 Validation plan addendum

Extending §26, the prose layer needs its own validity checks:

1. **Construct validity**: each readability formula matches its published grade scale on a held-out McCall-Crabbs or JLPT-aligned sample.
2. **Criterion validity**: scores correlate with human-labelled readability on a mehen-collected corpus of open-source READMEs and ADRs (targeting Spearman ρ ≥ 0.5 between FKGL and human grade labels).
3. **Cross-language validity**: a bilingual document (EN + JA) produces two coherent sub-scores; switching dominant language flips the active pipeline cleanly.
4. **Short-doc behavior**: a 40-word README produces no grade-level numbers, only raw counts and a warning.
5. **Tier equivalence**: Tier-0 Japanese (Tateishi) results correlate (ρ ≥ 0.7) with Tier-2 (jReadability) on the same text, so users without morphology dictionaries still get a useful signal.
6. **Anti-gaming**: none of the gaming attacks above shift the overall `WordingQualityScore` by more than 0.05.

---

## 38. Final Recommendation (Prose Layer)

The minimum strong first release of the prose layer should implement, in order:

```text
Tier 0:
  Unicode-script block-ratio language detection
  UAX #29 sentence/word segmentation with abbreviation list
  Vowel-group English syllables
  English readability suite: FRES, FKGL, Fog, ARI, Coleman-Liau
  English wording: passive, hedges, weasels, wordy-phrases, adverbs,
                   nominalizations, expletives, lexical illusions, nonwords,
                   long sentences, inclusive-language flags
  English lexical: MATTR, hapax ratio, lexical density
  Japanese script composition and register bands
  Japanese sentence splitter with bracket awareness
  Tateishi (1988) simplified RS
  JTF rules 1, 3, 5, 7, 8, 11 (mechanically checkable)
  textlint defaults: sentence-length, max-comma, max-ten,
                     max-kanji-continuous-len, no-doubled-joshi,
                     ja-no-weak-phrase, ja-no-redundant-expression
  Politeness (keitai/jōtai) classification
```

This delivers ~80% of the observable readability and wording signal with zero large-dictionary dependencies. Tiers 1–2 can follow as feature flags for users who need CMU-exact English syllables, Jōyō grade, JLPT bands, or Lindera/Vibrato morphology.

The prose layer never silently changes structural scores; it is always an additional, explicitly-labelled reporting surface that writers and reviewers can act on.

---

## 39. PR Comment Design (`mehen diff` → GitHub sticky comment)

The primary consumption surface for `mehen` today is a GitHub Action that runs `mehen diff` and posts a sticky PR comment (example: https://github.com/wharflab/tally/pull/633). The source-code section of that comment is already established; when the Markdown/prose layer ships, the same comment grows a **Documentation** section below the existing table. This chapter is the design spec for that section.

The non-negotiable constraint: **every character of output must be mechanically derivable from the AST, metric tables, and threshold bands defined in §§5–36.** No call to an LLM, no heuristic that "guesses cause", no speculative phrasing. Reviewers must trust that re-running `mehen diff` on the same commits produces byte-identical output.

### 39.1 Anchor and integration

The Documentation section is a sibling of the source-code section inside the existing sticky comment, demarcated by an HTML comment anchor so upserts replace the right region:

```markdown
<!-- mehen-docs -->
## 📝 Documentation Metrics (this PR vs `main`)

…table…

…callouts…

…drill-down <details>…

> Generated by [mehen](https://github.com/ophidiarium/mehen) — the code quality watcher.
```

The section is emitted only when at least one Markdown file is present in the PR diff (added, modified, or renamed). It is suppressed entirely — not rendered empty — when no Markdown changed, so the comment stays compact on code-only PRs.

### 39.2 Headline table — columns

Reviewers skim. The headline table carries five columns, chosen so that each row exposes one signal per structural dimension without duplication:

| Column | Source §§ | Why it's in the headline |
|---|---|---|
| **DMI** (0–100) | §10 | Single overall maintainability score; reviewer's first glance. |
| **Words** | §4 (W) | Size sanity; catches bulk additions that other signals may normalize away. |
| **FKGL** (English) or **Tateishi RS** (Japanese) | §31.2, §35.1 | The most recognizable readability number for the dominant language. |
| **Link Debt** (0–1) | §11.2 | Objective defects (broken/unresolved links) — hard to argue with. |
| **Filler Risk** (0–1) | §17 | AI-era flag; catches "big but vacuous" regressions. |

Rationale for the omissions: RCI, MCC, MRPC, WQS, Evidence Coverage, and Grounding all matter but are second-glance signals. They live in the `<details>` drill-down (§39.6). The research doc (§28) treats DMI and RCI as the headline axes; RCI is demoted here because reviewers already know a PR is worth reviewing (they are reading it), while DMI's trend is more actionable as a delta.

If the document is Japanese-dominant (§30), the third column header flips to **Tateishi RS** and the value uses the simplified formula from §35.1. Mixed-language docs report the dominant-language score and mark the file with a 🌏 suffix.

### 39.3 Cell format

Every cell follows one of four canonical shapes. No other shapes are allowed; if none fits, the cell is empty `—`.

| Shape | Example | Meaning |
|---|---|---|
| `new (main: old) indicator` | `74 (main: 71) 🟢` | Modified file: before/after + delta category |
| `value 🆕` | `58 🆕` | New file: no "main" baseline exists |
| `value ⚪` | `0 ⚪` | Deleted metric or undefined for this file type |
| `— footnote-mark` | `— ²` | Suppressed by guard (e.g., short-doc for FKGL); footnote explains |

Numbers are always rendered with fixed precision per column so columns align visually:
- DMI, RCI: integer
- Words, sentence counts, diagram/table/link counts: integer with thousands separators
- Ratios and scores (0–1): 2 decimal places
- FKGL, Fog, ARI, Tateishi RS: 1 decimal place

### 39.4 Delta indicator rules

Every indicator is a pure function of `(old, new, thresholds)` with no inference. The rules below are normative; implementations must match them exactly.

```text
🟢 improvement:  delta crosses a band boundary in the "better" direction
                 OR |delta| ≥ noticeable_threshold in the "better" direction
🔴 regression:   delta crosses a band boundary in the "worse" direction
                 OR |delta| ≥ noticeable_threshold in the "worse" direction
⚠️ attention:    value is in a "warn" or worse band AND did not improve
                 (used even when delta is zero — the state matters)
🆕 new:          file is new in the PR; no "main" value exists
⚪ unchanged:    none of the above applies
```

Per-metric band boundaries and noticeable thresholds:

| Metric | Direction | Band source | `noticeable_threshold` |
|---|---|---|---|
| DMI | ↑ better | §10.4 (85/70/50/30) | 3 points |
| RCI | — informational | §18.2 | never emits 🟢/🔴 by itself |
| FKGL | profile-specific target | §31.13 | 0.5 grade |
| Tateishi RS | ↑ better | §35.1 (centered at 50) | 2 points |
| Fog | profile-specific target | §31.13 | 0.5 grade |
| Link Debt | ↓ better | §11.2 (0/0.2/0.5/1) | 0.05 OR any new broken link |
| Filler Risk | ↓ better | §17.10 (0.2/0.4/0.6/0.8) | 0.05; ⚠️ when ≥ 0.60 regardless of delta |
| Evidence Coverage | ↑ better | §16.4 | 0.05 |
| MCC | ↓ better | §8.5 | 5 points OR band crossing |
| MRPC | profile-specific | §7.4 | 3 points AND profile-exceeded |
| Passive ratio | profile target | §31.13, §33.1 | 0.05 absolute |
| Long-sentence count | ↓ better | §33.10 thresholds | any new instance is 🔴 |
| Inclusive-language flags | ↓ better | §33.12 | any new flag is 🔴 |
| Repository Grounding | ↑ better | §15.3 | 0.05 |
| Jukugo / kanji-run warnings (JA) | profile target | §36.6 | any new violation is 🔴 |

Rationale for "any new broken link / long sentence / inclusive-language flag is 🔴": these are objective defects (§11, §33) that reviewers should always see, irrespective of ratio deltas.

The word count never emits 🟢 or 🔴 — it is informational only. Size changes are context for other deltas, not a quality signal.

### 39.5 Callout block — ranking, cap, and template catalog

The callout block is the most valuable part of the section: it tells reviewers *what specifically to look at*, with the exact document locations. Every callout must come from the template catalog below. No free-text generation is permitted.

#### 39.5.1 Ranking and cap

Callouts are ranked by severity class, then by magnitude within class:

```text
Severity rank (descending):
  1. broken_link_added, parse_error_added, inclusive_language_flag_added,
     nonword_added, lexical_illusion_added
  2. filler_risk_high (≥ 0.60), dmi_drop_band_crossing,
     evidence_coverage_drop_band_crossing
  3. long_sentence_added, passive_ratio_breach, readability_target_breach,
     table_burden_hard_warning, heading_skip_added
  4. diagram_added_unlabeled, code_fence_added_unlabeled,
     artifact_without_nearby_explanation
  5. improvements (band crossings in the better direction)
  6. informational (new-file summary, diagram/table counts when useful)
```

Default cap: **8 callouts**. Remaining callouts go into a `<details>` expander labelled `N more callouts`. The cap is profile-configurable.

Improvements are always emitted unless they would push the cap past 8 by displacing a regression; a regression always outranks an improvement of the same magnitude.

#### 39.5.2 Template catalog

Each template is identified by a `rule_id`. Implementations must emit callouts exclusively through these templates. Slot values are mechanically sourced from: AST nodes, metric fields, threshold constants, or the filename. Literal string slots (link targets, language tags, heading text) come verbatim from the AST — they are **not** synthesized.

Template grammar conventions:
- `{file}`: relative repo path (rendered as a Markdown link to the blob at the PR head SHA, as in the source-code bot)
- `{n}`, `{m}`, `{k}`: integer counts
- `{old}`, `{new}`: metric values at numeric precision documented in §39.3
- `{L:N}`: line number N in `{file}` (rendered as `L47`)
- `{s}`: literal string extracted from the AST (link target, fence info string, heading text) rendered in backticks
- `{band}`: band label from the relevant section (`HIGH`, `severe`, `hard`, etc.)

**Objective-defect callouts (severity 1):**

| `rule_id` | Template |
|---|---|
| `broken_relative_link_added` | `🔴 **{file}** — {n} unresolved relative link(s) added: {s₁} ({L:N₁}){, s₂ (L:N₂)…}` |
| `broken_anchor_added` | `🔴 **{file}** — {n} unresolved internal anchor(s) added: {s₁} ({L:N₁}){, …}` |
| `broken_external_link_added` | `🔴 **{file}** — {n} broken external link(s) added (link-check enabled): {s₁} ({L:N₁}){, …}` |
| `diagram_parse_error_added` | `🔴 **{file}** — {lang} diagram parse error at {L:N}` |
| `inclusive_language_flag_added` | `🔴 **{file}** — {n} inclusive-language flag(s) added: {s₁} ({L:N₁}){, …}` |
| `nonword_added` | `🔴 **{file}** — non-word {s} at {L:N} (suggest: {replacement})` |
| `lexical_illusion_added` | `🔴 **{file}** — doubled word {s} at {L:N}` |

**Band-crossing callouts (severity 2):**

| `rule_id` | Template |
|---|---|
| `filler_risk_high` | `⚠️ **{file}** — filler/lazy risk {new} ({band}); top contributors: {label₁} {v₁}, {label₂} {v₂}, {label₃} {v₃}` |
| `dmi_band_drop` | `🔴 **{file}** — DMI {old} → {new}, crossed {old_band} → {new_band} (§10.4)` |
| `evidence_band_drop` | `🔴 **{file}** — evidence coverage {old} → {new}, crossed {old_band} → {new_band} (§16.4)` |
| `repo_grounding_band_drop` | `🔴 **{file}** — repository grounding {old} → {new}, crossed {old_band} → {new_band} (§15.3)` |

The `{label_i} {v_i}` slots for `filler_risk_high` are the top three sub-scores from §17.11 (`large-unanchored-prose`, `low-repository-grounding`, `lazy-sectioning`, `low-artifact-density`, `near-duplicate-paragraphs`, `specificity-scarcity`, `hollow-references`, `placeholder-heavy`), sorted by magnitude. Each label comes from §17.11 verbatim — not paraphrased.

**Readability / wording regressions (severity 3):**

| `rule_id` | Template |
|---|---|
| `long_sentences_added` | `🔴 **{file}** — {n} sentence(s) exceed {threshold} words (new): {L:N₁}{, L:N₂…}` |
| `readability_target_breach` | `🔴 **{file}** — {formula} {old} → {new}, above {profile} target {target} (§31.13)` |
| `tateishi_band_drop` | `🔴 **{file}** — Tateishi RS {old} → {new} (harder; §35.1)` |
| `passive_ratio_breach` | `🔴 **{file}** — passive ratio {old} → {new}, above {profile} max {max} (§33.1)` |
| `heading_skip_added` | `🔴 **{file}** — heading skip {old_level} → {new_level} at {L:N}` |
| `table_burden_hard` | `⚠️ **{file}** — table at {L:N} has {cells} cells / {cols} columns / {rows} rows (hard warning; §13.2)` |
| `doubled_joshi_added` (JA) | `🔴 **{file}** — repeated particle {s} at {L:N}` |
| `kanji_run_too_long_added` (JA) | `🔴 **{file}** — kanji run of {n} chars exceeds limit {max}: {s} at {L:N}` |

**Artifact-hygiene callouts (severity 4):**

| `rule_id` | Template |
|---|---|
| `code_fence_unlabeled_added` | `⚠️ **{file}** — unlabelled code fence at {L:N}` |
| `diagram_missing_caption_added` | `⚠️ **{file}** — {lang} diagram at {L:N} has no caption or nearby explanation` |
| `image_missing_alt_added` | `⚠️ **{file}** — image {s} at {L:N} has no alt text` |
| `artifact_unexplained_added` | `⚠️ **{file}** — {artifact_type} at {L:N} has no explanatory prose within ±2 blocks` |

**Improvements (severity 5):**

| `rule_id` | Template |
|---|---|
| `dmi_band_improve` | `🟢 **{file}** — DMI {old} → {new}, crossed {old_band} → {new_band} (§10.4)` |
| `filler_risk_band_improve` | `🟢 **{file}** — filler/lazy risk {old} → {new}, crossed {old_band} → {new_band} (§17.10)` |
| `broken_links_resolved` | `🟢 **{file}** — {n} previously broken link(s) resolved` |
| `long_sentences_resolved` | `🟢 **{file}** — {n} sentence(s) previously over {threshold} words now under` |
| `readability_target_recovered` | `🟢 **{file}** — {formula} {old} → {new}, now within {profile} target {target}` |

**New-file summary (severity 6, emitted only for added .md files):**

| `rule_id` | Template |
|---|---|
| `new_file_summary` | `🆕 **{file}** — {words} words, {headings} headings, {code_fences} code fence(s), {diagrams} diagram(s), {tables} table(s); DMI {dmi}, filler risk {filler} ({band})` |

Everything that does not match a template is silently dropped from the callout block (it may still appear in the drill-down tables of §39.6). This is the mechanism that prevents free-form, LLM-style narration from creeping in.

#### 39.5.3 Permitted and forbidden language

The callout grammar is deliberately thin:

**Permitted verbs and connectors:** `added`, `resolved`, `exceed`, `crossed`, `above`, `below`, `has`, `missing`, `unresolved`, `broken`, `previously`, `now`, `within`, `no caption`, `no alt text`, `→`, `;`, `,`, `(`, `)`.

**Forbidden:** `after`, `because`, `due to`, `caused by`, `following`, `since`, `likely`, `probably`, `appears to`, `seems`, `may indicate`, `suggests`, `possibly`. Anything that implies causation or intent about the author's edits.

This rule is the hard line between "CI metrics report" and "automated review feedback that over-reaches". It also makes the output easy to test: a callout is correct iff it exactly matches its template with slots filled from documented sources.

### 39.6 Drill-down tables (`<details>`)

Below the callouts, a collapsed `<details>` block holds deeper tables for reviewers who want them. The layout mirrors the taxonomy of the research document:

```markdown
<details>
<summary>Full metric breakdown (structural · wording · lexical · readability)</summary>
```

Inside, four tables in this order:

1. **Structural / review** — RCI, MCC, MRPC, Evidence Coverage, Repository Grounding.
2. **English wording quality** (suppressed if no English file) — WQS, passive %, hedges/100w, long-sentence count, nominalization density.
3. **English lexical & readability ensemble** — MATTR₅₀, hapax ratio, Fog, SMOG (only when sentences ≥ 30), ARI, Coleman-Liau.
4. **Japanese composition & register** (suppressed if no Japanese file) — kanji %, hiragana %, katakana %, avg sentence chars, comma/period ratio, politeness dominant.

Each drill-down table follows the same cell-format rules as the headline table (§39.3). Columns where every row is ⚪ in a given PR are omitted from that table; if all four tables collapse to noise, the `<details>` block itself is omitted.

Below the tables, a **Filler risk contributors** block lists the top 3 filler sub-scores per file that has `filler_risk > 0.40`, using the §17.11 labels verbatim. This deliberately duplicates some callout content — the callout shows the worst offender, the drill-down shows the complete picture.

### 39.7 Short-document handling

Per §29.1 (item 4), grade-level formulas are suppressed when `words < 100` or `sentences < 5`. In the headline table this renders as `— ²` in the FKGL / Tateishi RS column, with a footnote below the table:

```markdown
> ² Below grade-scoring threshold (< 100 words or < 5 sentences).
```

Every other column keeps emitting because raw counts, link debt, and filler risk remain meaningful on short documents. If a file is *completely* empty of prose (e.g., a marker file with only front-matter), the whole row is rendered with `—` except for word count `0` and a footnote `³ Prose-empty after structural stripping.`

### 39.8 Large-PR handling

PRs that touch many Markdown files quickly overflow GitHub's comment render. Rules:

- **Headline table row cap: 10.** Sort by max severity class of any callout touching the file, descending. Files with only ⚪ cells rank lowest.
- Overflow rows go into `<details><summary>N more file(s)</summary>` immediately below the table, rendered with the same column format.
- If more than 25 files changed, append a one-line aggregate header above the table: `25 Markdown files changed (top 10 by severity shown).`
- **Callout cap: 8.** Overflow into `<details><summary>N more callouts</summary>`.
- Drill-down tables are **never** row-capped — they are already inside `<details>`. Reviewers who expand them have opted in to the full picture.

### 39.9 Reference mock (English-dominant PR)

This is the canonical shape for a PR that modifies one `README.md`, adds one architecture doc, regresses one API reference, leaves one generated file unchanged but on-alert, and touches the changelog. Every number below is mechanically derivable; every callout matches a template from §39.5.2.

```markdown
<!-- mehen-docs -->
## 📝 Documentation Metrics (this PR vs `main`)

| File | DMI | Words | FKGL | Link Debt | Filler Risk |
|---|---:|---:|---:|---:|---:|
| [README.md](https://github.com/wharflab/tally/blob/4709d1b/README.md) | 74 (main: 71) 🟢 | 1,240 (main: 1,180) ⚪ | 9.4 (main: 10.1) 🟢 | 0.08 (main: 0.12) 🟢 | 0.15 (main: 0.18) 🟢 |
| [docs/architecture/runtime.md](https://github.com/wharflab/tally/blob/4709d1b/docs/architecture/runtime.md) | 58 🆕 | 2,840 🆕 | 11.8 🆕 | 0.04 🆕 | 0.09 🆕 |
| [docs/api/auth.md](https://github.com/wharflab/tally/blob/4709d1b/docs/api/auth.md) | 62 (main: 68) 🔴 | 1,670 (main: 1,540) ⚪ | 12.1 (main: 11.6) 🔴 | 0.22 (main: 0.15) 🔴 | 0.14 (main: 0.12) ⚪ |
| [docs/generated/overview.md](https://github.com/wharflab/tally/blob/4709d1b/docs/generated/overview.md) | 76 (main: 76) ⚪ | 4,900 (main: 4,820) ⚪ | 10.2 (main: 10.2) ⚪ | 0.11 (main: 0.11) ⚪ | 0.79 (main: 0.78) ⚠️ |
| [CHANGELOG.md](https://github.com/wharflab/tally/blob/4709d1b/CHANGELOG.md) | 81 (main: 83) ⚪ | 2,110 (main: 2,090) ⚪ | 8.9 (main: 8.9) ⚪ | 0.06 (main: 0.06) ⚪ | 0.21 (main: 0.22) ⚪ |

**Callouts**

- 🔴 **docs/api/auth.md** — 2 unresolved relative link(s) added: `../../guide/sessions.md` (L47), `./tokens.md#refresh` (L112)
- 🔴 **docs/api/auth.md** — 3 sentence(s) exceed 35 words (new): L83, L104, L156
- 🔴 **docs/api/auth.md** — FKGL 11.6 → 12.1, above API-reference target 12.0 (§31.13)
- ⚠️ **docs/generated/overview.md** — filler/lazy risk 0.79 (HIGH); top contributors: large-unanchored-prose 0.82, lazy-sectioning 0.71, specificity-scarcity 0.64
- ⚠️ **docs/architecture/runtime.md** — mermaid diagram at L171 has no caption or nearby explanation
- ⚠️ **docs/architecture/runtime.md** — unlabelled code fence at L214
- 🟢 **README.md** — DMI 71 → 74, crossed "Good" → "Good" (§10.4); FKGL 10.1 → 9.4
- 🟢 **README.md** — 3 sentence(s) previously over 30 words now under

<details>
<summary>Full metric breakdown (structural · wording · lexical · readability)</summary>

### Structural / review

| File | RCI | MCC | MRPC | Evidence | Grounding |
|---|---:|---:|---:|---:|---:|
| README.md | 34 (main: 36) ⚪ | 14 (main: 17) 🟢 | 6 (main: 6) ⚪ | 0.68 (main: 0.62) 🟢 | 0.71 (main: 0.69) ⚪ |
| docs/architecture/runtime.md | 78 🆕 | 47 🆕 | 12 🆕 | 0.76 🆕 | 0.84 🆕 |
| docs/api/auth.md | 71 (main: 64) 🔴 | 38 (main: 31) 🔴 | 14 (main: 11) ⚪ | 0.58 (main: 0.63) 🔴 | 0.72 (main: 0.74) ⚪ |
| docs/generated/overview.md | 22 (main: 22) ⚪ | 12 (main: 12) ⚪ | 2 (main: 2) ⚪ | 0.24 (main: 0.25) ⚪ | 0.18 (main: 0.19) ⚪ |
| CHANGELOG.md | 18 (main: 19) ⚪ | 9 (main: 10) ⚪ | 3 (main: 3) ⚪ | 0.55 (main: 0.55) ⚪ | 0.81 (main: 0.80) ⚪ |

### English wording quality

| File | WQS | Passive % | Hedges /100w | Long sent. | Nominalizations |
|---|---:|---:|---:|---:|---:|
| README.md | 0.82 (main: 0.79) 🟢 | 11% (main: 14%) 🟢 | 1.8 (main: 2.1) ⚪ | 1 (main: 3) 🟢 | 6.2% ⚪ |
| docs/architecture/runtime.md | 0.74 🆕 | 22% 🆕 | 2.4 🆕 | 5 🆕 | 9.1% 🆕 |
| docs/api/auth.md | 0.68 (main: 0.75) 🔴 | 26% (main: 19%) 🔴 | 3.2 (main: 2.0) 🔴 | 4 (main: 1) 🔴 | 11.3% 🔴 |
| docs/generated/overview.md | 0.71 (main: 0.71) ⚪ | 18% ⚪ | 2.6 ⚪ | 2 ⚪ | 7.8% ⚪ |
| CHANGELOG.md | 0.86 (main: 0.86) ⚪ | 8% ⚪ | 0.9 ⚪ | 0 ⚪ | 5.4% ⚪ |

### English lexical & readability ensemble

| File | MATTR₅₀ | Hapax | Fog | SMOG | ARI | Coleman-Liau |
|---|---:|---:|---:|---:|---:|---:|
| README.md | 0.78 | 0.44 | 11.3 | 10.8 | 9.1 | 10.4 |
| docs/architecture/runtime.md | 0.81 | 0.52 | 14.2 | 12.7 | 11.9 | 12.8 |
| docs/api/auth.md | 0.74 | 0.48 | 14.7 | 13.1 | 12.4 | 13.2 |
| docs/generated/overview.md | 0.62 | 0.31 | 12.1 | 11.4 | 10.0 | 10.7 |
| CHANGELOG.md | 0.83 | 0.61 | 10.2 | 9.8 | 8.6 | 9.3 |

### Filler risk contributors (files with risk > 0.40)

- **docs/generated/overview.md (0.79)** — large-unanchored-prose 0.82, lazy-sectioning 0.71, specificity-scarcity 0.64

</details>

> Legend: 🟢 improvement · 🔴 regression · ⚠️ attention · 🆕 new file · ⚪ no material change

> Generated by [mehen](https://github.com/ophidiarium/mehen) — the code quality watcher.
```

Every callout in this mock matches a template in §39.5.2 exactly. No causal language appears. Every "likely helpful" or "after …" style phrase from earlier drafts has been removed.

### 39.10 What is deliberately *not* in scope

- **No causal explanations.** The report never says "after X", "because of Y", "due to Z". It only reports observed deltas and structural facts.
- **No author-intent inference.** The report never speculates about what the author "meant" or "should have done".
- **No LLM summaries.** Not now, not behind a flag, not as a plugin. The report is pure `f(metrics, thresholds, AST)`.
- **No trend lines or history.** The source-code bot renders a binary-size history via Mermaid; the docs section does not, because per-metric trendlines add signal-to-noise problems that deserve a separate design pass.
- **No suggested edits.** "Split this section" or "add a caption" suggestions are structural but prescriptive; they belong in a separate `mehen doc lint` command, not in a PR diff report.
- **No scoring gates by default.** The PR comment is advisory. A CI gate is available (`mehen diff --fail-on dmi-drop,new-broken-link`), but opt-in and independent of the comment.

### 39.11 Implementation checklist

For a first ship:

```text
[ ] Comment upsert anchored by <!-- mehen-docs -->
[ ] Headline table with 5 columns, fixed precision, delta indicator rules (§39.4)
[ ] Callout emitter driven by §39.5.2 template catalog only
[ ] Severity-sorted callout ranking with 8-callout cap (§39.5.1)
[ ] Short-doc footnote handling (§39.7)
[ ] 10-file headline cap with overflow <details> (§39.8)
[ ] Drill-down <details> with 3–4 tables; column suppression when all-⚪
[ ] Filler-risk contributors list sourced from §17.11 labels verbatim
[ ] Golden-output snapshot tests: byte-identical output for fixed inputs
[ ] Linter that forbids any string in the emitter module outside the template catalog
```

The last two items are the key correctness safeguards. A golden-output test (similar to how mehen already uses `insta` for source-metric snapshots) ensures reproducibility. An emitter linter (grep-based, checked in CI) ensures no one adds a `format!("…")` call that introduces free-form language.

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

### English readability formulas

16. Flesch, R. "A new readability yardstick." *Journal of Applied Psychology* 32(3):221–233, 1948. https://psycnet.apa.org/doi/10.1037/h0057532
17. Kincaid, J. P., Fishburne, R. P., Rogers, R. L., & Chissom, B. S. *Derivation of New Readability Formulas (Automated Readability Index, Fog Count and Flesch Reading Ease Formula) for Navy Enlisted Personnel.* Research Branch Report 8-75, Chief of Naval Technical Training, 1975. https://apps.dtic.mil/sti/tr/pdf/ADA006655.pdf
18. Gunning, R. *The Technique of Clear Writing.* McGraw-Hill, 1952.
19. McLaughlin, G. H. "SMOG Grading — a new readability formula." *Journal of Reading* 12(8):639–646, 1969. https://ogg.osu.edu/media/documents/health_lit/WRRSMOG_Readability_Formula_G._Harry_McLaughlin__1969_.pdf
20. Smith, E. A. & Senter, R. J. *Automated Readability Index.* AMRL-TR-66-22, Aerospace Medical Research Laboratories, 1967. https://apps.dtic.mil/sti/tr/pdf/AD0667273.pdf
21. Coleman, M. & Liau, T. L. "A computer readability formula designed for machine scoring." *Journal of Applied Psychology* 60(2):283–284, 1975. https://psycnet.apa.org/doi/10.1037/h0076540
22. Dale, E. & Chall, J. S. "A formula for predicting readability." *Educational Research Bulletin* 27:11–20, 37–54, 1948.
23. Chall, J. S. & Dale, E. *Readability Revisited: The New Dale-Chall Readability Formula.* Brookline Books, 1995.
24. Björnsson, C.-H. *Läsbarhet.* Stockholm: Liber, 1968.
25. Anderson, J. "Lix and Rix: Variations on a Little-known Readability Index." *Journal of Reading* 26(6):490–496, 1983. https://www.jstor.org/stable/40031755
26. Caylor, J. S., Sticht, T. G., Fox, L. C. & Ford, J. P. *Methodologies for Determining Reading Requirements of Military Occupational Specialties.* HumRRO Technical Report 73-5, 1973.
27. Fry, E. "A readability formula that saves time." *Journal of Reading* 11(7):513–516, 575–578, 1968.
28. Schriver, K. A. "Readability Formulas in the New Millennium: What's the Use?" *ACM SIGDOC*, 2000. https://www.karenschriverassociates.com/wp-content/uploads/2020/03/8-Schriver-Readability-formulas-whats-the-use.pdf
29. Klare, G. R. "A Second Look at the Validity of Readability Formulas." *Journal of Reading Behavior*, 1976. https://www.ideals.illinois.edu/items/15551/bitstreams/54962/data.pdf
30. Palmer, D. D. & Hearst, M. A. "Adaptive Multilingual Sentence Boundary Disambiguation." *Computational Linguistics* 23(2), 1997. https://people.ischool.berkeley.edu/~hearst/papers/cl-palmer.pdf
31. Browne, C., Culligan, B. & Phillips, J. *The New General Service List.* 2013. http://www.newgeneralservicelist.com/

### Lexical diversity and stylometry

32. Ure, J. "Lexical Density and Register Differentiation." In *Applications of Linguistics*, Cambridge University Press, 1971.
33. Halliday, M. A. K. *Spoken and Written Language.* Deakin University Press, 1985.
34. McCarthy, P. M. *An Assessment of the Range and Usefulness of Lexical Diversity Measures and the Potential of the Measure of Textual, Lexical Diversity (MTLD).* PhD dissertation, University of Memphis, 2005.
35. McCarthy, P. M. & Jarvis, S. "MTLD, vocd-D, and HD-D: A validation study of sophisticated approaches to lexical diversity assessment." *Behavior Research Methods* 42(2):381–392, 2010. https://pmc.ncbi.nlm.nih.gov/articles/PMC3813439/
36. Yule, G. U. *The Statistical Study of Literary Vocabulary.* Cambridge University Press, 1944.
37. Tanaka-Ishii, K. & Aihara, S. "Computational Constancy Measures of Texts — Yule's K and Rényi's Entropy." *Computational Linguistics* 41(3):481–502, 2015. https://direct.mit.edu/coli/article/41/3/481/1519/

### Japanese readability formulas

38. Tateisi, Y., Ono, Y. & Yamada, H. "A Computer Readability Formula of Japanese Texts for Machine Scoring." *COLING 1988* Vol. 2:649–654. https://aclanthology.org/C88-2135/
39. Sato, S., Matsuyoshi, S. & Kondoh, Y. "Automatic Assessment of Japanese Text Readability Based on a Textbook Corpus." *LREC 2008.* https://www.cs.brandeis.edu/~marc/misc/proceedings/lrec-2008/pdf/165_paper.pdf
40. Sato, S. et al. "Obi2: A System for Automatic Readability Assessment of Japanese Text." *LREC 2014.* http://www.lrec-conf.org/proceedings/lrec2014/pdf/633_Paper.pdf
41. Shibasaki, H. & Hara, H. *Constructing a Readability Scale of Japanese Texts and Developing a Software.* KAKENHI-PROJECT-19300277, 2010. https://kaken.nii.ac.jp/en/grant/KAKENHI-PROJECT-19300277/
42. Hasebe, Y. & Lee, J.-H. "Introducing a Readability Evaluation System for Japanese Language Education." *CASTEL/J 2015.* https://jreadability.net/file/hasebe-lee-2015-castelj.pdf
43. Lee, J.-H. & Hasebe, Y. "Readability Measurement for Japanese Text Based on Levelled Corpora." University of Tsukuba, 2020. http://jhlee.sakura.ne.jp/papers/lee-et-al2016rb.pdf
44. Mizuno, J. et al. "E-learning Japanese readability formula." *Journal of Natural Language Processing* 16(4), 2009. https://www.jstage.jst.go.jp/article/jnlp/16/4/16_4_4_3/_pdf

### Japanese language resources

45. NINJAL. *Balanced Corpus of Contemporary Written Japanese (BCCWJ) Frequency Lists.* https://clrd.ninjal.ac.jp/bccwj/en/freq-list.html
46. W3C. *Requirements for Japanese Text Layout (JLREQ).* https://www.w3.org/TR/jlreq/?lang=en
47. Japan Translation Federation. *JTF Japanese Style Guide 3.0.* 2019. https://www.jtf.jp/tips/styleguide — English translation https://www.jtf.jp/pdf/jtf_style_guide_e.pdf
48. Microsoft. *Japanese Localization Style Guide.* http://ftp.ntu.edu.tw/pub/cpatch/g/glossary/microsoft_styleguide_jpn.pdf
49. Ministry of Education of Japan. *Jōyō Kanji List.* 2010 revision (2,136 characters).
50. Tatsumi, H. *J-LEX Japanese Difficulty Tagger.* https://www17408ui.sakura.ne.jp/tatsum/J-LEX/
51. Premaratne, R. "Is the use of kanji increasing in the Japanese writing system?" *Electronic Journal of Contemporary Japanese Studies* 12(3), 2012. https://www.japanesestudies.org.uk/ejcjs/vol12/iss3/premaratne.html
52. Allen, D. "A Procedure for Determining Japanese Loanword Status." *Vocabulary Learning and Instruction* 9(1), 2021. https://vli-journal.org/wp/wp-content/uploads/2021/08/VLI_9_1_5_allen.pdf

### Prose-quality tooling and style guides

53. Ford, J. et al. *vale — a syntax-aware linter for prose.* https://vale.sh/docs/topics/styles
54. Ford, B. *write-good.* https://github.com/btford/write-good
55. Amperser. *proselint.* https://github.com/amperser/proselint
56. get-alex. *alex — catch insensitive, inconsiderate writing.* https://alexjs.com
57. retext authors. *retext plugin registry.* https://github.com/retextjs/retext/blob/master/doc/plugins.md
58. Hemingway Editor algorithm analysis. *Deconstructing the Hemingway App.* https://medium.com/free-code-camp/deconstructing-the-hemingway-app-8098e22d878d
59. textlint-ja. *textlint-rule-preset-ja-technical-writing.* https://github.com/textlint-ja/textlint-rule-preset-ja-technical-writing
60. Automattic. *harper — Rust grammar and prose checker.* https://github.com/Automattic/harper
61. Williams, J. *Style: Toward Clarity and Grace.* University of Chicago Press, 1990.
62. Pinker, S. *The Sense of Style: The Thinking Person's Guide to Writing in the 21st Century.* Viking, 2014.
63. Hyland, K. *Metadiscourse: Exploring Interaction in Writing.* Continuum, 2005.

### Rust ecosystem

64. lindera authors. *Lindera morphological analyzer.* https://github.com/lindera/lindera
65. daac-tools. *Vibrato tokenizer.* https://github.com/daac-tools/vibrato
66. pemistahl. *Lingua language detector (Rust port).* https://github.com/pemistahl/lingua-rs
67. Quickwit. *whichlang language detection library.* https://quickwit.io/blog/whichlang-language-detection-library
68. unicode-rs. *unicode-segmentation (UAX #29).* https://github.com/unicode-rs/unicode-segmentation
69. Unicode Consortium. *UAX #24 Unicode Script Property.* https://www.unicode.org/reports/tr24/
70. Unicode Consortium. *UAX #29 Unicode Text Segmentation.* https://www.unicode.org/reports/tr29/
71. CMU Pronouncing Dictionary. http://www.speech.cs.cmu.edu/cgi-bin/cmudict
72. syllarust authors. *syllarust — CMU-backed syllable counter.* https://lib.rs/crates/syllarust
