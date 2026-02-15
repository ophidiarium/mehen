# CLAUDE.md - AI Assistant Guide for Mehen

This document provides context for AI assistants working with the Mehen codebase.

## Project Overview

**Mehen** is a focused code analysis library that computes software metrics for Go, Python, Rust, and TypeScript/TSX source code. It uses Tree-sitter parsers for accurate AST-based analysis.

**Origin**: Forked from mozilla/rust-code-analysis, streamlined to support only 4 languages instead of 10+.

**Author**: Konstantin Vyatkin <tino@vtkn.io>
**Repository**: https://github.com/ophidiarium/mehen
**License**: MPL-2.0

## Supported Languages ONLY

This is critical - the codebase **only** supports these 4 languages:

1. **Go** (.go) - via tree-sitter-go v0.23.4
2. **Python** (.py) - via tree-sitter-python v0.23.6
3. **Rust** (.rs) - via tree-sitter-rust v0.23.2
4. **TypeScript** (.ts, .jsw, .jsmw) - via tree-sitter-typescript v0.23.2
5. **TSX** (.tsx) - via tree-sitter-typescript v0.23.2

### Removed Languages (DO NOT reference these)

The following were intentionally removed:
- Java, Kotlin, C, C++, JavaScript, Mozjs, Ccomment, Preproc
- Any references to `JavaCode`, `KotlinCode`, `CppCode`, `MozjsCode`, `JavascriptCode`, `CcommentCode`, `PreprocCode` are errors
- Any references to `JavaParser`, `CppParser`, etc. are errors

## Build Requirements

### Rust Version Requirements

**Minimum**: Rust **1.93.1** (current stable as of Feb 2025)
**Edition**: 2024

The codebase uses **let chains** syntax:

```rust
if let Some(label_child) = node.child(1)
    && let Label = label_child.kind_id().into()
{
    // ...
}
```

This feature requires Rust 1.88.0+ with edition 2024.

```bash
cargo build
cargo test
cargo check
```

## Project Structure

```
mehen/
├── src/                      # Core CLI and analysis engine
│   ├── languages/           # Language-specific AST enums (5 files)
│   │   ├── language_go.rs
│   │   ├── language_python.rs
│   │   ├── language_rust.rs
│   │   ├── language_tsx.rs
│   │   └── language_typescript.rs
│   ├── metrics/             # Metric implementations
│   │   ├── abc.rs          # ABC metric
│   │   ├── cognitive.rs    # Cognitive complexity
│   │   ├── cyclomatic.rs   # Cyclomatic complexity
│   │   ├── exit.rs         # Number of exits
│   │   ├── halstead.rs     # Halstead metrics
│   │   ├── loc.rs          # Lines of code (SLOC, PLOC, LLOC, CLOC)
│   │   ├── mi.rs           # Maintainability Index
│   │   ├── nargs.rs        # Number of arguments
│   │   ├── nom.rs          # Number of methods
│   │   ├── npa.rs          # Number of public attributes
│   │   ├── npm.rs          # Number of public methods
│   │   └── wmc.rs          # Weighted Methods per Class
│   ├── alterator.rs        # AST node transformation
│   ├── checker.rs          # Language-specific code checks
│   ├── getter.rs           # Extract information from nodes
│   ├── langs.rs            # Language definitions (mk_langs! macro)
│   ├── parser.rs           # Parser wrapper
│   ├── formats.rs          # Output serializers
│   └── main.rs             # CLI entry point
├── mehen-book/             # Documentation (mdBook)
└── enums/                  # Code generator for language enums

Tests: tests/, inline in src/metrics/*.rs
```

## Key Architecture Patterns

### 1. Language Definition Pattern

Languages are defined using the `mk_langs!` macro in `src/langs.rs`:

```rust
mk_langs!(
    (
        Rust,
        "The `Rust` language",
        "rust",
        RustCode,      // Type for trait implementations
        RustParser,    // Parser type
        tree_sitter_rust,
        [rs],          // File extensions
        ["rust"]       // Emacs modes
    ),
    // ... other languages
);
```

### 2. Trait Implementation Pattern

