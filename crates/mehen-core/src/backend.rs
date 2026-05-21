// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

use serde::{Deserialize, Serialize};

/// Identifies the parser backend that produced a [`crate::LanguageAnalysis`].
///
/// Surfaced in JSON output and snapshots so parity work and migration
/// snapshots can tell which backend was active. Per the rewrite plan, parser
/// choice is internal — there is no user-facing override flag — but the
/// label is still useful in reports.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AnalysisBackend {
    TreeSitter,
    /// Ruff parser + semantic. Reserved for the Phase 6 Python migration.
    PythonRuff,
    /// Oxc parser. Reserved for the Phase 7 TypeScript/JS migration.
    Oxc,
    /// Mago syntax. Reserved for the Phase 8 PHP migration.
    Mago,
    /// `ruby-prism`. Reserved for the Phase 9 Ruby migration.
    Prism,
    /// rust-analyzer's `ra_ap_syntax` parser. Used by `mehen-rust` from
    /// Phase 9 of the rewrite onward, replacing tree-sitter-rust.
    RaApSyntax,
    /// The current pre-1.0 Markdown analyzer.
    MarkdownLegacy,
    /// Comrak. Reserved for the Phase 10 Markdown evaluation.
    Comrak,
    /// Anything not yet covered.
    Other(String),
}

impl AnalysisBackend {
    pub fn label(&self) -> &str {
        match self {
            AnalysisBackend::TreeSitter => "tree-sitter",
            AnalysisBackend::PythonRuff => "python-ruff",
            AnalysisBackend::Oxc => "oxc",
            AnalysisBackend::Mago => "mago",
            AnalysisBackend::Prism => "prism",
            AnalysisBackend::RaApSyntax => "rust-ra-ap-syntax",
            AnalysisBackend::MarkdownLegacy => "markdown-legacy",
            AnalysisBackend::Comrak => "comrak",
            AnalysisBackend::Other(s) => s.as_str(),
        }
    }
}
