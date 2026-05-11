# Supported Languages

**Mehen** supports these programming languages:

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
