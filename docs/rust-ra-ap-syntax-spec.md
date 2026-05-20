# Rust analyzer spec — ra_ap_syntax backend

**Status:** implementation reference
**Date:** 2026-05-18
**Scope:** `mehen-rust` ra_ap_syntax-backed analyzer (Phase 9 of the v1
rewrite, replacing the tree-sitter-rust pipeline)

## 1. Goal

Phase 9 of the [v1 rewrite plan](mehen-1-0-from-scratch-rewrite-plan.md)
swaps the Rust parser from `tree-sitter-rust` to rust-analyzer's
published `ra_ap_syntax` (rowan-based concrete syntax tree, with a
typed AST overlay) plus its bundled `ra_ap_parser`. The two crates are
re-published from the rust-analyzer monorepo onto crates.io
automatically; we pin a specific 0.0.x release because rust-analyzer's
publication cadence is fast and AST-level breakage is not
semver-tracked.

The plan §6.1 originally listed Rust under "tree-sitter-rust for 1.0;
revisit rust-analyzer syntax later if needed." Phase 9 promotes that
"later" to "now" because the new analyzer crates Mehen has shipped
(Ruff for Python, Oxc for TypeScript) have established the
language-specific-parser pattern: each language gets the parser that
exposes the *richest* AST for that language's metrics. Tree-sitter
generates a single grammar table; ra_ap_syntax surfaces typed nodes
(`ast::IfExpr`, `ast::MatchArm`, `ast::BinExpr::op_kind()`,
`ast::TryExpr`, `ast::LetExpr`) plus a rowan green/red tree that's
error-tolerant by design.

The published metric contract is unchanged — Halstead, Cyclomatic,
Cognitive, ABC, NArgs, NOM, NExit, NPA, NPM, WMC, LOC, MI all keep
their semantics and serialization shape. What changes is the
*interpretation* of a handful of Rust-specific constructs where the
tree-sitter grammar's CST shape produced demonstrably wrong, fragile,
or weakly-grounded results. Each divergence below is justified from
the metric definition rather than from a desire to mirror legacy.
`crates/mehen-rust/tests/` carries the pinned snapshots.

Why `ra_ap_syntax` and not `rustc_parse`-via-`rustc_private`:

- `rustc_parse` from `rust-lang/rust` requires nightly + the
  `#![feature(rustc_private)]` gate + the `rustc-dev` rustup component.
  Pinning the workspace to a specific nightly turns every CI runner,
  release matrix entry, and contributor laptop into a "must install
  rustc-dev" surface. Mehen's release toolchain is stable Rust 1.93.1+;
  going nightly was rejected.
- The published `ra-ap-rustc_parse` *meta-crate* on crates.io
  intentionally does not include the actual `rustc_parse` /
  `rustc_ast` / `rustc_session` / `rustc_span` crates because those
  pull in nightly thread-local infrastructure (`SessionGlobals`).
  rust-analyzer's auto-publish bot only ships leaf crates that compile
  on stable.
- `ra_ap_syntax` *is* what rust-analyzer itself uses for its concrete
  syntax tree. It compiles on stable, has no `Session` global, is
  thread-safe, and exposes every AST node the metric walker needs.
  See `Cargo.toml`'s pinned `ra_ap_syntax = "=0.0.333"`.

## 2. What is *not* different

The following metric outputs match legacy where the underlying source
is well-formed Rust:

- `cyclomatic.{sum,min,max,avg}` for `if`/`else if`, `for`, `while`,
  `loop`, `match` arms, the `?` operator, and short-circuit `&&`/`||`.
- `cognitive.{sum,min,max,avg}` for nesting penalties, the boolean
  sequence collapser, function-depth penalty, labeled
  `break`/`continue`, and the legacy `Else`-token +1 rule.
- `nom.*` for function and lambda counts.
- `nexit.*` for `return` and `?`.
- `abc.*` for assignments (`=`, compound `+=`, `let` initializers,
  walrus-equivalent `let-else` bindings), branches (call expressions,
  method calls, macro invocations), and conditionals (`if`, `match`,
  loop heads, `?`, comparison operators, logical operators).
- `npa.*` / `npm.*` for class-body fields and methods, with
  `pub`/`pub(crate)`/`pub(super)` all classifying as public, and trait
  methods implicitly public.
- `wmc.*` summed from method cyclomatic.
- `mi.*` Maintainability Index variants.
- Halstead `volume` / `difficulty` / `effort` family on well-formed
  source.

## 3. What is different (and why)

### 3.1 Top-level statement fragments need wrapping in tests

