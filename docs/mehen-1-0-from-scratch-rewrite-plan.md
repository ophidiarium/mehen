# Mehen 1.0 from-scratch rewrite plan

**Status:** implementation proposal
**Date:** 2026-05-17
**Scope:** `mehen` 1.0 CLI, workspace architecture, metric ownership, parser strategy, GitHub Action integration

## 1. Goal

Rewrite `mehen` 1.0 as a CLI-first metrics tool with a small command surface and a language-owned analysis architecture.

The rewrite should preserve the product value that already exists:

- the same source-code metric set,
- the same Markdown metric behavior and definitions,
- deterministic output,
- fast changed-file analysis,
- GitHub pull request comments as the primary consumption path.

The rewrite should drop inherited library-first boundaries from the original `rust-code-analysis` shape. Internal Rust crates are allowed for ownership, compile-time isolation, and test hygiene, but the supported product API is the `mehen` binary and its documented output formats.

## 2. Product shape

### 2.1 Commands

`mehen` 1.0 exposes only commands that match the product.

```text
mehen metrics <path>
mehen diff
mehen top-offenders <paths>...
mehen --version
```

Removed from the public CLI:

- `--dump`
- `--find`
- `--count`
- `--function`
- root-level analysis flags such as `-m -p`

Parser and AST inspection is still useful for maintainers, but it belongs in `cargo xtask`, adapter fixtures, and debug snapshots. It should not be a supported end-user interface.

### 2.2 `mehen metrics`

Dedicated single-file metrics command.

```text
mehen metrics src/main.py
mehen metrics docs/adr.md --format json
mehen metrics app.tsx --language tsx --format markdown
```

Responsibilities:

- analyze exactly one file,
- auto-detect language from path and content,
- support explicit `--language`,
- emit a complete metrics report,
- return non-zero on read errors, unsupported language, parser fatal error, or invalid flags,
- never walk directories.

Suggested flags:

```text
--language <lang>
--format <json|markdown|yaml|toml>
--pretty
--profile <default|ci|strict>
```

There is no user-facing parser override. `--language` controls language identity when auto-detection is insufficient. Backend choice is internal and belongs to the owning language crate.

### 2.3 `mehen diff`

Primary CI and GitHub Action command.

```text
mehen diff --from origin/main --to HEAD
mehen diff --paths src docs --format github-markdown
mehen diff --format json --threshold cognitive=4 --threshold loc.lloc=120
```

Responsibilities:

- detect changed files from git refs,
- skip unsupported and generated files,
- analyze baseline and head versions,
- render stable per-file metric deltas,
- include the current Markdown documentation metrics section,
- support action-friendly Markdown and machine JSON output,
- support threshold failure policies.

Supported formats for 1.0:

```text
--format github-markdown
--format json
```

### 2.4 `mehen top-offenders`

Repository scan command for ranking files by one or more metrics.

```text
mehen top-offenders src --metric cognitive --metric loc.lloc --max-results 20
mehen top-offenders . --include 'crates/**' --exclude '**/tests/**' --format json
```

Responsibilities:

- walk files and directories,
- apply include/exclude filters,
- compute head-only metrics,
- rank by one or more metric selectors,
- produce deterministic sorted output.

Supported formats:

```text
--format markdown
--format json
```

### 2.5 Profiles

Profiles are built-in reporting and threshold presets, not external configuration files in 1.0.

| Profile | Purpose |
|---|---|
| `default` | Local developer output, no implicit failure policy. |
| `ci` | GitHub Action-friendly defaults, stable tables, standard metric selector set. |
| `strict` | Same output as `ci`, but enables stricter built-in threshold suggestions when explicit thresholds are not provided. |

Explicit CLI flags and action inputs override profile defaults. Repository-local configuration files can be designed after the 1.0 command surface is stable.

## 3. Core architectural decision

`mehen` should not have one central crate that "calculates all metrics for all languages." That would repeat the weakness of a universal AST model: it looks simple at the abstraction boundary, then loses the syntax-level nuance that makes metrics useful.

The correct split is:

- shared metric contracts, formulas, aggregators, output structs, and common helpers live in `mehen-metrics`,
- language crates own language-specific metric interpretation,
- `mehen-engine` orchestrates analysis and report assembly,
- `mehen-report` renders the results.

This means `mehen-python`, `mehen-typescript`, `mehen-php`, `mehen-ruby`, and tree-sitter-backed language crates are not just parser adapters. They are language analyzers. They may use better parsers, but their real job is to turn language-specific syntax into the same published metric family.

### 3.1 Why not a pure universal metric engine

Some metrics have universal math. Their interpretation is not universal.

Examples:

| Case | Why a universal AST loses signal |
|---|---|
| Python triple-nested f-string | May look like expression nesting, but human reading cost is closer to dense embedded code. |
| TypeScript method with many decorators | Usually not cyclomatic branching, but still a real review-complexity signal. |
| PowerShell pipelines and script blocks | Complexity often comes from command flow and implicit object passing, not only `if`/`for` shape. |
| Ruby blocks and modifier forms | Semantically important control flow can be compact and syntax-specific. |
| PHP attributes, promoted properties, magic methods | Class maintainability depends on PHP-specific member forms and conventions. |

The 1.0 design keeps the same metric names across languages, but lets each language define how its syntax contributes to those metrics.

### 3.2 Prior art

Other successful polyglot analysis systems draw a similar line.

- Semgrep has a generic AST for cross-language matching, but its docs also warn that generic pattern mode has lower and language-dependent quality because it does not understand the scanned language's syntax. Its implementation still requires language-specific mapping into the generic AST.
- CodeQL organizes support around language guides, language libraries, framework models, and query packs. Shared query concepts exist, but the useful analysis is deeply language-aware.
- SonarQube rules are tied to a language and to the analyzer or repository that contributes them. The product is polyglot, but rule ownership is language/analyzer scoped.

The lesson for `mehen`: share contracts and formulas, not all interpretation.

## 4. Workspace split

The 1.0 repository should be a Cargo workspace with internal crates.

```text
Cargo.toml
crates/
  mehen-cli/
  mehen-core/
  mehen-engine/
  mehen-metrics/
  mehen-markdown/
  mehen-python/
  mehen-typescript/
  mehen-php/
  mehen-ruby/
  mehen-rust/
  mehen-go/
  mehen-c/
  mehen-kotlin/
  mehen-powershell/
  mehen-tree-sitter/
  mehen-git/
  mehen-report/
  xtask/
tests/
  fixtures/
  snapshots/
action.yml
scripts/
  github-action.mjs
```

All internal crates should use:

```toml
publish = false
```

The crates are split for compile-time isolation and ownership clarity. They are not a stable SDK.

### 4.1 `mehen-cli`

The only binary crate.

Responsibilities:

- clap command definitions,
- environment initialization,
- dispatch to engine operations,
- stdout/stderr behavior,
- exit codes,
- no language-specific metric logic.

Target modules:

```text
src/main.rs
src/args.rs
src/commands/metrics.rs
src/commands/diff.rs
src/commands/top_offenders.rs
src/exit.rs
```

Exit code contract:

| Code | Meaning |
|---:|---|
| 0 | success |
| 1 | setup, IO, git, parser fatal, unsupported-language, or invalid-state error |
| 2 | threshold or policy failure |
| 3 | invalid machine-output serialization state |

### 4.2 `mehen-core`

Parser-neutral domain types and internal traits.

Responsibilities:

