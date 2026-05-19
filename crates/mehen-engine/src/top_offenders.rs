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
use mehen_core::{TopOffenderEntry, TopOffendersInput, TopOffendersReport};

/// Run `mehen top-offenders` against `input.paths` and return a ranked
/// report.
pub fn rank_top_offenders(input: TopOffendersInput) -> TopOffendersReport {
    let registry = Arc::new(AnalyzerRegistry::default_set());
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

            let scores: Vec<f64> = input
                .selectors
                .iter()
                .map(|s| read_metric(s, &analysis.root))
                .collect();

            entries.push(TopOffenderEntry {
                path: entry,
                language,
                scores,
            });
        }
    }

    entries.sort_by(|a, b| cmp_entries(a, b, input.selectors.len()));
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

/// Order entries from most concerning to least.
///
/// Higher score = more concerning. Cascade through every selector so
/// secondary keys break ties on the primary, tertiary keys break ties
/// on the secondary, etc. Path tie-breaks last for determinism.
fn cmp_entries(
    a: &TopOffenderEntry,
    b: &TopOffenderEntry,
    n_selectors: usize,
) -> std::cmp::Ordering {
    for i in 0..n_selectors {
        let av = a.scores.get(i).copied().unwrap_or(0.0);
        let bv = b.scores.get(i).copied().unwrap_or(0.0);
        let ord = bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal);
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }
    a.path.cmp(&b.path)
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

#[cfg(test)]
mod tests {
    use super::*;
    use mehen_core::Language;

    fn entry(path: &str, scores: &[f64]) -> TopOffenderEntry {
        TopOffenderEntry {
            path: Utf8PathBuf::from(path),
            language: Language::Rust,
            scores: scores.to_vec(),
        }
    }

    #[test]
    fn primary_score_ranks_first() {
        let mut xs = [entry("a.rs", &[10.0, 0.0]), entry("b.rs", &[20.0, 0.0])];
        xs.sort_by(|a, b| cmp_entries(a, b, 2));
        assert_eq!(xs[0].path, "b.rs");
        assert_eq!(xs[1].path, "a.rs");
    }

    #[test]
    fn secondary_selector_breaks_ties_on_primary() {
        // All three files tie on primary `loc.lloc = 100.0`. The
        // secondary `cognitive` selector must determine the order;
        // the file with the highest cognitive score is most
        // concerning.
        let mut xs = [
            entry("a.rs", &[100.0, 5.0]),
            entry("b.rs", &[100.0, 30.0]),
            entry("c.rs", &[100.0, 12.0]),
        ];
        xs.sort_by(|a, b| cmp_entries(a, b, 2));
        assert_eq!(xs[0].path, "b.rs");
        assert_eq!(xs[1].path, "c.rs");
        assert_eq!(xs[2].path, "a.rs");
    }

    #[test]
    fn tertiary_selector_breaks_ties_on_secondary() {
        let mut xs = [
            entry("a.rs", &[10.0, 5.0, 1.0]),
            entry("b.rs", &[10.0, 5.0, 9.0]),
            entry("c.rs", &[10.0, 5.0, 4.0]),
        ];
        xs.sort_by(|a, b| cmp_entries(a, b, 3));
        assert_eq!(xs[0].path, "b.rs");
        assert_eq!(xs[1].path, "c.rs");
        assert_eq!(xs[2].path, "a.rs");
    }

    #[test]
    fn fully_tied_falls_through_to_path() {
        let mut xs = [
            entry("zzz.rs", &[42.0, 7.0]),
            entry("aaa.rs", &[42.0, 7.0]),
            entry("mmm.rs", &[42.0, 7.0]),
        ];
        xs.sort_by(|a, b| cmp_entries(a, b, 2));
        assert_eq!(xs[0].path, "aaa.rs");
        assert_eq!(xs[1].path, "mmm.rs");
        assert_eq!(xs[2].path, "zzz.rs");
    }

    #[test]
    fn nan_score_is_treated_as_equal() {
        let mut xs = [
            entry("a.rs", &[f64::NAN, 5.0]),
            entry("b.rs", &[f64::NAN, 30.0]),
        ];
        xs.sort_by(|a, b| cmp_entries(a, b, 2));
        // NaN primaries compare equal; secondary breaks the tie.
        assert_eq!(xs[0].path, "b.rs");
        assert_eq!(xs[1].path, "a.rs");
    }
}
