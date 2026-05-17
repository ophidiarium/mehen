use serde::Serialize;

/// Cognitive complexity accumulator.
///
/// Per the rewrite plan §5.2, language crates own cognitive nesting rules,
/// language idioms, readability penalties, and shorthand forms. The math
/// (running sum, min/max/avg across functions) is shared.
///
/// The pre-1.0 implementation lives at `src/metrics/cognitive.rs`; the
/// field set matches it for parity.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct CognitiveStats {
    pub cognitive: u32,
    pub min: u32,
    pub max: u32,
    pub cognitive_sum: u32,
    pub cognitive_average: f64,
}

impl CognitiveStats {
    /// Record `amount` cognitive complexity points for the current space.
    ///
    /// Increments both `cognitive` (the per-space count) and
    /// `cognitive_sum` (the rolling total). Without updating the sum,
    /// merging immediately after a record would silently drop recent
    /// increments.
    pub fn record_increment(&mut self, amount: u32) {
        self.cognitive = self.cognitive.saturating_add(amount);
        self.cognitive_sum = self.cognitive_sum.saturating_add(amount);
    }

    /// Combine another space's stats into this one.
    ///
    /// Preserves min/max bounds across spaces.
    pub fn merge(&mut self, other: &CognitiveStats) {
        self.cognitive_sum = self.cognitive_sum.saturating_add(other.cognitive_sum);
        self.min = match (self.min, other.min) {
            (0, b) => b,
            (a, 0) => a,
            (a, b) => a.min(b),
        };
        self.max = self.max.max(other.max);
    }

    /// Fold the current per-space `cognitive` value into min/max bounds.
    /// Should be called once per space before merging into the parent.
    pub fn finalize_minmax(&mut self) {
        let value = self.cognitive;
        self.min = if self.min == 0 {
            value
        } else {
            self.min.min(value)
        };
        self.max = self.max.max(value);
    }

    pub fn finalize(&mut self, function_count: usize) {
        if function_count == 0 {
            self.cognitive_average = 0.0;
        } else {
            self.cognitive_average = (self.cognitive_sum as f64) / (function_count as f64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_increment_updates_sum() {
        let mut s = CognitiveStats::default();
        s.record_increment(2);
        s.record_increment(3);
        assert_eq!(s.cognitive, 5);
        assert_eq!(s.cognitive_sum, 5);
    }

    #[test]
    fn merge_preserves_min_max() {
        let mut a = CognitiveStats {
            cognitive: 4,
            min: 4,
            max: 4,
            cognitive_sum: 4,
            cognitive_average: 0.0,
        };
        let b = CognitiveStats {
            cognitive: 9,
            min: 9,
            max: 9,
            cognitive_sum: 9,
            cognitive_average: 0.0,
        };
        a.merge(&b);
        assert_eq!(a.min, 4);
        assert_eq!(a.max, 9);
        assert_eq!(a.cognitive_sum, 13);
    }
}
