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
    pub fn record_increment(&mut self, amount: u32) {
        self.cognitive = self.cognitive.saturating_add(amount);
    }

    pub fn merge(&mut self, other: &CognitiveStats) {
        self.cognitive_sum = self.cognitive_sum.saturating_add(other.cognitive_sum);
    }

    pub fn finalize(&mut self, function_count: usize) {
        if function_count == 0 {
            self.cognitive_average = 0.0;
        } else {
            self.cognitive_average = (self.cognitive_sum as f64) / (function_count as f64);
        }
    }
}
