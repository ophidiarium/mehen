//! `mehen-python` — Python language analyzer.
//!
//! Phase 6 implementation: Ruff-backed analyzer (`ruff_python_parser` +
//! `ruff_python_ast`). The crate exposes one analyzer
//! (`PythonAnalyzer`) so the engine registry dispatches `Language::Python`
//! to it directly.
//!
//! tree-sitter-python is no longer a dependency of this crate — per
//! `docs/python-ruff-spec.md`, every metric is computed from the Ruff
//! AST and Ruff's lexer token stream. Python-specific behavior (type
//! annotations as runtime objects, docstrings, `match`/`case`,
//! exception groups, comprehensions) is documented in that spec.

#![forbid(unsafe_code)]

mod analyzer;
mod walker;

pub use analyzer::PythonAnalyzer;
