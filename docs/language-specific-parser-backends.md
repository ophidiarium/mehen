# Language-specific parser backends for semantic metrics

**Status:** proposal  
**Date:** 2026-05-17  
**Scope:** `mehen` source-code metrics, parser architecture, and CLI behavior  

## Executive summary

`mehen` should relax the inherited `rust-code-analysis` restriction that every source language must be parsed through tree-sitter. Tree-sitter should remain a supported parser backend, especially for languages without a better Rust parser and for concrete syntax inspection, but metrics should not be coupled to tree-sitter node kinds.

The proposed design is:

1. Introduce a parser-backend boundary.
2. Move metric computation onto a stable `mehen` metric event model / light HIR.
3. Implement tree-sitter as one backend that emits the same metric events as today's code.
4. Add language-specific parser backends behind feature flags:
   - Python: `ruff_python_parser` / `ruff_python_ast`, if we accept a git or vendored dependency while Ruff's internal crate remains unpublished.
   - TypeScript / TSX / JavaScript / JSX: `oxc_parser` + `oxc_ast`, optionally `oxc_semantic`.
   - PHP: `mago-syntax`.
5. Keep the existing `FuncSpace` / `CodeMetrics` output model stable while comparing tree-sitter and semantic backends with snapshots before flipping defaults.

This keeps the CLI behavior centered on `mehen`, but lets high-value languages use ASTs designed for tooling rather than editor highlighting.

## Why change

Tree-sitter is excellent at fast, robust concrete syntax parsing. Its own documentation describes the output as a concrete syntax tree containing nodes for individual tokens such as commas and parentheses, and notes that some code analysis is easier with an abstract syntax tree where less important details are removed. That distinction matches the issue seen in Markdown work: a concrete syntax tree is often too low-level to express document or language semantics cleanly.

Today `mehen` metric logic often has to infer semantic facts from grammar-specific node IDs, parent checks, sibling checks, and generated enums. This is workable for basic constructs, but becomes fragile for:

- Python f-strings, t-strings, `match`, type parameters, exception groups, stub files, and version-specific syntax.
- TypeScript and TSX features such as decorators, class fields, parameter properties, `satisfies`, JSX trees, `using` declarations, and upcoming ECMAScript proposals.
- PHP language evolution such as enums, attributes, promoted properties, readonly constructs, property hooks, null-safe access, first-class callables, and pipe-style syntax.

The metrics engine should ask semantic questions directly:

- "Is this a function-like scope?"
- "How many parameters does it declare?"
- "Is this class member public?"
- "Does this construct introduce a decision path?"
- "Is this an operator or operand for Halstead?"
- "Is this call a method call, function call, static call, or constructor?"

Those are not tree-sitter questions. They are `mehen` metric-model questions.

## Current architecture

The current source-code path is tree-sitter all the way down:

- `Cargo.toml` depends on `tree-sitter` plus one grammar crate per language.
- `src/node.rs` wraps `tree_sitter::Tree` and `tree_sitter::Node`.
- `src/parser.rs` builds `Tree::new::<T>(&code)` from `T::get_lang().get_ts_language()`.
- `src/traits.rs` defines `ParserTrait::get_root() -> Node<'_>`.
- `src/spaces.rs` walks `Node` with a tree-sitter cursor and calls each metric trait per node.
- `src/checker.rs`, `src/getter.rs`, and `src/metrics/*.rs` match generated language enum values from `src/languages/language_*.rs`.
- `src/macros.rs` wires `LANG` variants directly to tree-sitter language factories.

That coupling means any non-tree-sitter parser must either fake a tree-sitter `Node` or force a broad rewrite. Faking a tree-sitter node is the wrong abstraction: it would throw away the value of typed ASTs and recreate the same grammar-kind problem under a compatibility layer.

## Upstream parser findings

### Tree-sitter

Tree-sitter remains useful but should be treated as a concrete syntax backend. Official docs emphasize concrete syntax trees, token-level nodes, fast incremental parsing, robust error handling, and syntax highlighting support.

Useful properties for `mehen`:

