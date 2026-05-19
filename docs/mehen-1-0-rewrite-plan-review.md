# Review: Mehen 1.0 from-scratch rewrite plan

**Status:** review of `docs/mehen-1-0-from-scratch-rewrite-plan.md`
**Date:** 2026-05-17
**Reviewer focus:** feasibility, completeness, future extensibility (Markdown ↔ embedded-code, declarative languages such as CloudFormation YAML and Terraform HCL), and what an implementing engineer is left to guess.

The plan is broadly sound. Owning parsing and metric interpretation per language is the right shape, the parser-rollout sequencing is conservative, and `mehen-metrics` as a contract crate matches the prior-art lesson the plan correctly draws from Semgrep/CodeQL/SonarQube. The issues below are not "this won't work" — they are "an engineer will hit these on day one and will have to invent answers that should have been written down."

The review is grouped:

- §1 Strengths worth keeping verbatim.
- §2 Feasibility flags that need plan edits before phase 0.
- §3 Architectural gaps the plan does not yet name.
- §4 Extensibility stress tests (embedded-code metrics; CloudFormation YAML; Terraform HCL).
- §5 Ambiguities the implementer will have to ask about.
- §6 Concrete suggested edits to the plan.

---

## 1. Strengths worth keeping verbatim

These parts are load-bearing and should survive any rework:

- **§3 the "no central all-language calculator" rule.** Backing it with the prior-art table (Semgrep generic mode caveats, CodeQL language libraries, SonarQube per-language ownership) is exactly the right defense — that's the section that justifies the whole split.
- **§4.4 split between `mehen-metrics` (math, contracts, accumulators) and language crates (interpretation).** That split is what keeps the workspace from collapsing back into a universal AST.
- **§5.4 metric evidence (`MetricContribution { metric, span, amount, reason }`).** This is the right shape for explainable output and for parity testing. It also gives `mehen diff` a way to report *why* a metric moved, not just that it moved.
- **§6 two-phase parser strategy** (rebuild the project layout while keeping tree-sitter; replace parsers per language afterwards). The plan correctly refuses to mix workspace rewrite + CLI rewrite + metric-ownership rewrite + parser-migration rewrite into one untestable step. That is the single most important call in the document.
- **§4.5 carving generated tree-sitter kind enums into per-language crates** (instead of the current global `src/languages/language_*.rs`). The current generator-into-shared-enum pattern is the chief obstacle to language-owned metric interpretation; flipping it to "kinds are an internal detail of the owning language crate" is the architectural unlock.
- **§14 risk register.** It anticipates the four right risks (shared crate sprawl, parser-migration delay, Ruff churn, binary size). Those framings should be preserved.

## 2. Feasibility flags — need plan edits before phase 0

These are concrete, ground-checked corrections.

### 2.1 Mago's MSRV exceeds the workspace's

`mago-syntax` 1.27.1 declares `rust-version = 1.95.0`. The workspace today is at `rust-version = 1.93.1` (`Cargo.toml`:53). The moment `mehen-php` adds the dep, every consumer — workspace check, the GitHub Action's runner image, CI matrix, mehen-book's CI — has to bump to 1.95 first. The plan does not call this out. **Edit:** add an explicit MSRV-bump prerequisite to phase 8 (Mago migration) and confirm the action's `install-method = release-binary` path means end users do not need to rebuild.

### 2.2 ruby-prism build dependencies on CI runners

`ruby-prism` 1.9.0 vendors prism C source but its `-sys` crate uses `bindgen` (libclang) and `cc`. Ubuntu and macOS GitHub-hosted runners ship libclang; Windows runners do not unless the workflow installs it. Phase 9 should add that prerequisite explicitly. This affects only contributors building `mehen` from source on Windows and the Windows leg of the release matrix — end users who download a release binary are unaffected.

Also: at least one recent ruby-prism crates.io revision lists its license as "non-standard." Verify upstream MIT before pinning a revision. Add an `xtask audit-licenses` deliverable to phase 11.

