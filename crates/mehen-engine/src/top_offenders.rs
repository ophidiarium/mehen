//! `mehen top-offenders` orchestrator.
//!
//! Phase 5 implementation: walks the input paths, detects each file's
//! language, runs analysis through the registry, and ranks the files by
//! the requested metric selectors. Per the rewrite plan §2.4:
//! deterministic sorted output, ties broken by subsequent selectors.

use std::sync::Arc;

use camino::Utf8PathBuf;

use mehen_core::{MetricKey, Polarity, SourceFile};
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

    let polarities: Vec<Polarity> = input.selectors.iter().map(default_polarity_for).collect();
    entries.sort_by(|a, b| cmp_entries(a, b, &polarities));
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
/// "Most concerning" depends on the metric's polarity. For
/// `HigherIsWorse` metrics (cyclomatic, cognitive, halstead.volume,
/// loc.*) a larger value is worse, so they sort descending. For
/// `HigherIsBetter` metrics (mi.original, mi.sei, mi.visual_studio)
/// a smaller value is worse, so they sort ascending.
///
/// Cascade through every selector so secondary keys break ties on
/// the primary, tertiary keys break ties on the secondary, etc.
/// Path tie-breaks last for determinism.
fn cmp_entries(
    a: &TopOffenderEntry,
    b: &TopOffenderEntry,
    polarities: &[Polarity],
) -> std::cmp::Ordering {
    for (i, polarity) in polarities.iter().enumerate() {
        let av = a.scores.get(i).copied().unwrap_or(0.0);
        let bv = b.scores.get(i).copied().unwrap_or(0.0);
        let base = av.partial_cmp(&bv).unwrap_or(std::cmp::Ordering::Equal);
        let ord = match polarity {
            // Worst-first: larger value is more concerning, so a > b
            // should put `a` first → reverse the natural ordering.
            Polarity::HigherIsWorse => base.reverse(),
            // Worst-first: smaller value is more concerning, so a < b
            // should put `a` first → use the natural ordering.
            Polarity::HigherIsBetter => base,
        };
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }
    a.path.cmp(&b.path)
}

/// Resolve a metric's "higher is worse / better" polarity from its
/// key. Maintainability-index variants (`mi.*`) are higher-is-better;
/// every other metric the engine publishes (cyclomatic, cognitive,
/// loc.*, halstead.*, abc, nom, nargs, nexit, npa, npm, wmc) is
/// higher-is-worse. This mirrors the legacy `KNOWN_METRICS` catalog
/// and the rewrite plan §5.1 metric contract.
fn default_polarity_for(selector: &MetricSelector) -> Polarity {
    if selector.key.as_str().starts_with("mi.") || selector.key.as_str() == "mi" {
        Polarity::HigherIsBetter
    } else {
        Polarity::HigherIsWorse
    }
}

pub(crate) fn read_metric(selector: &MetricSelector, root: &mehen_core::MetricSpace) -> f64 {
    let lookup = |key: &MetricKey| root.metrics.get(key).map(|v| v.as_f64());
    match selector.aggregator {
        SelectorAggregator::Root => lookup(&selector.key).unwrap_or(0.0),
        SelectorAggregator::Sum => suffixed_lookup(&selector.key, &["sum"], &lookup),
        SelectorAggregator::Min => suffixed_lookup(&selector.key, &["min"], &lookup),
        SelectorAggregator::Max => suffixed_lookup(&selector.key, &["max"], &lookup),
        // Per `mehen-metrics::state`, average is published as either
        // `<key>.avg` (cyclomatic, loc.*) or `<key>.average`
        // (cognitive, nom, nargs, nexit, npa, npm). Try the short form
        // first to match the selector spelling, then fall back.
        SelectorAggregator::Avg => suffixed_lookup(&selector.key, &["avg", "average"], &lookup),
    }
}

