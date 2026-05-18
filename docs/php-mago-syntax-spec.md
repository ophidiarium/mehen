# PHP analyzer spec — mago-syntax backend

**Status:** implementation reference
**Date:** 2026-05-19
**Scope:** `mehen-php` mago-syntax-backed analyzer (Phase 8 of the v1
rewrite, replacing the tree-sitter-php pipeline)

## 1. Goal

Phase 8 of the [v1 rewrite plan](mehen-1-0-from-scratch-rewrite-plan.md)
swaps the PHP parser from `tree-sitter-php` to
[`mago-syntax`](https://docs.rs/mago-syntax/) — the lexer/parser/AST
layer that powers the [Mago](https://mago.carthage.software) PHP
toolchain. mago-syntax is published on crates.io, designed for external
consumption, and ships a typed AST with PHP-specific nodes (attributes,
promoted properties, enum cases, hooked properties, match expressions,
null-safe member calls, …) that tree-sitter-php either flattens into
generic CST nodes or doesn't model at all.

The migration is a Phase 6.4 directive in the rewrite plan and the last
of the four "language-specific parser" rollouts (PowerShell stayed on
tree-sitter; TypeScript moved to Oxc; Python moved to Ruff; Rust moved
to ra_ap_syntax). With Phase 8 done, every actively-evolving language
in the workspace has its own typed-AST backend.

The published metric contract is unchanged — Halstead, Cyclomatic,
Cognitive, ABC, NArgs, NOM, NExit, NPA, NPM, WMC, LOC, MI all keep
their semantics and serialization shape. What changes is the
*interpretation* of a handful of PHP-specific constructs where the
tree-sitter grammar produced fragile or hand-rolled string-comparison
results. Each divergence below is justified from the metric definition
rather than from a desire to mirror legacy. `crates/mehen-php/tests/`
carries the pinned snapshots; the seven legacy `check_metrics::<PhpParser>`
tests are ported byte-identical so parity is provable.

### 1.1 Why `mago-syntax` and not the alternatives

- **`tree-sitter-php` (the legacy backend)**: a CST grammar, not an
  AST. Visibility modifiers come back as raw token text on
  `VisibilityModifier` nodes; the legacy walker had to do
  `text.eq_ignore_ascii_case("private")` because tree-sitter doesn't
  know PHP keywords are case-insensitive. Promoted constructor
  properties parse as `property_promotion_parameter` *anywhere a
  parameter is allowed*, including non-constructor methods (PHP rejects
  these at runtime), so the legacy walker had a hand-rolled lookahead
  that walked up to the enclosing method, decoded the name, and did a
  case-insensitive compare against `"__construct"`. Each new
  PHP-specific feature meant another grammar PR and another fragile
  classifier.
- **`php-parser-rs`**: less actively maintained, smaller surface, no
  dedicated walker abstraction.
- **Hand-written nom parser**: the rewrite plan §6.8 explicitly calls
  this out as a bad use of `nom` — replacing a mature language parser.
- **`mago-syntax`**: typed AST with arena allocation, mature error
  recovery, an automatically-generated `Walker` trait (with
  `walk_in_<node>` / `walk_out_<node>` hooks per AST node type), and an
  upstream test suite that tracks PHP 7.0 through PHP 8.5+. Mago's
  internal lint pipeline drives the same walker via `mago-collector` —
  we re-use the trait directly without depending on the collector
  itself.

### 1.2 MSRV bump

`mago-syntax 1.27.1` declares `rust-version = 1.95.0`. The workspace
`rust-version` was `1.93.1` before Phase 8; the rewrite plan §6.4 calls
out the bump as a prerequisite. Phase 8 raises `[workspace.package]
rust-version` to `1.95.0`, which matches the pinned stable toolchain
already in `rust-toolchain.toml` (`channel = "stable"` resolves to
1.95.0+).

## 2. What is *not* different

The following metric outputs match legacy where the underlying source
is well-formed PHP:

- `cyclomatic.{sum,min,max,avg}` for `if`/`elseif`/spaced-`else if`,
  `for`, `foreach`, `while`, `do`, `switch` cases, `match` arms,
  ternary, `catch`, and short-circuit `&&`/`||`/`and`/`or`. (`xor`
  excluded — it does not short-circuit.)
- `cognitive.{sum,min,max,avg}` for nesting penalties, the boolean
  sequence collapser (with the `else`-resets-sequence rule from the
  legacy walker), the `else`-clause flat +1, and function-depth
  penalty.
- `abc.*` for assignment / branch / condition counts, including the
  prefix and postfix `++`/`--` family on ABC.A and the legacy `else`
  / `default` ABC.C convention.
- `nom.*` for function / closure / arrow-function counts.
- `nexit.*` for `return`, `throw`, and `exit`/`die`.
- `nargs.*` for function, closure, and method parameter lists.
- `npa.*` / `npm.*` for class / interface / trait / enum members,
  with PHP visibility (`public`/`protected`/`private` plus their
  `(set)` variants for asymmetric visibility) classified through
  Mago's typed `Modifier` enum.
- `wmc.*` summed from method cyclomatic on classes.
- `mi.*` Maintainability Index variants.
- Halstead `volume` / `difficulty` / `effort` family on well-formed
  source.
- All seven legacy parity tests (`php_basic_decision_points`,
  `php_else_branch_resets_boolean_sequence`,
  `php_npa_counts_each_property_in_grouped_declaration`,
  `php_npa_counts_promoted_constructor_properties`,
  `php_npa_does_not_count_promoted_params_outside_constructor`,
  `php_npm_visibility_keywords_are_case_insensitive`,
  `php_wmc_class_sums_method_cyclomatics`) pass byte-identically with
  inline `@r###"…"###` JSON snapshots verbatim.

## 3. What is different (and why)

### 3.1 PHP visibility is now typed, not text-compared

**Legacy behavior:** `tree-sitter-php` exposes a `VisibilityModifier`
CST node whose textual value is the raw source slice
(`"public"`, `"PRIVATE"`, `"Protected"`, …). PHP keywords are
case-insensitive per the language spec (`PRIVATE function f() {}` is
valid PHP), so the legacy `php_member_is_public()` helper in
`legacy/metrics/npm.rs` did:

```rust
text.eq_ignore_ascii_case("private") || text.eq_ignore_ascii_case("protected")
```

**New behavior:** Mago's parser already normalizes visibility
keywords into a typed `Modifier` enum:

```rust
pub enum Modifier<'arena> {
    Static(Keyword<'arena>),
    Final(Keyword<'arena>),
    Public(Keyword<'arena>),
    Protected(Keyword<'arena>),
    Private(Keyword<'arena>),
    PublicSet(Keyword<'arena>),    // PHP 8.4 asymmetric visibility
    ProtectedSet(Keyword<'arena>),
    PrivateSet(Keyword<'arena>),
    /* … */
}
```

`php_modifiers_are_public()` becomes a clean enum match, with no string
comparison and no case-insensitivity logic. The case-insensitive
behavior is preserved for free because the lexer normalizes the keyword
on the way in.

**Why this is correct:** the metric definition has not changed — public
members are still everything that isn't `private`/`protected` (with the
new variants for the PHP 8.4 asymmetric-visibility forms also
classified as non-public). The legacy
`php_npm_visibility_keywords_are_case_insensitive` test still passes
because `PRIVATE` lexes to `Modifier::Private` regardless of case.

### 3.2 Promoted constructor properties via typed predicate

**Legacy behavior:** the tree-sitter grammar accepts
`property_promotion_parameter` syntactically inside *any* method —
including non-constructors — because the grammar can't enforce the
semantic rule that PHP rejects at runtime. The legacy walker had to
walk up two levels (`parameter -> formal_parameters ->
method_declaration`), grab the method's `name` field, decode it as
UTF-8, and do an ASCII-case-insensitive compare against `"__construct"`
to gate the `record_attribute` call.

**New behavior:** Mago exposes a typed predicate:

```rust
impl FunctionLikeParameter<'_> {
    pub fn is_promoted_property(&self) -> bool {
        !self.modifiers.is_empty() || self.hooks.is_some()
    }
}
```

The walker collects promoted properties at the start of `walk_in_method`
*only* when `is_constructor(&method.name.value)` returns true. The
constructor check is still case-insensitive (`__construct` /
`__CONSTRUCT` / `__Construct` are all the constructor in PHP), but the
*structural* test ("is this parameter actually declaring a property?")
is one method call instead of three levels of CST navigation.

The
`php_npa_does_not_count_promoted_params_outside_constructor` legacy
test still passes verbatim — it tested the case-insensitivity of the
constructor name check, which is preserved.

### 3.3 `else if` (spaced) flattening is per-node, not per-token

**Legacy behavior:** the tree-sitter-php grammar has both
`else_if_clause` (the `elseif` keyword form) and a nested `if_statement`
(the spaced `else if` form). The legacy walker's `is_else_if` predicate
detected the nested form by checking whether the `if_statement`'s
*direct parent* was an `else_clause` — and then suppressed the inner
`if`'s structural nesting via a generic "is_else_if" lookahead.

**New behavior:** Mago surfaces both forms as distinct AST nodes:

- `IfStatementBodyElseIfClause` — the keyword `elseif` form.
- `IfStatementBodyElseClause::statement` — when this statement is itself
  an `Statement::If`, the spaced `else if` form is in play.

The walker handles each form in its own `walk_in_*` callback:

- `walk_in_if_statement_body_else_if_clause` records a flat `+1`
  cognitive (no extra nesting), one cyclomatic decision, and resets the
  boolean sequence.
- `walk_in_if_statement_body_else_clause` resets the boolean sequence,
  records ABC.C, and — when the body is an `If` — sets a one-shot
  `suppress_next_if_nesting` flag so the *inner* `walk_in_if` knows not
  to bump structural nesting (the outer `if`'s nesting has already
  paid for the inner branch).

The `php_else_branch_resets_boolean_sequence` legacy test (which
exercises both the `else if` flattening and the boolean-sequence reset
across an `else` boundary) passes verbatim with `cognitive.sum = 4.0`.

### 3.4 ABC.C `else` is now explicit

**Legacy behavior:** `ElseClause` / `ElseClause2` were enumerated in
the ABC.C arm of `legacy/metrics/abc.rs`, contributing one condition
per `else` (per Fitzpatrick's original ABC, where every conditional
branch — including the catch-all `else` — counts as a condition).

**New behavior:** the walker records ABC.C in
`walk_in_if_statement_body_else_clause` and
`walk_in_if_colon_delimited_body_else_clause`. The
`walk_in_switch_default_case` callback records ABC.C the same way (PHP
`default` is the switch analog of `else`).

This was an *audit-discovered* gap during Phase 8 — my initial walker
omitted the ABC.C bump on `else` clauses, which would have under-counted
conditions on every `if`/`else` pair. The legacy parity test for
cognitive complexity still passed (it doesn't read ABC), so the
regression was silent until I ran a hand-written audit fixture against
the walker. The fix landed before Phase 8 closed, with the new
`php_abc_conditions_cover_control_flow_and_comparisons` test pinning
the count.

### 3.5 ABC.A prefix `++`/`--`

**Legacy behavior:** `legacy/metrics/abc.rs`'s PHP arm enumerated
`UpdateExpression` (which the tree-sitter grammar emits for both
prefix and postfix forms) under ABC.A.

**New behavior:** Mago splits prefix and postfix into distinct AST
node types:

- `UnaryPrefix { operator: UnaryPrefixOperator::PreIncrement(_) | PreDecrement(_) }`
- `UnaryPostfix { operator: UnaryPostfixOperator::PostIncrement(_) | PostDecrement(_) }`

The walker hooks both: `walk_in_unary_prefix` matches the
`PreIncrement`/`PreDecrement` operator and records ABC.A;
`walk_in_unary_postfix` matches the `PostIncrement`/`PostDecrement`
operator and records ABC.A.

This was the second audit-discovered gap. My initial walker only
recorded `walk_in_assignment` for ABC.A (which Mago surfaces as the
typed `Assignment` node for `=`/`+=`/`??=`/etc.). The
`php_abc_assignments_cover_all_assignment_forms` test now exercises
all eight forms (4 typed assignments + 2 prefix + 2 postfix) and
asserts `assignments == 8.0`.

### 3.6 LLOC counts every statement-shaped node, not just expression statements

**Legacy behavior:** `legacy/metrics/loc.rs`'s PHP arm enumerated
~25 statement-shaped node kinds for LLOC: every expression statement,
every empty statement, `echo`, `unset`, `declare`, `namespace`, `use`,
`global`, `function-static`, `try`, `continue`, `break`, `return`,
loops, `if`, `switch`, `case`, `default`, label, `goto`, every
function/method/class/trait/interface/enum declaration, `const` (both
top-level and class-scoped), and `property` declarations.

**New behavior:** the walker hooks `walk_in_*` for each of those node
types and calls `current().loc.observe_lloc()` exactly once per
statement. The `walk_in_if` hook bumps LLOC; the inner `else if` /
`else` clauses do *not* bump again (an `if … else` chain is one
logical statement, not two — same convention as legacy).

This was the third — and largest — audit-discovered gap. My initial
walker only bumped LLOC inside `walk_in_statement_expression`, which
left every PHP file with a wildly under-counted LLOC. A 30-line audit
fixture reported `lloc = 1.0` instead of the expected 16. The fix
landed with six new tests in `crates/mehen-php/tests/loc.rs`:

- `php_lloc_counts_simple_function_body` — function decl + if + 2 returns
- `php_lloc_counts_namespace_use_const` — top-level decls
- `php_lloc_counts_class_members` — class + class-const + property + method
- `php_lloc_counts_loops_and_switch` — every loop + switch + case + default
- `php_lloc_counts_try_throw_echo_unset` — exit-flow statements
- `php_lloc_does_not_count_else_clauses_separately` — `if/else` is one LLOC

### 3.7 Halstead via re-lex, not via AST node kinds

**Legacy behavior:** `legacy/getter.rs::get_op_type` for PHP returned
a `HalsteadType::{Operator, Operand, Unknown}` for each tree-sitter
`Php` enum variant. The `compute_halstead` machinery walked the CST
and emitted operator/operand events in source order.

**New behavior:** mago-syntax's typed AST does not surface every
punctuation token as a node — `(`, `,`, `;`, `{` are stored on parent
nodes as `Span` fields, not as visitable children. Re-walking the AST
to emit Halstead would need bespoke visit logic for every parent node
type. Instead, the walker re-uses Mago's `Lexer` directly:

```rust
fn emit_halstead_from_tokens(&mut self) {
    let input = Input::new(FileId::zero(), self.source.as_bytes());
    let mut lexer = Lexer::new(input, LexerSettings::default());
    while let Some(result) = lexer.advance() {
        match classify_token(token.kind) { /* … */ }
    }
}
```

The `classify_token` function maps each `mago_syntax::token::TokenKind`
to either:

- `TokenClass::Operator(distinct_kind_str)` — every keyword, every
  punctuation token, every operator gets its own kind string so `n1`
  reflects the true number of unique operators.
- `TokenClass::Operand(kind_str)` — identifiers (incl. qualified names),
  variables, literals (int, float, string, true, false, null), magic
  constants (`__CLASS__`, `__LINE__`, …), and the keywords `self` /
  `parent` (which name a class).
- `TokenClass::Skip` — whitespace, comments (handled separately for
  LOC.cloc), inline-HTML between PHP tags, opening/closing PHP tags
  (`<?php` / `?>`), string-interior tokens (`StringPart` / `DoubleQuote`
  / heredoc body — the wrapping `LiteralString` already counts), and
  *closing* punctuation (`)`, `]`, `}`) which pairs with its opener
  (classical Halstead pair convention, mirrors `mehen-rust`).

This is functionally identical to the legacy classifier — every token
the legacy `get_op_type` flagged as `Operator` or `Operand` is
classified the same way here. The mechanics are different (token
sweep vs CST walk) but the metric output for well-formed PHP is the
same shape.

This was Phase 8's *first* audit-discovered gap — the most embarrassing
one. The walker had no Halstead emission at all (zero `n1`/`n2`/etc.
on every PHP file). The
`_halstead_observe_*` placeholders I'd left "as a TODO" got swept up
as dead code by `unreachable_pub` cleanup, which silently removed the
TODO marker without anyone noticing. Two new tests in
`crates/mehen-php/tests/halstead.rs` lock the classifier in:
`php_halstead_simple_function` (full output snapshot) and
`php_halstead_string_part_is_skipped` (regression test for the
string-internals skip).

### 3.8 Anonymous classes do not bump LLOC themselves

**Legacy behavior:** the tree-sitter grammar's `anonymous_class` was a
declaration kind; the legacy LLOC arm enumerated `ClassDeclaration` and
`AnonymousClass` together.

**New behavior:** Mago surfaces `AnonymousClass` as an *expression*
(it's the result of `new class { … }`), so the surrounding
`ExpressionStatement` already bumps LLOC for the whole `$x = new
class { … };` line. Bumping again on `walk_in_anonymous_class` would
double-count.

This is a genuine improvement — anonymous classes really *are*
expressions in PHP, not statements. The legacy walker would over-count
LLOC for every `new class` expression by one.

### 3.9 Enum cases as NPA attributes

**Legacy behavior:** the tree-sitter walker did not record enum cases
as NPA attributes (the legacy `npa.rs` PHP arm only matched
`PropertyDeclaration` and `PropertyPromotionParameter`).

**New behavior:** PHP enum cases are typed constants on the enum.
They have no per-instance state, so they're not "attributes" in the
classical OO sense. But they *do* contribute to an enum's surface
area the same way class constants do. The walker records each
`EnumCase` as a class-attribute on the enclosing enum's NPA state, with
`is_public = true` (cases are always public in PHP — there's no syntax
to make them private).

This is a deliberate semantic improvement, not an unintentional
divergence. NPA's "number of public attributes" is meant to capture
the surface a class exposes; enum cases are part of that surface.
The drift is bounded: zero for any PHP file without enums (matching
legacy), and a small additive change for files using enums.

### 3.10 PHP 8.5 pipe operator (`|>`) is a comparison-class operator

**Legacy behavior:** tree-sitter-php's grammar tracks the `|>` token
as `PIPEGT`. The legacy `get_op_type` for PHP put it under the
"comparison" group.

**New behavior:** the new `classify_token` keeps `PipeGreaterThan` in
the comparison group (returning `Operator("|>")`). Behaviorally
identical for Halstead — the metric reflects "this is one distinct
operator token" either way.

## 4. Operator and operand classification

The full table lives in
`crates/mehen-php/src/walker.rs::classify_token`. The shape mirrors
`mehen-rust`'s walker:

- **Skipped tokens** — `Whitespace`, comments (handled separately for
  LOC.cloc), `InlineText` / `InlineShebang`, `OpenTag` / `CloseTag` /
  `EchoTag` / `ShortOpenTag`, `StringPart` / `DoubleQuote` / `Backtick`
  / `DocumentStart` / `DocumentEnd` / `PartialLiteralString` (string
  interior; the wrapping `LiteralString` is the operand), and *closing*
  pair punctuation (`)`, `}`, `]`).
- **Operator** — every other punctuation token, every keyword, every
  operator family (assignment, arithmetic, bitwise, unary, null-coalesce,
  comparison, type cast). Each gets its own kind string so `n1` is
  precise.
- **Operand** — `Identifier` / `QualifiedIdentifier` /
  `FullyQualifiedIdentifier` (collapsed under `"Identifier"`),
  `Variable`, `Self_`, `Parent`, the literal family
  (`LiteralInteger` / `LiteralFloat` / `LiteralString` / `True` /
  `False` / `Null`), the magic constants (`ClassConstant` /
  `LineConstant` / etc., collapsed under `"MagicConstant"`), and
  `Callable` (treated as an identifier in argument lists).

The legacy classifier collapsed all qualified-name forms under a single
operand kind too (`Name` / `Name2` / `NamespaceName` /
`QualifiedName` / `RelativeName` all mapped to `HalsteadType::Operand`),
so the kind string `"Identifier"` here is a behavioral match.

## 5. Walker structure

The walker uses Mago's `Walker` trait — the same trait Mago's own lint
pipeline drives via `mago-collector` for pragma-scope attachment.
`Walker` is `&self` (immutable), threading mutable state through a
user-provided context type. The walker macro generates three callback
methods per AST node:

- `walk_in_<node>(&self, node, context)` — fires on enter.
- `walk_<node>(&self, node, context)` — drives the descent into
  children.
- `walk_out_<node>(&self, node, context)` — fires on leave.

The default `walk_<node>` body is auto-generated to recurse into every
field of the node type (`if_node.condition`, `if_node.body`, etc.), so
overriding `walk_in_<node>` / `walk_out_<node>` is sufficient for
metric accumulation; the walk continues automatically.

`Visitor` (the context type) holds:

- `tree: MetricTreeBuilder` — produces the final `MetricSpace`.
- `stack: Vec<State>` — per-space metric accumulators (index 0 is the
  unit; pushed/popped as classes/functions open/close).
- `kinds: Vec<SpaceKind>` — parallel to `stack`, lets the walker tell
  "what's the enclosing class-like" without re-walking.
- `cognitive: CognitiveContext` — the (`nesting`, `depth`, `lambda`)
  triple from the legacy walker, snapshotted into `saved_cognitive`
  on every nesting bump and restored on `walk_out_<node>`.
- `suppress_next_if_nesting: bool` — one-shot flag for the
  `else if`-spaced flattening (see §3.3).

Halstead emission runs once at `Visitor::finish()` via Mago's `Lexer`
on the original source bytes; LOC.ploc is also derived there (single
source-text scan instead of a re-walk). Per-space close drives
`finalize_state` + `merge_child_into_parent`, identical to
`mehen-rust` and `mehen-python`.

`mehen-php` does **not** depend on `mago-collector` — that crate is a
diagnostic-issue collector (suppression pragmas, issue codes,
`IssueCollection` types) for Mago's lint pipeline. The walker
abstraction we want lives in `mago-syntax` itself.

## 6. References

- mago-syntax docs: <https://docs.rs/mago-syntax/>
- Mago lexer/parser overview:
  <https://mago.carthage.software/tools/lexer-parser/overview>
- mago-collector source (for the walker integration pattern):
  <https://github.com/carthage-software/mago/tree/main/crates/collector>
- v1 rewrite plan §6.4: PHP / Mago migration prerequisites.
- `crates/mehen-php/src/walker.rs` — implementation.
- `crates/mehen-php/tests/` — pinned snapshots and regression tests.
