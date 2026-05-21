// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

use serde::{Deserialize, Serialize};

/// Configuration handed to a [`crate::LanguageAnalyzer::analyze`] call.
///
/// Kept intentionally small in 1.0 — analyzer-specific options should live
/// inside the analyzer's own crate. This struct exists so analyzers see
/// engine-level decisions (max recursion depth for embedded analysis,
/// whether to compute contributions, …) without each analyzer reinventing
/// the parameter shape.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// If true, analyzers should populate `LanguageAnalysis::contributions`
    /// with explainable evidence. When false, analyzers may skip the work
    /// for performance.
    ///
    /// `Default::default()` and [`AnalysisConfig::benchmark`] leave this
    /// `false`; [`AnalysisConfig::production`] sets it to `true`.
    pub emit_contributions: bool,

    /// Maximum recursion depth for [`crate::LanguageDispatcher::analyze`]
    /// requests. Used by Markdown's embedded-code path to bound nested
    /// fence-in-fence cases. Zero disables nested analysis entirely.
    pub max_dispatch_depth: u8,

    /// The current dispatch depth — incremented by the dispatcher on each
    /// recursive call. Analyzers do not need to read this; the dispatcher
    /// uses it to enforce `max_dispatch_depth`.
    pub dispatch_depth: u8,
}

/// Default `max_dispatch_depth` for `production()` / `benchmark()` /
/// `Default`. Bounds embedded-code recursion (Markdown fences, future
/// dispatch-driven analyzers); the value is large enough to cover every
/// realistic doc-in-doc chain we ship.
const DEFAULT_MAX_DISPATCH_DEPTH: u8 = 4;

impl Default for AnalysisConfig {
    /// Produce a config that callers can use without immediately tripping
    /// the dispatch-depth guard. The derived `Default` would have set
    /// `max_dispatch_depth = 0`, which makes `EngineDispatcher::analyze`
    /// reject on the very first call — see PR #95 review and the
    /// `default_allows_at_least_one_dispatch` test below.
    fn default() -> Self {
        Self {
            emit_contributions: false,
            max_dispatch_depth: DEFAULT_MAX_DISPATCH_DEPTH,
            dispatch_depth: 0,
        }
    }
}

impl AnalysisConfig {
    /// Defaults appropriate for production CLI use.
    pub fn production() -> Self {
        Self {
            emit_contributions: true,
            max_dispatch_depth: DEFAULT_MAX_DISPATCH_DEPTH,
            dispatch_depth: 0,
        }
    }

    /// Defaults appropriate for benchmarks where contribution evidence is
    /// not consumed.
    pub fn benchmark() -> Self {
        Self {
            emit_contributions: false,
            max_dispatch_depth: DEFAULT_MAX_DISPATCH_DEPTH,
            dispatch_depth: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_allows_at_least_one_dispatch() {
        // Regression: the derived `Default` impl set `max_dispatch_depth`
        // to `0`, which made `EngineDispatcher::analyze` (which rejects
        // when `dispatch_depth >= max_dispatch_depth`) fail on the very
        // first dispatch with "max dispatch depth exceeded (0)". The
        // manual impl below sets the depth to a realistic ceiling so
        // callers using `AnalysisConfig::default()` aren't immediately
        // blocked.
        let config = AnalysisConfig::default();
        assert!(
            config.max_dispatch_depth > config.dispatch_depth,
            "Default config must allow at least one dispatch; got \
             max_dispatch_depth={} dispatch_depth={}",
            config.max_dispatch_depth,
            config.dispatch_depth
        );
    }

    #[test]
    fn default_matches_production_depth_budget() {
        // The dispatch budget is shared across the named constructors so
        // that callers who pick `default()` get the same recursion ceiling
        // as `production()` — only `emit_contributions` differs.
        let default = AnalysisConfig::default();
        let production = AnalysisConfig::production();
        assert_eq!(default.max_dispatch_depth, production.max_dispatch_depth);
        assert_eq!(default.dispatch_depth, production.dispatch_depth);
        assert_eq!(default.dispatch_depth, 0);
    }
}