Tree-sitter-rust accepts a free-standing `let a = ();` at the top
level of a source file because its grammar has a permissive
`source_file -> _statement*` production. ra_ap_syntax — like rustc
itself — requires every statement to live inside a function body or
const initializer. A bare `let` at the top level produces an `ERROR`
node containing the raw tokens, not a `LET_STMT`.

This is purely a *test-fixture* issue. Real `.rs` files shipped to
mehen always have proper top-level structure. Where the legacy LOC
test corpus used a bare statement (`"let a = ();"`), the Phase 9
ports wrap the fragment in `fn _wrap() { … }` and document the
file-level totals (`sloc`, `ploc`, `lloc`, `cloc`, `blank`) — the
per-space `_min`/`_max` shift because of the added function space,
but the *statement*'s LLOC contribution is unchanged.

This is the same kind of fixture-correctness adjustment documented
in `docs/python-ruff-spec.md` §3.5 (Python indentation) and
`docs/rust-ra-ap-syntax-spec.md` (this document). Production input
is always real Rust; only test fixtures need adjusting.

Test: `crates/mehen-rust/tests/loc.rs` — every test that uses
`analyze_wrapped` documents the wrap inline.

### 3.2 LOC cyclomatic / cognitive `null` average is now `0.0`

Legacy serialized `cognitive.average` and `nexit.average` as JSON
`null` when the unit had zero functions (because the legacy
`Stats::cognitive_average: Option<f64>` was rendered through serde's
default `Option` → `null`). The Phase-1+ shared accumulators in
`mehen-metrics::cognitive` and `mehen-metrics::counters::NexitStats`
emit `0.0` — there is nothing to average, and `0.0` is mathematically
defensible for "no contribution."

Same drift documented for Python (`docs/python-ruff-spec.md` §2 implicit)
and applies workspace-wide.

Test: `crates/mehen-rust/tests/cognitive.rs::rust_no_cognitive`,
`tests/exit.rs::rust_no_exit`.

### 3.3 NargsStats `_min` is gated on `is_function`/`is_closure`

Before Phase 6, `NargsStats::compute_minmax` folded *every* space's
per-space `fn_nargs` and `closure_nargs` (defaulting to 0 for
unit/class spaces) into the rolled-up min. Result: any source with at
least one function and a unit space reported `functions_min: 0.0`
even when the only function had 2 args.

Phase 6's `NargsStats::finalize_minmax` gates the per-space fold on
`is_function`/`is_closure` flags. The Rust port inherits the fix
unchanged — `fn f(a: bool, b: usize)` now reports `functions_min: 2.0`
(legacy: `0.0`), matching the metric's intended definition.

Same drift documented for Python (`docs/python-ruff-spec.md` §3.4)
and PowerShell (`crates/mehen-powershell/tests/nargs.rs` module
header). Rust's NomStats follows the same shape.

Test: `crates/mehen-rust/tests/nargs.rs::rust_single_function` and
all other Rust nargs ports carry the corrected `_min` snapshots.

### 3.4 NPM publishes "public-method count" instead of "container count"

The legacy NPM JSON serialization used `interfaces` to mean "number
of trait containers" and `interfaces_average` to mean
`interface_methods / interfaces`. The Phase-1+ pipeline's
`mehen-metrics::counters::NpmStats::publish_npm` re-uses those field
names with different semantics: `interfaces` is the total
*public-method* count in interfaces, and `interfaces_average` is the
public-ratio (`public / total`). This is a deliberate metric-
definition change shared across all Phase 9 ports — every language's
NPM follows the same `publish_npm` shape.

For Rust this means: a trait with 2 methods (both implicitly public)
publishes `interfaces: 2.0, interface_methods: 2.0,
interfaces_average: 1.0` — not `interfaces: 1.0,
interfaces_average: 2.0` as legacy did.

Test: `crates/mehen-rust/tests/npm.rs::rust_npm_counts_trait_signature_and_default_methods`.

### 3.5 Trait method NPM bookkeeping happens on the enclosing space

The python walker's pattern is: when a class body contains a `def`,
record the method on the *class's* state (via
`classify_class_body_member`), and let the function space's own NPM
counters stay at zero. The Rust walker follows the same pattern —
when entering a `Fn` whose grandparent is `Trait` or `Impl`, the NPM
contribution lands on the *enclosing trait/impl state*, not on the
function's own state.

The reason is double-counting: if both the trait and the
function's own state record the method, the trait's `finalize_minmax`
folds its own per-space NPM into the sum, AND merges the child
function's NPM sum, producing 2× the expected count. By recording on
the enclosing scope only, the merge path stays consistent.

Test: `crates/mehen-rust/tests/npm.rs::rust_npm_counts_pub_in_impl_block`
(2 impl methods, 1 public).