- language identifiers,
- source file model,
- line index,
- spans,
- diagnostics,
- analyzer backend identity,
- report envelope types,
- internal `LanguageAnalyzer` trait.

Core types:

```rust
pub enum Language {
    Python,
    TypeScript,
    Tsx,
    JavaScript,
    Jsx,
    Php,
    Ruby,
    Rust,
    Go,
    Kotlin,
    PowerShell,
    C,
    Markdown,
}

pub struct SourceFile {
    pub path: Utf8PathBuf,
    pub language: Language,
    pub text: String,
    pub line_index: LineIndex,
}

pub struct SourceSpan {
    pub start_byte: u32,
    pub end_byte: u32,
    pub start_line: u32,
    pub end_line: u32,
}

pub enum SpaceKind {
    Unit,
    Function,
    Closure,
    Class,
    Interface,
    Trait,
    Impl,
    Enum,
    Custom(SmolStr),
}

pub struct MetricSpace {
    pub id: SpaceId,
    pub kind: SpaceKind,
    pub name: Option<String>,
    pub span: SourceSpan,
    pub metrics: MetricSet,
    pub spaces: Vec<MetricSpace>,
}

pub struct LanguageAnalysis {
    pub language: Language,
    pub backend: AnalysisBackend,
    pub diagnostics: Vec<ParseDiagnostic>,
    pub root: MetricSpace,
    pub contributions: Vec<MetricContribution>,
}

pub trait LanguageAnalyzer {
    fn language(&self) -> Language;
    fn backend(&self) -> AnalysisBackend;
    fn analyze(&self, source: &SourceFile, config: &AnalysisConfig) -> Result<LanguageAnalysis>;
}

pub trait LanguageDispatcher: Send + Sync {
    fn analyze(&self, source: SourceFile, config: &AnalysisConfig) -> Result<LanguageAnalysis>;
}
```

Use `camino::Utf8PathBuf` internally for report paths and convert from `PathBuf` only at IO boundaries.

`LanguageAnalysis` is the owned result returned by every language analyzer. It is not a parser AST and it must not borrow from parser arenas, bump allocators, source buffers, tree cursors, or vendor AST nodes. Parser-specific data lives only for the duration of one `analyze` call. This avoids Oxc/Mago/Ruff/Prism lifetime leakage and makes `LanguageAnalysis: Send + 'static`.

`SpaceKind` is intentionally open through `Custom(SmolStr)`. Source-code analyzers should use the standard variants where they fit. Declarative analyzers can publish spaces such as `cloudformation.resource`, `terraform.module`, or `kubernetes.object` without changing the shared enum.

The `Language` enum is not feature-gated. A variant can exist even when its analyzer crate is disabled; in that case `mehen-engine` returns an unsupported-language diagnostic. This keeps `match` statements stable across feature combinations.

`--language` accepts stable lowercase identifiers and aliases:

| Canonical language | Accepted identifiers |
|---|---|
| Python | `python`, `py` |
| TypeScript | `typescript`, `ts`, `mts`, `cts` |
| JavaScript | `javascript`, `js`, `mjs`, `cjs` |
| TSX | `tsx` |
| JSX | `jsx` |
| PHP | `php` |
| Ruby | `ruby`, `rb` |
| Rust | `rust`, `rs` |
| Go | `go` |
| Kotlin | `kotlin`, `kt`, `kts` |
| PowerShell | `powershell`, `pwsh`, `ps1` |
| C | `c`, `h` |
| Markdown | `markdown`, `md`, `mdx` |

Detection policy:

- `.py` is Python in 1.0; `.pyi` support is not shipped until the Ruff migration explicitly adds stub-file fixtures and metric rules,
- Jupyter notebooks are not a 1.0 input format; notebook cell extraction can be considered later,
- `.ts`, `.mts`, and `.cts` are TypeScript,
- `.js`, `.mjs`, and `.cjs` are JavaScript,
- `.tsx` is TSX,
- `.jsx` is JSX,
- Markdown detection includes `.md` and `.mdx`, but MDX-specific syntax remains part of the existing Markdown metric compatibility work.

### 4.3 `mehen-metrics`

Shared metric contract and helper kit.

This crate must not become the central all-language calculator. It owns the common metric model and math, not language-specific interpretation.

Responsibilities:

- metric identifiers and selectors,
- metric output structs,
- metric accumulator structs,
- formula helpers,
- finalization helpers,
- aggregation helpers,
- generic LOC helpers where safe,
- threshold polarity and extraction helpers,
- test utilities for metric arithmetic.

Metric keys are an open namespace, not a closed enum. The minimum source-code metric family uses stable keys such as `cyclomatic`, `cognitive`, `loc.lloc`, `halstead.volume`, and `mi.visual_studio`. Language families may add namespaced keys such as `cloudformation.resource_count`, `cloudformation.iam_spcm`, or `terraform.dependency_depth`.

Examples of what belongs here:

- `CyclomaticStats`, `CognitiveStats`, `HalsteadStats`, `LocStats`, `MiStats`, `AbcStats`,
- maintainability index formulas,
- Halstead volume/difficulty/effort formulas,
- min/max/average finalization,
- `MetricTreeBuilder`,
- generic line indexing and line classification helpers,
- selector catalogue for `diff` and `top-offenders`.

Halstead protocol:

- language crates emit token-level operator and operand events into a shared `HalsteadBuilder`,
- language crates own classification, including rules such as "Python string literal is an operand unless it is a docstring",
- `mehen-metrics` owns `n1`, `n2`, `N1`, `N2` deduplication/totals and the volume/difficulty/effort formulas,
- token text normalization rules must be deterministic and tested per language.

Examples of what does not belong here:

- "Python `ExceptHandler` increments cognitive complexity",
- "TypeScript decorator stacks add human review burden",
- "PowerShell pipeline chains should affect ABC/cognitive",
- "Ruby rescue modifier counts as a decision",
- "PHP promoted properties count toward public attributes".

Those rules live in the owning language crate.

### 4.4 Language analyzer crates

Each language crate owns both parsing and metric interpretation for that language.

| Crate | Initial backend | Later backend | Responsibility |
|---|---|---|---|
| `mehen-python` | tree-sitter-python | Ruff parser + Ruff semantic | Python metrics and Python-specific complexity signals. |
| `mehen-typescript` | tree-sitter-typescript | Oxc | TypeScript, JavaScript, TSX, and JSX metrics. |
| `mehen-php` | tree-sitter-php | Mago syntax | PHP metrics. |
| `mehen-ruby` | tree-sitter-ruby | Prism | Ruby metrics. |
| `mehen-markdown` | current analyzer | Comrak only if parity succeeds | Markdown metrics, kept behavior-compatible first. |
| `mehen-rust` | tree-sitter-rust | tree-sitter-rust for 1.0 | Rust metrics. |
| `mehen-go` | tree-sitter-go | tree-sitter-go for 1.0 | Go metrics. |
| `mehen-c` | tree-sitter-c | tree-sitter-c for 1.0 | C metrics. |
| `mehen-kotlin` | tree-sitter-kotlin-sg | tree-sitter-kotlin-sg for 1.0 | Kotlin metrics. |
| `mehen-powershell` | tree-sitter-pwsh | tree-sitter-pwsh for 1.0 | PowerShell metrics. |

Each source-language crate should have a local layout like:

```text
src/
  lib.rs
  analyzer.rs
  parser/
  syntax.rs
  metrics/
    loc.rs
    cyclomatic.rs
    cognitive.rs
    halstead.rs
    abc.rs
    nargs.rs
    nom.rs
    exits.rs
    class_metrics.rs
  fixtures/
  snapshots/
```