### 2.3 Comrak `Sourcepos` is line/column, not byte offsets

§6.6 says "Comrak provides … source positions" and the plan's overall report-shape uses `start_byte` / `end_byte` everywhere. Comrak's `Sourcepos { start: LineColumn, end: LineColumn }` does not directly give byte spans — they have to be reconstructed via `LineIndex`. This is a small adapter, not a blocker, but the Markdown port plan should call it out so the engineer doesn't waste a day discovering it.

Bigger concern: Comrak's GFM extensions are individually toggled, and "alerts," math, and wikilinks are Comrak-proprietary, *not* part of GFM. The current `tree-sitter-markdown-text` grammar already classifies these. A blind switch will silently drop or reclassify nodes the analyzer relies on. Phase 10's "evaluate Comrak" is the right framing — keep it experimental, do not bundle with phase 4.

### 2.4 Oxc bumpalo lifetimes propagate into adapter code

Already noted in `language-specific-parser-backends.md` ("emit owned `MetricEvent`s during parse while arena/source are alive"). The 1.0 plan does not surface it. Without that constraint stated up front, a junior implementer will try to return references out of `mehen-typescript::analyze` and run into a borrow checker wall the morning of phase 7. **Edit:** in §4.4 ("Language analyzer crates"), add a one-line rule: "Adapter is responsible for owning the parser arena's lifetime; `LanguageAnalysis` must contain only owned data."

## 3. Architectural gaps

These are real holes in the plan, not phrasing nits.

### 3.1 `LanguageAnalyzer::analyze` returns a typed result — but the plan never specifies what it contains

§4.2 declares:

```rust
pub trait LanguageAnalyzer {
    fn analyze(&self, source: &SourceFile, config: &AnalysisConfig) -> Result<LanguageAnalysis>;
}
```

