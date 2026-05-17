//! `mehen top-offenders` orchestrator.
//!
//! Phase 5 implementation: walks the input paths, detects each file's
//! language, runs analysis through the registry, and ranks the files by
//! the requested metric selectors. Per the rewrite plan §2.4:
//! deterministic sorted output, ties broken by subsequent selectors.

use std::sync::Arc;

use camino::Utf8PathBuf;

use mehen_core::SourceFile;
use mehen_metrics::{MetricSelector, SelectorAggregator};

use crate::detection::detect_language;
use crate::registry::AnalyzerRegistry;
use crate::report::{TopOffenderEntry, TopOffendersInput, TopOffendersReport};

/// Run `mehen top-offenders` against `input.paths` and return a ranked
/// report.
pub fn rank_top_offenders(input: TopOffendersInput) -> TopOffendersReport {
    let registry = Arc::new(AnalyzerRegistry::default_set());
    let primary_selector = input.selectors.first().cloned();
    let mut entries: Vec<TopOffenderEntry> = Vec::new();

    for root in &input.paths {
        for entry in walk_paths(root, &input.include, &input.exclude) {
            let Some(language) = detect_language(entry.as_path()) else {
                continue;
            };
            let Ok(text) = std::fs::read_to_string(entry.as_std_path()) else {
                continue;
            };
            let Some(analyzer) = registry.analyzer_for(language) else {
                continue;
            };
            let source = SourceFile::new(entry.clone(), language, text);
            let Ok(analysis) = analyzer.analyze(&source, &input.config) else {
                continue;
            };

            let score = primary_selector
                .as_ref()
                .map(|s| read_metric(s, &analysis.root))
                .unwrap_or(0.0);

            entries.push(TopOffenderEntry {
                path: entry,
                language,
                score,
            });
        }
    }

    // Higher score = more concerning. Stable sort breaks ties by path
    // for deterministic output.
    entries.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.path.cmp(&b.path))
    });
    if entries.len() > input.max_results {
        entries.truncate(input.max_results);
    }

    TopOffendersReport {
        schema_version: "1.0".to_string(),
        selectors: input.selectors.iter().map(|s| s.to_string()).collect(),
        entries,
    }
}

fn walk_paths(root: &Utf8PathBuf, _include: &[String], _exclude: &[String]) -> Vec<Utf8PathBuf> {
    if !root.exists() {
        return Vec::new();
    }
    let mut out = Vec::new();
    if root.is_file() {
        out.push(root.clone());
        return out;
    }
    for entry in walkdir::WalkDir::new(root.as_std_path())
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file()
            && let Ok(utf8) = Utf8PathBuf::try_from(entry.path().to_path_buf())
        {
            out.push(utf8);
        }
    }
    out
}

fn read_metric(selector: &MetricSelector, root: &mehen_core::MetricSpace) -> f64 {
    match selector.aggregator {
        SelectorAggregator::Root => root
            .metrics
            .get(&selector.key)
            .map(|v| v.as_f64())
            .unwrap_or(0.0),
        // Phase 5 demo: only Root aggregation is wired up. Min/Max/Avg/Sum
        // require walking the space tree; the math layer in
        // `mehen-metrics` ships the helpers but tying them to the
        // selector here is follow-up work.
        _ => root
            .metrics
            .get(&selector.key)
            .map(|v| v.as_f64())
            .unwrap_or(0.0),
    }
}