The language crate may choose its internal style:

- direct metric accumulation while walking the parser AST,
- a local syntax-fact stream,
- a local normalized AST,
- shared helpers from `mehen-metrics`,
- tree-sitter generated kind enums.

The only required output is `LanguageAnalysis` with the standard metric report shape.

Each language crate owns parser lifetimes internally. Arena-backed parsers such as Oxc and Mago must emit owned metric data before the arena is dropped. `LanguageAnalysis` and every value inside it must be owned, deterministic, and safe to move across worker threads.

### 4.5 `mehen-tree-sitter`

Shared support for tree-sitter-backed language crates.

Responsibilities:

- small wrapper around `tree_sitter::Parser`,
- shared CST traversal helpers,
- source span helpers,
- tree-sitter kind generator support,
- generated kind enum utilities,
- no metric interpretation.

This crate is not the owner of Rust, Go, C, Kotlin, or PowerShell semantics. It only helps those language crates use tree-sitter cleanly.

### 4.6 `mehen-engine`

Pipeline orchestration.

Responsibilities:

- language detection,
- language analyzer registry,
- `mehen metrics` orchestration,
- `mehen diff` orchestration,
- `mehen top-offenders` orchestration,
- concurrency,
- generated-file filtering,
- threshold evaluation.

Internal workflow APIs:

```rust
pub fn analyze_metrics(input: AnalyzeMetricsInput) -> Result<MetricsReport>;
pub fn analyze_diff(input: DiffInput) -> Result<DiffReport>;
pub fn rank_top_offenders(input: TopOffendersInput) -> Result<TopOffendersReport>;
```

The registry depends on enabled language crates and selects an analyzer by `Language`. The CLI never selects parsers directly.

Concurrency contract:

- per-file analysis is the parallelism unit,
- baseline and head versions in `mehen diff` may be analyzed in parallel,
- analyzers are constructed per worker or per analysis call, not shared as parser instances,
- parser arenas and mutable parser state live for one `analyze` call,
- `LanguageAnalysis` is owned and `Send + 'static`,
- report rendering happens after analysis results are collected and sorted.

`mehen-engine` also implements `LanguageDispatcher` for recursive analysis requests. This is used by Markdown embedded-code metrics and future analyzers that need to analyze nested language fragments. The dispatcher is the only supported re-entrance point from one analyzer into another. It enforces recursion limits, source-size limits, and feature availability.

### 4.7 `mehen-markdown`

Markdown analysis remains special because it is not source-code function/class analysis.

Responsibilities:

- current Markdown metric implementation,
- current Markdown metric structs,
- current Markdown fixtures and snapshots,
- optional Comrak experiment after parity.

Initial rule:

> Move current Markdown analysis as-is before changing parser internals.

Markdown embedded-code analysis must not depend directly on every source-language crate. Instead, `mehen-markdown` accepts a `&dyn LanguageDispatcher` from `mehen-engine`.

```rust
pub fn analyze_markdown(
    source: &SourceFile,
    config: &MarkdownConfig,
    dispatcher: &dyn LanguageDispatcher,
) -> Result<LanguageAnalysis>;
```

The embedded-code path maps fence info strings to `Language`, constructs an embedded `SourceFile`, and calls the dispatcher. Tests can pass a mock dispatcher with canned `LanguageAnalysis` values. This keeps `mehen-markdown` dependent only on `mehen-core` and avoids a dependency fan-out from Markdown to every language crate.

### 4.8 `mehen-git`

Git and repository operations.

Responsibilities:

- open repo,
- resolve refs,
- list changed files,
- read blobs,
- detect generated files via attributes,
- normalize repo-relative paths.

Port current `gix` usage, but hide it behind smaller internal structs.

All report paths are repository-relative UTF-8 paths normalized with `/` separators, including on Windows. Filesystem paths can use platform separators internally, but serialized JSON, Markdown tables, snapshots, and sticky comments must never emit backslash-separated paths.

### 4.9 `mehen-report`

Rendering and serialization.

Responsibilities:

- JSON reports,
- GitHub Markdown diff comment,
- top-offenders Markdown,
- single-file metrics Markdown,
- stable table sorting,
- stable callout ordering.

The current Markdown documentation diff renderer moves here, while consuming `mehen-markdown` report data.

### 4.10 GitHub Action wrapper

The GitHub Action remains a Node.js composite-action wrapper around the binary for 1.0. Do not add a Rust `mehen-action` crate unless the Node wrapper becomes a maintenance burden after 1.0.

Responsibilities:

- parse action inputs,
- construct `mehen diff` command lines,
- parse diff JSON,
- evaluate threshold outputs,
- write GitHub output variables,
- own sticky comment update policy.

### 4.11 `xtask`

Developer-only commands.

Responsibilities:

- snapshot refresh helpers,
- backend parity reports,
- current-vs-new binary comparison,
- tree-sitter kind generation,
- AST dumps for adapter developers,
- local syntax-fact dumps,
- dependency/license audit helper,
- Ruff revision update helper,
- generated fixture audits.

## 5. Metric calculation ownership

### 5.1 Shared metric contract

Source-code languages publish this minimum metric family:

- cyclomatic complexity,
- cognitive complexity,
- maintainability index,
- Halstead metrics,
- ABC metrics,
- NArgs,
- NOM,
- NExit,
- LOC family,
- NPA,
- NPM,
- WMC.

Other language families can publish additional metrics under the same `MetricKey` namespace. For example, future CloudFormation and Terraform analyzers can add resource, block, graph, dependency, policy-depth, or intrinsic-function metrics without changing the source-code metric contract.

The shared contract includes:

- metric key names,
- output shapes,
- value types,
- polarity,
- min/max/average semantics,
- aggregation rules,
- threshold extraction.

### 5.2 Language-owned interpretation

Each language crate owns the rules that feed the shared contract.

Example ownership:

| Metric | Shared in `mehen-metrics` | Language-owned |
|---|---|---|
| LOC | line accounting helpers, blank/comment/code counters | what counts as comment, doc string, heredoc, template literal, preprocessor line, shell pipeline continuation |
| Cyclomatic | accumulator and min/max/average | which syntax constructs are decisions in that language |
| Cognitive | accumulator, nesting helpers, finalization | nesting rules, language idioms, readability penalties, shorthand forms |
| Halstead | formula math, token maps helper types | operator/operand classification and language-specific token categories |
| ABC | accumulator and magnitude formula | assignment/branch/condition recognition |
| NArgs | accumulator | parameter syntax, destructuring, splats, receivers, implicit params |
| NOM | accumulator | functions, methods, lambdas, blocks, anonymous functions |
| NExit | accumulator | return/throw/raise/break/continue/exit semantics |
| NPA/NPM/WMC | accumulators and aggregation | visibility, class-like forms, traits/interfaces/enums/modules, decorators/attributes |

### 5.3 Optional common facts

A small common fact model is still useful, but it should be optional and deliberately incomplete.

```rust
pub enum CommonFact {
    SpaceStart(SpaceStart),
    SpaceEnd(SpaceId),
    Comment(SourceSpan),
    Line(LineFact),
    Diagnostic(ParseDiagnostic),
}
```

Language crates can use common facts for boring plumbing, then add language-specific metric logic directly. The design should not require every language to squeeze all nuance through a universal event stream.

### 5.4 Metric evidence

Language analyzers should be able to attach evidence to metric contributions.