Nowhere is `LanguageAnalysis` defined. Is it `FuncSpace` (today's tree)? Is it a stream of `MetricContribution`s? Is it a `MetricsReport` ready for serialization? §5.4 hints at evidence (`MetricContribution`); §9.1 implies a finished report (`metrics: {}`, `spaces: []`). These are different shapes. The plan needs one section that picks one and states it:

- **Option A — analyzer returns a tree (`FuncSpace` shape preserved).** Lowest churn, but pushes language-specific math into language crates and forces re-implementing things like the cognitive nesting state machine in N places.
- **Option B — analyzer returns a stream of events (the design from `language-specific-parser-backends.md` §3 — "metric event model").** Lowest duplication but means the cognitive nesting state machine moves into `mehen-metrics` and language crates emit `Decision`/`NestingStart` events.
- **Option C — analyzer returns finalized `MetricContribution`s plus span tags; the engine bins them.** Furthest from current shape, most flexible.

The companion document already argues B; the 1.0 plan should pick one explicitly and reference it. My read: B is consistent with §5.4 and is the only option that makes Halstead implementable per-language *without* duplicating the volume/difficulty/effort math.

### 3.2 Halstead operator/operand classification is not split

Halstead today is the most coupled metric to per-language token enums (`getter.rs:46-71` for Python alone — pages of token kinds classified as `Operator | Operand | Unknown`). The plan says (§5.2) "operator/operand classification" lives in language crates, and (§5.1) "Halstead formula math" lives in `mehen-metrics`. That split is right.

What the plan does not specify: does the language crate hand `mehen-metrics` *tokens* (`Operator { kind: "+", text: Some("+")}`, `Operand { kind: "Identifier", text: Some("x") }`), or does it pre-classify into `n1/N1/n2/N2` integers? This matters because:

- Token-level events let `mehen-metrics` own deduplication of operands by text. Cross-language consistent rules.
- Pre-classified counts let language crates do tricks like "Python `String` is operand only when it's not a docstring" inside the language without leaking string-content checks into `mehen-metrics`.

`getter.rs:59-67` already does the "only if not a docstring" trick. The plan should state which side owns dedup (n1 distinct operators) — otherwise the implementer will pick differently for each metric. **Edit:** in §5.1 add: "Language crates emit per-token operator/operand events; `mehen-metrics` owns set-based n1/n2 dedup and N1/N2 totals."

### 3.3 The embedded-code metric is unaddressed

Today `src/markdown/embedded_code.rs:74-94` calls `crate::langs::get_function_spaces(&lang, ...)` directly. That function is defined by the `mk_action!` macro across all enabled languages — it's the cross-language entry point for source metric calculation. Markdown's `embedded_volume` consumes its result.

In the proposed workspace:

- `mehen-markdown` cannot depend on every language crate without re-introducing the universal-everything dependency the workspace split is trying to break (Rust-side compile-time dependencies become an N×N matrix).
- `mehen-engine` is the natural owner of the language registry. So the embedded-code call should route Markdown → `mehen-engine::analyze_metrics(...)` (or a subset of it), not Markdown → `mehen-<language>::analyze(...)`.

The plan does not name this. As written, an engineer reaching phase 4 (Markdown port) hits an architectural fork and has to invent the answer. **Edit:** add a §4.7 sub-section: "Markdown's embedded-code adjustment calls back into `mehen-engine` via a `recursive_analyze(SourceFile) -> Option<MetricsReport>` re-entrance hook. This is the only place where one language analyzer triggers another. The engine implements the hook, not `mehen-markdown`."

That single sentence answers ten implementation-day questions.

### 3.4 Concurrency model is not stated

§4.6 says "concurrency" is one of `mehen-engine`'s responsibilities, but never says which model. Today `src/concurrent_files.rs:1-280` uses `crossbeam-channel` to fan out per-file work to a thread pool. Important questions the plan does not answer:

- Is `LanguageAnalyzer` `Send + Sync`? Oxc's bumpalo allocator is `!Sync`; Mago's is similar. So either (a) one analyzer instance per worker thread, or (b) `LanguageAnalyzer` holds no parser state and creates one per call.
- Is the registry shared across threads? If yes, it must be `Sync`. If analyzers are `!Sync`, the registry has to return `Box<dyn LanguageAnalyzer>` per thread or use `thread_local!`.
- For diff: are baseline and head versions of one file analyzed in parallel? They are independent — should be — but it's unstated.

**Edit:** in §4.6 add: "Per-file analysis is the parallelism unit. Analyzers are constructed per worker. `LanguageAnalysis` is `Send + 'static`. Parser arenas (Oxc's `Allocator`, Mago's `Bump`) live for the duration of one analyze call."

### 3.5 No error model

The plan mentions "parser fatal error" as an exit code (§4.1) and `ParseDiagnostic` as a type (§5.3) but never says:

- Are diagnostics fatal or warnings? Today `src/parser.rs` cannot fail — `Tree::new` can't fail because tree-sitter accepts everything. With Ruff/Oxc/Mago, parse errors are first-class.
- Does `MetricsReport` always carry partial metrics on parse error? If yes, what's the contract — "we computed everything we could before the first hard error" or "we recover at every node and you get a degraded report"?
- Does `mehen diff` treat a baseline-side parse error differently from a head-side one? (It must — a regression introduced by a parse error in head should still surface.)

This matters because §3 of the diff report has `threshold_violations` but not `parse_errors`. If the action is supposed to comment "this PR introduces a file we can no longer parse," that's a separate threshold class.

**Edit:** add §9.3 "Diagnostics contract": diagnostics are non-fatal; partial reports are produced; `mehen diff` separates `threshold_violations` from `analysis_errors`; exit code 2 is reserved for thresholds, code 1 covers analysis errors.

### 3.6 Configuration / profiles are floated but never specified

§2.2 lists `--profile <default|ci|strict>`. §10 lists `profile` as a possible action input. Nothing in §5–§9 says what a profile *is*: which metrics? which thresholds? which polarity? Where does it live — embedded in the binary, a TOML file in the repo, an action input?

If the answer is "we'll figure it out in phase 5," that's fine — but the plan should say so. Otherwise the engineer building `mehen-engine` can't know whether profiles need a config-loading layer or a Rust enum.

**Edit:** either delete `--profile` until phase 5 designs it, or write one paragraph in §2.2 / §10 specifying what it loads.

### 3.7 Snapshot + parity policy is unstated

§7's phases say "parity snapshots" eight times but never define:

- What constitutes parity? Bit-identical JSON? Bit-identical Markdown? Or "no metric within a tolerance"? Floating-point Halstead (`difficulty = (n1/2) * (N2/n2)`) is not bit-stable across architectures by default.
- How are intentional drifts recorded? `xtask` deliverable? Changelog entry? Inline in `tests/snapshots/`?
- When tree-sitter is replaced (phase 6 Ruff, etc.), the new snapshots *will* differ. What's the merge process?

**Edit:** add §12.3.1 "Parity contract": JSON tolerance is exact for integer metrics; tolerance for floats is documented per metric (e.g., MI within 0.001). Drifts are documented in `mehen-book/src/release-notes.md`.

## 4. Extensibility stress tests

The reviewer was specifically asked to test the architecture against (a) extending Markdown with embedded-language analysis and (b) adding declarative languages (CloudFormation YAML, Terraform HCL). Below is the result of running those scenarios through the proposed workspace.

### 4.1 Embedded-language analysis in Markdown

**Today:** `src/markdown/embedded_code.rs:25-134` walks the Markdown CST, finds fenced code blocks, maps the info string to a `LANG`, and calls `langs::get_function_spaces` to compute Halstead/cognitive/LOC for the fence body. The result is rolled into `Halstead.embedded_volume` and `RCI.embedded_code_complexity`.

**Future:** the user wants to extend this further — surface per-language complexity contributions, count cognitive-complex examples in code blocks, possibly diff embedded-code complexity changes per fence.

**How the proposed plan handles this:**

- `mehen-markdown` cannot depend on `mehen-python`, `mehen-typescript`, `mehen-php`, etc. directly without re-creating the all-languages-everywhere coupling. (See §3.3 above.)
- The plan does not name the re-entrance hook. The implementer has to invent it — and the most natural place is `mehen-engine`. So `mehen-markdown::embedded_code` becomes a callback that takes `&dyn LanguageDispatcher` (or `fn(SourceFile) -> Option<MetricsReport>`) provided by `mehen-engine`.

**Verdict:** the architecture *can* support this, but the plan is silent on the seam. The fix is small (one trait, one section, see §3.3 above) but it must be written down. Without it, the engineer either:

- moves `embedded_code.rs` out of `mehen-markdown` into `mehen-engine`, which is wrong (it's Markdown-specific), or
- pulls every language crate as a dependency of `mehen-markdown`, which is also wrong.

**Recommended seam:**

```rust
// mehen-core
pub trait LanguageDispatcher: Send + Sync {
    fn analyze(&self, source: SourceFile, config: &AnalysisConfig)
        -> Result<LanguageAnalysis>;
}

// mehen-markdown::embedded_code
pub fn embedded_volume(
    root: &MarkdownNode,
    source: &str,
    dispatcher: &dyn LanguageDispatcher,
) -> EmbeddedVolume { ... }

// mehen-engine wires it
let dispatcher = EngineDispatcher::new(registry);
let md = mehen_markdown::analyze(source, &dispatcher);
```

This:

- Keeps Markdown's dependency surface to `mehen-core` only.
- Makes the re-entrance explicit and testable (tests can pass a mock `LanguageDispatcher` that returns canned analyses).
- Fits §3.1's choice of analysis output type — embedded code consumes the same `LanguageAnalysis` everything else does.

This pattern also generalizes (see §4.3 for the declarative case).

### 4.2 New declarative-language: CloudFormation YAML

The user wants to add CloudFormation analysis without premature abstraction. Working through it:

**What CFN metrics would look like** (from cfn-lint, cfn_nag's SPCM, rain tree):

- *Counts:* resources, parameters, outputs, conditions, mappings.
- *Per-resource complexity:* IAM policy depth (SPCM), `!If` / `!Sub` / `!FindInMap` nesting depth, `Fn::Transform` usage.
- *Graph metrics:* `Ref` / `GetAtt` / `DependsOn` edge graph; cycles; fan-in/fan-out; longest dep chain.
- *Size limits:* template byte size, total resource count vs CloudFormation's hard caps.

**Parser fit:** CFN templates ship as YAML (with `!Ref` short tags) or JSON. The honest Rust parser path is `saphyr-parser` (or `yaml-rust2`) low-level event stream; `marked-yaml` drops short tags, `serde_yml`/`serde_norway` drops spans. Petgraph covers the graph metrics. There is no Rust CFN-aware parser; every prior-art tool (cfn-lint, cfn_nag, rain) parses YAML/JSON and walks.

**Where it fits in the proposed workspace:**

- New crate `mehen-cloudformation` follows the §4.4 language-analyzer pattern. It owns `saphyr-parser` (or similar), the CFN-specific YAML tag handling, and the SPCM/graph metric interpretation.
- `Language` enum (§4.2) gains a `CloudFormation` variant.
- File detection: `*.template`, `*.template.yaml`, `*.cfn.yaml`, content-sniffing for `AWSTemplateFormatVersion`.

**Where the architecture works as-is:**

- Per-language ownership of parser + interpretation is the right shape. CFN's "complexity model" *is* different from Python's — the prior-art metric catalog (resource count, IAM SPCM, dependency graph) is genuinely declarative-specific.
- The `MetricContribution` evidence design (§5.4) generalizes cleanly: `cloudformation.iam_policy_depth`, `cloudformation.intrinsic_nesting`, `cloudformation.dependency_cycle`.

**Where the plan gets in the way:**

1. **`mehen-metrics`'s contract is "function/class-shaped" today.** §5.1 names cyclomatic, cognitive, Halstead, ABC, NArgs, NOM, NExit, LOC, NPA, NPM, WMC. None of those map cleanly to "resource count" or "IAM policy depth" or "dependency cycle." If `mehen-metrics` is a *fixed* shared contract, CFN can't publish its metrics through it. If `mehen-metrics` is *extensible* (new metric categories per language family), the plan should say so.

   **Recommendation:** in §5.1 add a paragraph: "The shared metric contract names a *minimum* metric set for source-code languages. Language analyzers may publish additional metric categories (for example, `cloudformation.resource_count`, `cloudformation.iam_spcm`) under the same `MetricKey` namespace. `mehen-metrics` owns the type system for keys, polarity, and aggregation; it does not own which keys exist."

2. **`SpaceKind` enum (§4.2 / `src/spaces.rs:31-47`) is hardcoded to function/class/trait/impl.** CFN doesn't have functions; it has resources. Two ways out:

   - Add `Resource` to `SpaceKind`. Quick, but starts a slippery slope (Terraform `module`? K8s `Deployment`? CFN `Resource`? CDK construct?).
   - Make `SpaceKind` open: `SpaceKind::Custom(&'static str)` or a string identifier. Best for extensibility.

   **Recommendation:** in §4.2 declare `SpaceKind` as `enum { Function, Class, Trait, Impl, Interface, Unit, Custom(SmolStr) }` so declarative languages can name their own scope kinds without adding to a closed enum.

3. **The graph-metrics dimension is missing from `mehen-metrics`.** Petgraph-based analysis of `Ref`/`DependsOn` is the chunk a CFN analyzer needs. None of the source-code metrics need it. This is an example of "not premature abstraction": the first language to need it (CFN) builds the helper inside `mehen-cloudformation`. *Only* when a second language (Terraform) needs the same thing does it move to a shared `mehen-graph-metrics` helper. The plan's §14 risk row "Language crates duplicate too much code" already adopts this rule — call it out positively in §3 as the design principle.

**Verdict for CFN:** the architecture supports it cleanly with two small plan edits (extensible `MetricKey` namespace, extensible `SpaceKind`). No rewrite needed.

### 4.3 New declarative-language: Terraform HCL

**Parser fit:** `hcl-edit` from the `hcl-rs` workspace, MIT/Apache, ships span-preserving AST. `hcl-rs::eval` does not support partial evaluation (cannot evaluate templates with un-bound `var.*` / `local.*`), so the analyzer walks the AST statically — same pattern HashiCorp's own Go tooling uses (`hcl.Expression.Variables()`).

**Metrics prior art:** TerraMetrics (Java, github.com/stilab-ets/terametrics) catalogs ~100 metrics: per-block McCabe CC, attribute count, function-call count, comparison/logical operator count, dynamic-block count, references, heredocs, plus Total/Avg/Max/Min aggregations. parse-hcl's dependency graph builds the cross-block graph: `resource.<type>.<name>` / `module.<n>` / `var.*` nodes, edges from interpolation refs.

**Where it fits in the proposed workspace:**

- New crate `mehen-terraform` (or `mehen-hcl`) following the §4.4 pattern.
- `hcl-edit` as the parser dependency.
- Same petgraph-based dependency graph machinery as CFN (here is where extracting `mehen-graph-metrics` becomes justified — second consumer).

**Notable shape collision:** Terraform metrics include a *per-block McCabe CC* (TerraMetrics) that *is* close to the source-code cyclomatic metric. So Terraform straddles the source-code / declarative line. The MetricKey namespace handles this: `terraform.block_cyclomatic` for the per-block number, plus `terraform.dependency_depth` for the graph-only number.

**Verdict for Terraform:** same as CFN — supported with the same two plan edits, no rewrite needed.

### 4.4 Generalizing what these stress tests show

The plan handles new *source-code* languages well. It handles new *declarative* languages only after two specific edits (extensible MetricKey, extensible SpaceKind) — but with those edits, the architecture extends without abstraction churn. The defining feature is: each language crate owns *its own metric catalog*, not just its own implementation of a fixed catalog. The plan today implies a fixed catalog (§5.1). Stating that the catalog is open-extensible (per the recommendation above) is the change that makes "add CloudFormation" a one-week task instead of a re-architecture.

The recommended seam from §4.1 (Markdown's embedded-code re-entrance) and the recommended extensions from §4.2 (open MetricKey, open SpaceKind) are the two design moves that make everything else possible. Both fit the existing plan; neither is large.

## 5. Ambiguities the implementer will have to ask about

These are unanswered questions that block coding. Some duplicate §3 above; this section is the punch list.

1. **What does `LanguageAnalyzer::analyze` return?** (§3.1)
2. **Does the language crate emit token-level Halstead events, or pre-classified counts?** (§3.2)
3. **How does Markdown's embedded-code analysis call back into other language crates?** (§3.3, §4.1)
4. **What's the concurrency model? Is the analyzer `Send + Sync`? Per-call or per-thread?** (§3.4)
5. **Is `LanguageAnalysis` `'static` or borrows from a parser arena?** (§2.4, §3.4)
6. **What's the parse-error contract — fatal, warning, partial report?** (§3.5)
7. **What does `--profile` do? What loads it?** (§3.6)
8. **What does "parity snapshot" mean numerically? Bit-identical or tolerance?** (§3.7)
9. **Are MetricKeys a closed enum or an open namespace?** (§4.2)
10. **Is `SpaceKind` a closed enum or open?** (§4.2)
11. **Does `--language` accept the same identifiers as the action input `metrics`?** (Plan §2.2 vs §10.) Today CLI accepts `python`, `typescript`, etc.; the new `Language` enum (§4.2) names them differently (`Tsx`, `Jsx` are split out, today's `tree-sitter-typescript` covers both with one grammar). Migration guide (§13) should map old → new identifiers.
12. **What is `xtask tree-sitter generate <language>`'s output location and check-in policy?** §6.7 says "into the owning language crate." Is the generated file checked in (current answer for `src/languages/language_*.rs` is yes) or generated at build time? §6.7 implies checked in but does not say the rule.
13. **Markdown's `LANG::Markdown` membership is conditional today** (`#[cfg(feature = "markdown")]`). Does the new `Language` enum have a feature gate too? If not, the variant exists in code that can't analyze it; if yes, every `match` arm is conditionally compiled.
14. **What's the policy for `.pyi` stub files, Jupyter notebooks, `.tsx` vs `.jsx` defaults?** §6.2 mentions "stub file handling if `.pyi` support is added" without committing. State whether 1.0 ships with these or not.
15. **Is `mehen-action` a Rust crate or a Node.js wrapper?** §4.10 says "if we want it in Rust" — the answer changes the build surface and packaging story.
16. **What's the repo-relative-path normalization rule on Windows?** §4.8 says `mehen-git` will normalize, but path-separator handling is the kind of thing that breaks deterministic snapshots. State that paths are forward-slash-normalized in all output.
17. **Stable-anchor format for sticky comments.** §10 lists `<!-- mehen-source -->` and `<!-- mehen-docs -->` — should these include a hash of the comment-body schema version so the action can decide between *update* and *append-new*? Today the action uses substring-match.

## 6. Concrete suggested edits to the plan

In priority order:

**P0 — Block phase 0 until resolved:**

1. **§4.2** define `LanguageAnalysis` (the trait return type). Pick option A/B/C from §3.1 above and write one paragraph.
2. **§5.1** declare `MetricKey` namespace as open-extensible per language; declare the §5.1 list as the *minimum* set source-code languages must publish.
3. **§4.2** make `SpaceKind` open (add `Custom(SmolStr)` or equivalent).
4. **§4.7 (new sub-section)** specify the Markdown-embedded-code re-entrance hook: `LanguageDispatcher` trait in `mehen-core`, implemented by `mehen-engine`, consumed by `mehen-markdown`.

**P1 — Block phase 7/8/9:**

5. **§6.4** add explicit MSRV bump (1.95) to phase 8 (Mago).
6. **§6.6** clarify Comrak source-position translation requirement (line/column → byte offsets via `LineIndex`).
7. **§4.6** state concurrency contract: per-file parallelism, analyzers constructed per worker, parser arenas live for one analyze call, results are owned (`'static`).
8. **§5.1** state Halstead operator/operand event protocol: language crates emit token-level events, `mehen-metrics` owns dedup/totals.

**P2 — Block phase 5 (CLI parity):**

9. **§9.3 (new)** define diagnostics contract: non-fatal, partial-report on error, separate `analysis_errors` from `threshold_violations` in diff JSON.
10. **§2.2** specify `--profile` or remove it.
11. **§12.3.1 (new)** define parity tolerance: integer-exact, float-tolerance documented per metric.

**P3 — Documentation polish:**

12. **§13 migration guide** add a `--language` identifier map (old → new) including any `Tsx`/`Jsx` splits.
13. **§4.10** decide whether `mehen-action` is Rust or Node, and stop hedging.
14. **§6.7** state the generated-file check-in policy explicitly.

---

## 7. Bottom line

The plan is implementable. The two-phase parser strategy is correct. The shared-contract / language-owned-interpretation split is correct. The risks the plan names are the right risks.

What it lacks is the level of precision that lets one engineer pick up phase 0 and not have to invent the `LanguageAnalysis` shape, the embedded-code re-entrance seam, the concurrency model, the error model, the parity tolerance, the MetricKey openness, and the SpaceKind openness on their own. Each of those decisions made independently by an implementer, in the wrong direction, would either close off the declarative-language extension path or force a partial rewrite at phase 7.

The fixes are small. None require rethinking the workspace shape. With the P0/P1 edits above, the plan goes from "good architectural sketch" to "directly buildable spec."
