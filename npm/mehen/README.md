# mehen

Rust-powered CLI for detecting heuristic source code metrics at scale: complexity, maintainability, lines of code, and more.

Designed for fast, deterministic analysis over large codebases, helping both human and AI engineers track how complexity evolves over time.

## Install

```bash
npm install mehen
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

## Supported Languages

- Python (.py)
- TypeScript (.ts)
- TSX (.tsx)
- Rust (.rs)
- Go (.go)

## Usage

Analyze metrics for a directory:

```bash
npx -y mehen -m -p src
```

Export metrics as JSON:

```bash
npx -y mehen -m -p src -O json -o ./metrics
```

Other supported output formats: YAML, TOML, CBOR.

## What Mehen Computes

- **Cyclomatic complexity** -- control flow complexity
- **Cognitive complexity** -- human-perceived complexity with nesting
- **Maintainability Index** -- overall maintainability score
- **Halstead metrics** -- volume, difficulty, effort, bugs prediction
- **ABC metric** -- assignments, branches, conditionals
- **LOC family** -- SLOC, PLOC, LLOC, CLOC, blanks
- **NArgs / NOM / NExit** -- arguments, methods, exit points
- **NPA / NPM / WMC** -- public attributes, public methods, weighted methods per class

## CI Integration

`mehen` ships a GitHub Action that computes changed-file metric trends on pull requests, compares against the base branch, and posts a summary comment:

```yaml
permissions:
  contents: read
  pull-requests: write
  issues: write

steps:
  - uses: actions/checkout@v5
    with:
      fetch-depth: 0

  - uses: ophidiarium/mehen@v1
    with:
      paths: src
      thresholds: |
        cyclomatic=5
        cognitive=4
```

The action is backed by `mehen diff`, so polyglot repositories can pass multiple roots and let `mehen` pick supported languages from changed files.

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

- [GitHub](https://github.com/ophidiarium/mehen)
- [Issues](https://github.com/ophidiarium/mehen/issues)
- [PyPI package](https://pypi.org/project/mehen/)

## License

[MPL-2.0](https://www.mozilla.org/MPL/2.0/)