```rust
pub struct MetricContribution {
    pub metric: MetricKey,
    pub span: SourceSpan,
    pub amount: f64,
    pub reason: ContributionReason,
}
```

This is useful for future explainable output and for parity tests. It also lets the project represent language-specific complexity without pretending that every contribution is a generic `Decision`.

Example reasons:

- `python.f_string_nested_expression`
- `python.match_case`
- `typescript.decorator_stack`
- `typescript.jsx_expression_nesting`
- `powershell.pipeline_chain`
- `ruby.rescue_modifier`
- `php.promoted_property`
- `markdown.heading_skip`

The first 1.0 release does not need to expose every contribution in default output, but tests should be able to snapshot them.

## 6. Parser strategy

The rewrite has two parser phases:

1. Rebuild the project organization while keeping existing tree-sitter behavior for source languages.
2. Replace parsers one language at a time, using the richer parser to improve that language's metrics.

This sequence avoids mixing a workspace rewrite, CLI rewrite, metric ownership rewrite, and parser migration into one untestable step.

### 6.1 Parser matrix

| Language | Reorganization backend | Target backend | Notes |
|---|---|---|---|
| Python | tree-sitter-python | Ruff parser + Ruff semantic | Ruff is consumed as a pinned git dependency. |
| TypeScript / JavaScript | tree-sitter-typescript | Oxc | Oxc covers JS, TS, JSX, and TSX. |
| TSX / JSX | tree-sitter-typescript TSX | Oxc | JSX should become first-class. |
| PHP | tree-sitter-php | Mago syntax | PHP-native AST should improve classes, attributes, enums, properties. |
| Ruby | tree-sitter-ruby | Prism | Prism is Ruby's parser with Rust bindings. |
| Markdown | current analyzer | current analyzer, evaluate Comrak later | Preserve current Markdown behavior first. |
| Rust | tree-sitter-rust | tree-sitter-rust for 1.0 | Revisit rust-analyzer syntax later if needed. |
| Go | tree-sitter-go | tree-sitter-go for 1.0 | Good enough for 1.0. |
| C | tree-sitter-c | tree-sitter-c for 1.0 | Good enough for 1.0. |
| Kotlin | tree-sitter-kotlin-sg | tree-sitter-kotlin-sg for 1.0 | Keep current support. |
| PowerShell | tree-sitter-pwsh | tree-sitter-pwsh for 1.0 | Keep current support, but metric rules stay PowerShell-specific. |

### 6.2 Python and Ruff

Use Ruff crates from a pinned git revision:

```toml
ruff_python_parser = { git = "https://github.com/astral-sh/ruff", rev = "<sha>", package = "ruff_python_parser" }
ruff_python_semantic = { git = "https://github.com/astral-sh/ruff", rev = "<sha>", package = "ruff_python_semantic" }
ruff_python_ast = { git = "https://github.com/astral-sh/ruff", rev = "<sha>", package = "ruff_python_ast" }
ruff_text_size = { git = "https://github.com/astral-sh/ruff", rev = "<sha>", package = "ruff_text_size" }
```

Ruff crates are internal upstream crates with `publish = false`, so git dependency churn is accepted for 1.0.

`mehen-python` should use Ruff to improve Python-specific metrics, not merely to produce generic function/branch facts.

Python-specific metric opportunities:

- nested f-string and t-string readability penalties,
- `match`/`case` handling,
- exception groups,
- async comprehension handling,
- class and method visibility conventions,
- decorators as class/function review burden,
- stub file handling if `.pyi` support is added,
- semantic model use for future recursion and symbol-aware metrics.

### 6.3 TypeScript, JavaScript, TSX, JSX and Oxc

Use Oxc as the target backend for `mehen-typescript`.

TypeScript-specific metric opportunities:

- decorators and decorator stacks,
- class fields and private fields,
- parameter properties,
- interfaces and type-heavy APIs,
- JSX expression nesting,
- optional chaining and nullish coalescing readability,
- `satisfies`,
- `using`,
- dynamic import and async generators.

Some of these should not inflate cyclomatic complexity, but they may affect cognitive complexity or a language-specific contribution bucket that rolls into the same published metric family.

### 6.4 PHP and Mago

Use Mago syntax as the target backend for `mehen-php`.

Mago migration prerequisite:

- `mago-syntax` 1.27.1 declares `rust-version = 1.95.0`, which is newer than this workspace's pre-1.0 MSRV; if the pinned Mago version differs, use that version's declared MSRV,
- the Mago phase must first bump the workspace MSRV and all CI/release toolchains to the required version for the pinned Mago release,
- the GitHub Action's normal release-binary path means end users do not need that Rust toolchain unless they build `mehen` from source,
- the release matrix must prove the new MSRV before `mehen-php` flips from tree-sitter to Mago.

PHP-specific metric opportunities:

- attributes,
- promoted properties,
- enums,
- traits,
- anonymous classes,
- readonly members,
- property hooks if supported,
- magic methods,
- null-safe calls,
- first-class callables,
- `match` expressions.

### 6.5 Ruby and Prism

Use `ruby-prism` as the target backend for `mehen-ruby`.

Prism migration prerequisites:

- pin a `ruby-prism` revision only after license audit confirms the crate and vendored Prism sources are compatible with the project license,
- verify build requirements for every release target; the `ruby-prism` sys layer may require `cc`, `bindgen`, and libclang on platforms where prebuilt bindings are unavailable,
- Windows CI/release jobs must install the required native toolchain or skip source builds in favor of release binaries.

Ruby-specific metric opportunities:

- blocks,
- lambdas,
- numbered parameters,
- modifier conditionals,
- rescue modifiers,
- endless methods,
- pattern matching,
- safe navigation,
- class/module nesting,
- `attr_reader` / `attr_writer` / `attr_accessor` policy if class metrics expand.

### 6.6 Markdown and Comrak

The 1.0 rewrite should first port current Markdown analysis unchanged.

Comrak is a strong future parser candidate because it provides a CommonMark/GFM-compatible Rust AST, extension options, source positions, and `parse_document`. However, changing Markdown parsing can move many metrics at once. Do not bundle that risk with the workspace rewrite.

Comrak caveats:

- Comrak source positions are line/column based, so byte spans must be reconstructed through `LineIndex`,
- GFM extensions are individually toggled,
- alerts, math, and wikilinks are extension features rather than core GFM; the current analyzer's classifications must be checked one by one before any parser switch,
- Comrak evaluation must stay experimental until snapshots prove the same Markdown metric definitions still hold.

Plan:

1. move current Markdown implementation into `mehen-markdown`,
2. preserve existing Markdown fixtures and snapshots,
3. add a Comrak experiment behind a feature or `xtask`,
4. compare extracted structures and metric output,
5. flip only if parity changes are documented and accepted.

### 6.7 Tree-sitter backend and generator relationship

Tree-sitter remains a first-class backend where it is still the best tradeoff.

The current pre-1.0 new-language workflow is tree-sitter-first:

1. add a grammar dependency to `enums/Cargo.toml`,
2. register it in `enums/src/languages.rs`,
3. update `enums/src/macros.rs` for the grammar's `LANGUAGE` constant shape,
4. run `./recreate-grammars.sh`,
5. consume generated `src/languages/language_*.rs` enums from global `checker`, `getter`, and metric implementations.

That workflow should not remain the central architecture. It assumes every language is tree-sitter-backed and makes generated node IDs part of the shared metric engine.

New tree-sitter generator policy:

