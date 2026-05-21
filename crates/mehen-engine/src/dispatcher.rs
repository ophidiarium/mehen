// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

use mehen_core::{
    AnalysisConfig, AnalysisError, LanguageAnalysis, LanguageDispatcher, Result, SourceFile,
};

use crate::registry::AnalyzerRegistry;

/// The only `LanguageDispatcher` in 1.0.
///
/// Owned by `mehen-engine` and handed to `mehen-markdown::analyze_markdown`
/// (and any future analyzer that needs to recursively analyze a nested
/// fragment) so the caller never needs a compile-time dependency on every
/// language crate.
///
/// Recursion is bounded by `AnalysisConfig::max_dispatch_depth`. Going past
/// the limit returns an `Internal` error rather than producing partial
/// results — the dispatcher is the right layer to enforce the bound.
pub struct EngineDispatcher<'r> {
    registry: &'r AnalyzerRegistry,
}

impl<'r> EngineDispatcher<'r> {
    pub fn new(registry: &'r AnalyzerRegistry) -> Self {
        Self { registry }
    }
}

impl<'r> LanguageDispatcher for EngineDispatcher<'r> {
    fn analyze(&self, source: SourceFile, config: &AnalysisConfig) -> Result<LanguageAnalysis> {
        if config.dispatch_depth >= config.max_dispatch_depth {
            return Err(AnalysisError::Internal(format!(
                "max dispatch depth exceeded ({})",
                config.max_dispatch_depth
            )));
        }
        let mut child_config = config.clone();
        child_config.dispatch_depth = config.dispatch_depth.saturating_add(1);

        let analyzer = self
            .registry
            .analyzer_for(source.language)
            .ok_or(AnalysisError::AnalyzerUnavailable(source.language))?;
        analyzer.analyze(&source, &child_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatcher_enforces_depth() {
        // Build a registry without any registered analyzer to ensure the
        // depth check fires before the lookup. We probe with depth equal
        // to the limit, so the dispatcher should refuse without trying to
        // call any analyzer.
        let registry = AnalyzerRegistry::new();
        let dispatcher = EngineDispatcher::new(&registry);

        let source = SourceFile::new("x.md".into(), mehen_core::Language::Markdown, String::new());
        let mut config = AnalysisConfig::production();
        config.dispatch_depth = config.max_dispatch_depth;

        let err = dispatcher.analyze(source, &config).unwrap_err();
        match err {
            AnalysisError::Internal(msg) => assert!(msg.contains("dispatch depth")),
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn default_config_does_not_trip_depth_guard_on_first_dispatch() {
        // Regression for PR #95 review: when `AnalysisConfig` derived
        // `Default`, `max_dispatch_depth` was `0` and the very first
        // dispatch hit `dispatch_depth (0) >= max_dispatch_depth (0)`.
        // The manual `Default` impl on `AnalysisConfig` now reserves a
        // realistic depth budget, so callers using `default()` should
        // sail past the depth check and only fail (in this test setup)
        // because the empty registry has no analyzer registered.
        let registry = AnalyzerRegistry::new();
        let dispatcher = EngineDispatcher::new(&registry);

        let source = SourceFile::new("x.md".into(), mehen_core::Language::Markdown, String::new());
        let config = AnalysisConfig::default();

        let err = dispatcher.analyze(source, &config).unwrap_err();
        match err {
            AnalysisError::AnalyzerUnavailable(_) => {}
            other => panic!(
                "expected AnalyzerUnavailable (depth guard should not fire on first \
                 dispatch with default config), got {other:?}"
            ),
        }
    }
}
