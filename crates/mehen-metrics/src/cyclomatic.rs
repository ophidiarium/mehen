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
    pub fn record_decision(&mut self) {
        self.cyclomatic = self.cyclomatic.saturating_add(1);
    }

    pub fn merge(&mut self, other: &CyclomaticStats) {
        self.cyclomatic_sum = self.cyclomatic_sum.saturating_add(other.cyclomatic_sum);
    }

    pub fn finalize_minmax(&mut self) {
        self.min = self.min.min(self.cyclomatic).max(1);
        self.max = self.max.max(self.cyclomatic);
    }
}