### 3.6 Trait function signatures (no body) do not open a func space

A trait method declared without a body (`fn next(&self);`) is not a
function space in the legacy walker — there's nothing to walk. The
ra_ap_syntax walker mirrors this: when entering an `ast::Fn` whose
`body()` is `None`, we skip the `open_space` call but still record
the method on the enclosing trait via `classify_method`.

Default-bodied trait methods (`fn count(&self) -> usize { 0 }`) flow
through the regular FuncSpace + merge path.

Test: `tests/parity.rs::rust_trait_associated_types_and_consts_are_not_methods`.

### 3.7 Else-token +1 attributed to the parent IF_EXPR

The legacy walker emitted a flat `+1 cognitive` on every `Else` token,
covering both `else if` (the connecting `else` between two `if`
expressions) and bare `else { … }` (the alternative branch).

ra_ap_syntax's typed AST does not surface a dedicated `Else` node —
each `IfExpr` exposes its own `else_token()` and `else_branch()`. The
walker attributes the +1 to the *parent* `IF_EXPR` whose
`else_token()` is present. The behavioral count is unchanged.

For nested `if A { … } else if B { … } else { … }`, this means the
outer `if A` emits +1 (it has an else branch), the inner `if B` does
NOT emit a nesting bump (it's an `else if`), and the inner `if B`
emits +1 (it has its own bare `else`). Total: +1 (A's else) + +1 (B's
else) + nesting penalty for `if A`. Matches legacy.

Test: `tests/cognitive.rs::rust_1_level_nesting_complex`,
`rust_break_continue`, `rust_if_let_else_if_else`.

### 3.8 Block tail expression is a logical line of code

A function body's *tail expression* (`fn f() { 42 }`'s `42`, no
semicolon) is a logical line of code per the legacy
`is_rust_tail_expression` rule. tree-sitter exposed this via parent-
chain inspection; ra_ap_syntax exposes it directly via
`StmtList::tail_expr()`. The walker emits +1 LLOC for any expression
that is a tail of its enclosing `STMT_LIST`. The kind-specific
handling still fires (a `for` expression that is also a tail still
records its cyclomatic decision).

Test: `tests/loc.rs::rust_tail_expressions_are_lloc`,
`rust_lloc_for_if`, `rust_function_in_if_lloc`.

### 3.9 Macro bodies are opaque — same as legacy

Tokens inside a `MacroCall`'s argument tree (or `macro_rules!` body)
do not contribute to cyclomatic, cognitive, ABC, or exit counters. The
macro path identifier itself counts as a branch (call). This matches
the legacy `is_inside_rust_macro_tokens` filter.

The implementation tracks the depth of macro-opaque scopes via a
counter that increments on enter and decrements on leave. While
inside a macro body, the structural walk early-returns from
`enter_node` for non-macro kinds, but still tracks nested macro
boundaries so the depth unwinds correctly.

For Halstead, every macro-opaque range is recorded once during the
structural walk; the post-AST token sweep then skips any token whose
span falls inside any of those ranges. This is more robust than
walking parent chains for every token.

Test: `tests/cyclomatic.rs::rust_macro_tokens_are_opaque_for_cyclomatic`,
`tests/cognitive.rs::rust_macro_tokens_are_opaque_for_cognitive`,
`tests/parity.rs::rust_macro_body_control_flow_is_opaque`.

### 3.10 Type annotations contribute to Halstead

Rust types are not erased at runtime — they describe the shape of
values, are visible to `mem::size_of`, drive trait dispatch, and
appear in `TypeId` reflection. A type identifier like `Vec<T>` is a
*thing* — a real operand in the running program. The walker emits
type-position identifiers as Halstead operands, exactly like
expression-position identifiers.

This is the same reasoning as Python (`docs/python-ruff-spec.md`
§3.1) and the *opposite* of TypeScript (`docs/typescript-halstead-spec.md`),
where TS-only `TSTypeAnnotation` / `TSInterfaceDeclaration` nodes are
excluded because TS types are erased at compile time.

### 3.11 Doc comments reach LOC `cloc`, not Halstead

`///` outer doc comments and `//!` inner doc comments are
`SyntaxKind::COMMENT` tokens at the lexer level. The walker's token
sweep folds every comment token into LOC `cloc` on the unit, but
classifies them as `Skip` in the Halstead sweep — they are
documentation, not running code. This matches the legacy
`add_cloc_lines` for `LineComment` / `BlockComment`.

### 3.12 `let-else` is an assignment

