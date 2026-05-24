# Core

- CLI-first Rust workspace for `mehen`; default cargo member is `crates/mehen-cli`, so plain `cargo build` / `cargo run` targets the binary.
- Workspace members are language analyzer crates plus shared crates: `mehen-core`, `mehen-metrics`, `mehen-tree-sitter`, `mehen-engine`, `mehen-git`, `mehen-report`, `mehen-cli`, and `xtask`.
- Read `mem:tech_stack` for language/tool pins and parser substrate notes.
- Read `mem:conventions` before editing analyzers or generated parser artifacts.
- Read `mem:suggested_commands` and `mem:task_completion` for local validation commands.