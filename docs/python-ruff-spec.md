# Python analyzer spec — Ruff backend

**Status:** implementation reference
**Date:** 2026-05-18
**Scope:** `mehen-python` Ruff-backed analyzer (Phase 6 of the v1
rewrite, replacing the tree-sitter-python pipeline)

## 1. Goal

Phase 6 of the [v1 rewrite plan](mehen-1-0-from-scratch-rewrite-plan.md)
swaps the Python parser from `tree-sitter-python` to Ruff's
unpublished `ruff_python_parser` + `ruff_python_ast` + `ruff_text_size`
crates (pinned to git tag `0.15.13`). The analyzer (in
`crates/mehen-python/`) feeds the same `mehen-metrics` accumulators as
every other language crate, so the published metric contract is
unchanged — Halstead, Cyclomatic, Cognitive, ABC, NArgs, NOM, NExit,
NPA, NPM, WMC, LOC, MI all keep their semantics and serialization
shape.

What changes is the *interpretation* of a handful of Python-specific
constructs where the tree-sitter grammar's CST shape produced
demonstrably wrong or weakly-grounded results. Each divergence below
is justified from the metric definition rather than from a desire to
mirror legacy. `mehen-python/tests/` carries the pinned snapshots.

## 2. What is *not* different

The following metric outputs match legacy byte-for-byte where the
underlying source is well-formed:

- `cyclomatic.{sum,min,max,avg}` for `if/elif/else`, `for`, `while`,
  `try`/`except`, `match`/`case`, `and`/`or`, ternary `a if b else c`,
  comprehension generators and `if` filters.
- `cognitive.{sum,min,max,avg}` for nesting penalties, the boolean
  sequence collapser, lambda lambda-bonus, function-depth penalty.
- `nom.*` for function and lambda counts.
- `nexit.*` for `return` / `raise`.
- `abc.*` for assignments, calls, comparisons, conditionals.
- `loc.*` for blank/comment/code lines, including docstring-as-cloc.
- `npa.*` / `npm.*` for class-body assignments and methods, with the
  PEP-8 leading-underscore visibility convention (dunders count as
  public).
- `wmc.*` summed from method cyclomatic.
- `mi.*` Maintainability Index variants.
- The `embedded_code_large.md` markdown fence — the Python fence's
  `volume`, `cognitive_sum`, and `sloc` are byte-identical to the
  legacy walker (volume=361.21, cognitive=8, sloc=17), so the §9.4
  `embedded_volume = Σ 0.20·√volume + 0.50·cognitive + 0.10·sloc`
  rollup is unchanged for that fixture.

## 3. What is different (and why)

### 3.1 Type annotations participate in Halstead

**TypeScript precedent doesn't apply to Python.** The Phase 7
TypeScript walker (`crates/mehen-typescript/`) deliberately excludes
TS-only AST subtrees (`TSTypeAnnotation`, `TSInterfaceDeclaration`,
class `implements` clauses, predefined-type keywords, …) from the
Halstead token sweep, because those tokens are erased at compile
time. Python types are not erased — they are runtime-accessible
objects:

- `typing.get_type_hints(f)` returns a dict of evaluated annotation
  objects.
- `pydantic` reads model annotations at class-definition time to
  build validators.
- `dataclasses` reads field annotations to generate `__init__`.
- `inspect.signature(f).parameters[name].annotation` is the
  annotation expression.

Per Halstead's "operators do things, operands are things" definition,
an annotation like `int` or `list[Shape]` is a *thing* — a real
runtime operand. The Ruff walker therefore treats annotation tokens
exactly like any other expression token: `:` and `->` are operators,
the type identifier is an operand. This is a deliberate divergence
from the TS analyzer.

Test: `crates/mehen-python/tests/parity.rs::python_type_annotations_participate_in_halstead`.

### 3.2 Module / class / function docstrings are excluded from Halstead

Per PEP 257, a string literal that is the first statement of a module,
class, or function body is the docstring — a structural language
feature, not arbitrary code. Docstring tokens contribute zero
Halstead operators and operands.

LOC accounting still counts those lines as `cloc` (matching the legacy
behavior), since the `Loc` metric's "cloc = comment-like lines"
definition includes docstrings.

Test: `crates/mehen-python/tests/parity.rs::python_module_docstring_excluded_from_halstead`.

### 3.3 Attribute access does not emit a synthetic "attribute" operand

The TypeScript walker emits *three* operand entries for `console.log`:
the leaf `console`, the leaf `log`, and the joined wrapper
`MemberExpression("console.log")`. This is parity with the legacy
tree-sitter-typescript walker (which counted `member_expression` as
a named-CST operand).

The legacy tree-sitter-python walker does NOT classify the `attribute`
named node as an operand (see the deleted `Getter for PythonCode`
match arms — `Identifier`, `Integer`, `Float`, `String`, `True`,
`False`, `None` are the only operand forms). The new Ruff walker
follows this: `a.b.c` produces three operand tokens (`a`, `b`, `c`)
plus two operator dots, no synthetic chain operand.

The two languages differ here because their legacy walkers differed,
and each language's chosen convention leaves its own metric output
internally consistent. Cross-language comparisons of Halstead numbers
were never first-class (each language has its own operator/operand
classification anyway).

### 3.4 NArgs `*_min` is per-function/closure, not per-space

Legacy `nargs::compute_minmax` folded *every* space's per-space
`fn_nargs` and `closure_nargs` (defaulting to 0 for unit/class spaces)
into the rolled-up min. That meant any source with at least one
function and a unit space reported `functions_min: 0.0` — never
`functions_min: 2.0` even when the only function had 2 args.

