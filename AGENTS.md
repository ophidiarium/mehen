# AGENTS

## Scope
This repository is a CLI-first Rust project (`mehen`). Prefer changes that keep behavior centered on the `mehen` binary.

## Build and Test
Use these commands from the repo root:

```bash
cargo build
cargo check
cargo fmt --all
cargo clippy --all-targets --all-features --locked
```

## Recommended Test Runner
Use `nextest` as the default test runner for local and CI work.

```bash
cargo nextest run --all-features
```

## Snapshot Tests (insta)
`insta` is used heavily in metric tests. Prefer running snapshot checks via `cargo insta` on top of `nextest`:

```bash
cargo insta test --all-features --check --unreferenced reject --test-runner nextest --no-test-runner-fallback --disable-nextest-doctest
```

When intentionally updating snapshots:

```bash
cargo insta test --all-features --review --test-runner nextest --no-test-runner-fallback --disable-nextest-doctest
```

## Notes for Code Changes
- Keep metric behavior deterministic across platforms.
- Avoid introducing dead code paths; this project is consumed as a CLI.
- If touching grammars/language enums, keep `enums/` generation flow in sync.