Each language must implement these traits:
- `Checker` - Language-specific checks (comments, functions, etc.)
- `Getter` - Extract space kinds, operators, operands
- `Alterator` - Transform AST nodes
- Metric traits: `Abc`, `Cognitive`, `Cyclomatic`, `Exit`, `Halstead`, `Loc`, `Mi`, `NArgs`, `Nom`, `Npa`, `Npm`, `Wmc`

### 3. Metric Trait Pattern

Most metrics follow this pattern:

```rust
pub trait MetricName {
    fn compute(node: &Node, stats: &mut Stats, ...);
}

impl MetricName for GoCode {
    fn compute(node: &Node, stats: &mut Stats, ...) {
        use crate::Go::*;
        match node.kind_id().into() {
            // Handle language-specific nodes
            _ => {}
        }
    }
}

// For languages with empty/default implementations
implement_metric_trait!(MetricName, PythonCode, RustCode);
```

## Important Implementation Details

### Language-Specific Type Safety

Each language has its own enum type defined in `src/languages/language_*.rs`:
- `Go` enum for Go AST nodes
- `Python` enum for Python AST nodes
- `Rust` enum for Rust AST nodes
- `Typescript` enum for TypeScript AST nodes
- `Tsx` enum for TSX AST nodes

These enums are auto-generated and should NOT be manually edited (marked with `// Code generated; DO NOT EDIT.`).

### TypeScript/TSX Relationship

TypeScript and TSX share the same tree-sitter grammar but have separate enums. They often share implementation logic.

### JavaScript Handling

There is NO JavaScript support. TypeScript handles `.ts` files, TSX handles `.tsx` files. Do not add JavaScript-specific code.

### Preprocessing Infrastructure

All C/C++ preprocessing infrastructure has been removed:
- No `PreprocParser`, `PreprocResults`, `PreprocCode`
- No `get_macros()`, `fix_includes()`, `preprocess()` functions
- No `c_macro.rs` or `c_langs_macros/` module

## Testing

### Test Organization

1. **Inline tests**: Each metric file has `#[cfg(test)] mod tests { ... }`
2. **Integration tests**: `tests/` directory (minimal - most removed)
3. **Test helper**: Uses `insta` for snapshot testing

### Running Tests

```bash
cargo test                # All tests
cargo test metrics::      # Filter by test name
cargo test -- --nocapture # With output
```

### Test Pattern

```rust
#[test]
fn go_simple_function() {
    check_metrics::<GoParser>(
        "package main\n\nfunc f() { ... }",
        "foo.go",
        |metric| {
            insta::assert_json_snapshot!(metric.cyclomatic, @r###"..."###);
        },
    );
}
```

## Common Tasks

### Adding Support for a Metric to a Language

1. Find the metric trait in `src/metrics/*.rs`
2. Add `impl MetricName for LanguageCode { fn compute(...) { ... } }`
3. Use `crate::LanguageName::*` to access AST node types
4. Add tests following existing patterns

### Adding a New Language (Hypothetically)

1. Add tree-sitter dependency to `Cargo.toml` and `enums/Cargo.toml`
2. Generate enum with enums tool: `cd enums && cargo run -- --language NewLang`
3. Add language module to `src/languages/mod.rs`
4. Add language to `src/langs.rs` using `mk_langs!` macro
5. Add to `enums/src/languages.rs`
6. Implement required traits in trait files
7. Add tests for each metric

### Modifying Metrics

- Metrics are computed during AST traversal
- Each node type can increment counters in `Stats`
- Use pattern matching on `node.kind_id().into()` to handle specific nodes
- Reference existing implementations for similar languages

## Critical Files

### Language Configuration
- `src/langs.rs` - Main language registry (mk_langs! macro)
- `enums/src/languages.rs` - Enum generator registry
- `enums/src/macros.rs` - get_language() match statement

### Core Traits
- `src/checker.rs` - Define what is a comment, function, closure, etc.
- `src/getter.rs` - Extract space kinds, Halstead operators/operands
- `src/alterator.rs` - Transform AST nodes for serialization

