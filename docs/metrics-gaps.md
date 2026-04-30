# Metrics Implementation Gaps

Audit of per-language metric completeness across Rust, Python, Go, TypeScript, TSX, and Ruby. Ordered from most challenging (full redesign / missing primitives) to trivial (add a node kind to a match arm).

Supported languages in the matrix: R = Rust, Py = Python, G = Go, TS = TypeScript, X = TSX, Rb = Ruby.

## 1. Class-based metrics

- [ ] **`wmc` (Weighted Methods per Class) is a no-op for every language.** `src/metrics/wmc.rs:117` registers all six languages through `implement_metric_trait!(Wmc, …)`, which hits the fallback arm in `src/macros.rs:45` and emits an empty `compute`. Requires (a) iterating methods inside a class space, (b) summing each method's cyclomatic, (c) a per-language predicate for "is this node a method of the current class." Applies to R / Py / TS / X / Rb; n/a for Go.
- [ ] **`npa` (Number of Public Attributes) is a no-op for every language.** `src/metrics/npa.rs:181`, same fallback-arm stub. Needs per-language detection of class attribute declarations and a visibility rule (Rust `pub`, Python `_` convention, TS `public`/default, Ruby `attr_accessor`/ivars). Applies to R / Py / TS / X / Rb.
- [ ] **`npm` (Number of Public Methods) is a no-op for every language.** `src/metrics/npm.rs:181`, same fallback-arm stub. Needs per-language class-method detection plus the same visibility rule as `npa`. Applies to R / Py / TS / X / Rb.

## 2. Structural gaps in existing metrics

- [ ] **`cognitive` does not nest `TryStatement` in TypeScript / TSX.** `js_cognitive!` macro at `src/metrics/cognitive.rs:358` increments nesting on `CatchClause` but not on the surrounding `TryStatement`, so code inside `try` blocks is not counted as nested. Fix requires adding `TryStatement` to the `increase_nesting` arm in both TS and TSX impls.
- [ ] **`cognitive` does not nest `TryStatement` in Python.** `src/metrics/cognitive.rs:250` handles `ExceptClause` but not `TryStatement` itself — the `try` body does not add nesting depth.
- [ ] **`cognitive` does not handle `LoopExpression` or `TryExpression` in Rust.** `src/metrics/cognitive.rs:319` nests on `IfExpression | ForExpression | WhileExpression | MatchExpression` but omits `LoopExpression` (infinite loops) and `TryExpression` (`?`).
- [ ] **`exit` misses `throw` in TypeScript / TSX.** `src/metrics/exit.rs:126` and `:134` only count `ReturnStatement`. Add `ThrowStatement` to reach parity with Rust's treatment of `TryExpression` as an exit point.
- [ ] **`exit` misses `raise` in Python.** `src/metrics/exit.rs:118` only counts `ReturnStatement`. Add `RaiseStatement`.

## 3. Language-semantic inconsistencies

- [ ] **`halstead` under-classifies Python operators.** `src/getter.rs:60` omits `LPAREN | LBRACK | LBRACE | COLON | SEMI` from the operator set, while TS / TSX / Rust / Go / Ruby all classify these as operators. Python call-parens and indexing brackets therefore do not contribute to N/n, depressing Python's Halstead volume vs. other languages.
- [ ] **`cyclomatic` counts Go `DefaultCase` as a decision.** `src/metrics/cyclomatic.rs:172` includes `DefaultCase` in the decision-point set; by the standard McCabe definition `default` is the fallthrough and should not count. This inflates Go cyclomatic relative to other languages.
- [ ] **`cyclomatic` counts Python `With | Assert` as decisions.** `src/metrics/cyclomatic.rs:113` adds `With` and `Assert`, neither of which introduces a branch. This inflates Python cyclomatic relative to other languages.
- [ ] **`cyclomatic` does not count `do…while` in TypeScript / TSX.** `src/metrics/cyclomatic.rs:133` and `:146` enumerate `If | For | While | Case | Catch | TernaryExpression | AMPAMP | PIPEPIPE` but omit `DoStatement`. `while` matches through the `While` token but `do…while` has its own kind and is silently dropped.

## 4. Stubbed predicates feeding real code

- [ ] **`is_primitive` returns `false` for Python, Go, and Ruby.** `src/checker.rs:157`, `:398`, `:484`. Consumed at `src/ops.rs:78` and `:194` to filter primitive type tokens out of the operand output. For these three languages, built-in type identifiers leak into the `ops` output as operands. Rust, TS, TSX implement it against their `PredefinedType` / `PrimitiveType` kinds.
- [ ] **`is_useful_comment` is `false` for TS / TSX / Go / Ruby.** `src/checker.rs:167`, `:226`, `:351`, `:408`. Consumed at `src/comment_rm.rs:21`. Only Python (coding declarations) and Rust (`/// cbindgen:`) preserve meaningful comments during stripping — other languages drop all comments uniformly, even ones that carry metadata.
- [ ] **`is_else_if` returns `false` for Python.** `src/checker.rs:153`. Python's grammar has a dedicated `ElifClause` so this is likely harmless, but the default means `node.has_ancestors` at `src/node.rs:135` never treats an ancestor `If` as else-if-skippable. Worth verifying with a test rather than leaving as a silent default.

## 5. Trivial alignments

- [ ] **Rust `cognitive` comment in `src/metrics/cognitive.rs:316` says `//TODO: Implement macros`.** Macro invocations are currently invisible to cognitive complexity in Rust. If macro bodies are meant to count, this needs grammar-level handling.
- [ ] **TS / TSX `cyclomatic` relies on `For` keyword matching for `for…of`/`for…in`.** `src/metrics/cyclomatic.rs:138`, `:151`. Confirm `For` token fires for all three loop kinds; if not, add `ForInStatement` / `ForOfStatement` explicitly.

---

## Summary matrix

| Metric       | R    | Py   | G    | TS   | X    | Rb   |
|--------------|------|------|------|------|------|------|
| loc          | full | full | full | full | full | full |
| halstead     | full | partial (§3) | full | full | full | full |
| cyclomatic   | full | partial (§3) | partial (§3) | partial (§3) | partial (§3) | full |
| cognitive    | partial (§2) | partial (§2) | full | partial (§2) | partial (§2) | full |
| nargs        | full | full | full | full | full | full |
| nom          | full | full | full | full | full | full |
| mi           | full | full | full | full | full | full |
| exit         | full | partial (§2) | full | partial (§2) | partial (§2) | full |
| abc          | full | full | full | full | full | full |
| wmc          | noop | noop | n/a  | noop | noop | noop |
| npa          | noop | noop | n/a  | noop | noop | noop |
| npm          | noop | noop | n/a  | noop | noop | noop |