- move generator ownership from `enums/` to `mehen-tree-sitter` plus `xtask`,
- replace `./recreate-grammars.sh` with `cargo xtask tree-sitter generate <language>`,
- generate kind enums into the owning language crate, for example `crates/mehen-rust/src/generated/kinds.rs`,
- check generated kind files into git so release builds do not need grammar introspection at build time,
- CI must run `cargo xtask tree-sitter check-generated` to verify generated files are current,
- never generate global `src/languages/language_*.rs` files,
- never expose generated kind enums to `mehen-metrics`,
- keep generator config next to the owning language crate,
- preserve the "do not edit generated files directly" rule.

New tree-sitter-backed language workflow:

1. decide that tree-sitter is the right parser for the language,
2. create `crates/mehen-<language>/`,
3. add the grammar dependency to that crate and generator config,
4. run `cargo xtask tree-sitter generate <language>`,
5. implement language-local metric calculation using generated kinds and `mehen-metrics` helpers,
6. add fixtures and parity snapshots,
7. register the crate in `mehen-engine`.

### 6.8 Nom-based future parsers

`nom` should be treated as an adapter toolkit, not as a single parser backend.

Good uses:

- config-like languages,
- embedded fragments,
- frontmatter,
- small DSLs,
- fenced-code metadata,
- future syntax subsets where no full parser exists.

Bad uses:

- replacing mature language parsers for Python, TS, PHP, Ruby, or Markdown,
- writing a general parser in `mehen` for a language with active dedicated tooling.

Create shared nom helpers only when the second language needs them. Before that, keep nom code inside the first language crate that uses it.

### 6.9 Future declarative analyzers

The architecture must support declarative languages without forcing them into function/class source-code shapes.

Potential future analyzers:

| Analyzer crate | Likely parser | Metric shape |
|---|---|---|
| `mehen-cloudformation` | YAML/JSON parser with tag preservation, likely `saphyr-parser` or similar | resources, parameters, outputs, conditions, intrinsic-function nesting, IAM policy depth, dependency graph metrics. |
| `mehen-terraform` or `mehen-hcl` | `hcl-edit` / HCL parser with spans | blocks, resources, modules, expressions, dynamic blocks, references, dependency graph metrics. |

Design rules:

- declarative analyzers use `SpaceKind::Custom(...)` for scopes such as `cloudformation.resource` or `terraform.module`,
- declarative metrics use namespaced `MetricKey`s such as `cloudformation.resource_count` and `terraform.dependency_depth`,
- graph helpers remain local to the first declarative analyzer that needs them,
- extract a shared `mehen-graph-metrics` helper only after at least two analyzers need the same graph algorithms,
- declarative analyzers can still reuse shared accumulators for LOC, Halstead-like token metrics, or block-level complexity where appropriate,
- Markdown embedded-code and declarative fragment analysis use the same `LanguageDispatcher` re-entrance seam.

CloudFormation and Terraform support are real future candidates, but they should not be included in the 1.0 rewrite scope unless the source-code rewrite has already reached parity.

## 7. Rebuild order

The rewrite must first reorganize the project while keeping tree-sitter everywhere for source languages. Parser replacement comes after parity.

### Phase 0: freeze current behavior

Deliverables:

- snapshot current `mehen metrics` equivalent output from the existing CLI path,
- snapshot current `diff` Markdown and JSON output,
- snapshot current `top-offenders` output,
- snapshot current Markdown analysis,
- collect fixtures for every supported language.

This phase creates the reference set for the rewrite.

### Phase 1: workspace skeleton

Deliverables:

- root workspace `Cargo.toml`,
- internal crates with `publish = false`,
- `mehen-cli` command skeleton,
- CI runs `cargo check --workspace`,
- old implementation remains available for parity comparison.

No metric behavior changes yet.

### Phase 2: shared contracts and report schema

Deliverables:

- `mehen-core` source, span, diagnostics, language, and analyzer traits,
- `mehen-metrics` metric structs, selectors, formulas, aggregation helpers,
- report schema for metrics, diff, and top offenders,
- empty-report JSON snapshots.

This phase defines the stable output contract and shared math.

### Phase 3: tree-sitter baseline in per-language crates

Move current source-language behavior into language crates while keeping tree-sitter backends.

Deliverables:

- `mehen-python` using tree-sitter-python,
- `mehen-typescript` using tree-sitter-typescript,
- `mehen-php` using tree-sitter-php,
- `mehen-ruby` using tree-sitter-ruby,
- `mehen-rust`, `mehen-go`, `mehen-c`, `mehen-kotlin`, `mehen-powershell`,
- `mehen-tree-sitter` helper crate,
- metric formula/stat code extracted into `mehen-metrics`,
- language-specific match arms moved into owning crates,
- parity snapshots against the pre-rewrite implementation.

This is the core architecture migration. It intentionally does not improve parsers yet.

### Phase 4: Markdown and report port

Deliverables:

- current Markdown analyzer moved into `mehen-markdown`,
- current Markdown diff renderer moved into `mehen-report`,
- Markdown snapshots passing,
- `mehen metrics <markdown-file>` working.

### Phase 5: CLI and action parity

Deliverables:

- `mehen metrics`,
- `mehen diff`,
- `mehen top-offenders`,
- action wrapper updated,
- GitHub sticky comment output matches current behavior except for command names,
- threshold behavior preserved.

At this point the new project organization is usable with tree-sitter source-language behavior.

### Phase 6: Ruff Python migration

Deliverables:

- pinned Ruff git revision,
- Ruff parser and semantic integration,
- Python metric rules updated to use Ruff AST and semantic model,
- parity snapshots for old Python fixtures,
- improvement snapshots for Python 3.14, f-strings, t-strings where supported, match, exception groups, decorators, and async constructs,
- documented metric drift.

### Phase 7: Oxc TypeScript migration

Deliverables:

- Oxc integration in `mehen-typescript`,
- TS/JS/TSX/JSX parser migration,
- parity snapshots,
- improvement snapshots for decorators, class fields, private fields, parameter properties, JSX nesting, `satisfies`, `using`, dynamic import.

### Phase 8: Mago PHP migration

Deliverables:

- workspace MSRV bumped to the Rust version required by the pinned Mago release,
- CI/release toolchains updated before adding Mago as a required build dependency,
- Mago integration in `mehen-php`,
- parity snapshots,
- improvement snapshots for attributes, promoted properties, enums, traits, anonymous classes, readonly members, null-safe calls, match expressions.

### Phase 9: Prism Ruby migration

Deliverables:

- license audit for pinned `ruby-prism` and vendored Prism sources,
- CI/release native build prerequisites documented and installed where source builds require them,
- Prism integration in `mehen-ruby`,
- parity snapshots,
- improvement snapshots for blocks, lambdas, modifier forms, rescue modifiers, endless methods, pattern matching, safe navigation.

### Phase 10: Comrak evaluation

Deliverables:

- optional Comrak experiment,
- Markdown parity report,
- decision record: keep current Markdown parser path for 1.0 or migrate to Comrak.

### Phase 11: release hardening

Deliverables:

- full parity suite,
- repository-scale benchmark,
- binary size report,
- `cargo xtask audit-licenses`,
- action integration test,
- npm/PyPI packaging update,
- migration guide from pre-1.0 CLI flags,
- README and book documentation updates.

## 8. What to adapt from current code

### 8.1 Move mostly unchanged

