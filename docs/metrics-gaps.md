# Metrics Implementation Gaps

Audit of per-language metric completeness across Rust, Python, Go, TypeScript, TSX, and Ruby. Ordered from most challenging (full redesign / missing primitives) to trivial (add a node kind to a match arm).

Supported languages in the matrix: R = Rust, Py = Python, G = Go, TS = TypeScript, X = TSX, Rb = Ruby.

## 1. Class-based metrics

- [x] **`wmc` (Weighted Methods per Class).** Implemented for R / Py / TS / X / Rb. The method's function-space forwards its cyclomatic into the merge routine, which accumulates it into the enclosing class (or `impl`) / interface (or `trait`). Unit-level aggregation hides the metric when a file contains no class-like spaces. Go stays `n/a`.
- [x] **`npa` (Number of Public Attributes).** Implemented for R / Py / TS / X / Rb. Per-language attribute detection: Python class-body assignments, TS/TSX `public_field_definition` / `property_signature`, Rust `field_declaration`, Ruby `@instance_variable` assignments. Visibility rule: leading-`_` for Python, `private` / `protected` modifiers for TS / TSX, `pub` for Rust, conservative non-public for Ruby ivars (since `attr_accessor` tracking is out of scope). Go stays `n/a`.
- [x] **`npm` (Number of Public Methods).** Implemented for R / Py / TS / X / Rb. Detects methods by node kind + AST-parent check, so `space_kind = Function` of the method's own space does not confuse the lookup. Visibility rule matches `npa`. Go stays `n/a`.

## 2. Structural gaps in existing metrics

- [x] **`cognitive` does not nest `TryStatement` in TypeScript / TSX.** `js_cognitive!` macro at `src/metrics/cognitive.rs:358` increments nesting on `CatchClause` but not on the surrounding `TryStatement`, so code inside `try` blocks is not counted as nested. Fix requires adding `TryStatement` to the `increase_nesting` arm in both TS and TSX impls.
- [x] **`cognitive` does not nest `TryStatement` in Python.** `src/metrics/cognitive.rs:250` handles `ExceptClause` but not `TryStatement` itself — the `try` body does not add nesting depth.
- [x] **`cognitive` does not handle `LoopExpression` or `TryExpression` in Rust.** `src/metrics/cognitive.rs:319` nests on `IfExpression | ForExpression | WhileExpression | MatchExpression` but omits `LoopExpression` (infinite loops) and `TryExpression` (`?`).
- [x] **`exit` misses `throw` in TypeScript / TSX.** Fixed: `src/metrics/exit.rs` now counts `ThrowStatement` alongside `ReturnStatement`.
- [x] **`exit` misses `raise` in Python.** Fixed: `src/metrics/exit.rs` now counts `RaiseStatement` alongside `ReturnStatement`.

## 3. Language-semantic inconsistencies

- [x] **`halstead` under-classifies Python operators.** Fixed: `LPAREN | LBRACK | LBRACE | COLON | SEMI` are now classified as Python operators, bringing it in line with TS / TSX / Rust / Go / Ruby. The Python `get_operator_id_as_str` now uses the shared `get_operator!` macro so `()` / `[]` / `{}` render correctly in the `ops` output.
- [x] **`cyclomatic` counts Go `DefaultCase` as a decision.** `src/metrics/cyclomatic.rs:172` includes `DefaultCase` in the decision-point set; by the standard McCabe definition `default` is the fallthrough and should not count. This inflates Go cyclomatic relative to other languages.
- [x] **`cyclomatic` counts Python `With | Assert` as decisions.** `src/metrics/cyclomatic.rs:113` adds `With` and `Assert`, neither of which introduces a branch. This inflates Python cyclomatic relative to other languages.
- [x] **`cyclomatic` does not count `do…while` in TypeScript / TSX.** `src/metrics/cyclomatic.rs:133` and `:146` enumerate `If | For | While | Case | Catch | TernaryExpression | AMPAMP | PIPEPIPE` but omit `DoStatement`. `while` matches through the `While` token but `do…while` has its own kind and is silently dropped. — Verified: the `While` keyword token fires for `do…while` as well (see `typescript_do_while` test), so no enumeration change is needed.

## 4. Stubbed predicates feeding real code

- [ ] **`is_primitive` returns `false` for Python, Go, and Ruby.** `src/checker.rs:157`, `:398`, `:484`. These languages have no dedicated "primitive type" AST node; distinguishing `int` / `str` / `bool` from user types would require name-based matching, which needs a signature change to pass node text. Left as-is; only affects the cosmetics of the `ops` output.
- [ ] **`is_useful_comment` is `false` for TS / TSX / Go / Ruby.** `src/checker.rs:167`, `:226`, `:351`, `:408`. Consumed at `src/comment_rm.rs:21`. Only Python (coding declarations) and Rust (`/// cbindgen:`) preserve meaningful comments during stripping — other languages drop all comments uniformly, even ones that carry metadata. Left as-is pending a concrete use-case.
- [x] **`is_else_if` returns `false` for Python.** Verified with `metrics::cognitive::tests::python_nested_if_in_else_is_not_else_if`: Python's dedicated `ElifClause` means a plain `if` in an `else:` block is a real nested `if`, so returning `false` here is correct.

## 5. Trivial alignments

- [ ] **Rust `cognitive` comment in `src/metrics/cognitive.rs:316` says `//TODO: Implement macros`.** Macro invocations are currently invisible to cognitive complexity in Rust. If macro bodies are meant to count, this needs grammar-level handling.
- [x] **TS / TSX `cyclomatic` relies on `For` keyword matching for `for…of`/`for…in`.** `src/metrics/cyclomatic.rs:138`, `:151`. Confirm `For` token fires for all three loop kinds; if not, add `ForInStatement` / `ForOfStatement` explicitly. — Confirmed via the `typescript_do_while` pattern: the `For` / `While` anonymous keyword tokens fire uniformly for all loop kinds. No change required.

---

## Summary matrix

| Metric       | R    | Py   | G    | TS   | X    | Rb   |
|--------------|------|------|------|------|------|------|
| loc          | full | full | full | full | full | full |
| halstead     | full | full | full | full | full | full |
| cyclomatic   | full | full | full | full | full | full |
| cognitive    | full | full | full | full | full | full |
| nargs        | full | full | full | full | full | full |
| nom          | full | full | full | full | full | full |
| mi           | full | full | full | full | full | full |
| exit         | full | full | full | full | full | full |
| abc          | full | full | full | full | full | full |
| wmc          | full | full | n/a  | full | full | full |
| npa          | full | full | n/a  | full | full | full |
| npm          | full | full | n/a  | full | full | full |
