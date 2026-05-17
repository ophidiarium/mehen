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
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct CyclomaticStats {
    pub cyclomatic: u32,
    pub min: u32,
    pub max: u32,
    pub cyclomatic_sum: u32,
    pub cyclomatic_average: f64,
}

impl CyclomaticStats {
    /// Record one decision point. Increments both `cyclomatic` (the
    /// per-space count) and `cyclomatic_sum` (the rolling total used by
    /// `merge` and `finalize_average`) so a `merge` immediately after a
    /// `record_decision` does not lose the latest decisions.
    pub fn record_decision(&mut self) {
        self.cyclomatic = self.cyclomatic.saturating_add(1);
        self.cyclomatic_sum = self.cyclomatic_sum.saturating_add(1);
    }

    /// Combine another space's stats into this one.
    ///
    /// Preserves min/max bounds and the sum so report-level aggregations
    /// (max-of-spaces, avg-of-spaces) reflect every contribution.
    pub fn merge(&mut self, other: &CyclomaticStats) {
        self.cyclomatic_sum = self.cyclomatic_sum.saturating_add(other.cyclomatic_sum);
        self.min = match (self.min, other.min) {
            (0, b) => b,
            (a, 0) => a,
            (a, b) => a.min(b),
        };
        self.max = self.max.max(other.max);
    }

    /// Compute the average cyclomatic per function once `cyclomatic_sum`
    /// has been merged across all spaces.
    pub fn finalize_average(&mut self, function_count: u32) {
        self.cyclomatic_average = if function_count == 0 {
            0.0
        } else {
            f64::from(self.cyclomatic_sum) / f64::from(function_count)
        };
    }

    /// Fold the current per-space `cyclomatic` value into min/max bounds.
    /// Should be called once per space before merging into the parent.
    pub fn finalize_minmax(&mut self) {
        let value = self.cyclomatic.max(1);
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
    fn record_decision_updates_sum_and_per_space() {
        let mut s = CyclomaticStats::default();
        s.record_decision();
        s.record_decision();
        assert_eq!(s.cyclomatic, 2);
        assert_eq!(s.cyclomatic_sum, 2);
    }

    #[test]
    fn merge_preserves_min_max() {
        let mut a = CyclomaticStats {
            cyclomatic: 3,
            min: 3,
            max: 3,
            cyclomatic_sum: 3,
            cyclomatic_average: 0.0,
        };
        let b = CyclomaticStats {
            cyclomatic: 7,
            min: 7,
            max: 7,
            cyclomatic_sum: 7,
            cyclomatic_average: 0.0,
        };
        a.merge(&b);
        assert_eq!(a.cyclomatic_sum, 10);
        assert_eq!(a.min, 3);
        assert_eq!(a.max, 7);
    }

    #[test]
    fn finalize_average_handles_zero_functions() {
        let mut s = CyclomaticStats {
            cyclomatic_sum: 5,
            ..Default::default()
        };
        s.finalize_average(0);
        assert_eq!(s.cyclomatic_average, 0.0);
        s.finalize_average(2);
        assert_eq!(s.cyclomatic_average, 2.5);
    }
}