/// Look the selector key up under each suffix in order (e.g.
/// `["avg", "average"]` for the avg aggregator), returning the first
/// hit. `0.0` if none match — keeps the behavior of a missing metric
/// the same as a missing root-level key.
fn suffixed_lookup(
    base: &MetricKey,
    suffixes: &[&str],
    lookup: &dyn Fn(&MetricKey) -> Option<f64>,
) -> f64 {
    for suffix in suffixes {
        let key = MetricKey::new(format!("{base}.{suffix}"));
        if let Some(v) = lookup(&key) {
            return v;
        }
    }
    0.0
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

    const HIW2: &[Polarity] = &[Polarity::HigherIsWorse, Polarity::HigherIsWorse];
    const HIW3: &[Polarity] = &[
        Polarity::HigherIsWorse,
        Polarity::HigherIsWorse,
        Polarity::HigherIsWorse,
    ];

    #[test]
    fn primary_score_ranks_first() {
        let mut xs = [entry("a.rs", &[10.0, 0.0]), entry("b.rs", &[20.0, 0.0])];
        xs.sort_by(|a, b| cmp_entries(a, b, HIW2));
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
        xs.sort_by(|a, b| cmp_entries(a, b, HIW2));
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
        xs.sort_by(|a, b| cmp_entries(a, b, HIW3));
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
        xs.sort_by(|a, b| cmp_entries(a, b, HIW2));
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
        xs.sort_by(|a, b| cmp_entries(a, b, HIW2));
        // NaN primaries compare equal; secondary breaks the tie.
        assert_eq!(xs[0].path, "b.rs");
        assert_eq!(xs[1].path, "a.rs");
    }

    #[test]
    fn higher_is_better_metric_sorts_smallest_first() {
        // For maintainability index a low value is the worst offender,
        // so `bad.rs` (mi = 10) must rank above `good.rs` (mi = 120).
        let mut xs = [
            entry("good.rs", &[120.0]),
            entry("bad.rs", &[10.0]),
            entry("mid.rs", &[60.0]),
        ];
        xs.sort_by(|a, b| cmp_entries(a, b, &[Polarity::HigherIsBetter]));
        assert_eq!(xs[0].path, "bad.rs");
        assert_eq!(xs[1].path, "mid.rs");
        assert_eq!(xs[2].path, "good.rs");
    }

    #[test]
    fn mixed_polarities_sort_each_axis_independently() {
        // Primary loc.lloc (lower-is-worse): 200 > 10, so high-LOC
        // files rank first. Secondary mi (higher-is-worse): when LOC
        // ties, the file with the *lower* mi should rank first.
        let mut xs = [
            entry("low_loc_high_mi.rs", &[10.0, 120.0]),
            entry("high_loc_high_mi.rs", &[200.0, 120.0]),
            entry("high_loc_low_mi.rs", &[200.0, 30.0]),
        ];
        xs.sort_by(|a, b| cmp_entries(a, b, &[Polarity::HigherIsWorse, Polarity::HigherIsBetter]));
        assert_eq!(xs[0].path, "high_loc_low_mi.rs");
        assert_eq!(xs[1].path, "high_loc_high_mi.rs");
        assert_eq!(xs[2].path, "low_loc_high_mi.rs");
    }

    #[test]
    fn default_polarity_treats_mi_variants_as_higher_is_better() {
        for s in ["mi.original", "mi.sei", "mi.visual_studio", "mi"] {
            assert_eq!(
                default_polarity_for(&sel(s)),
                Polarity::HigherIsBetter,
                "selector {s}",
            );
        }
    }

    #[test]
    fn default_polarity_treats_other_metrics_as_higher_is_worse() {
        for s in [
            "cyclomatic",
            "cognitive",
            "loc.lloc",
            "halstead.volume",
            "abc",
            "nom.functions",
        ] {
            assert_eq!(
                default_polarity_for(&sel(s)),
                Polarity::HigherIsWorse,
                "selector {s}",
            );
        }
    }

    fn space_with_metrics(pairs: &[(&str, f64)]) -> mehen_core::MetricSpace {
        use mehen_core::{MetricSpace, SourceSpan, SpaceId, SpaceKind};
        let mut space = MetricSpace::new(SpaceId(0), SpaceKind::Unit, SourceSpan::empty());
        for (k, v) in pairs {
            space.metrics.insert(MetricKey::new(*k), *v);
        }
        space
    }

    fn sel(s: &str) -> MetricSelector {
        s.parse().unwrap()
    }

    #[test]
    fn root_aggregator_reads_bare_key() {
        let space = space_with_metrics(&[("loc.lloc", 42.0), ("loc.lloc.max", 999.0)]);
        assert_eq!(read_metric(&sel("loc.lloc"), &space), 42.0);
    }

    #[test]
    fn sum_aggregator_reads_sum_suffixed_key() {
        let space = space_with_metrics(&[
            ("cyclomatic", 1.0),
            ("cyclomatic.sum", 17.0),
            ("cyclomatic.max", 9.0),
        ]);
        assert_eq!(read_metric(&sel("cyclomatic.sum"), &space), 17.0);
    }

    #[test]
    fn min_aggregator_reads_min_suffixed_key() {
        let space = space_with_metrics(&[
            ("loc.lloc", 100.0),
            ("loc.lloc.min", 3.0),
            ("loc.lloc.max", 50.0),
        ]);
        assert_eq!(read_metric(&sel("loc.lloc.min"), &space), 3.0);
    }

    #[test]
    fn max_aggregator_reads_max_suffixed_key() {
        let space = space_with_metrics(&[
            ("loc.lloc", 100.0),
            ("loc.lloc.min", 3.0),
            ("loc.lloc.max", 50.0),
        ]);
        assert_eq!(read_metric(&sel("loc.lloc.max"), &space), 50.0);
    }

    #[test]
    fn avg_aggregator_prefers_avg_then_average() {
        // `cyclomatic` publishes `.avg`; `cognitive` publishes
        // `.average`. The aggregator must locate either spelling so
        // selectors written `cognitive.avg` still resolve to the
        // analyzer's `cognitive.average` value.
        let cyclomatic = space_with_metrics(&[("cyclomatic.avg", 2.5)]);
        assert_eq!(read_metric(&sel("cyclomatic.avg"), &cyclomatic), 2.5);

        let cognitive = space_with_metrics(&[("cognitive.average", 3.5)]);
        assert_eq!(read_metric(&sel("cognitive.avg"), &cognitive), 3.5);
    }

    #[test]
    fn missing_aggregated_key_falls_back_to_zero() {
        // When the analyzer didn't publish the requested aggregation,
        // matches the existing root-key contract: 0.0 instead of
        // panicking, so a single missing metric doesn't break the
        // whole rank pass.
        let space = space_with_metrics(&[("loc.lloc", 100.0)]);
        assert_eq!(read_metric(&sel("loc.lloc.max"), &space), 0.0);
    }
}
