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

`mehen` works well in CI pipelines. Here is a real-world example from a GitHub Actions workflow that computes metrics on pull requests, compares against the `main` branch, and posts a summary comment:

```yaml
steps:
  - uses: actions/checkout@v5
    with:
      fetch-depth: 0

  - name: Set up Node.js
    uses: actions/setup-node@v5
    with:
      node-version: '22'

  # Run mehen on the PR branch
  - run: mkdir -p $HOME/mehen-json
  - run: npx -y mehen -m -O json -o "$HOME/mehen-json" -p src

  # Run mehen on main for baseline comparison
  - uses: actions/checkout@v5
    with:
      ref: main
      path: main
  - run: mkdir -p $HOME/mehen-json-base
  - run: npx -y mehen -m -O json -o "$HOME/mehen-json-base" -p main/src

  # Compare and comment on PR (using actions/github-script or similar)
```

The JSON output per file contains structured metric data that can be diffed across branches to surface regressions.

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