- Fast parsing.
- Good error recovery.
- Uniform API across many languages.
- Raw node tree useful for `--dump`, `--find`, and `--count`.
- Strong fallback for languages where no better Rust parser exists.

Limits for semantic metrics:

- CST node kinds change with grammar releases.
- Anonymous punctuation and named syntactic wrappers leak into metric rules.
- Typed language constructs often need parent/sibling inference.
- Visibility, type parameters, async-ness, string interpolation parts, and class member kinds are easier to model from a language AST.

### Ruff Python parser

Ruff's parser is a hand-written recursive-descent parser with Pratt expression parsing. The public API exposes helpers such as `parse_module`, `parse_expression`, and the general `parse(source, ParseOptions)` path. `Parsed<T>` carries:

- syntax,
- tokens,
- parse errors,
- unsupported-version syntax errors.

Ruff's Python AST model is generated from `ast.toml`, with typed statement and expression nodes such as `StmtFunctionDef`, `StmtClassDef`, `StmtIf`, `StmtTry`, `ExprBoolOp`, `ExprCall`, `ExprFString`, and `ExprTString`. The AST stores structured fields like function `name`, `parameters`, `body`, `decorator_list`, `type_params`, and `is_async`.

Important packaging note: the upstream Ruff workspace currently marks `crates/ruff_python_parser` as `publish = false` in its `Cargo.toml`. A `mehen` integration would need one of:

- a git dependency pinned to an exact revision,
- a vendored internal copy,
- a published wrapper/fork,
- waiting for Astral to publish a stable parser crate.

Ruff's current Python version model includes constants through `PY314` and `PY315`, with `latest()` returning `PY314` in the inspected source. It also exposes version checks such as PEP 701 support. That is exactly the kind of semantic versioning hook tree-sitter cannot provide on its own.

### Oxc parser

Oxc is a Rust JavaScript / TypeScript toolchain. Its parser supports JavaScript, TypeScript, JSX, TSX, modern ECMAScript, stage-3 decorators, arena allocation, comments, errors, and source spans. `Parser::parse()` returns a `ParserReturn` containing:

- `program`,
- module record,
- parse errors,
- comments / trivia,
- tokens when enabled,
- a `panicked` flag for unrecoverable parser aborts.

Oxc explicitly separates parser work from semantic analysis: scope binding, symbol resolution, and some syntax checks are delegated to `oxc_semantic`. For `mehen`, that separation is a benefit. We can start with parser AST mapping, then use semantic analysis where parent links, scope facts, or stricter syntax diagnostics matter.

Oxc is the lowest-risk first semantic backend from a packaging standpoint because `oxc_parser` is published on crates.io and designed for external consumption.

### Mago syntax

Mago's PHP docs describe a high-performance resilient lexer and parser that turns PHP source into a structured AST. The `mago-syntax` crate contains lexer, parser, AST definitions, utilities, and a walker. The parser API accepts a bump arena, file id, content, and parser settings, then returns a `Program<'arena>` containing:

- file id,
- source text,
- trivia,
- statements,
- parse errors.

The AST surface includes PHP-specific nodes such as `Function`, `Closure`, `ArrowFunction`, `Class`, `Interface`, `Trait`, `Enum`, `Method`, `Property`, `Attribute`, `Match`, `Try`, `Switch`, `NullSafeMethodCall`, and many more. That is a much better substrate for PHP metrics than recovering these facts from `tree-sitter-php`.

## Design goals

- Keep `mehen` CLI output stable for existing users.
- Preserve deterministic metrics across platforms.
- Let each language use the best parser backend available in Rust.
- Avoid exposing vendor ASTs to all metric implementations.
- Keep tree-sitter support as fallback and raw syntax inspection.
- Allow backend-by-backend rollout with snapshot parity.
- Avoid making Markdown documentation metrics depend on source-code parser changes.

## Non-goals

- Do not build a full compiler IR.
- Do not perform type checking.
- Do not require semantic backends for every language at once.
- Do not remove tree-sitter.
- Do not make `--dump`, `--find`, and `--count` identical across all parser backends in the first phase.

## Recommended architecture

### 1. Add a parser registry

Introduce a backend registry that is selected by `LANG`, command intent, feature flags, and optional CLI config.

```rust
pub(crate) enum ParserUse {
    Metrics,
    FunctionSpans,
    Dump,
    Find,
    Count,
}

