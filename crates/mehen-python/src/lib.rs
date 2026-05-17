//! `mehen-python` — Python language analyzer.
//!
//! Phase 1 scope: skeleton with the tree-sitter backend wired through
//! `LanguageAnalyzer`. Phase 3 moves the per-language metric interpretation
//! here from the pre-1.0 `src/languages/language_python.rs` and the
//! Python-specific match arms in `src/metrics/*.rs`. Phase 6 replaces the
//! tree-sitter backend with Ruff parser + semantic.

#![forbid(unsafe_code)]

mod analyzer;

pub use analyzer::PythonAnalyzer;
