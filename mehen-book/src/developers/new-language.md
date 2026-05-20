# Supporting a new language

This section helps developers add support for a new language in `mehen`.

A number of [metrics are supported](../metrics.md); per-metric implementation guides live elsewhere in this book.

## High-level shape

Each language is owned by a single per-language analyzer crate at `crates/mehen-<lang>/`. The analyzer:

- Pins its own parser (a tree-sitter grammar, or a richer language-specific parser like Ruff / Oxc / Mago / Prism / `ra_ap_syntax`).
- Owns metric interpretation for that language's syntax — what counts as a decision, an operator, a method, a comment, etc.
- Returns `LanguageAnalysis` (owned, `Send + 'static`) so `mehen-engine` can analyze files in parallel and never holds onto parser arenas.

For tree-sitter-backed languages the analyzer also owns its `grammar.rs` kind enum, generated from the pinned grammar's `node-kind` table.

## Adding a tree-sitter-backed language

**Prerequisite:** a `tree-sitter-<lang>` crate compatible with the `tree-sitter` version pinned in the workspace (`Cargo.toml` `[workspace.dependencies]`).

1. Pin the grammar in two places — both must stay in sync so the analyzer and the kind-enum generator link the same grammar:

   - `xtask/Cargo.toml`: add a direct dependency line. The kind-enum generator imports the grammar at codegen time.
   - `crates/mehen-<lang>/Cargo.toml`: add the same dependency. The analyzer imports the grammar at runtime to drive `tree_sitter::Parser`.

   Workspace-managed grammars (those listed in root `Cargo.toml` `[workspace.dependencies]`) can be referenced as `{ workspace = true }` from both places. Inline-pinned grammars must be kept in lockstep manually.

2. Register the language in `xtask/src/tree_sitter.rs::TARGETS`:

   ```rust
   GeneratorTarget {
       slug: "rust",
       enum_name: "Rust",
       crate_dir: "crates/mehen-rust/src",
       language: || tree_sitter_rust::LANGUAGE.into(),
   },
   ```

3. Generate the kind enum:

   ```bash
   cargo xtask tree-sitter generate rust
   ```

   The result lands at `crates/mehen-rust/src/grammar.rs`. **Never edit it directly** — `cargo xtask tree-sitter check-generated` runs in CI and fails the build if the checked-in file doesn't match a fresh render.

4. Implement the analyzer in `crates/mehen-<lang>/src/lib.rs`:

   - Define a `<Lang>Analyzer` struct that implements `mehen_core::LanguageAnalyzer`.
   - Walk the parse tree and emit metrics through `mehen_metrics::{State, MetricTreeBuilder, ...}` and the per-metric helpers.
   - Use the generated `crate::grammar::<Lang>` enum for kind-id matching — it deduplicates positional kinds and exposes mnemonic identifiers (`PLUS`, `EQ_EQ`, etc.).

5. Register the analyzer in `mehen-engine`'s registry (`crates/mehen-engine/src/registry.rs`) so `Language::<YourLang>` dispatches to it.

6. Add per-metric integration tests under `crates/mehen-<lang>/tests/` — typically one file per metric family, snapshotting the rendered metric JSON via `insta`.

## Adding a non-tree-sitter language

Per the rewrite plan §6, some languages flow through richer parsers (Ruff for Python, Oxc for TS/JS, Mago for PHP, Prism for Ruby, `ra_ap_syntax` for Rust). Those analyzers don't need a `grammar.rs` — they walk the parser's typed AST directly. Skip steps 1–3 above and go straight to step 4, importing the parser crates you need into `crates/mehen-<lang>/Cargo.toml`.

## Bumping a pinned grammar

When dependabot bumps a `tree-sitter-<lang>` version (or you do it manually):

1. Update both `xtask/Cargo.toml` and `crates/mehen-<lang>/Cargo.toml` to the new version. The `regenerate-grammars` workflow does this automatically for inline-pinned grammars.
2. Run `cargo xtask tree-sitter generate --all` and commit the regenerated `grammar.rs` files.
3. CI's `cargo xtask tree-sitter check-generated` will fail until the regenerated files are committed.

## Validation

```bash
cargo check --workspace
cargo nextest run --all-features
cargo insta test --all-features --check --unreferenced reject \
    --test-runner nextest --no-test-runner-fallback --disable-nextest-doctest
cargo clippy --all-targets --all-features --locked
cargo xtask tree-sitter check-generated
```