pub(crate) enum ParserBackendKind {
    TreeSitter,
    RuffPython,
    Oxc,
    MagoPhp,
}

pub(crate) struct ParseConfig {
    pub(crate) language: LANG,
    pub(crate) backend: ParserBackendKind,
    pub(crate) use_case: ParserUse,
}
```

Default backend policy:

| Language | Initial default | Future default after parity |
|---|---|---|
| Python | tree-sitter | Ruff, if dependency policy is resolved |
| TypeScript / JavaScript | tree-sitter | Oxc |
| TSX / JSX | tree-sitter | Oxc |
| PHP | tree-sitter | Mago |
| Rust / Go / Ruby / Kotlin / PowerShell / C | tree-sitter | tree-sitter until a better parser is selected |
| Markdown | existing Markdown path | unchanged |

`auto` mode should allow fallback from a semantic backend to tree-sitter on unrecoverable parser failure. Explicit `--parser-backend semantic` should not silently fallback.

### 2. Split metric analysis from raw syntax inspection

There are two different capabilities in today's `ParserTrait`:

- metric traversal,
- raw syntax tree inspection.

They should become separate facets.

```rust
pub(crate) trait MetricsBackend {
    fn analyze_metrics(&self, source: SourceFile) -> MetricAnalysis;
}

pub(crate) trait SyntaxInspectionBackend {
    fn dump(&self, cfg: DumpCfg) -> io::Result<()>;
    fn find(&self, filters: &[String]) -> Vec<InspectableNode>;
    fn count(&self, filters: &[String]) -> (usize, usize);
}
```

Tree-sitter can implement both. Semantic backends should implement `MetricsBackend` first. `SyntaxInspectionBackend` can come later through a normalized inspectable AST, or the CLI can route `--dump`, `--find`, and `--count` to tree-sitter until backend-specific dumping is designed.

This avoids blocking metric improvements on a perfect replacement for tree-sitter's raw node UI.

### 3. Add a stable metric event model

The core change is to make metrics consume `mehen` events, not parser nodes.

```rust
pub(crate) struct SourceFile {
    pub(crate) path: PathBuf,
    pub(crate) language: LANG,
    pub(crate) text: String,
    pub(crate) line_index: LineIndex,
}

pub(crate) struct SourceSpan {
    pub(crate) start_byte: u32,
    pub(crate) end_byte: u32,
    pub(crate) start_line: u32,
    pub(crate) end_line: u32,
}

pub(crate) enum SpaceEventKind {
    Unit,
    Function,
    Closure,
    Class,
    Interface,
    Trait,
    Impl,
    Enum,
}

pub(crate) struct SpaceStart {
    pub(crate) id: MetricNodeId,
    pub(crate) parent: Option<MetricNodeId>,
    pub(crate) kind: SpaceEventKind,
    pub(crate) name: Option<String>,
    pub(crate) span: SourceSpan,
    pub(crate) visibility: Visibility,
    pub(crate) asyncness: Asyncness,
    pub(crate) parameter_count: Option<u32>,
}

