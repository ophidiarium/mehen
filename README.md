# mehen

**mehen** is a Rust-powered CLI for detecting heuristic source code metrics at scale: complexity,
maintainability, lines of code, documentation health, and more.

It is designed for fast, deterministic analysis over large codebases, helping both human and AI
engineers track how complexity evolves over time.

📚 **Documentation: <https://mehen.ophi.dev>**

## What is Mehen?

In Ophidiarium projects, names matter. **Mehen** is a mythical ancient Egyptian serpent associated with
guarding Ra. In the same spirit, `mehen` helps guard your codebase from slowly collapsing under
complexity.

## Why teams use mehen

- **Polyglot by design** — per-file language detection across nine source languages plus Markdown.
  Useful for monorepos.
- **Code and documentation in one tool** — source-code complexity *and* Markdown documentation health.
- **Deterministic, no network** — pure static analysis. Same input → same output. Safe for air-gapped
  CI.
- **Pull-request native** — built-in `mehen diff` plus a sticky comment GitHub Action.

## Install

```bash
# npm
npm install -g mehen

# PyPI / uv
uv tool install mehen
# or: pip install mehen

# cargo binstall
cargo binstall --git https://github.com/ophidiarium/mehen mehen
```

Full installation guide: <https://mehen.ophi.dev/installation>.

## Quick start

```bash
# Compute metrics for a directory
mehen -m -p src

# Export as JSON
mehen -m -p src -O json -o ./metrics

# Diff metrics against main
mehen diff --from main --to HEAD --paths src
```

Quickstart: <https://mehen.ophi.dev/quickstart>.

## GitHub Action

Drop the action into a workflow to publish per-PR metric trends:

```yaml
permissions:
  contents: read
  pull-requests: write
  issues: write

steps:
  - uses: actions/checkout@v6
    with:
      fetch-depth: 0
  - uses: ophidiarium/mehen@v0
    with:
      paths: src
```

Full reference: <https://mehen.ophi.dev/guides/github-action>.

## Documentation

Everything else lives in the docs site:

- [Code metrics](https://mehen.ophi.dev/metrics/code/overview) — cyclomatic, cognitive, Halstead, MI,
  ABC, LOC family, NOM, NPA, NPM, WMC.
- [Markdown metrics](https://mehen.ophi.dev/metrics/markdown/overview) — DMI, MRPC, MCC, link debt,
  filler/lazy risk, English/Japanese prose layer.
- [SQL metrics (preview)](https://mehen.ophi.dev/metrics/sql/overview) — roadmap for `mehen-sql`.
- [Commands](https://mehen.ophi.dev/commands/overview) — `mehen`, `mehen diff`, AST inspection.
- [Developers guide](https://mehen.ophi.dev/developers/overview) — build, test, contribute, add a
  language.

## Contributing

Issues and pull requests welcome at <https://github.com/ophidiarium/mehen/issues>.

## License

`mehen` is released under the [GNU Affero General Public License v3.0](https://www.gnu.org/licenses/agpl-3.0.html).
