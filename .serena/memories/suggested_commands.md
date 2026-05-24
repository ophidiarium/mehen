# Suggested Commands

- Build binary/default member: `cargo build`
- Type/check default member: `cargo check`
- Format workspace: `cargo fmt --all`
- Lint all targets/features with lockfile: `cargo clippy --all-targets --all-features --locked`
- Preferred test runner: `cargo nextest run --all-features`
- Snapshot check: `cargo insta test --all-features --check --unreferenced reject --test-runner nextest --no-test-runner-fallback --disable-nextest-doctest`
- Snapshot review/update: `cargo insta test --all-features --review --test-runner nextest --no-test-runner-fallback --disable-nextest-doctest`
- Tree-sitter generation: `cargo xtask tree-sitter generate <language>` or `cargo xtask tree-sitter generate --all`.