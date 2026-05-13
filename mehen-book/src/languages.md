# Supported Languages

**Mehen** supports these programming languages:

- [x] **C** (.c, .h) - via tree-sitter-c v0.25.0
- [x] **Go** (.go) - via tree-sitter-go v0.25.0
- [x] **Kotlin** (.kt, .kts) - via tree-sitter-kotlin-sg v0.4.0 (fwcd/tree-sitter-kotlin grammar)
- [x] **PowerShell** (.ps1, .psm1, .psd1) - via tree-sitter-pwsh v0.38.0 (wharflab/tree-sitter-powershell grammar)
- [x] **Python** (.py) - via tree-sitter-python v0.25.0
- [x] **Ruby** (.rb) - via tree-sitter-ruby v0.23.1
- [x] **Rust** (.rs) - via tree-sitter-rust v0.24.2
- [x] **TypeScript / JavaScript** (.ts, .mts, .cts, .js, .mjs, .cjs) - via tree-sitter-typescript v0.23.2
- [x] **TSX / JSX** (.tsx, .jsx) - via tree-sitter-typescript v0.23.2

TypeScript is a superset of JavaScript, so `mehen` uses the TypeScript grammar
to analyze both `.ts` and `.js` source files, and the TSX grammar for `.tsx`
and `.jsx` files.

## Documentation

**Markdown** (.md, .markdown, .mdown, .mkd, .mkdn, .mdx) is supported for
**documentation metrics only** — the Markdown family covers LOC variants,
reading path complexity, cognitive complexity, Halstead, Documentation
Maintainability Index, link debt, table burden, visual scaffold, repository
grounding, evidence coverage, filler/lazy risk, review criticality, and an
opt-in language-aware prose layer for English and Japanese.

Code-style metrics such as cyclomatic complexity, cognitive complexity per
function, `NOM` / `NPA` / `NPM`, and `WMC` do **not** apply to Markdown:
Markdown has no functions, classes, or interfaces to score. See
[Markdown Metrics](./metrics/markdown.md) and
[Markdown Prose Metrics](./metrics/markdown-prose.md) for the full list.