`let-else` (RFC 3137, stable in Rust 1.65) was supported by
tree-sitter-rust 0.24+. The legacy walker classified it via
`LetDeclaration if node.is_child(EQ)`. ra_ap_syntax exposes
`LetStmt::let_else()` directly. The walker emits `+1 ABC.assignments`
when the `LetStmt` has an `=` token (i.e. it's a real bind, not a
bare `let x;` declaration). The diverging branch's body participates
in cognitive / cyclomatic counters normally.

Test: `tests/parity.rs::rust_let_else_is_assignment_with_diverging_else`.

### 3.13 `if let` chains collapse via the boolean-sequence rule

`if let` chains (RFC 2497, stable in Rust 1.88) parse as a chain of
`LetExpr` operands joined by `&&`. The Phase-1+ shared
`BoolSequence::eval_based_on_prev` collapses same-op runs into a
single +1, so `if let A = … && let B = … && cond` adds exactly +1
cognitive (collapsed run) on top of the +1 for the `if`.

The legacy walker had a special `LetChain` named node; the
ra_ap_syntax walker does not need one — the `&&` operands are regular
`BinExpr` nodes that the standard cognitive rule handles.

Test: `tests/parity.rs::rust_if_let_chain_collapses_to_single_cognitive_bump`,
`tests/abc.rs::rust_abc_counts_let_chain_operators_in_conditions`.

### 3.14 Async functions are still functions

`async fn` is an `ast::Fn` AST node with `async_token()` set; it still
opens a function space, contributes to `nom`, and resets cognitive
nesting on entry. The `.await` postfix is a regular expression that
contributes nothing structural.

Test: `tests/parity.rs::rust_async_fn_opens_function_space`.

## 4. Operator and operand classification

The walker maps `ra_ap_syntax::SyntaxKind` values to one of
`Operator(&str)`, `Operand(&str)`, or `Skip`. The mapping in
`crates/mehen-rust/src/walker.rs::classify_token` covers every Rust
keyword, every punctuation / operator token, and every literal kind
(`IDENT`, `INT_NUMBER`, `FLOAT_NUMBER`, `STRING`, `BYTE_STRING`,
`C_STRING`, `CHAR`, `BYTE`, `LIFETIME_IDENT`, plus the keyword-as-
literal `true`/`false`).

Closing punctuation (`)`, `]`, `}`) is `Skip` to avoid double-counting
under the classical Halstead pair convention. Trivia (`WHITESPACE`,
`COMMENT`, `TOMBSTONE`, `EOF`) is `Skip`.

Where a keyword is also a syntactic token (e.g. `Self` vs `self`,
`pub` vs `pub(crate)`), the `T!` macro from ra_ap_syntax disambiguates
the variant. The `T!` macro is preferred over bare `SyntaxKind::*`
imports because Rust's pattern grammar treats unimported uppercase
identifiers as fresh bindings, which silently shadowed our match arms
in early implementation drafts and produced "unreachable pattern"
warnings.

## 5. Walker structure

The walker in `crates/mehen-rust/src/walker.rs` follows the same
per-space `State` accumulator pattern used by `mehen-typescript`:

- One `State` (in `mehen-metrics::state`) for the unit, plus one for
  every opened function / closure / impl / trait space.
- Cyclomatic / cognitive / ABC / nexit / LOC / NPA / NPM are driven
  per-AST-node via an explicit `WalkEvent::Enter`/`WalkEvent::Leave`
  loop over `SyntaxNode::preorder()`. The explicit loop (vs recursion)
  is deliberate so the per-space stack can finalize on Leave events
  even for deeply-nested input.
- Halstead is driven by a post-AST token sweep over the source file's
  `descendants_with_tokens()`. Each token maps to one of
  `Operator(kind)`, `Operand(kind)`, or `Skip`. Tokens whose span
  falls inside a recorded macro-opaque range are skipped.

The `CognitiveContext` tracks `(nesting, depth, lambda)` exactly as
the pre-1.0 `cognitive::rust_code` did. The boolean-sequence
collapser lives in `mehen-metrics::cognitive::BoolSequence`; the
walker calls `observe_boolean("&&")` / `observe_boolean("||")` on
`BinExpr` nodes whose `op_kind()` is `LogicOp::And`/`Or`.

## 6. References

- Pinned `ra_ap_syntax` version: `=0.0.333` (workspace
  `Cargo.toml:105`).
- ra_ap_syntax docs: <https://docs.rs/ra_ap_syntax/0.0.333>
- The walker's typed AST entry points are documented inline next to
  each `enter_node` arm.
- Migration commit: see `git log --oneline | grep 'phase-9\|ra_ap_syntax\|Rust analyzer'`
  for the patch series.
