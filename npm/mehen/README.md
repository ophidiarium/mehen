# mehen

Rust-powered CLI for heuristic source code and documentation metrics: complexity, maintainability,
lines of code, and Markdown documentation health.

📚 **Documentation: <https://mehen.ophi.dev>**

## Install

```bash
npm install -g mehen
```

Or run without installing:

```bash
npx -y mehen --help
bunx mehen --help
```

Also available on [PyPI](https://pypi.org/project/mehen/):

```bash
uvx mehen --help
```

## What mehen computes

For source code: cyclomatic complexity, cognitive complexity, Halstead suite, Maintainability Index,
ABC, LOC family (SLOC, PLOC, LLOC, CLOC, blank), NARGS, NEXITS, NOM, NPA, NPM, WMC.

For Markdown documentation: Documentation Maintainability Index (DMI), Markdown Reading Path Complexity
(MRPC), Markdown Cognitive Complexity (MCC), Markdown Halstead, Link Debt, Table Burden, Visual
Scaffold, Artifact Debt, Repository Grounding, Evidence Coverage, Filler / Lazy Structure Risk, Review
Criticality Index, plus an opt-in English / Japanese prose layer.

Full metric catalog with formulas and references: <https://mehen.ophi.dev/metrics/code/overview>.

## Supported languages

Python, TypeScript / JavaScript (`.ts` / `.mts` / `.cts` / `.js` / `.mjs` / `.cjs`),
TSX / JSX, Rust, Go, Ruby, Kotlin, PowerShell, C, and Markdown.

## Quick start

```bash
# Compute metrics for a directory
npx -y mehen -m -p src

# Export as JSON
npx -y mehen -m -p src -O json -o ./metrics

# Diff metrics against main
npx -y mehen diff --from main --to HEAD --paths src
```

Other supported output formats: YAML, TOML, CBOR.

Full quickstart: <https://mehen.ophi.dev/quickstart>.

## CI integration

`mehen` ships a GitHub Action that computes changed-file metric trends on pull requests, compares
against the base branch, and posts a summary comment:

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
      thresholds: |
        cyclomatic=5
        cognitive=4
```

Full reference: <https://mehen.ophi.dev/guides/github-action>.

## Platforms

Native binaries are provided for:

| OS | x64 | arm64 |
|---|---|---|
| Linux (glibc) | `@mehen/linux-x64-gnu` | `@mehen/linux-arm64-gnu` |
| Linux (musl) | `@mehen/linux-x64-musl` | `@mehen/linux-arm64-musl` |
| macOS | `@mehen/darwin-x64` | `@mehen/darwin-arm64` |
| Windows | `@mehen/win32-x64` | `@mehen/win32-arm64` |

The correct binary is selected automatically at runtime.

Requires Node.js >= 18.

## Links

- [Documentation](https://mehen.ophi.dev)
- [GitHub](https://github.com/ophidiarium/mehen)
- [Issues](https://github.com/ophidiarium/mehen/issues)
- [PyPI package](https://pypi.org/project/mehen/)

## License

[AGPL-3.0-only](https://www.gnu.org/licenses/agpl-3.0.html)