pub(crate) enum MetricEvent {
    SpaceStart(SpaceStart),
    SpaceEnd(MetricNodeId),
    Decision { span: SourceSpan, kind: DecisionKind },
    NestingStart { id: MetricNodeId, kind: NestingKind },
    NestingEnd(MetricNodeId),
    BooleanOperator { span: SourceSpan, op: BooleanOp },
    Call { span: SourceSpan, kind: CallKind },
    Assignment { span: SourceSpan, kind: AssignmentKind },
    Exit { span: SourceSpan, kind: ExitKind },
    PublicMember { span: SourceSpan, kind: MemberKind, name: Option<String> },
    Operator { span: SourceSpan, kind: OperatorKind, text: Option<String> },
    Operand { span: SourceSpan, kind: OperandKind, text: Option<String> },
    Comment { span: SourceSpan },
    StringLiteral { span: SourceSpan, interpolation: InterpolationKind },
    SyntaxError { span: SourceSpan, message: String },
}
```

Metric code then becomes parser-neutral:

- LOC uses source lines plus comment/string/code spans.
- NOM counts `SpaceStart(Function | Closure)`.
- NArgs reads `parameter_count`.
- Exit counts `Exit`.
- Cyclomatic counts `Decision` and relevant boolean operators.
- Cognitive consumes `Decision`, `NestingStart`, `NestingEnd`, and boolean sequence events.
- Halstead consumes `Operator` and `Operand`.
- ABC consumes `Assignment`, `BooleanOperator`, and `Call`.
- WMC, NPA, and NPM consume class-like spaces and public member events.

This is intentionally lighter than a general AST. It captures what `mehen` metrics need and leaves language-specific details inside adapter modules.

### 4. Keep output structures stable

`FuncSpace`, `CodeMetrics`, and serialized metric keys should remain the compatibility boundary. Internally, replace `spaces::metrics<T: ParserTrait>` with:

```rust
pub(crate) fn metrics_from_events(
    source: &SourceFile,
    diagnostics: &[ParseDiagnostic],
    events: impl IntoIterator<Item = MetricEvent>,
) -> Option<FuncSpace>
```

During migration, keep the existing tree-sitter implementation and add an event-based path in parallel. Only switch a language when snapshots show parity or intentional improvements.

### 5. Preserve source order and determinism

All backends must emit events in source order. Any maps used for scopes, members, or diagnostics must be sorted before event emission. Span conversion must use one shared `LineIndex` so UTF-8, CRLF, and byte-vs-column differences do not leak into metrics.

## Backend adapter design

### Tree-sitter adapter

The tree-sitter adapter should initially preserve current behavior.

Implementation options:

1. Short term: keep existing `ParserTrait` path and only route non-tree-sitter languages through `MetricsBackend`.
2. Medium term: add a `TreeSitterMetricEmitter` that walks the current `Node` tree and emits `MetricEvent`s using existing `Checker`, `Getter`, and metric predicate knowledge.
3. Long term: delete metric-specific tree-sitter traversal once every metric has an event equivalent.

The medium-term emitter is important because it allows one metric engine for all backends while retaining snapshot parity.

### Python / Ruff adapter

Map Ruff AST constructs directly to metric events.

Spaces:

- `ModModule` -> `Unit`.
- `StmtFunctionDef` -> `Function`, using `name`, `parameters`, `body`, `is_async`, decorators, and `type_params`.
- `StmtClassDef` -> `Class`, using `name`, `arguments`, decorators, `type_params`, and body.
- `ExprLambda` -> `Closure`.

Control flow:

- `StmtIf` plus `elif_else_clauses` -> decisions and nesting.
- `StmtFor`, `StmtWhile` -> decisions, including `orelse` awareness.
- `StmtTry` -> try nesting, handler decisions, `is_star` for exception groups.
- `StmtMatch` / match cases -> decisions.
- Comprehension generators and `if` filters -> decisions.
- `ExprIf` -> ternary decision.
- `ExprBoolOp` -> boolean operators with sequence semantics.

Other metrics:

- `ExprCall` -> call.
- `StmtAssign`, `StmtAnnAssign`, `StmtAugAssign`, `ExprNamed` -> assignment.
- `StmtReturn`, `StmtRaise`, `StmtBreak`, `StmtContinue` -> exit events as applicable.
- `ExprFString` and `ExprTString` -> string literal events with interpolation detail.
- Ruff tokens -> Halstead punctuation and operators where AST-level operators are not enough.
- Class-body assignments and leading-underscore convention -> `PublicMember` for NPA.
- Methods in `StmtClassDef.body` plus leading-underscore convention -> `PublicMember(Method)` for NPM.

Special value:

- Version-aware parsing through `ParseOptions::with_target_version`.
- Python 3.14 syntax modeling through `PythonVersion::PY314`.
- PEP 701 f-string support and typed string interpolation nodes.
- Stub and notebook source-type support if `mehen` chooses to analyze `.pyi` or notebook-extracted code later.

Risk:

- Upstream crate packaging is not stable for external use. Treat this backend as experimental until dependency strategy is resolved.

### TypeScript / TSX / JavaScript / JSX / Oxc adapter

Use Oxc `SourceType` from extension and parse with tokens enabled when Halstead needs token-level operators.

Spaces:

- `Program` -> `Unit`.
- Function declarations, function expressions, arrow functions, generator functions, and methods -> `Function` or `Closure`.
- Class declarations / expressions -> `Class`.
- TypeScript interfaces -> `Interface`.
- Optional: type aliases and namespaces should not become function spaces unless metrics need a new `SpaceKind`.

Control flow:

- `if`, loops, `switch` cases, `catch`, ternary, logical operators -> decisions.
- `try` body and `catch` / `finally` blocks -> cognitive nesting.
- Optional chaining should not count as a decision unless the metric definition explicitly changes.

Other metrics:

- Call expressions, `new`, dynamic import, JSX call-like components if desired -> call events.
- Assignment expressions, update expressions, variable declarators with initializers -> assignment events.
- Return / throw / break / continue -> exit events.
- Class members and TS visibility modifiers -> `PublicMember`.
- Tokens and AST operator enums -> Halstead events.

Use `oxc_semantic` selectively:

- parent tree for difficult function-name inference,
- scope/symbol data for future recursion checks,
- stricter syntax checking when parser errors are not enough.

Special value:

- One backend covers `.ts`, `.mts`, `.cts`, `.js`, `.mjs`, `.cjs`, `.tsx`, and `.jsx`.
- JSX / TSX can be analyzed as first-class AST, not as TypeScript with grammar heuristics.
- Published crates and active toolchain ecosystem reduce packaging risk.

### PHP / Mago adapter

Use `mago-syntax` parser and AST walker.

Spaces:

- `Program` -> `Unit`.
- `Function`, `Closure`, `ArrowFunction`, `Method` -> function-like spaces.
- `Class`, `Interface`, `Trait`, `Enum`, `AnonymousClass` -> class-like spaces.

Control flow:

- `If`, `SwitchCase`, `SwitchDefaultCase`, `MatchArm`, `For`, `Foreach`, `While`, `DoWhile`, `TryCatchClause`, `Conditional`, boolean binary operators -> decisions.
- `Try`, loops, conditionals, `match`, and switch bodies -> cognitive nesting.

Other metrics:

- `FunctionCall`, `MethodCall`, `StaticMethodCall`, `NullSafeMethodCall`, `Instantiation` -> call events.
- `Assignment`, `UnaryPostfix`, relevant `Construct`s -> assignment / operator events.
- `Return`, `Throw`, `Break`, `Continue`, `ExitConstruct`, `DieConstruct` -> exit events.
- Properties, methods, constants, enum cases, modifiers -> public member events.
- Trivia and tokens -> LOC comments and Halstead punctuation.

Special value:

- PHP syntax is represented with PHP-native nodes, including class-like constructs and modern member forms.
- Mago's walker should make adapter code less error-prone than hand-walking enum trees.

## CLI and configuration

Add parser selection only when at least one semantic backend exists.

Suggested flags:

```text
--parser-backend auto
--parser-backend tree-sitter
--parser-backend semantic
--parser-backend python=ruff,typescript=oxc,tsx=oxc,php=mago
```

Suggested policy:

- `auto`: use the configured default per language; fallback to tree-sitter only on unrecoverable semantic parser failure.
- `tree-sitter`: force current behavior.
- `semantic`: require the semantic backend for languages where one exists; fail or skip when unavailable.
- Per-language mapping: useful for snapshot and rollout work.

Do not change default output format or metric key names as part of parser selection.

## Feature flags and dependencies

Proposed Cargo features:

```toml
[features]
default = ["markdown"]
semantic-parsers = ["parser-oxc", "parser-mago-php"]
parser-oxc = ["dep:oxc_allocator", "dep:oxc_ast", "dep:oxc_parser", "dep:oxc_span"]
parser-oxc-semantic = ["parser-oxc", "dep:oxc_semantic"]
parser-mago-php = ["dep:mago-syntax", "dep:mago-database", "dep:bumpalo"]
parser-ruff-python = ["dep:ruff_python_parser", "dep:ruff_python_ast", "dep:ruff_text_size"]
```

Do not include `parser-ruff-python` in `semantic-parsers` until the dependency source is settled.

Dependency rules:

- Pin exact versions, like existing tree-sitter dependencies.
- Keep all semantic parser dependencies optional at first.
- Run `cargo clippy --all-targets --all-features --locked` before enabling a backend in CI.
- Track compile-time and binary-size impact before flipping defaults.

## Migration plan

### Phase 0: decision record

- Land this document.
- Decide whether the first prototype is Oxc or Ruff.
- Decide whether semantic parser integration can use git dependencies.

Recommended first prototype: Oxc, because it is published and covers TypeScript, TSX, JavaScript, and JSX in one backend. If Python 3.14 support is the immediate product driver, run a Ruff spike in parallel but keep it experimental.

### Phase 1: backend boundary without behavior changes

- Add `SourceFile`, `LineIndex`, `ParseDiagnostic`, and backend registry types.
- Keep tree-sitter as the only implementation.
- Route existing `metrics`, `function`, `dump`, `find`, and `count` through the registry without changing outputs.
- Add tests proving default behavior is unchanged.

### Phase 2: event-based metric engine

- Implement `MetricEvent` and `metrics_from_events`.
- Add a tree-sitter event emitter.
- Snapshot existing fixtures through both old and event engines.
- Fix parity gaps or record intentional changes.

### Phase 3: first semantic backend

Implement one backend end-to-end behind a feature flag.

For Oxc:

- Parse `.ts`, `.tsx`, `.js`, `.jsx` fixtures.
- Emit events for all existing code metrics.
- Compare against current tree-sitter metrics.
- Add fixtures for decorators, class fields, interfaces, JSX, optional chaining, `satisfies`, and `using`.

For Ruff:

- Resolve dependency source.
- Add fixtures for Python 3.12 f-strings / PEP 701, 3.14 target-version checks, t-strings, `match`, exception groups, type params, `pyi`, async functions, nested lambdas.

### Phase 4: PHP semantic backend

- Add Mago adapter behind `parser-mago-php`.
- Compare PHP metrics against tree-sitter.
- Add fixtures for enums, attributes, traits, promoted properties, readonly, property hooks, null-safe calls, match, and first-class callables.

### Phase 5: default flips

Flip defaults one language at a time:

1. Oxc for TypeScript / TSX / JavaScript / JSX.
2. Mago for PHP.
3. Ruff for Python, if dependency stability is acceptable.

Each flip needs:

- snapshot comparison,
- changelog entry,
- `--parser-backend tree-sitter` escape hatch,
- benchmark data on a representative repository,
- documentation update in the book.

### Phase 6: deprecate tree-sitter-only language docs

Update:

- `README.md` implementation notes,
- `mehen-book/src/languages.md`,
- `mehen-book/src/developers/new-language.md`,
- `mehen-book/src/developers/update-grammars.md`.

The new rule should be:

> A language needs a parser backend that can emit `mehen` metric events. Tree-sitter is the default backend class, but language-specific parsers are preferred when they provide a richer and actively maintained AST.

## Testing strategy

Use three layers of tests.

### Parser adapter unit tests

Each semantic backend should test event emission directly:

- source span correctness,
- source order,
- function and class names,
- parameter counts,
- visibility,
- decision events,
- Halstead operator / operand events,
- parse diagnostics.

### Cross-backend parity snapshots

For constructs that both tree-sitter and semantic parsers understand, snapshots should compare:

- `FuncSpace` tree shape,
- metric totals,
- min / max / average,
- line spans,
- class metrics.

Differences must be classified:

- parity bug,
- intentional semantic improvement,
- parser limitation,
- metric-definition change.

### Semantic superiority fixtures

Add fixtures where tree-sitter is expected to be weaker or ambiguous:

- Python nested f-string expressions, t-strings, type params, `match`.
- TSX nested JSX expression containers and class-field visibility.
- PHP promoted properties, property hooks, enum members, null-safe calls.

These tests should assert the semantic backend's behavior, not parity.

Run locally:

```bash
cargo nextest run --all-features
cargo insta test --all-features --check --unreferenced reject --test-runner nextest --no-test-runner-fallback --disable-nextest-doctest
cargo clippy --all-targets --all-features --locked
```

## Risk register

| Risk | Impact | Mitigation |
|---|---|---|
| Ruff parser crate is unpublished upstream | Python backend cannot be normal crates.io dependency | Start with git-pinned experimental feature or vendor; do not make default until settled |
| Metric output drift | Users may see unexplained CI deltas | Snapshot diff every language and document intentional changes |
| Compile time / binary size grows | CLI becomes heavier | Keep semantic parsers optional, benchmark binary size, avoid enabling all by default at first |
| Vendor AST lifetimes differ | Adapter complexity and borrow issues | Emit owned `MetricEvent`s during parse while arena/source are alive |
| Token/trivia handling differs | LOC and Halstead may change | Use shared `LineIndex`; prefer parser tokens for operators; add LOC-specific fixtures |
| `--dump` expectations diverge | Users may rely on tree-sitter node names | Keep inspection commands tree-sitter-backed until a normalized dump format is designed |
| Parser error behavior differs | Partial metrics may be inconsistent | Standardize `ParseDiagnostic`; explicit fallback policy; no silent fallback in forced semantic mode |
| Upstream API churn | Maintenance cost | Pin exact versions and isolate all vendor usage inside adapter modules |

## Open decisions

1. Should `--dump` eventually show the vendor AST, the normalized metric event stream, or remain tree-sitter-only?
2. Should `semantic-parsers` ever be in `default`, or should packaged releases choose per target?
3. Is a git-pinned Ruff dependency acceptable for an experimental feature?
4. Should `LANG::Typescript` be split into JavaScript and TypeScript internally once Oxc is available, while keeping output display names stable?
5. Should parser diagnostics become part of machine-readable metric output, or only logs / debug output?

## Recommendation

Adopt the backend + metric-event design. Do not attempt to make non-tree-sitter parsers look like tree-sitter. Keep tree-sitter as a concrete syntax backend and compatibility fallback, but make semantic parser ASTs first-class metric inputs.

Start with an Oxc backend because it is published, designed for external tooling, and covers four current `mehen` language modes. Add Mago next for PHP. Run a Ruff spike for Python 3.14 features, but treat it as experimental until the dependency story is stable.

## Sources

- Tree-sitter basic parsing docs: <https://tree-sitter.github.io/tree-sitter/using-parsers/2-basic-parsing.html>
- Tree-sitter syntax highlighting docs: <https://tree-sitter.github.io/tree-sitter/3-syntax-highlighting.html>
- Ruff Python parser README: <https://github.com/astral-sh/ruff/tree/main/crates/ruff_python_parser>
- Ruff parser source API: <https://github.com/astral-sh/ruff/blob/main/crates/ruff_python_parser/src/lib.rs>
- Ruff parser options: <https://github.com/astral-sh/ruff/blob/main/crates/ruff_python_parser/src/parser/options.rs>
- Ruff Python version model: <https://github.com/astral-sh/ruff/blob/main/crates/ruff_python_ast/src/python_version.rs>
- Ruff AST model source: <https://github.com/astral-sh/ruff/blob/main/crates/ruff_python_ast/ast.toml>
- Oxc parser README: <https://github.com/oxc-project/oxc/tree/main/crates/oxc_parser>
- Oxc parser source API: <https://github.com/oxc-project/oxc/blob/main/crates/oxc_parser/src/lib.rs>
- Oxc parser architecture docs: <https://oxc.rs/docs/learn/architecture/parser>
- Mago lexer/parser docs: <https://mago.carthage.software/tools/lexer-parser/overview>
- Mago syntax crate source: <https://github.com/carthage-software/mago/tree/main/crates/syntax>
