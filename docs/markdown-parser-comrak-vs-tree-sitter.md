# Comrak vs `tree-sitter-markdown-text` — concrete impact on `mehen-markdown`

This document records the analysis of switching Mehen's Markdown analyzer
(`crates/mehen-markdown/`) from the current `tree-sitter-markdown-text` grammar
to the [Comrak](https://crates.io/crates/comrak) CommonMark/GFM parser. It is
based on the actual usage in the codebase as of the time of writing — every
claim is anchored to a file path and line range so future readers can verify
that the underlying code still matches the assumption.

## How Mehen actually uses the parser today

Across `crates/mehen-markdown/src/` Mehen leans on **~290 distinct grammar
kinds** (the `Markdown` enum in `grammar.rs`, with **170+** referenced in
code). The hot patterns are:

- **Byte-precise spans on every node** — `start_byte`/`end_byte`/`start_row`/
  `end_position` in every walker (LOC classification `loc.rs:184-198`, table
  cells `tables.rs:217-218`, prose extraction `prose/lang_detect.rs:226-280`,
  fence content `embedded_code.rs:209-225`). Mehen slices the original `&str`
  directly out of the source using node byte offsets — the prose extractor for
  instance excises skip ranges by byte and substitutes a `U+FFFC` sentinel.
- **Implicit section nesting** — `section1..section6` containers used in
  `sections.rs:135-146` and `mrpc.rs:201-240` to compute parent/child
  relationships, hierarchy edges, and word/block counts per section without
  rebuilding hierarchy.
- **Sub-token leaves inside paragraphs** — `WordToken`, `WordToken1..3`,
  `IdentifierLikeToken`, `PathLikeToken`, `NumericToken`, `Terminator`,
  `Separator`, `Bracket`, `OperatorLike`, `UnicodePunctuationRun`,
  `UnicodeSymbolRun` are read by `words.rs`, `halstead.rs`, `grounding.rs`,
  `mcc.rs`, `ecu.rs`. §4 `W`, §9 Halstead operators/operands, §15 grounding's
  identifier/path/version densities, and §6 ECU `math_tokens` all *depend* on
  these leaves.
- **Seven HTML block flavors + comments + CDATA + PIs + declarations** —
  `analyzer.rs:307-318`, `loc.rs:133-149`, `mcc.rs:115-130` distinguish
  `HtmlBlock1..HtmlBlock7`, `HtmlCommentBlock`, `MdxJsxBlock`. Halstead
  operator classes (`halstead.rs:257-272`) bucket `HtmlOpenTag`/`HtmlCloseTag`/
  `HtmlComment`/`HtmlCdata`/`HtmlDeclaration`/`HtmlProcessingInstruction`
  separately.
- **Reference-link definitions surviving in the AST** — `links.rs:136-141,
  220-234`, `mrpc.rs:174-194` walk `LinkReferenceDefinition` to build a label
  table for shortcut/collapsed forms and to verify same-MRPC behavior between
  inline and reference forms.
- **Callouts and directives** — `Callout`, `CalloutHeaderParagraph`,
  `CalloutMarkerOpen/Close`, `CalloutType`, `DirectiveBlock`,
  `DirectiveBlockDelimiter`, `DirectiveName` used in `mcc.rs:230-238`,
  `halstead.rs`, `ecu.rs`, `prose/lang_detect.rs`, `loc.rs`.
- **MDX JSX** — `MdxJsxBlock`, `MdxJsxInline`, `MdxJsxOpenTag/2`,
  `MdxJsxCloseTag/2`, `MdxJsxExpression` are first-class operators
  (`halstead.rs:271-272`).
- **Frontmatter as a tagged node** — `MinusMetadata` / `PlusMetadata` skipped
  from prose, classified as `OtherArtifact` (`loc.rs:148-149`,
  `prose/lang_detect.rs:137-138`).
- **Pipe table internals** — `PipeTableHeader`, `PipeTableRow`,
  `PipeTableCell`, `PipeTableDelimiterRow`, `PipeTableDelimiterCell`,
  `PipeTableAlignLeft`, `PipeTableAlignRight` (`tables.rs`,
  `halstead.rs:182-186`).
- **Math, both block and inline** — `MathBlock`, `MathBlockDelimiter`,
  `MathInline`, `MathInlineDelimiter/2`, `MathBlockContent`,
  `MathInlineContent`.
- **`field_name` accessors** — `child_by_field_name("level")` for ATX/Setext
  heading levels and `child_by_field_name("heading_content")`
  (`sections.rs:191-225`, `mcc.rs:533-543`).
- **Error-tolerant parsing** — Mehen never has to handle parser failure;
  tree-sitter always returns a tree (`analyzer.rs:78-79` uses `.expect()`),
  and `Error`/`ErrorSentinel` kinds exist if we ever wanted to localize parse
  breakdown.

## What Comrak provides

Based on `comrak` ≥ 0.52 surveyed via docs.rs:

- **Single flat `NodeValue` enum** (~48 variants): `Document`, `FrontMatter`,
  `BlockQuote`, `List`, `Item`, `DescriptionList/Item/Term/Details`,
  `CodeBlock(NodeCodeBlock)`, `HtmlBlock(NodeHtmlBlock)`, `Paragraph`,
  `Heading(NodeHeading)`, `ThematicBreak`, `FootnoteDefinition`,
  `Table(NodeTable)`, `TableRow(bool /* header */)`, `TableCell`,
  `TaskItem`, `MultilineBlockQuote`, `Alert(NodeAlert)`, `BlockDirective`,
  `Text(Cow<'static,str>)`, `SoftBreak`, `LineBreak`, `Code(NodeCode)`,
  `HtmlInline(String)`, `Raw(String)`, `Emph`, `Strong`, `Strikethrough`,
  `Highlight`, `Insert`, `Underline`, `Subscript`, `Superscript`,
  `SpoileredText`, `EscapedTag`, `Link(NodeLink)`, `Image(NodeLink)`,
  `FootnoteReference`, `ShortCode(NodeShortCode)`, `Math(NodeMath)`,
  `Escaped`, `WikiLink`, `Subtext`.
- **Extensions are runtime flags on `ExtensionOptions`**, not Cargo features:
  `strikethrough`, `tagfilter`, `table`, `autolink`, `tasklist`, `superscript`,
  `header_ids`, `footnotes`, `description_lists`, `front_matter_delimiter`,
  `multiline_block_quotes`, `math_dollars`, `math_code`, `wikilinks_*`,
  `underline`, `subscript`, `spoiler`, `greentext`, `alerts`,
  `cjk_friendly_emphasis`. (`ShortCode` and `phoenix_heex` are real Cargo
  features.)
- **Source positions**: each `Ast` carries `sourcepos: Sourcepos { start:
  LineColumn, end: LineColumn }` — **1-based line/column, not byte offsets**.
  Block-level spans are reliable; inline span fidelity is historically weaker
  than tree-sitter's, especially across container boundaries.
- **No inline tokenization**: paragraph text becomes one `Text(Cow<'static,
  str>)` per inline run. There is no `WordToken` / `IdentifierLikeToken` /
  `NumericToken` / punctuation-class equivalent.
- **No section containers**: headings are flat siblings, hierarchy must be
  reconstructed by walking children and grouping on `Heading.level`.
- **Tables**: `Table { alignments: Vec<TableAlignment>, num_columns,
  num_rows, num_nonempty_cells }` with `TableRow(is_header)` → `TableCell`
  children. `TableAlignment` = `None | Left | Center | Right`.
- **Links/refdefs**: `Link/Image { url, title }`. Inline, reference (full /
  collapsed / shortcut), and autolink forms collapse to one `Link`/`Image`
  shape — discriminator is **lost**. Reference definitions are stripped from
  the AST after resolution.
- **HTML blocks**: `NodeHtmlBlock { block_type: u8, literal: String }`. The
  CommonMark numeric type 1–7 is preserved; comment / CDATA / PI / declaration
  are not separately typed.
- **MDX/JSX**: not parsed. Falls through to `HtmlBlock` / `HtmlInline`.
- **Resilience**: `parse_document(arena, md, options) -> &'a AstNode<'a>` is
  **infallible**. Malformed input degrades to `Paragraph`/`Text`/`HtmlBlock`;
  no `Result`, no error nodes, no parse-error localization.
- **API**: arena-allocated via `typed_arena::Arena<AstNode<'a>>`. Walk via
  `node.children()`, `node.descendants()`, `node.traverse()` (yields
  `NodeEdge::Start/End`). Strings on payloads are owned `String`/`Cow<'static,
  str>` — not borrowed slices into the source.
- **Compliance**: passes the official cmark + GFM test suites; advertised
  100% CommonMark + GFM compliance.

## What Mehen would gain from Comrak

1. **Cleaner extension surface.** Native `Alert(NodeAlert { alert_type,
   title, multiline, … })` replaces our hand-rolled `Callout` walking.
   `extension.alerts` covers GFM `[!NOTE]`/`[!TIP]`/`[!IMPORTANT]`/
   `[!WARNING]`/`[!CAUTION]`. `BlockDirective` exists for `:::warning :::`.
   Wiki links, description lists, multiline blockquotes, task lists,
   autolinks, strikethrough, footnotes, math (`$..$`/`$$..$$`/```` ```math
   ````), front matter — all behind `ExtensionOptions` flags.
2. **CommonMark/GFM compliance.** Comrak passes the official cmark + GFM test
   suites. `tree-sitter-markdown-text` is a fork with non-standard tokenizer
   extensions (`WordToken1..3`); edge cases around CommonMark precedence
   rules are easier to reason about with Comrak.
3. **Smaller per-language burden.** No more `xtask tree-sitter generate
   markdown` codegen step, no 685-line `grammar.rs` to maintain, no
   `num-derive`/`num-traits` indirection, no enum-id remapping when grammar
   versions bump. The generation pipeline (`xtask/src/tree_sitter.rs`,
   `mehen-tree-sitter` crate) gets one fewer entry.
4. **Simpler walks.** Comrak's typed `NodeValue` enum with `node.children()`,
   `node.descendants()`, `node.traverse()` removes the `kind_id().into()`
   round-trip and the `legacy_node::Node`/`Cursor` wrapper
   (`legacy_node.rs:1-61` was always documented as transitional).
5. **Per-node typed payloads.** `NodeCodeBlock { fenced, info, literal,
   fence_char, fence_length, fence_offset }` replaces our manual
   `find_first(Markdown::InfoString) → find_first(Markdown::Language)` chain
   in `tree_helpers.rs:73-111` and `embedded_code.rs:149-227`. `NodeTable {
   alignments, num_columns, num_rows, num_nonempty_cells }` precomputes what
   `tables.rs:43-146` walks the AST to derive.
6. **Resolved link payloads.** `Link/Image { url, title }` removes the
   `LinkDestination` / `LinkDestinationParenthesis` / `LinkTitle` discovery
   chain in `tree_helpers.rs:150-180`, `links.rs:158-169`,
   `visuals.rs:113-123`, `mrpc.rs:884-917`.

## What Mehen would lose

1. **No byte spans — only `Sourcepos { start: LineColumn, end: LineColumn }`**
   (1-based line/col). Every `start_byte()`/`end_byte()` slice in the
   codebase has to be rewritten to map line/col → byte. Affected:
   `links.rs:498-512`, `tables.rs:217-218`, `code_burden.rs:132-153`,
   `embedded_code.rs:209-225`, `halstead.rs:343-410`, `math_burden.rs:73-99`,
   `prose/lang_detect.rs:226-280` (large — relies on byte-range excision and
   re-stitching with sentinels), `grounding.rs:328-336`,
   `tree_helpers.rs:235-239`. Most of these are 5–30-line site-local
   rewrites, but the prose extractor's skip-range merge logic is non-trivial.
   Block-level `Sourcepos` is reliable; **inline `Sourcepos` precision is a
   known weak spot** in Comrak — multi-paragraph spans across container
   boundaries can drift, which the tree-sitter byte offsets don't.
2. **No sub-token leaves.** §9 Halstead operands (`words.rs`, `halstead.rs`),
   §4 `W`, §6 ECU `math_tokens`, §15.2
   `identifier_density`/`version_fact_density`/`path_resolution_rate`, §16
   per-section path-resolution credit — all read `WordToken`,
   `IdentifierLikeToken`, `PathLikeToken`, `NumericToken` directly. Comrak
   gives a `Text(Cow<'static, str>)` blob per inline run. **Mehen would have
   to ship its own tokenizer** that replicates `tree-sitter-markdown-text`'s
   `_word_token1..3` rules and the path/identifier/numeric classification,
   plus the `Terminator`/`Separator`/`Bracket`/`OperatorLike` punctuation
   classes. That tokenizer would have to be Unicode-aware and stable across
   releases (snapshot tests bake the current grammar's behavior). This is
   the dominant migration cost — without it, several headline metrics change
   values.
3. **No section containers.** `sections.rs:135-180` and `mrpc.rs:201-240`
   use `section1..section6` as native parents; Comrak gives a flat block
   list with `Heading{level, setext}`. Reconstructable in O(N) by walking
   children and grouping on level, but `populate_word_and_block_counts`
   (`sections.rs:237-284`) and the MRPC hierarchy edge generator
   (`mrpc.rs:218-238`) need rewriting. Word/block counting becomes "scan
   blocks between heading at level L and next heading at level ≤ L" —
   straightforward but new code.
4. **Reference link definitions disappear after resolution.**
   `links.rs:136-141, 220-234` and `mrpc.rs:174-194` rely on walking
   `LinkReferenceDefinition` nodes to build a label→URL map *and* to surface
   them as artifacts (§19 reference definitions count toward `total` when
   they don't resolve). Comrak strips them after parsing. To recover them,
   Mehen would need to either pre-scan source via regex or fork Comrak's
   parse to preserve refdefs. **This is a real semantic loss**: the §11 link
   debt, §15 grounding, and §16 evidence-coverage scores all use refdef
   presence/absence.
5. **Lost link-form discrimination.** `is_bare_url` detection
   (`links.rs:171`), §11.2 information scent (`links.rs:642-651`), and the
   §7 "inline vs reference-style produce same MRPC" guarantee
   (`mrpc.rs:1161-1183`) all require knowing whether a `[text](url)` was
   inline, autolink, shortcut, collapsed, or full reference. Comrak collapses
   every form to `Link{ url, title }` plus an `Image` variant. Detection has
   to happen during a pre-AST source scan — feasible, but it duplicates the
   parser's job.
6. **HTML block flavor erased.** `analyzer.rs:307-318` distinguishes
   `HtmlBlock1..7` and `HtmlCommentBlock` for line classification and
   Halstead operator classes. Comrak gives `NodeHtmlBlock { block_type: u8,
   literal: String }` — `block_type` is the CommonMark numeric type (1–7),
   so the distinction *is* preserved, but `HtmlComment`/`HtmlCdata`/
   `HtmlDeclaration`/`HtmlProcessingInstruction` are not separately typed;
   they live inside the literal. §9.1 Halstead would collapse those operator
   classes (3 ops merge into one) — fixable but the snapshots will move.
7. **MDX JSX falls back to HTML.** `MdxJsxBlock`, `MdxJsxInline`,
   `MdxJsxOpenTag`, `MdxJsxCloseTag`, `MdxJsxExpression`
   (`halstead.rs:271-272`, `loc.rs:140-149`, `analyzer.rs:317`,
   `prose/lang_detect.rs:202`, `prose/mod.rs:202-203`) are recognized as a
   distinct artifact class. Comrak parses `<Component {...} />` as ordinary
   HTML, so MDX-specific operator buckets and the `mdx` entry in
   `blocks_stripped` (`prose/mod.rs:244-246`) would need a regex/tag-shape
   heuristic. Expression braces `{foo}` lose their typed classification
   entirely.
8. **No error tolerance.** Tree-sitter inserts `Error`/`ErrorSentinel` nodes
   and keeps going — Mehen would gain the option to report parse-trouble
   regions. Comrak is infallible: malformed markdown silently degrades to
   `Paragraph`/`HtmlBlock`/`Text`. We don't currently emit parse-error
   diagnostics, so this is mostly a future-capability loss, not a today-loss.
9. **Owned strings break zero-copy slicing.** Comrak stores
   `String`/`Cow<'static, str>` on payloads instead of borrowed `&'a str`;
   the lifetime story is `&'a AstNode<'a>` from a `typed_arena::Arena`, not
   borrows into the source buffer. Mehen's allocator pressure would rise
   (every `Text` allocates), and `node_text(n, source)` patterns
   (`tree_helpers.rs:235-239`) become "read `node.literal` directly" but at
   the cost of more heap traffic during walks.
10. **`heading_content`/`level` field accessors gone.** Replace
    `child_by_field_name("level")` with `NodeHeading.level`; replace
    `child_by_field_name("heading_content")` with iterating the heading's
    inline children. Mechanical but every heading-touching site changes.

## Snapshot impact

There are **32 snapshot fixtures** with locked expected outputs in
`crates/mehen-markdown/tests/snapshots/markdown__assert_fixture_snapshot@*.snap`.
Halstead operator counts, link-record `is_bare_url`, MDX-related counts,
refdef-related fields in `link_records`, and any test that distinguishes
HtmlBlock1..7 vs HtmlComment will move. Most of the §10 / §17 / §18
aggregate scores are downstream of those, so the entire snapshot set should
be expected to re-baseline as part of the migration.

## Migration shape (for context, not a recommendation)

If you decide to switch:

1. Build a Markdown source tokenizer (Unicode-aware) that reproduces
   `WordToken*`/`Identifier`/`Path`/`Numeric`/punctuation classification on a
   `&str`. Drive Halstead/words/grounding/ECU off it instead of off AST
   leaves.
2. Pre-scan source for `LinkReferenceDefinition` lines (regex on lines
   starting with `[label]:`) before invoking Comrak — preserves §11/§15/§16
   inputs.
3. Pre-scan inline link forms (`[x](y)` vs `[x][y]` vs `[x]` vs
   `<https://>`) for `is_bare_url`/scent/MRPC equivalence checks.
4. Detect MDX JSX with a lightweight regex (`<[A-Z][...]/>` etc.) before
   handing the source to Comrak; record those line ranges as MDX artifacts.
5. Map `Sourcepos { start, end }` to byte offsets via a precomputed
   line-start table built from the source once — drop into a `LineIndex`
   (`mehen-core::LineIndex` already exists).
6. Reconstruct sections by linear pass over the top-level block list grouped
   on `Heading.level`.
7. Re-baseline all 32 snapshots intentionally and audit each diff for
   behavior change vs cosmetic change.

## Verdict

The **win** is real but mostly ergonomic — cleaner extension surface
(alerts, footnotes, wikilinks, math, frontmatter built in), CommonMark/GFM
compliance, less codegen plumbing, fewer transitively-pinned crates.

The **loss** is concentrated in three places that matter for Mehen's
specific design: byte-precise spans (mostly recoverable via a `LineIndex`),
sub-token leaves inside prose (which forces Mehen to ship its own tokenizer
if Halstead/grounding/ECU values are to remain comparable), and the loss of
`LinkReferenceDefinition` and inline-link-form discrimination (which forces
a side-channel source scan).

For a CLI focused on metric *stability*, those three reproducibility costs
likely outweigh the ergonomic gains — switching is feasible but is
realistically a phase-scale change with mandatory snapshot re-baseline, not
a parser swap.