The new `NargsStats::finalize_minmax` gates the per-space fold on
`is_function` and `is_closure`. Only spaces that *are* functions or
closures contribute to the corresponding `_min` / `_max`. The legacy
output for `def f(a, b): ...` was `functions_min: 0.0`; the new
output is `functions_min: 2.0` — matching the metric's intended
definition ("minimum number of arguments across function spaces").

This change touches more than Python: PowerShell's
`crates/mehen-powershell/tests/nargs.rs` snapshots were updated to
the new (more correct) values too. See the doc-comment at the top of
that file.

Test: `crates/mehen-python/tests/nargs.rs::python_single_function`.

### 3.5 Indentation correctness

Tree-sitter-python is error-tolerant — it silently absorbs
inconsistent indentation rather than reporting a parse error. The
legacy Python tests in
`crates/mehen-engine/src/legacy/metrics/{cyclomatic,nargs,...}.rs`
were written with deeply-indented multi-line source strings where
each `def` / `if` ended up at a different column. Tree-sitter
"smoothed over" the inconsistency; Ruff (correctly) reports
indentation errors and emits an empty / partial AST.

Where the ported tests in `crates/mehen-python/tests/` use the
legacy fixture, the source has been reformatted to consistent
indentation that expresses what the legacy test was *intending* to
test (siblings at column 0, body at 4 spaces). The expected values
are unchanged — the AST shape is what the legacy test was actually
trying to validate.

Test inline-comments document each such reformatting.

### 3.6 PEP 654 exception groups (`try*` / `except*`)

Python 3.11 added `except*` for exception groups. Tree-sitter-python's
0.25.0 grammar may or may not parse `except*` correctly (it's a
recent addition). Ruff supports it.

Each `except*` handler counts as a regular `except` for cyclomatic
(one decision) and cognitive (nesting + 1). The
`StmtTry::is_star: bool` flag is recorded as evidence (future
contribution-reason output) but does not change metric numerics.

Test: `crates/mehen-python/tests/parity.rs::python_except_star_handler_counts_as_decision`.

## 4. Operator and operand classification

The Ruff walker maps Ruff `TokenKind` values to one of `Operator(&str)`,
`Operand(&str)`, or `Skip`. The mapping in
`crates/mehen-python/src/walker.rs::classify_token` covers:

- All Python keywords (`if`, `elif`, `else`, `for`, `while`, `try`,
  `except`, `finally`, `with`, `return`, `raise`, `yield`, `assert`,
  `import`, `from`, `as`, `pass`, `break`, `continue`, `def`, `class`,
  `lambda`, `in`, `is`, `async`, `await`, `global`, `nonlocal`, `del`,
  `not`, `and`, `or`) → operators.
- Soft keywords `match`, `case`, `type` → operators.
- Punctuation: `( [ {  , : ; . @ + - * / % | & ^ ~ ** // << >> < >
   = == != <= >= += -= *= /= %= &= |= ^= **= //= <<= >>= := @= ->`
  → operators.
- Closing punctuation `) ] }` → skip (paired with their opening
  counterpart, which is the operator).
- `Name` → `Identifier` operand.
- `Int`, `Float`, `Complex` → `Number` operand.
- `String`, `FStringStart`, `FStringMiddle`, `FStringEnd`,
  `TStringStart`, `TStringMiddle`, `TStringEnd` → `String` operand.
- `True`, `False`, `None`, `Ellipsis` → operand.
- Newlines, indents, dedents, comments, EOF → skip.

`PrivateIdentifier` is not in Python — the `_internal` / `__name`
naming conventions are not lexer-level distinctions. Visibility for
NPA/NPM is computed at the AST level by examining the `Identifier.id`
text against the leading-underscore rule.

## 5. Walker structure

The walker in `crates/mehen-python/src/walker.rs` follows the same
per-space `State` accumulator pattern used by `mehen-typescript`:

- One `State` (in `mehen-metrics::state`) for the unit, plus one for
  every opened function / closure / class space.
- Cyclomatic / cognitive / ABC / nexit / LOC / NPA / NPM are driven
  per-statement via `visit_stmt` and per-expression via `visit_expr`.
- Halstead is driven by a post-AST token sweep over the parsed
  module's `parsed.tokens()`. Each token maps to one of
  `Operator(kind)`, `Operand(kind)`, or `Skip`. Tokens whose span
  falls inside a recorded docstring are skipped.

The `CognitiveContext` tracks `(nesting, depth, lambda)` exactly as
the pre-1.0 `cognitive::python` did, plus a `bool_op_depth` counter
that detects the *outermost* boolean operator inside a statement
(only that one gets the legacy lambda-ancestor bonus per
`mehen-engine/src/legacy/metrics/cognitive.rs:281`'s
`count_specific_ancestors` of Lambda boundaries).

## 6. References

- Pinned Ruff revision: `https://github.com/astral-sh/ruff` tag
  `0.15.13` (commit `2afb467ce397e4a89c13a0a814c62cfecb0e9e49`).
- Ruff's `parse_module` returns a
  `Parsed<ModModule>` with `.syntax()` (the AST), `.tokens()` (the
  lexer stream), and `.errors()` (parse diagnostics).
- The walker uses the AST for structural metrics and the lexer
  stream for Halstead, exactly mirroring the TypeScript walker's
  Oxc-based approach.
- Migration commit: see `git log --oneline | grep 'phase-6\|Ruff'`
  for the patch series.
