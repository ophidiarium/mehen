# CLAUDE.md - AI Assistant Guide for Mehen

This file gives coding assistants fast, current context for this repository.

## Project Scope
- `mehen` is a **CLI-only** Rust project.
- Users consume the binary; do not re-introduce a public library API surface.
- Keep code changes focused on CLI behavior, correctness, and maintainability.

## Repository Structure (current)
- `src/main.rs`: CLI entry point, command routing.
- `src/langs.rs`: language registration via `mk_langs!`.
- `src/macros.rs`: language/action/codegen helper macros.
- `src/languages/`: generated language enums (`language_*.rs`).
- `src/metrics/`: metric implementations and tests.
- `src/checker.rs`, `src/getter.rs`, `src/parser.rs`, `src/node.rs`, `src/spaces.rs`, `src/ops.rs`: core analysis pipeline.
- `src/output/`: dump/metrics/ops output.
- `enums/`: grammar enum generator crate.
- `mehen-book/`: mdBook documentation.

## Supported Languages
Only these are supported:
- Rust
- Python
- Go
- TypeScript
- TSX

Do not reference removed language families from upstream history (Java/Kotlin/C/C++/MozJS/etc.) unless explicitly required for migration context.

## Build and Lint
Run from repo root:

```bash
cargo check
cargo build
cargo fmt --all
cargo clippy --all-targets --all-features --locked
```

For dead-code cleanup work, use:

```bash
cargo clippy --all-targets --all-features --locked -- -W dead_code -W unreachable_pub
```

## Testing (nextest-first)
Use `nextest` by default when available.

Detection + fallback:

```bash
if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run --all-features
else
  cargo test --all-targets --locked
fi
```

## Snapshot Tests (`insta`)
`insta` is heavily used in metric tests.

Check snapshots (CI-style):

```bash
cargo insta test --all-features --check --unreferenced reject --test-runner nextest --no-test-runner-fallback --disable-nextest-doctest
```

Update snapshots intentionally:

```bash
cargo insta test --all-features --review --test-runner nextest --no-test-runner-fallback --disable-nextest-doctest
```

## Language/Grammar Changes
When adding or updating a language:
1. Update `enums/Cargo.toml` and `enums/src/languages.rs`.
2. Update `enums/src/macros.rs` `mk_get_language` mapping.
3. Regenerate enums with `./recreate-grammars.sh`.
4. Wire language in `src/languages/mod.rs` and `src/langs.rs`.
5. Implement or adjust behavior in `checker/getter/metrics`.

## Coding Expectations
- Keep internals internal (`pub(crate)`/private) unless a real external API is needed.
- Prefer explicit imports over wildcard re-exports.
- Avoid dead code; this is a CLI-focused codebase.
- Preserve deterministic metric behavior across platforms.

## Useful References
- `README.md`
- `AGENTS.md`
- `mehen-book/src/developers/new-language.md`