| Current area | New home | Notes |
|---|---|---|
| `src/markdown/**` | `crates/mehen-markdown/src/**` | Preserve behavior first. |
| `src/markdown/tests/**` | `crates/mehen-markdown/tests/**` | Keep fixture names stable. |
| `src/diff_markdown.rs` | `crates/mehen-report/src/github_markdown/docs.rs` | Preserve template catalog behavior. |
| `src/git.rs` | `crates/mehen-git/src/lib.rs` | Keep `gix`; simplify API. |
| `src/ci.rs` | `crates/mehen-engine/src/ci.rs` | Keep GitHub Actions environment detection inside Rust engine code. |
| `action.yml` and `scripts/github-action.mjs` | root and `scripts/` | Update command names and output paths. |

### 8.2 Split carefully

Current `src/metrics/*.rs` files mix shared math with language interpretation. Split them, do not move them wholesale.

| Current content | New home |
|---|---|
| Stats structs, formulas, finalization, aggregation | `mehen-metrics` |
| Python metric match arms and helper rules | `mehen-python` |
| TypeScript / TSX metric match arms and helper rules | `mehen-typescript` |
| PHP metric rules | `mehen-php` |
| Ruby metric rules | `mehen-ruby` |
| Rust metric rules | `mehen-rust` |
| Go metric rules | `mehen-go` |
| C metric rules | `mehen-c` |
| Kotlin metric rules | `mehen-kotlin` |
| PowerShell metric rules | `mehen-powershell` |
| Metric selector catalogue | `mehen-metrics` or `mehen-core` |

`src/checker.rs` and `src/getter.rs` should not survive as global traits. Their logic should be folded into language analyzers.

### 8.3 Delete, not port

| Current area | Reason |
|---|---|
| `src/parser.rs` | Old generic tree-sitter parser wrapper. |
| `src/node.rs` | Leaks tree-sitter nodes into all metric code. |
| `src/traits.rs` callback/action model | Library-first boundary no longer needed. |
| `src/macros.rs` language generation macros | Tied to tree-sitter-only architecture. |
| global `src/languages/language_*.rs` ownership | Generated kinds become per-language crate internals. |
| `src/alterator.rs` | No longer meaningful. |
| `src/find.rs`, `src/count.rs`, `src/function.rs`, `src/output/dump.rs` | Public commands are dropped. |

## 9. Report schema

### 9.1 Metrics report

```json
{
  "schema_version": "1.0",
  "tool": "mehen",
  "path": "src/main.py",
  "language": "python",
  "analysis_backend": "python-ruff",
  "diagnostics": [],
  "metrics": {},
  "spaces": []
}
```

For Markdown, preserve the current Markdown metric structure under a Markdown-specific report kind.

For source code, preserve current source metric keys.

### 9.2 Diff report

```json
{
  "schema_version": "1.0",
  "base": "origin/main",
  "head": "HEAD",
  "files": [],
  "markdown_files": [],
  "analysis_errors": [],
  "threshold_violations": []
}
```

The GitHub Action consumes JSON for decisions and Markdown output for the comment body.

### 9.3 Diagnostics contract

Parser diagnostics are not automatically fatal. Every analyzer should produce the best partial report it can and attach diagnostics to the report.

Diagnostic classes:

| Class | Meaning | Exit behavior |
|---|---|---|
| `warning` | Recoverable parser or analyzer issue; metrics are usable but may be degraded. | Exit 0 unless thresholds fail. |
| `error` | Analysis for a file failed or is too incomplete to trust. | `mehen metrics` exits 1; `mehen diff` records `analysis_errors`. |
| `fatal` | IO, unsupported language, toolchain, or internal invariant failure. | Exit 1. |

`mehen diff` separates `analysis_errors` from `threshold_violations`. Exit code 2 remains reserved for threshold or policy failures. Exit code 1 covers setup, IO, unsupported-language, parser-fatal, or analysis-fatal failures.

Baseline and head diagnostics are reported independently. A head-side analysis error should be visible in the GitHub comment even when the baseline parsed cleanly.

## 10. GitHub Action architecture

The GitHub Action remains the primary consumer.

The action should:

1. install or locate the `mehen` binary,
2. run `mehen diff --format json`,
3. run or derive `mehen diff --format github-markdown`,
4. apply threshold policy,
5. create or update the sticky PR comment,
6. expose output paths and violation count.

The CLI owns report content. The action owns GitHub API interaction.

Stable anchors:

```text
<!-- mehen-source -->
<!-- mehen-docs -->
```

The action should also include a schema marker near the top of the rendered comment:

```text
<!-- mehen-schema:1 -->
```

The action updates an existing comment when both the anchor and compatible schema marker are present. If the schema marker is missing or incompatible, the action should append a fresh comment rather than corrupting an older format.

Preserve useful current inputs:

- `paths`,
- `include`,
- `exclude`,
- `exclude-tests`,
- `metrics`,
- `from`,
- `to`,
- `thresholds`,
- `fail-on-threshold`,
- `comment`,
- `github-token`,
- `comment-title`,
- `version`,
- `install-method`,
- `mehen-path`.

Add only if needed:

- `profile`,
- `format-version`.

Do not add parser-selection action inputs.

## 11. Feature flags

Initial feature plan:

```toml
[features]
default = ["markdown", "python", "typescript", "php", "ruby", "rust", "go", "c", "kotlin", "powershell"]
markdown = []
markdown-comrak = ["dep:comrak"]
python = [
  "dep:ruff_python_parser",
  "dep:ruff_python_semantic",
  "dep:ruff_python_ast",
  "dep:ruff_text_size",
]
typescript = [
  "dep:oxc_allocator",
  "dep:oxc_ast",
  "dep:oxc_parser",
  "dep:oxc_span",
]
typescript-semantic = ["typescript", "dep:oxc_semantic"]
php = ["dep:mago-syntax", "dep:mago-database", "dep:mago-php-version", "dep:bumpalo"]
ruby = ["dep:ruby-prism"]
tree-sitter-support = ["dep:tree-sitter"]
rust = ["tree-sitter-support", "dep:tree-sitter-rust"]
go = ["tree-sitter-support", "dep:tree-sitter-go"]
c = ["tree-sitter-support", "dep:tree-sitter-c"]
kotlin = ["tree-sitter-support", "dep:tree-sitter-kotlin"]
powershell = ["tree-sitter-support", "dep:tree-sitter-pwsh"]
nom-support = ["dep:nom"]
```

Default features can be narrowed if binary size or compile time becomes unacceptable, but the 1.0 product should aim to ship one binary that supports all advertised languages.

## 12. Testing strategy

### 12.1 Unit tests

- `LineIndex` byte-to-line conversion.
- Shared metric formulas.
- Shared metric aggregators.
- Metric selector parsing.
- Threshold evaluation.
- Report rendering.
- Git path normalization.

### 12.2 Language tests

Each language crate gets:

- parser diagnostic tests,
- metric contribution snapshots,
- span tests,
- function/class/member tests,
- language-specific syntax fixtures,
- parity snapshots against current behavior.

### 12.3 Parity snapshots

For every existing language and metric, compare:

- current pre-1.0 output,
- new tree-sitter-per-language-crate output,
- later parser-specific output.

Differences must be classified as:

- parity bug,
- intentional semantic improvement,
- parser limitation,
- metric-definition bug found during rewrite,
- unsupported syntax now supported.

### 12.3.1 Parity contract

Parity is measured on normalized JSON reports.

Rules:

- integer metrics must match exactly,
- string, path, language, backend, and diagnostic keys must match exactly unless the phase intentionally changes them,
- floating-point metrics use per-metric tolerance; default tolerance is `0.001`,
- Markdown and GitHub comment output must be byte-identical after stable timestamp/version redaction,
- all intentional drift must be recorded with fixture name, metric key, old value, new value, and reason.

Intentional drift records belong in the relevant language crate's test data and in release notes when the drift can affect user-facing CI output.

### 12.4 GitHub Action tests

Use a fixture repo and test:

- PR diff with source files only,
- PR diff with Markdown files only,
- mixed source and Markdown diff,
- generated-file skip,
- threshold violation,
- sticky comment update.

### 12.5 Benchmarks

Benchmark:

- single-file Python,
- large TSX project,
- PHP project,
- Ruby project,
- Markdown-heavy docs repo,
- mixed monorepo diff.

Track:

- wall time,
- peak memory,
- binary size,
- cold action runtime,
- warm action runtime.

## 13. Migration guide for users

Old usage:

```bash
mehen -m -p src
mehen --dump -p file.py
mehen --find function -p src
```

New usage:

```bash
mehen metrics src/main.py
mehen diff --paths src
mehen top-offenders src --metric cognitive
```

No replacement is provided for parser dump/find/count/function commands in the production CLI. Developer inspection moves to:

```bash
cargo xtask ast-dump --language python fixtures/example.py
cargo xtask metric-contributions fixtures/example.py
cargo xtask tree-sitter generate rust
```

## 14. Optional licensing reset to AGPL

This section is an implementation option, not legal advice. Before executing it, get counsel to review the final plan, contributor history, package metadata, third-party data, and dependency graph.

The 1.0 rewrite is a chance to stop carrying the inherited MPL-2.0 licensing shape from `rust-code-analysis` and release the new implementation under AGPL. The cleanest route is not "edit `LICENSE` and keep coding." It is a clean-room rewrite that treats the current MPL-covered source tree as reference behavior and design input, not as source material to copy.

### 14.1 Licensing target

Pick the exact SPDX identifier before the rewrite starts:

```text
AGPL-3.0-only
```

or:

```text
AGPL-3.0-or-later
```

`AGPL-3.0-or-later` is more future-compatible. `AGPL-3.0-only` is more conservative about future license-version changes. The repository, packages, release notes, and generated artifacts must use the same choice.

### 14.2 Why clean-room instead of direct relicensing

MPL-2.0 is compatible with GNU-family "Secondary Licenses" in specific larger-work scenarios, and the MPL text explicitly lists AGPLv3 as a Secondary License. However, MPL-covered source files still carry MPL conditions unless properly dual-distributed under the license's secondary-license mechanism. Mozilla's FAQ also emphasizes that new files containing no MPL code are not MPL "Modifications," while files containing MPL code remain in the MPL scope.

That means there are two different paths:

1. **Compatibility path:** combine or distribute some existing MPL-covered files with AGPL work and continue satisfying MPL obligations for those files.
2. **Clean AGPL-only path:** do not copy MPL-covered source expression, comments, tests, generated files, or snapshots into 1.0; reimplement behavior from requirements and observed outputs.

For the stated goal, choose the clean AGPL-only path.

### 14.3 What must not be carried forward

Do not copy these into the AGPL rewrite:

| Current artifact | Clean-room action |
|---|---|
| `src/**/*.rs` | Reimplement from the 1.0 spec and behavior snapshots. Do not copy code blocks, match arms, comments, helper names, or module structure wholesale. |
| `src/metrics/*.rs` | Split formulas from language rules; rewrite both. Formulas can be rederived from public metric definitions, but implementation expression must be new. |
| `src/checker.rs`, `src/getter.rs`, `src/parser.rs`, `src/node.rs`, `src/traits.rs`, `src/macros.rs` | Drop completely. Rebuild the new workspace concepts from scratch. |
| `src/languages/language_*.rs` | Do not carry generated files. Write the new generator and regenerate per-language kind enums. |
| `enums/**` | Treat as MPL-covered implementation; replace with `cargo xtask tree-sitter generate <language>` written fresh. |
| `src/markdown/**` | If the Markdown analyzer is kept behavior-compatible, reimplement it from the metric spec and fixtures rather than moving files. |
| `src/markdown/tests/snapshots/**` and `src/metrics/snapshots/**` | Do not copy as AGPL source. Use them only as reference outputs from the old binary, then generate new snapshots from new fixtures. |
| Existing test helper code | Rewrite. Test ideas and expected behavior can be preserved; code expression should not be copied. |
| Existing docs prose copied from current repository | Rewrite docs in the new AGPL tree unless clearly authored solely by the relicensing copyright holder and approved for AGPL. |
| Package metadata declaring MPL | Replace only after AGPL source tree exists. |

### 14.4 What may be preserved as behavior

The rewrite may preserve:

- metric names,
- metric formulas from public definitions,
- command behavior described in this spec,
- output schema intentionally defined for 1.0,
- externally observable old-binary outputs used as regression references,
- public language grammar node names as facts when regenerating tree-sitter kind enums,
- third-party dependencies used through their published licenses.

Do not copy protected implementation expression to preserve those behaviors.

### 14.5 Founder-owned post-baseline code

Use commit `98bf95eb943605009a122b811f00e7a4947183f4` (`Clean up scripts: remove Mozilla/unsupported language references`, 2026-02-14) as the provenance baseline for code added after the fork.

Code added after that baseline can be copied into the AGPL rewrite only if the project can establish that the AGPL licensor owns the relevant copyright.

Practical rule:

- If a file or module was authored solely by the relicensing copyright holder after the fork and does not contain copied MPL-covered implementation from `rust-code-analysis`, it can be relicensed and copied into the AGPL rewrite.
- If the code was added to an existing MPL-covered file, only the new contribution can be relicensed by its author. The inherited file body, surrounding implementation, and any modified MPL-covered code remain MPL-covered unless every relevant copyright holder grants relicensing permission.
- If another contributor touched the file or module, get explicit permission for AGPL relicensing or rewrite the affected portions.
- If generated files are involved, regenerate them with the new AGPL generator and verify the grammar/source licenses rather than copying old generated output.

Current likely AGPL-copy candidates, subject to contributor-history audit:

| Area | Likely treatment |
|---|---|
| `src/markdown/**` analyzer modules and Markdown tests added after the baseline | Can be copied or moved under AGPL if solely authored by the relicensing copyright holder and compatible with bundled data licenses. |
| `src/diff.rs`, `src/top_offenders.rs`, `src/ci.rs`, `src/git.rs`, `src/metric_selector.rs`, and GitHub Action implementation (`action.yml`, `scripts/github-action.mjs`, related tests/scripts) added after the baseline | Can be copied or moved under AGPL if solely authored by the relicensing copyright holder. |
| Go, Ruby, PowerShell, C, PHP, and Kotlin language support added after the baseline | Language-specific new files can be relicensed if solely authored by the relicensing copyright holder. Rules interleaved into inherited global files should be extracted by copying only owned expression or, safer, rewritten into the new per-language crates. |
| Post-baseline tests and fixtures for those commands/languages | Can be relicensed if solely authored by the relicensing copyright holder and not copied from third-party projects under incompatible terms. |
| Generated tree-sitter kind enums for those languages | Prefer regeneration in the AGPL tree; do not copy old generated files unless counsel confirms the generator output is safe and dependency licenses are satisfied. |
| Dependency metadata and bot-authored maintenance commits after the baseline | Audit separately. Dependency version bumps are not useful source expression to copy; generated lockfiles and package metadata should be recreated in the AGPL workspace. |

