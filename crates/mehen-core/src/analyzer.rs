// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

use crate::Result;
use crate::analysis::LanguageAnalysis;
use crate::backend::AnalysisBackend;
use crate::config::AnalysisConfig;
use crate::language::Language;
use crate::source::SourceFile;

/// One language's analyzer.
///
/// Implementors:
/// - own their parser instance per call (or per worker); analyzers are
///   constructed by the engine, never shared as parser instances,
/// - return owned [`LanguageAnalysis`] — no borrows from parser arenas,
/// - emit recoverable issues via `LanguageAnalysis::diagnostics`, not via
///   the `Result`.
///
/// Crates that ship multiple analyzers (a tree-sitter baseline plus a future
/// Ruff/Oxc/Mago/Prism backend) implement this trait once per backend.
pub trait LanguageAnalyzer: Send + Sync {
    fn language(&self) -> Language;
    fn backend(&self) -> AnalysisBackend;
    fn analyze(&self, source: &SourceFile, config: &AnalysisConfig) -> Result<LanguageAnalysis>;
}

/// The re-entrance hook used by Markdown's embedded-code metric and any
/// future analyzer that must analyze a nested language fragment.
///
/// `mehen-engine` is the only implementor in 1.0. The seam exists so that
/// `mehen-markdown` does not need a compile-time dependency on every
/// language crate (rewrite plan review §3.3, §4.1).
pub trait LanguageDispatcher: Send + Sync {
    /// Analyze a nested source file. Recursion limits, source-size limits,
    /// and feature availability checks are enforced by the dispatcher
    /// implementation, not by the caller.
    fn analyze(&self, source: SourceFile, config: &AnalysisConfig) -> Result<LanguageAnalysis>;
}
