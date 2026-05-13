//! Good Scaffold Score per §21.
//!
//! ```text
//! GoodScaffoldScore = clamp01(
//!     0.25 * VisualScaffoldScore
//!   + 0.20 * TableScaffoldScore
//!   + 0.20 * bounded_labelled_code_example_score
//!   + 0.15 * InformationScentScore
//!   + 0.10 * section_summary_score
//!   + 0.10 * successful_internal_navigation_score
//! )
//! ```
//!
//! Phase D inputs:
//!
//! - `VisualScaffoldScore` → already populated by Phase C (`visuals`).
//! - `TableScaffoldScore` → already populated by Phase C (`tables`).
//! - `bounded_labelled_code_example_score` → computed here from
//!   `ArtifactRecord` rows (labelled code fences of bounded size with
//!   nearby explanation).
//! - `InformationScentScore` → already populated by Phase C (`links`).
//! - `section_summary_score` → stays at `0.0` in Phase D. This requires a
//!   natural-language summariser which is out of scope; when/if a
//!   Phase F summariser lands we'll wire it here without changing the
//!   schema.
//! - `successful_internal_navigation_score` → resolved internal anchors /
//!   total internal anchors.
//!
//! This score is only a modest offset to DMI. §21 explicitly states it
//! "should never erase objective defects like broken links or parse
//! failures".

use crate::markdown::mathops::clamp01;
use crate::markdown::types::{
    ArtifactKind, ArtifactRecord, LinkClass, LinkRecord, Links, Tables, Visuals,
};

/// §21 output plus intermediate sub-scores so later Phase F / mehen diff
/// can surface them.
#[derive(Debug, Default, Clone)]
pub(crate) struct GoodScaffold {
    pub(crate) good_scaffold_score: f64,
    #[allow(dead_code)]
    pub(crate) bounded_labelled_code_example_score: f64,
    #[allow(dead_code)]
    pub(crate) successful_internal_navigation_score: f64,
    /// Reserved for a future natural-language summariser; always 0.0 for now.
    #[allow(dead_code)]
    pub(crate) section_summary_score: f64,
}

/// Compute §21 from Phase A/B/C outputs.
pub(crate) fn analyze_good_scaffold(
    artifacts: &[ArtifactRecord],
    links_records: &[LinkRecord],
    links_agg: &Links,
    visuals: &Visuals,
    tables: &Tables,
) -> GoodScaffold {
    let bounded = bounded_labelled_code_example_score(artifacts);
    let internal_nav = successful_internal_navigation_score(links_records);
    let section_summary_score = 0.0;

    let raw = 0.25 * visuals.visual_scaffold_score
        + 0.20 * tables.table_scaffold_score
        + 0.20 * bounded
        + 0.15 * links_agg.information_scent_score
        + 0.10 * section_summary_score
        + 0.10 * internal_nav;

    GoodScaffold {
        good_scaffold_score: clamp01(raw),
        bounded_labelled_code_example_score: bounded,
        successful_internal_navigation_score: internal_nav,
        section_summary_score,
    }
}

/// A labelled code fence is "bounded" when it has a language tag, is not
/// oversized (`oversized = false`), and has a nearby explanation. We measure
/// the fraction of code fences that satisfy all three properties.
fn bounded_labelled_code_example_score(artifacts: &[ArtifactRecord]) -> f64 {
    let mut code_total: u64 = 0;
    let mut bounded: u64 = 0;
    for a in artifacts {
        if a.kind != ArtifactKind::Code {
            continue;
        }
        code_total += 1;
        if a.has_label && !a.oversized && a.has_explanation {
            bounded += 1;
        }
    }
    if code_total == 0 {
        return 0.0;
    }
    clamp01(bounded as f64 / code_total as f64)
}

fn successful_internal_navigation_score(links: &[LinkRecord]) -> f64 {
    let mut total_internal: u64 = 0;
    let mut resolved: u64 = 0;
    for l in links {
        if l.class == LinkClass::Internal {
            total_internal += 1;
            if matches!(l.resolved, Some(true)) {
                resolved += 1;
            }
        }
    }
    if total_internal == 0 {
        0.0
    } else {
        resolved as f64 / total_internal as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_code(lang: Option<&str>, oversized: bool, explained: bool) -> ArtifactRecord {
        ArtifactRecord {
            id: 0,
            kind: ArtifactKind::Code,
            start_line: 1,
            end_line: 10,
            language_tag: lang.map(String::from),
            size: 10,
            has_explanation: explained,
            has_label: lang.is_some(),
            oversized,
            burden: 0.0,
        }
    }

    #[test]
    fn bounded_score_is_fraction() {
        let arts = vec![
            mk_code(Some("rust"), false, true),
            mk_code(Some("py"), false, false),
            mk_code(None, false, true),
        ];
        let score = bounded_labelled_code_example_score(&arts);
        assert!((score - (1.0 / 3.0)).abs() < 1e-9);
    }

    #[test]
    fn bounded_score_no_code_is_zero() {
        let score = bounded_labelled_code_example_score(&[]);
        assert_eq!(score, 0.0);
    }
}