### Parser Infrastructure
- `src/parser.rs` - Main Parser<T> struct
- `src/node.rs` - Node wrapper around tree-sitter::Node
- `src/traits.rs` - Core trait definitions

## Dependencies

### Required Tree-sitter Versions (Exact)
- tree-sitter = "=0.25.3"
- tree-sitter-typescript = "=0.23.2"
- tree-sitter-python = "=0.23.6"
- tree-sitter-rust = "=0.23.2"
- tree-sitter-go = "=0.23.4"

These versions must match exactly - the `=` prefix means no automatic updates.

## Metrics Computed

All metrics are computed per-function and aggregated:

1. **Cyclomatic Complexity (CC)** - Control flow complexity
2. **Cognitive Complexity** - Human-perceived complexity with nesting
3. **Halstead Metrics** - Volume, difficulty, effort, bugs prediction
4. **Lines of Code**:
   - SLOC: Source lines (total non-blank)
   - PLOC: Physical lines (excluding comments)
   - LLOC: Logical lines (statements)
   - CLOC: Comment lines
5. **ABC Metric** - Assignments, Branches, Conditionals
6. **Maintainability Index (MI)** - Overall maintainability score
7. **NOM** - Number of Methods
8. **NArgs** - Number of Arguments per function
9. **NExit** - Number of exit points
10. **NPA** - Number of Public Attributes
11. **NPM** - Number of Public Methods
12. **WMC** - Weighted Methods per Class

## Common Patterns to Avoid

### Don't Add Back Removed Languages

If you see references to these in git history, ignore them:
- ❌ Java/Kotlin implementations
- ❌ C/C++ preprocessing
- ❌ Mozilla JavaScript (Mozjs) variants
- ❌ Ccomment/Preproc parsers

### Rust Version Matters

The project requires Rust 1.88.0+ (or nightly for older versions) due to `let_chains` feature usage.

### Don't Break the Macro System

The `mk_langs!` macro generates lots of boilerplate. Changes to language registration must be synchronized across:
1. `src/langs.rs`
2. `enums/src/languages.rs`
3. `enums/src/macros.rs`
4. All trait implementation files

## Workspace Members

The workspace has 1 package:

1. **mehen** (root) - Command-line tool (binary name: `mehen`)

The `enums` crate is excluded from the workspace (it's a build-time code generator).

## Git Conventions

- Run tests before committing
- This project inherited history from mozilla/rust-code-analysis
- The fork point is commit `4ed54eb` (feat: Add Go language support)

## Documentation

- Code documentation: `cargo doc --open`
- Book: `mehen-book/` (mdBook format)
- CLI/user docs live in the README and `mehen-book/`

## External Links to Preserve

Do NOT change these legitimate external references:
- https://tree-sitter.github.io/tree-sitter/ (Tree-sitter homepage)
- https://www.mozilla.org/MPL/2.0/ (MPL 2.0 license text)
- Academic paper citations with original authors

## Quick Reference

```bash
# Build everything
cargo build

# Run all tests
cargo test

# Check compilation
cargo check

# Format code
cargo fmt --all

# Run CLI
cargo run -- -m -p test.go
```

## Key Insights from the Cleanup

1. **Massive Simplification**: Removed 861k+ lines, kept 628 lines of new code
2. **Language Focus**: 4 languages cover most modern development needs
3. **Dead Code**: Removed entire subsystems (preprocessing, C-specific macros)
4. **Type Safety**: Each language has its own strongly-typed AST enum
5. **Metric Completeness**: All 5 supported languages have full metric coverage

## When Working on This Codebase

✅ **Do**:
- Use nightly Rust
- Test with all 4 supported languages
- Follow existing trait implementation patterns
- Add tests for new functionality
- Keep language support focused

❌ **Don't**:
- Add back removed languages without discussion
- Break the macro-generated code
- Add preprocessing or C/C++ specific features
- Reference removed parser types in code

## Contact

For questions about this codebase, refer to:
- GitHub Issues: https://github.com/ophidiarium/mehen/issues
- Original upstream: https://github.com/mozilla/rust-code-analysis (for historical context only)
