// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! `mehen-typescript` — TypeScript, JavaScript, TSX, and JSX analyzer.
//!
//! Phase 7 implementation: Oxc-backed analyzer (`oxc_parser` +
//! `oxc_ast` + `oxc_ast_visit`). The crate exposes one analyzer per
//! `Language` it handles (`TypeScript`, `JavaScript`, `Tsx`, `Jsx`)
//! so the engine registry can dispatch by `Language` without selecting
//! grammars at the call site.
//!
//! tree-sitter-typescript is no longer a dependency; per-language
//! parser ownership lives entirely inside this crate.

#![forbid(unsafe_code)]

mod analyzer;
mod walker;

pub use analyzer::{JavaScriptAnalyzer, JsxAnalyzer, TsxAnalyzer, TypeScriptAnalyzer};
