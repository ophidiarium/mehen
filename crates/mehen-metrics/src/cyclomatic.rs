// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

use serde::Serialize;

/// Cyclomatic complexity accumulator.
///
/// Per the rewrite plan §5.2, the language crate decides which syntax
/// constructs are decisions (`if`, `while`, `for`, `case`, `&&`, `||`,
/// `?`, …) and emits an increment via `record_decision`. Min/max/sum/avg
/// across nested spaces are computed in `mehen-metrics`.
///
/// The pre-1.0 implementation lives at `src/metrics/cyclomatic.rs`; the
/// field set here matches it so parity snapshots compare directly.
///
/// `cyclomatic` stores the raw *decision* count for the current space.
/// The published McCabe value is `cyclomatic + 1`. `cyclomatic_sum` is
/// the running total of *McCabe* values across closed spaces; it stays
/// 0 until `finalize_minmax` snapshots the current space.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct CyclomaticStats {
    pub cyclomatic: u32,
    pub min: u32,
    pub max: u32,
    pub cyclomatic_sum: u32,
    pub cyclomatic_average: f64,
    /// Number of spaces folded into `cyclomatic_sum` — used by
    /// `finalize_average` so callers don't have to track nspace
    /// separately.
    pub n: u32,
}

impl CyclomaticStats {
    /// Record one decision point. The `+1` McCabe constant is added at
    /// finalize time; `cyclomatic_sum` aggregates closed-space values.
    pub fn record_decision(&mut self) {
        self.cyclomatic = self.cyclomatic.saturating_add(1);
    }

    /// Combine another space's already-finalized stats into this one.
    pub fn merge(&mut self, other: &CyclomaticStats) {
        self.cyclomatic_sum = self.cyclomatic_sum.saturating_add(other.cyclomatic_sum);
        self.n = self.n.saturating_add(other.n);
        self.min = match (self.min, other.min) {
            (0, b) => b,
            (a, 0) => a,
            (a, b) => a.min(b),
        };
        self.max = self.max.max(other.max);
    }

    /// Compute the average cyclomatic-per-space once `cyclomatic_sum`
    /// has been merged across all spaces.
    pub fn finalize_average(&mut self) {
        self.cyclomatic_average = if self.n == 0 {
            0.0
        } else {
            f64::from(self.cyclomatic_sum) / f64::from(self.n)
        };
    }

    /// Fold the current per-space McCabe value (`decisions + 1`) into
    /// `cyclomatic_sum`, `min`, `max`, and bump `n`. Should be called
    /// once per space before merging into the parent.
    pub fn finalize_minmax(&mut self) {
        let value = self.cyclomatic.saturating_add(1);
        self.cyclomatic_sum = self.cyclomatic_sum.saturating_add(value);
        self.n = self.n.saturating_add(1);
        self.min = if self.min == 0 {
            value
        } else {
            self.min.min(value)
        };
        self.max = self.max.max(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_decision_only_bumps_per_space_count() {
        let mut s = CyclomaticStats::default();
        s.record_decision();
        s.record_decision();
        assert_eq!(s.cyclomatic, 2);
        // sum stays 0 until finalize_minmax snapshots a closed space.
        assert_eq!(s.cyclomatic_sum, 0);
    }

    #[test]
    fn finalize_minmax_publishes_mccabe_value() {
        let mut s = CyclomaticStats::default();
        s.record_decision();
        s.record_decision();
        s.finalize_minmax();
        // 2 decisions + 1 = 3 (McCabe).
        assert_eq!(s.cyclomatic_sum, 3);
        assert_eq!(s.min, 3);
        assert_eq!(s.max, 3);
        assert_eq!(s.n, 1);
    }

    #[test]
    fn merge_preserves_min_max() {
        let mut a = CyclomaticStats {
            cyclomatic_sum: 3,
            min: 3,
            max: 3,
            n: 1,
            ..Default::default()
        };
        let b = CyclomaticStats {
            cyclomatic_sum: 7,
            min: 7,
            max: 7,
            n: 1,
            ..Default::default()
        };
        a.merge(&b);
        assert_eq!(a.cyclomatic_sum, 10);
        assert_eq!(a.min, 3);
        assert_eq!(a.max, 7);
        assert_eq!(a.n, 2);
    }

    #[test]
    fn finalize_average_handles_zero_n() {
        let mut s = CyclomaticStats::default();
        s.finalize_average();
        assert_eq!(s.cyclomatic_average, 0.0);
        s.cyclomatic_sum = 5;
        s.n = 2;
        s.finalize_average();
        assert_eq!(s.cyclomatic_average, 2.5);
    }
}
