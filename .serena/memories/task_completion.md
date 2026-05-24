# Task Completion

- Standard code-change closeout from repo root: `cargo fmt --all`, `cargo check`, `cargo build`, `cargo clippy --all-targets --all-features --locked`.
- Run tests with `cargo nextest run --all-features`.
- For snapshot-sensitive changes, run `cargo insta test --all-features --check --unreferenced reject --test-runner nextest --no-test-runner-fallback --disable-nextest-doctest`; use the review form only when intentionally updating snapshots.
- For parser generation work, regenerate with `cargo xtask tree-sitter generate ...` instead of hand-editing generated grammar files.