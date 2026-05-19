# Ruby analyzer spec — ruby-prism backend

**Status:** implementation reference
**Date:** 2026-05-19
**Scope:** `mehen-ruby` ruby-prism-backed analyzer (Phase 9 of the v1
rewrite, replacing the tree-sitter-ruby pipeline)

## 1. Goal

Phase 9 of the [v1 rewrite plan](mehen-1-0-from-scratch-rewrite-plan.md)
swaps the Ruby parser from `tree-sitter-ruby` to
[`ruby-prism`](https://docs.rs/ruby-prism/) — the Rust binding for
[Prism](https://github.com/ruby/prism), Ruby's canonical parser shipped
with CRuby 3.3+ and JRuby 9.4+. Prism is the parser the Ruby
maintainers themselves use; it tracks the language week-to-week,
recognizes every Ruby 3.x syntax form (numbered block parameters, `it`
parameters, endless methods, pattern matching, `&.`, `=>` rightward
assignment, `in` patterns, hash shorthand, …), and exposes them as
distinct typed AST nodes rather than CST fragments.

The migration is the Phase 6.5 directive in the rewrite plan and
follows the same pattern as Phase 6 (Ruff Python), Phase 7 (Oxc
TypeScript), Phase 8 (Mago PHP), and Phase 9 Rust (rust-analyzer
syntax). With this phase done, every actively-evolving source language
in the workspace has its own typed-AST backend; only Go, C, Kotlin,
and PowerShell still use tree-sitter (and the rewrite plan §6.1 leaves
those on tree-sitter for 1.0).

The published metric contract is unchanged — Halstead, Cyclomatic,
Cognitive, ABC, NArgs, NOM, NExit, NPA, NPM, WMC, LOC, MI all keep
their semantics and serialization shape. What changes is the
_interpretation_ of a handful of Ruby-specific constructs where the
tree-sitter grammar exposed flat CST nodes the legacy walker had to
disambiguate by string comparison or sibling lookup. Each divergence
below is justified from the metric definition rather than from a
desire to mirror legacy. `crates/mehen-ruby/tests/` carries the pinned
snapshots; the 23 legacy `check_metrics::<RubyParser>` tests are
ported byte-identical (modulo the two intentional-drift items in §3),
and seven new Ruby-specific tests pin the prism-only behavior.

### 1.1 Why `ruby-prism` and not the alternatives

- **`tree-sitter-ruby` (the legacy backend)**: a CST grammar that
  treats modifier and block forms as separate node kinds (`if` /
  `if_modifier`, `unless` / `unless_modifier`, `rescue` /
  `rescue_modifier`, `while` / `while_modifier`, `until` /
  `until_modifier`). Convenient — every form has its own visit hook —
  but the grammar lags Ruby releases, mis-parses some valid 3.2+ syntax
  (numbered params, `it` blocks, endless methods, hash shorthand), and
  classifies operator-method calls (`a + b`) and real method dispatch
  (`obj.foo()`) under the same `call` node, forcing the legacy walker
  to inspect raw operator-token text to tell ABC.B (branch) apart from
  Halstead operator emission.
- **`lib-ruby-parser`**: a Rust port of the now-superseded
  `parser-rs`. Smaller surface, no longer canonical (Prism replaced it
  inside CRuby itself), and missing the recent syntax forms.
- **Hand-written nom parser**: the rewrite plan §6.8 explicitly calls
  this out as a bad use of `nom` — replacing a mature language parser.
- **`ruby-prism`**: typed AST built on Prism's C parser via
  `bindgen`-generated FFI. 156-method `Visit<'pr>` trait
  auto-generated from upstream's `config.yml`, mirroring the
  ruff/mago architecture (override the metric-bearing nodes; call
  the matching `visit_<name>_node(self, node)` free function to keep
  walking). Tracks every Ruby release because it IS the Ruby parser.

### 1.2 License audit (plan §6.5)

- `ruby-prism` (Rust crate): MIT — Steve Loveless, Ian Ker-Seymer,
  Kevin Newton.
- `ruby-prism-sys` (Rust FFI crate): MIT.
- Bundled upstream Prism C parser (vendored under
  `ruby-prism-sys/vendor/`): MIT — Shopify Inc., 2022–present.

All three layers are permissive, no copyleft, compatible with mehen's
licensing. The `vendor/` directory is shipped with the published
`ruby-prism-sys` crate, so the build never fetches Ruby source over
the network.

### 1.3 Build prerequisites (plan §6.5)

`ruby-prism-sys` invokes `bindgen 0.72` _unconditionally_ in its
`build/main.rs` (regenerating Rust bindings against the vendored Prism
C headers on every build), and compiles the vendored Prism sources via
`cc 1.0` when its default `vendored` feature is on. That means every
target that builds mehen from source needs:

- a working `libclang` available at build time (for `bindgen`);
- a C compiler (`clang` / `gcc` / `cl`) on `PATH` (for `cc`).

End users who install the release `mehen` binary do NOT need either.
The CI/release matrix does — see the workspace `Cargo.toml`'s pin
comment for the per-platform notes (Linux glibc/musl, macOS, Windows).

## 2. What is _not_ different

The following metric outputs match legacy where the underlying source
is well-formed Ruby:

- `cyclomatic.{sum,min,max,avg}` for `if`/`elsif`/`unless`,
  `while`/`until`, `for`, `case`/`when`, `case`/`in`, `rescue` (block
  - modifier), `&&`/`||`/`and`/`or`, conditional `?:`, every modifier
    form (`x if y`, `x unless y`, `x while y`, `x until y`, `expr rescue
fallback`).
- `cognitive.{sum,min,max,avg}` — nesting bumps for control-flow
  scopes, +1 (no nesting) for modifiers / `else` / `elsif`, the
  same boolean-sequence collapser, depth-on-method-nesting, and
  lambda-tracking rules.
- `abc.{assignments,branches,conditions,magnitude,...}` — every
  written form (`=`, `+=`, `&&=`, `||=`) counts assignment, every
  real method call counts branch, every comparison op + control-flow
  predicate counts condition.
- `halstead.{n1,N1,n2,N2,length,vocabulary}` — operator/operand
  classification still emits one operator entry per keyword and
  operator-method call, one operand entry per identifier / variable /
  literal.
- `loc.{sloc,ploc,lloc,cloc,blank}` — comment and code-line accounting
  preserved; LLOC bumped on the same set of statement-shaped nodes
  (`def`, `class`, `module`, `if`, `unless`, `while`, `until`, `for`,
  `case`, modifiers, `return`, `break`, `next`, `redo`, `yield`,
  every assignment, every call).
- `nargs.{total,average,min,max}` per function and per closure.
- `nom.{functions,closures,total,average,min,max}` — every `def` and
  `def self.foo` is a function, every block (`do…end` / `{ … }`) and
  lambda (`->{}` / `lambda { }`) is a closure (with the legacy
  exception: a block whose direct parent is a lambda is the lambda
  body, not a separate closure).
- `nexit.{sum,min,max,avg}` — `return`, `break`, `next`, `redo` count;
  `yield` does NOT (it hands off, it doesn't exit the method).
- `npa.{class_attributes,total_attributes,...}` — class-body `@x =`
  ivar assignments count as non-public attributes (Ruby ivars are
  non-public by convention; `attr_reader` exposure is out of scope).
- `npm.{class_methods,total_methods,...}` — every `def` inside a
  class / module / singleton-class body is a method, all counted as
  public (Ruby `private` / `protected` are runtime calls; without
  semantic flow analysis we treat every `def` as public, matching the
  legacy walker's default).
- `wmc.{classes,interfaces,total}` — class WMC sums each method's
  cyclomatic.
- `mi.{*}` — derived from the above; unchanged.

## 3. What _is_ different

Each item below is a deliberate divergence from the legacy walker.
They fall into two buckets:

- **§3.1–3.6**: parity-preserving on the metric _definition_; the
  difference is only that prism's typed AST exposes the underlying
  fact more cleanly than tree-sitter's CST. Numeric output is
  byte-identical to legacy.
- **§3.7–3.8**: intentional drift — same behavioural change adopted in
  Phase 6 (Python) and Phase 8 (PHP) when mehen-metrics' `State`
  helpers landed. Documented here for completeness.

### 3.1 Modifier-form detection via location options, not separate node kinds

`tree-sitter-ruby` had `if` / `if_modifier`, `unless` /
`unless_modifier`, `while` / `while_modifier`, `until` /
`until_modifier` as **distinct node kinds**. The legacy walker
matched each pair separately.

Prism collapses each pair into a single AST struct (`IfNode`,
`UnlessNode`, `WhileNode`, `UntilNode`) and distinguishes block from
modifier form via the absence of `end_keyword_loc` / `closing_loc`:

| Form                                 | `end_keyword_loc` / `closing_loc` |
| ------------------------------------ | --------------------------------- |
| `if y; x; end` (block)               | `Some(loc)`                       |
| `x if y` (modifier)                  | `None`                            |
| `a ? b : c` (ternary, only `IfNode`) | `if_keyword_loc.is_none()`        |

Metric output is identical (we still emit +1 cyclomatic for each
form, +1 cognitive without nesting for modifier and ternary, +1+nesting
for block forms). The only change is _how_ the walker classifies.

### 3.2 `RescueModifierNode` is a distinct node kind in prism

Ruby's `expr rescue fallback` postfix form. Both tree-sitter-ruby and
prism expose it as a separate node from the block-form `RescueNode`
inside a `BeginNode`. The legacy walker had `RescueModifier`,
`RescueModifier2`, `RescueModifier3` arms (tree-sitter generated
numbered duplicates for ambiguous grammar paths); prism collapses to
just `RescueModifierNode`. Metric output unchanged.

### 3.3 Operator-method calls vs real method dispatch

In Ruby `a + b` parses (semantically) as a method call `a.+(b)` —
both tree-sitter-ruby and prism expose this through their generic
"call"-shaped node. The legacy walker disambiguated via the parent
node kind (`binary` for arithmetic/comparison, `call`/`command` for
real dispatch).

Prism uses a single `CallNode` for both forms, distinguished by the
method name. The walker checks `CallNode::name()`:

- If the name is in `is_ruby_operator_method` (`+`, `-`, `*`, `/`,
  `%`, `**`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `<=>`, `===`, `<<`,
  `>>`, `&`, `|`, `^`, `~`, `!`, `+@`, `-@`, `[]`, `[]=`): emit a
  Halstead `call_op` operator only. Do NOT count ABC.B.
- If the name is a comparison method (`==`, `!=`, `<`, `>`, `<=`,
  `>=`, `<=>`, `===`): also emit ABC.C (matches legacy `binary`-arm
  classification).
- Otherwise: real method dispatch — emit ABC.B (branch), record the
  method name as a Halstead operand.

Numeric output matches legacy.

### 3.4 Op-write families are distinct typed nodes

Tree-sitter-ruby parses every `=` / `+=` / `&&=` / `||=` / `*=` / `<<=`
as a single `assignment` or `operator_assignment` node with an
operator-token child. Prism splits each into its own typed node:
`LocalVariableWriteNode`, `LocalVariableOperatorWriteNode`,
`LocalVariableAndWriteNode`, `LocalVariableOrWriteNode` — and the
same five-shape split for `instance`, `class`, `global`, `constant`,
`constant_path`, `call`, `index`. The walker can dispatch directly
without inspecting the operator field.

`*AndWriteNode` (`x &&= 1`) and `*OrWriteNode` (`x ||= 1`) ALSO count
as ABC.C (condition) and +1 cyclomatic — they are short-circuit
operators that introduce a branch. This matches the legacy walker's
treatment of `&&=` / `||=` via the `binary | unary` ABC.C arm.

### 3.5 Halstead derivation is AST-driven (not token-driven)

ruby-prism does NOT expose a public token stream — `pm_token_t` and
`pm_lex_*` symbols are not allowlisted by `ruby-prism-sys`'s bindgen
build. So unlike Phase 6 (Ruff `TokenKind` sweep) and Phase 8 (Mago
`Lexer`), Halstead must derive from the AST.

The legacy tree-sitter walker also derived Halstead from the CST (no
flat token stream there either) — it visited every CST node and
classified by `kind_id`. We do the same with prism: dedicated
`visit_*_node` hooks emit Halstead operators for keywords / structural
punctuation / call-operators, and `visit_local_variable_read_node` /
`visit_constant_read_node` / `visit_required_parameter_node` /
`visit_integer_node` / `visit_string_node` / `visit_true_node` / etc.
emit operands. Numeric output (`n1`, `n2`, `N1`, `N2`) matches the
legacy snapshot.

### 3.6 PLOC keyword-line accounting

Tree-sitter-ruby's grammar surfaces `end`, `do`, `then`, `=>`, and
similar keyword tokens as anonymous-keyword child nodes. The legacy
`Loc::compute` `_` arm therefore inserted those tokens' start_row
into `ploc.lines`, so a one-line-per-keyword `end` contributed to
PLOC even though it has no semantic content beyond block termination.

Prism does NOT expose keyword tokens as separate AST nodes — they
live as `Option<Location>` fields on the parent (e.g.
`DefNode::end_keyword_loc()`). To preserve PLOC parity (`end` IS a
physical line of code per the SLOC/PLOC definition), the walker
explicitly observes `code_line` for those keyword locations from the
parent's visit hook (see `Visitor::observe_keyword_line` /
`observe_optional_keyword_line`). Numeric output matches legacy.

### 3.7 Empty-aggregate average serializes as `0.0`, not `null`

**Intentional drift, shared with Phase 6 (Python) and inherited from
the 1.0 mehen-metrics design.**

When a Ruby source has no functions (e.g. `a = 42` at the unit
level), the legacy walker emitted `"average": null` for `cognitive`
and `nexit` (zero denominator → JSON null). The 1.0 mehen-metrics
`CognitiveStats::finalize` and `NexitStats::finalize` default to
`0.0` instead, matching the `as_f64` helper used by every other
language crate. The Phase-6 `python_no_cognitive` test pinned this
convention; we follow it for Ruby.

### 3.8 `nargs.*_min` no longer dilutes via the unit space

**Intentional drift, shared with Phase 6 (Python) and Phase 8 (PHP).**

The legacy walker's `compute_minmax` ran _unconditionally_ for every
space — so the unit space's `fn_nargs = 0` always pulled the
`functions_min` floor down to 0, even when every actual function in
the file had >0 args. The 1.0 mehen-metrics `NargsStats::finalize_minmax`
gates the function bounds on `is_function == true`, so the unit no
longer participates. Result: for `def f(a, b)\n  a + b\nend`,
`functions_min` is `2.0` (the only function's arg count), not `0.0`.

This is a parity-improvement, not a regression: the legacy zero was
spurious. The Phase-6 `python_single_function` test pinned this
convention; we follow it for Ruby.

## 4. New tests pinned by Phase 9

`crates/mehen-ruby/tests/parity.rs` exercises Ruby idioms the legacy
fixture set didn't cover. Each test pins prism-specific behavior we
expect to hold long-term:

- `ruby_safe_navigation_does_not_crash` — `obj&.bar&.baz` parses, two
  ABC.B branches recorded.
- `ruby_pattern_matching_each_in_branch_is_a_decision` — `case x; in
pat => g; …; end` adds one cyclomatic per `in` clause + one for the
  `if` guard.
- `ruby_endless_method_definition` — Ruby 3.0 `def square(x) = x * x`
  counts as one method (`DefNode::end_keyword_loc().is_none()` for
  endless methods; we still record NOM=1).
- `ruby_numbered_block_parameters_count_correctly` — `do; puts _1 +
_2; end` recovers arity 2 from `NumberedParametersNode::maximum`.
- `ruby_singleton_class_body_contributes_to_class_metrics` — `class
<< self; def …; end; end` opens a class-like space; methods inside
  count toward NPM.
- `ruby_modifier_if_does_not_increase_nesting` — two sibling `x if y`
  modifier statements each contribute +1 cognitive; they do NOT
  collapse into a nested-if pattern.
- `ruby_op_assignment_writes_count_as_assignment_and_decision` —
  `x &&= 1` and `x ||= 2` each contribute one ABC.A + one ABC.C +
  one cyclomatic decision.

## 5. Walker structure

`crates/mehen-ruby/src/walker.rs` drives recursion through prism's
[`Visit<'pr>`](https://docs.rs/ruby-prism/latest/ruby_prism/trait.Visit.html)
trait — the auto-generated 156-method visitor mirroring the same
shape as `mago_syntax::walker::Walker` (Phase 8 PHP) and
`ruff_python_ast::visitor::source_order::SourceOrderVisitor` (Phase 6
Python). We override the metric-bearing hooks, call the matching
`visit_<name>_node(self, node)` free function to descend, and use
`visit_branch_node_enter` / `visit_leaf_node_enter` for the
PLOC-line-set sweep (mirrors the legacy `_` arm of
`Loc::compute`).

The walker follows the same per-space `State` accumulator pattern
(`mehen-metrics::state`):

- One `State` for the unit, plus one for every opened
  function / closure / class space.
- Cyclomatic / cognitive / ABC / nexit / LOC / NPA / NPM are driven
  by the per-shape overrides.
- Halstead is unit-level only — every operator/operand observation
  goes to `self.stack[0].halstead`.
- `CognitiveContext` tracks `(nesting, depth, lambda)` exactly as the
  legacy `cognitive::RubyCode` did.
- `inside_lambda_body` flag suppresses the per-block lambda bump for
  a `BlockNode` whose immediate parent is a `LambdaNode` (the lambda
  body is not a separate closure).

`ParseResult<'pr>` is `!Send + !Sync`, so the analyzer parses + walks

- collects metrics in one stack frame and discards the parse result
  before returning `LanguageAnalysis` (which is `Send + 'static`).

## 6. References

- Pinned ruby-prism revision: `1.9.0` on crates.io.
- Prism upstream: <https://github.com/ruby/prism>.
- ruby-prism docs: <https://docs.rs/ruby-prism/1.9.0/ruby_prism/>.
- `Visit` trait usage example: `lib.rs::tests::visitor_test` in the
  ruby-prism source distribution.
- Plan §6.5 — Ruby and Prism rationale.
- Plan §12.3.1 — parity contract.
- Migration commit: see `git log --oneline | grep 'phase-9'` for the
  patch series.
