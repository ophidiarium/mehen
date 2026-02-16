# mehen

**mehen** is a Rust-powered CLI for detecting heuristic source code metrics at scale: complexity, maintainability, lines of code, and more.

It is designed for fast, deterministic analysis over large codebases, helping both human and AI engineers track how complexity evolves over time.

## What is Mehen?
In Ophidiarium projects, names matter.

**Mehen** is a mythical ancient Egyptian serpent associated with guarding Ra. In the same spirit, `mehen` helps guard your codebase from slowly collapsing under complexity.

## Why Teams Use Mehen
Most common usage patterns we see:

- CI jobs that compute metrics for changed files and publish trend reports
- Pre-PR / pre-CR hooks that provide immediate quality feedback
- Automation workflows that enrich pull request templates with metric deltas

## Current Language Support
Today `mehen` supports:

- Python
- TypeScript
- TSX
- Rust
- Go

Planned next: Ruby and PHP.

## What Mehen Computes
`mehen` provides a broad metric set, including:

- Cyclomatic complexity
- Cognitive complexity
- Maintainability Index
- Halstead metrics
- ABC metrics
- NArgs / NOM / NExit
- LOC family (SLOC, PLOC, LLOC, CLOC, blanks)
- NPA / NPM / WMC

## Distribution
`mehen` ships native binaries through both ecosystems:

- npm (`mehen` + platform packages)
- PyPI (`mehen` via maturin binary packaging)

## Quick Start
### Run without installation
From npm ecosystem:

```bash
bunx mehen --help
```

From Python/uv ecosystem:

```bash
uv tool run mehen --help
# or
uvx mehen --help
```

### Run locally from source

```bash
cargo run -- --help
```

### Typical examples
Analyze metrics for a directory:

```bash
mehen -m -p src
```

Export metrics as JSON/TOML/YAML/CBOR:

```bash
mehen -m -p src -O json -o ./metrics
```

## Reporting and Integrations
Current machine-readable outputs:

- JSON
- YAML
- TOML
- CBOR

Roadmap direction:

- Native git integration for changed-file detection
- Rich markdown reports for AI/human review flows
- More polished console reporting for local developer loops

## Implementation Notes
Internally, `mehen` is built on:

- [tree-sitter](https://tree-sitter.github.io/tree-sitter/) for parsing
- The excellent foundational work from Mozilla's [rust-code-analysis](https://github.com/mozilla/rust-code-analysis)

`mehen` continues in its own CLI-focused direction while preserving and evolving that foundation.

## Development
Build and check:

```bash
cargo check
cargo build
cargo fmt --all
cargo clippy --all-targets --all-features --locked
```

Tests:

```bash
cargo test --all-targets --locked
```

Snapshot tests (`insta`):

```bash
cargo insta test --all-features --check --unreferenced reject --test-runner nextest --no-test-runner-fallback --disable-nextest-doctest
```

See `mehen-book/src/developers/` for developer docs, including language and grammar updates.

## Contributing
Contributions are welcome via issues and pull requests:

- https://github.com/ophidiarium/mehen/issues

## License
`mehen` is released under the [Mozilla Public License v2.0](https://www.mozilla.org/MPL/2.0/).
