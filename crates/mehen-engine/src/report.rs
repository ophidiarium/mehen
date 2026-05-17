use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use mehen_core::{
    AnalysisBackend, AnalysisConfig, Language, LanguageAnalysis, MetricSpace, ParseDiagnostic,
    SourceFile,
};
use mehen_metrics::{MetricSelector, ThresholdViolation};

/// Inputs to `analyze_metrics`.
#[derive(Clone, Debug)]
pub struct AnalyzeMetricsInput {
    pub source: SourceFile,
    pub config: AnalysisConfig,
}

/// `mehen metrics` JSON output shape (rewrite plan §9.1).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricsReport {
    pub schema_version: String,
    pub tool: String,
    pub path: Utf8PathBuf,
    pub language: Language,
    pub analysis_backend: AnalysisBackend,
    pub diagnostics: Vec<ParseDiagnostic>,
    pub root: MetricSpace,
}

impl MetricsReport {
    pub fn empty() -> Self {
        // Used as the seed shape in tests / docs. Production callers go
        // through `From<LanguageAnalysis>`.
        Self {
            schema_version: "1.0".to_string(),
            tool: "mehen".to_string(),
            path: Utf8PathBuf::new(),
            language: Language::Markdown,
            analysis_backend: AnalysisBackend::TreeSitter,
            diagnostics: Vec::new(),
            root: MetricSpace::new(
                mehen_core::SpaceId(0),
                mehen_core::SpaceKind::Unit,
                mehen_core::SourceSpan::empty(),
            ),
        }
    }
}

impl From<LanguageAnalysis> for MetricsReport {
    fn from(analysis: LanguageAnalysis) -> Self {
        Self {
            schema_version: "1.0".to_string(),
            tool: "mehen".to_string(),
            path: Utf8PathBuf::new(),
            language: analysis.language,
            analysis_backend: analysis.backend,
            diagnostics: analysis.diagnostics,
            root: analysis.root,
        }
    }
}

/// Inputs to `analyze_diff`.
///
/// Phase 1 ships the type so that later phases can fill in the orchestration
/// without reshaping the public surface.
#[derive(Clone, Debug)]
pub struct DiffInput {
    pub from: String,
    pub to: String,
    pub paths: Vec<Utf8PathBuf>,
    pub thresholds: Vec<mehen_metrics::Threshold>,
    pub config: AnalysisConfig,
}

/// `mehen diff --format json` output shape (rewrite plan §9.2).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DiffReport {
    pub schema_version: String,
    pub base: String,
    pub head: String,
    pub files: Vec<DiffFile>,
    pub markdown_files: Vec<DiffFile>,
    pub analysis_errors: Vec<AnalysisErrorRecord>,
    pub threshold_violations: Vec<ThresholdViolation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiffFile {
    pub path: Utf8PathBuf,
    // Phase 5 fills in metric deltas. Kept skeletal here so diff JSON has a
    // documented shape even before the orchestrator lands.
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnalysisErrorRecord {
    pub path: Utf8PathBuf,
    pub side: DiffSide,
    pub diagnostics: Vec<ParseDiagnostic>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffSide {
    Base,
    Head,
}

/// Inputs to `rank_top_offenders`.
#[derive(Clone, Debug)]
pub struct TopOffendersInput {
    pub paths: Vec<Utf8PathBuf>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub selectors: Vec<MetricSelector>,
    pub max_results: usize,
    pub config: AnalysisConfig,
}

/// `mehen top-offenders --format json` output shape.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TopOffendersReport {
    pub schema_version: String,
    pub selectors: Vec<String>,
    pub entries: Vec<TopOffenderEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TopOffenderEntry {
    pub path: Utf8PathBuf,
    pub language: Language,
    pub score: f64,
}
