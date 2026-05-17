use serde::{Deserialize, Serialize};

/// Configuration handed to a [`crate::LanguageAnalyzer::analyze`] call.
///
/// Kept intentionally small in 1.0 — analyzer-specific options should live
/// inside the analyzer's own crate. This struct exists so analyzers see
/// engine-level decisions (max recursion depth for embedded analysis,
/// whether to compute contributions, …) without each analyzer reinventing
/// the parameter shape.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AnalysisConfig {
    /// If true, analyzers should populate `LanguageAnalysis::contributions`
    /// with explainable evidence. When false, analyzers may skip the work
    /// for performance.
    ///
    /// Defaults to `false` via the derived `Default` impl (matches `bool`'s
    /// default). [`AnalysisConfig::production`] sets it to `true`;
    /// [`AnalysisConfig::benchmark`] keeps it `false`.
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

impl AnalysisConfig {
    /// Defaults appropriate for production CLI use.
    pub fn production() -> Self {
        Self {
            emit_contributions: true,
            max_dispatch_depth: 4,
            dispatch_depth: 0,
        }
    }

    /// Defaults appropriate for benchmarks where contribution evidence is
    /// not consumed.
    pub fn benchmark() -> Self {
        Self {
            emit_contributions: false,
            max_dispatch_depth: 4,
            dispatch_depth: 0,
        }
    }
}
