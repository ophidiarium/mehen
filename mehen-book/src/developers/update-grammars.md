# Update grammars

Each tree-sitter-backed language in `mehen` is parsed via a pinned `tree-sitter-<lang>` crate. Grammars change over time and need periodic updates — both for bug fixes and to keep up with new syntax.

Grammars can be updated on **Linux** and **macOS** natively, or on **Windows** using **WSL**.

## Currently supported tree-sitter grammars

The pin lives in `xtask/Cargo.toml` (consumed by the kind-enum generator) and in the owning `crates/mehen-<lang>/Cargo.toml` (consumed by the analyzer at runtime). Most grammars are routed through `[workspace.dependencies]` in the root `Cargo.toml` and referenced as `{ workspace = true }` from both call sites; some, like `tree-sitter-c`, are inline-pinned because only one analyzer consumes them.

- `tree-sitter-c` — used by `mehen-c`
- `tree-sitter-go` — used by `mehen-go`
- `tree-sitter-kotlin-sg` — used by `mehen-kotlin`
- `tree-sitter-markdown-text` — used by `mehen-markdown`
- `tree-sitter-pwsh` — used by `mehen-powershell`

Other languages (Python, TS/JS/JSX/TSX, PHP, Ruby, Rust) flow through richer parsers (Ruff, Oxc, Mago, Prism, `ra_ap_syntax`) — they have no `grammar.rs` and need no kind-enum regeneration.

## Update process

1. Bump the grammar version in **all** places it's pinned. For workspace-routed grammars, that's just the line in root `Cargo.toml` `[workspace.dependencies]`. For inline-pinned grammars, both `xtask/Cargo.toml` and the owning `crates/mehen-<lang>/Cargo.toml`:

   ```toml
   tree-sitter-c = "=x.xx.x"
   ```

2. Regenerate the kind enums:

   ```bash
   cargo xtask tree-sitter generate --all
   ```

   This rewrites every `crates/mehen-<lang>/src/grammar.rs` from the pinned grammar's `node-kind` table. CI runs `cargo xtask tree-sitter check-generated` to ensure pinned-grammar bumps without a regenerate are caught at PR time.

3. Fix any failing analyzer code or tests introduced by the grammar change. New node kinds may appear (and need handling), and existing node kinds may be renamed or restructured.

4. Run the validation suite:

   ```bash
   cargo nextest run --all-features
   cargo insta test --all-features --check --unreferenced reject \
       --test-runner nextest --no-test-runner-fallback --disable-nextest-doctest
   cargo clippy --all-targets --all-features --locked
   ```

5. Commit and open a pull request.

## Automation

Dependabot raises grammar bump PRs automatically. The `regenerate-grammars` workflow detects those PRs (branch name contains `tree-sitter`) and runs `cargo xtask tree-sitter generate --all` plus the test suite, then commits the regenerated files back to the PR branch. Reviewers should still verify analyzer code still handles any renamed kinds.
