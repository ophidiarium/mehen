//! Review Criticality Index (RCI) per §18.
//!
//! RCI answers: "Should I review this document carefully?" A small document
//! can still be high-priority if it is dense with technical anchors.
//!
//! ```text
//! DensityScore = clamp01(
//!     0.25 * sat(MCC / max(1, W / 500); 4, 18)
//!   + 0.20 * sat(MDH_volume_total / max(1, W); 20, 120)
//!   + 0.20 * RepositoryGroundingScore
//!   + 0.15 * EvidenceCoverageScore
//!   + 0.10 * sat(LinkReviewBurden / max(1, W / 500); 2, 10)
//!   + 0.10 * sat(embedded_code_complexity / max(1, W / 500); 2, 12)
//! )
//! ```
//!
//! ```text
//! RCI = clamp01(
//!     0.65 * DensityScore
//!   + 0.20 * sat(abs(metric_delta_percent); 10, 60)
//!   + 0.15 * sat(changed_links_or_artifacts; 2, 20)
//! ) * 100
//! ```
//!
//! `metric_delta_percent` and `changed_links_or_artifacts` are activated by
//! `mehen diff` (Phase F). In Phase D the baseline is absent so both
//! default to `0` — DensityScore drives RCI directly.

use crate::mathops::{clamp01, sat};

/// Inputs RCI needs from Phase A/B/C/D outputs. All densities are already
/// pre-aggregated; the formula is a weighted sum.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RciInputs {
    /// §8 MCC final value.
    pub(crate) mcc: f64,
    /// §4 narrative word count `W`.
    pub(crate) words: u64,
    /// §9 Markdown Halstead `total_volume` (includes embedded §9.4).
    pub(crate) mdh_volume_total: f64,
    /// §15 RepositoryGroundingScore in `[0, 1]`.
    pub(crate) repository_grounding_score: f64,
    /// §16 EvidenceCoverageScore in `[0, 1]`.
    pub(crate) evidence_coverage_score: f64,
    /// §11.4 Link Review Burden (unbounded, typically 0-100).
    pub(crate) link_review_burden: f64,
    /// §9.4 embedded_volume. A proxy for embedded_code_complexity.
    pub(crate) embedded_code_complexity: f64,
    /// Absolute % change vs baseline. Always `0.0` in Phase D.
    pub(crate) metric_delta_percent: f64,
    /// Baseline diff: count of changed links + artifacts. Always `0` in Phase D.
    pub(crate) changed_links_or_artifacts: u64,
}

/// Output on the `[0, 100]` scale.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct RciResult {
    pub(crate) review_criticality_index: f64,
    /// DensityScore before the diff-aware aggregation. Retained for
    /// Phase F (`mehen diff`) and auditability; not read by the analyzer.
    #[allow(dead_code)]
    pub(crate) density_score: f64,
}

/// Computes RCI per §18.1.
pub(crate) fn compute_rci(inputs: RciInputs) -> RciResult {
    let w = inputs.words as f64;
    let denom_500 = (w / 500.0).max(1.0);
    let denom_w = w.max(1.0);

    let density = clamp01(
        0.25 * sat(inputs.mcc / denom_500, 4.0, 18.0)
            + 0.20 * sat(inputs.mdh_volume_total / denom_w, 20.0, 120.0)
            + 0.20 * inputs.repository_grounding_score.clamp(0.0, 1.0)
            + 0.15 * inputs.evidence_coverage_score.clamp(0.0, 1.0)
            + 0.10 * sat(inputs.link_review_burden / denom_500, 2.0, 10.0)
            + 0.10 * sat(inputs.embedded_code_complexity / denom_500, 2.0, 12.0),
    );

    let raw = 0.65 * density
        + 0.20 * sat(inputs.metric_delta_percent.abs(), 10.0, 60.0)
        + 0.15 * sat(inputs.changed_links_or_artifacts as f64, 2.0, 20.0);
    let review_criticality_index = clamp01(raw) * 100.0;
    RciResult {
        review_criticality_index,
        density_score: density,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_inputs_produce_zero_rci() {
        let r = compute_rci(RciInputs {
            mcc: 0.0,
            words: 0,
            mdh_volume_total: 0.0,
            repository_grounding_score: 0.0,
            evidence_coverage_score: 0.0,
            link_review_burden: 0.0,
            embedded_code_complexity: 0.0,
            metric_delta_percent: 0.0,
            changed_links_or_artifacts: 0,
        });
        assert_eq!(r.review_criticality_index, 0.0);
    }

    #[test]
    fn dense_small_doc_rci_is_high() {
        // A small but technically dense doc should tip DensityScore above
        // 0.7. Without a baseline (metric_delta / changed_artifacts both 0)
        // RCI caps at 0.65 × DensityScore × 100, so we expect > 45.
        let r = compute_rci(RciInputs {
            mcc: 40.0,
            words: 650,
            mdh_volume_total: 200.0,
            repository_grounding_score: 0.9,
            evidence_coverage_score: 0.85,
            link_review_burden: 15.0,
            embedded_code_complexity: 40.0,
            metric_delta_percent: 0.0,
            changed_links_or_artifacts: 0,
        });
        assert!(
            r.review_criticality_index > 45.0,
            "got {}",
            r.review_criticality_index
        );
    }

    #[test]
    fn rci_is_bounded() {
        let r = compute_rci(RciInputs {
            mcc: 1e9,
            words: 1,
            mdh_volume_total: 1e9,
            repository_grounding_score: 1.0,
            evidence_coverage_score: 1.0,
            link_review_burden: 1e9,
            embedded_code_complexity: 1e9,
            metric_delta_percent: 1e6,
            changed_links_or_artifacts: 1_000_000,
        });
        assert!(r.review_criticality_index <= 100.0 + 1e-9);
    }
}