The old MPL release remains valid for recipients who already received that code under MPL. Relicensing owned post-baseline code affects the new AGPL release; it does not revoke prior MPL grants.

Do not treat "after `98bf95e`" as an automatic blanket rule. The audit still needs to separate:

- standalone post-baseline files authored by the relicensing copyright holder,
- owned hunks inside inherited MPL-covered files,
- generated artifacts,
- third-party fixtures/data,
- dependency and packaging metadata,
- commits authored by bots or external contributors.

### 14.6 Clean-room workflow

Use a two-role process when practical.

1. **Spec/reference role:** works in the current MPL repository, writes behavior specs, records CLI outputs, documents edge cases, and produces fixture inputs plus expected JSON/Markdown outputs.
2. **Implementation role:** works in the new AGPL workspace and does not read old implementation files while implementing modules.

If the same person must do both roles, keep a written implementation log:

- which old outputs were inspected,
- which public standards or parser docs were used,
- which files were implemented,
- confirmation that no source code was copied,
- commit references for each new module.

This is not as strong as separate teams, but it is still better than editing the old MPL code in place.

### 14.7 Repository and metadata changes

When the AGPL implementation is ready:

- replace `LICENSE` with the AGPLv3 text,
- update `Cargo.toml` workspace license,
- update crate/package manifests,
- update `pyproject.toml`,
- update npm package metadata and templates,
- update README and book licensing pages,
- keep or rewrite `LICENSE-THIRD-PARTY` for bundled data and dependencies,
- add `COPYRIGHT` or `NOTICE` if counsel wants explicit provenance,
- add SPDX headers to new source files if the project chooses file headers.

Package metadata should use the selected SPDX identifier, for example:

```toml
license = "AGPL-3.0-or-later"
```

or:

```toml
license = "AGPL-3.0-only"
```

### 14.8 Third-party dependency and data audit

AGPL licensing of `mehen` does not erase third-party license obligations.

Before release:

- audit all Rust dependencies for AGPL compatibility,
- audit npm and Python packaging wrappers,
- audit bundled Markdown data files,
- keep attribution and notice requirements from `LICENSE-THIRD-PARTY`,
- avoid bundling data with licenses that conflict with AGPL distribution,
- verify git dependencies such as Ruff crates can be redistributed in the chosen packaging model.

The current bundled data license notes must be rechecked rather than copied blindly.

### 14.9 Contributor audit

If any existing code is considered for direct reuse, do not assume it can be relicensed.

Required checks:

- list all contributors to the files in question,
- determine whether the project has a CLA or other relicensing grant,
- get explicit relicensing consent where needed,
- otherwise rewrite the file clean-room.

For a clean AGPL-only 1.0, the default answer should be "rewrite."

### 14.10 Release rule

Do not publish the 1.0 AGPL package from a tree containing MPL-covered implementation files unless the release intentionally follows the compatibility path and includes MPL compliance. For the AGPL-only path, the release branch should contain new AGPL source, regenerated tree-sitter artifacts, rewritten tests, rewritten docs, and fresh snapshots.

## 15. Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Shared metric crate grows into a hidden universal analyzer | Loses language nuance | Keep `mehen-metrics` limited to contracts, formulas, accumulators, helpers, and aggregation. |
| Language crates duplicate too much code | Maintenance cost | Extract helpers only after duplication appears in at least two languages. |
| Metric parity takes longer than expected | Delayed 1.0 | Reorganize with tree-sitter first; replace parsers only after parity. |
| Ruff git dependencies change APIs often | Build churn | Pin exact revision, isolate Ruff usage inside `mehen-python`, update through `xtask update-ruff`. |
| Binary grows too much | Slower action install/start | Measure early; narrow default features only if needed. |
| Markdown parser migration changes too much | Docs metrics regress | Port current analyzer unchanged first; evaluate Comrak separately. |
| Tree-sitter fallback hides parser bugs | Debugging becomes hard | Production fallback records diagnostics; developer `xtask` can force analyzer variants during parity work. |
| GitHub Action behavior diverges | Primary integration breaks | Build action tests before parser migrations. |
| AGPL relicensing accidentally carries MPL-covered expression | Licensing goal fails | Use clean-room workflow, regenerate artifacts, rewrite tests/docs, and get legal review before release. |

## 16. Ready-to-implement checklist

1. Freeze current output snapshots.
2. Create workspace skeleton.
3. Add `mehen-core` schema and analyzer trait.
4. Add `mehen-metrics` shared metric contract, formulas, and accumulators.
5. Create per-language crates with current tree-sitter behavior.
6. Move language-specific metric logic out of global metric files.
7. Move Markdown analyzer unchanged.
8. Wire `mehen metrics`.
9. Wire `mehen diff`.
10. Wire `mehen top-offenders`.
11. Update the GitHub Action wrapper.
12. Migrate tree-sitter generator into `xtask` and per-language generated files.
13. Add Ruff Python backend and Python-specific metric improvements.
14. Add Oxc TypeScript/JS/TSX/JSX backend and language-specific metric improvements.
15. Add Mago PHP backend and PHP-specific metric improvements.
16. Add Prism Ruby backend and Ruby-specific metric improvements.
17. Evaluate Comrak for Markdown.
18. Run parity snapshots and benchmarks.
19. Write migration guide.
20. If pursuing AGPL, complete clean-room and third-party license audit.
21. Cut 1.0 alpha.

## 17. Source references

- Semgrep generic pattern matching docs: <https://semgrep.dev/docs/writing-rules/generic-pattern-matching>
- Semgrep core contributing docs: <https://semgrep.dev/docs/contributing/semgrep-core-contributing>
- CodeQL documentation: <https://codeql.github.com/docs/>
- CodeQL query language guides: <https://codeql.github.com/docs/writing-codeql-queries/about-codeql-queries/>
- SonarQube rules overview: <https://docs.sonarsource.com/sonarqube/latest/user-guide/rules/overview/>
- Ruff contributing docs and crate map: <https://docs.astral.sh/ruff/contributing/>
- Ruff parser crate manifest: <https://github.com/astral-sh/ruff/blob/main/crates/ruff_python_parser/Cargo.toml>
- Ruff semantic crate manifest: <https://github.com/astral-sh/ruff/blob/main/crates/ruff_python_semantic/Cargo.toml>
- Oxc parser docs: <https://docs.rs/oxc_parser/latest/oxc_parser/>
- Mago syntax docs: <https://docs.rs/mago-syntax/latest/mago_syntax/>
- Mago syntax crate manifest: <https://github.com/carthage-software/mago/blob/main/crates/syntax/Cargo.toml>
- Ruby Prism Rust docs: <https://ruby.github.io/prism/rust/doc/ruby_prism/index.html>
- Comrak docs: <https://docs.rs/comrak/latest/comrak/>
- nom docs: <https://docs.rs/nom/latest/nom/>
- saphyr-parser docs: <https://docs.rs/saphyr-parser/>
- hcl-edit docs: <https://docs.rs/hcl-edit/>
- MPL 2.0 license text: <https://www.mozilla.org/en-US/MPL/2.0/>
- MPL 2.0 FAQ: <https://www.mozilla.org/en-US/MPL/2.0/FAQ/>
- GNU AGPLv3 license text: <https://www.gnu.org/licenses/agpl-3.0.en.html>
- Current tree-sitter new-language workflow: `mehen-book/src/developers/new-language.md`
