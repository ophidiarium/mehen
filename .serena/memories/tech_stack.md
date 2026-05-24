# Tech Stack

- Rust workspace, edition 2024, rust-version 1.95.0.
- Root `Cargo.toml` pins shared dependencies; single-consumer dependencies are usually pinned in the owning crate.
- Tree-sitter remains the parser substrate for several language analyzers through `mehen-tree-sitter` and generated `grammar.rs` files.
- `xtask` is reached as `cargo xtask ...` via `.cargo/config.toml` and owns tree-sitter kind-enum generation.
- Snapshot testing uses `insta`; local test execution prefers `cargo nextest`.