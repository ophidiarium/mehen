//! `mehen-typescript` — TypeScript, JavaScript, TSX, and JSX analyzer.
//!
//! Phase 1 scope: skeleton with the tree-sitter backend wired through
//! `LanguageAnalyzer`. The crate exposes one analyzer per `Language` it
//! handles (`TypeScript`, `JavaScript`, `Tsx`, `Jsx`) so the engine
//! registry can dispatch by `Language` without selecting grammars at the
//! call site. Phase 7 replaces the tree-sitter backend with Oxc.

#![forbid(unsafe_code)]

mod analyzer;

pub use analyzer::{JavaScriptAnalyzer, JsxAnalyzer, TsxAnalyzer, TypeScriptAnalyzer};
