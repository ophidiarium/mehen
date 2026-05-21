// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

use serde::Serialize;

/// Cognitive complexity accumulator.
///
/// Mirrors the pre-1.0 `cognitive::Stats`. Per-space `structural` is
/// the running cognitive count; `cognitive_sum` is the rolled-up total
/// across closed spaces; `min`/`max` are per-space bounds. Averages
/// divide by the function count (NOM total). The accumulator also
/// carries the `nesting` counter (used by `increase_nesting`) and the
/// `BoolSequence` state machine that collapses same-operator boolean
/// runs per Sonar's whitepaper.
///
/// `cognitive` is exposed as a field for backwards compatibility with
/// existing callers; it mirrors `structural` (the running per-space
/// count).
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct CognitiveStats {
    pub cognitive: u32,
    pub structural: u32,
    pub nesting: u32,
    pub min: u32,
    pub max: u32,
    pub cognitive_sum: u32,
    pub cognitive_average: f64,
    pub boolean_seq: BoolSequence,
    pub minmax_seen: bool,
}

/// Same-operator sequence collapser per Sonar's whitepaper. Each
/// observed boolean operator is compared against the last; same kind
/// adds nothing, different kind (or first occurrence) adds +1. Reset
/// at statement boundaries (assignment, pipeline, control-flow
/// clause).
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct BoolSequence {
    /// Stable string identifier for the most recent boolean operator.
    /// `None` when the sequence has been reset or not yet started.
    pub last_op: Option<smol_str::SmolStr>,
}

impl BoolSequence {
    pub fn reset(&mut self) {
        self.last_op = None;
    }

    pub fn not_operator(&mut self, not_id: &str) {
        self.last_op = Some(smol_str::SmolStr::new(not_id));
    }

    /// Update `structural` and the recorded last-op based on the new
    /// boolean operator. Returns the new structural value.
    pub fn eval_based_on_prev(&mut self, op_id: &str, structural: u32) -> u32 {
        let new_value = if let Some(prev) = &self.last_op {
            if prev.as_str() != op_id {
                structural.saturating_add(1)
            } else {
                structural
            }
        } else {
            structural.saturating_add(1)
        };
        self.last_op = Some(smol_str::SmolStr::new(op_id));
        new_value
    }
}

impl CognitiveStats {
    /// Record `amount` cognitive complexity points for the current
    /// space. Adds to `structural` (and the legacy `cognitive` mirror).
    pub fn record_increment(&mut self, amount: u32) {
        self.structural = self.structural.saturating_add(amount);
        self.cognitive = self.structural;
    }

    /// Add `nesting + 1` to the structural count. Mirrors the pre-1.0
    /// `increment(stats)` (which used `stats.structural += stats.nesting + 1`).
    pub fn increase_nesting(&mut self, nesting: u32) {
        self.nesting = nesting;
        let bump = nesting.saturating_add(1);
        self.structural = self.structural.saturating_add(bump);
        self.cognitive = self.structural;
    }

    /// Add 1 to the structural count without touching nesting. Used for
    /// `elseif`, `else`, `finally`, `trap` clauses.
    pub fn increment_by_one(&mut self) {
        self.structural = self.structural.saturating_add(1);
        self.cognitive = self.structural;
    }

    /// Feed one boolean operator through the BoolSequence collapser.
    /// Updates `structural` according to the same-op vs. transition
    /// rule, mirroring the pre-1.0
    /// `stats.structural = boolean_seq.eval_based_on_prev(...)`.
    pub fn observe_boolean(&mut self, op_id: &str) {
        self.structural = self.boolean_seq.eval_based_on_prev(op_id, self.structural);
        self.cognitive = self.structural;
    }

    /// Combine another space's stats into this one.
    pub fn merge(&mut self, other: &CognitiveStats) {
        self.cognitive_sum = self.cognitive_sum.saturating_add(other.cognitive_sum);
        if !other.minmax_seen {
            return;
        }
        if self.minmax_seen {
            self.min = self.min.min(other.min);
        } else {
            self.min = other.min;
            self.minmax_seen = true;
        }
        self.max = self.max.max(other.max);
    }

    /// Fold the current per-space `structural` into `cognitive_sum` /
    /// min / max. Should be called once per space before merging into
    /// the parent.
    pub fn finalize_minmax(&mut self) {
        let value = self.structural;
        self.cognitive_sum = self.cognitive_sum.saturating_add(value);
        if self.minmax_seen {
            self.min = self.min.min(value);
        } else {
            self.min = value;
            self.minmax_seen = true;
        }
        self.max = self.max.max(value);
    }

    /// Compute `cognitive_average = cognitive_sum / function_count`.
    pub fn finalize(&mut self, function_count: u32) {
        if function_count == 0 {
            self.cognitive_average = 0.0;
        } else {
            self.cognitive_average = f64::from(self.cognitive_sum) / f64::from(function_count);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_increment_only_bumps_per_space_count() {
        let mut s = CognitiveStats::default();
        s.record_increment(2);
        s.record_increment(3);
        assert_eq!(s.structural, 5);
        assert_eq!(s.cognitive, 5);
        // sum stays 0 until finalize_minmax snapshots a closed space.
        assert_eq!(s.cognitive_sum, 0);
    }

    #[test]
    fn finalize_minmax_snapshots_per_space_value() {
        let mut s = CognitiveStats::default();
        s.record_increment(5);
        s.finalize_minmax();
        assert_eq!(s.cognitive_sum, 5);
        assert_eq!(s.min, 5);
        assert_eq!(s.max, 5);
    }

    #[test]
    fn merge_preserves_min_max() {
        let mut a = CognitiveStats::default();
        a.record_increment(4);
        a.finalize_minmax();
        let mut b = CognitiveStats::default();
        b.record_increment(9);
        b.finalize_minmax();
        a.merge(&b);
        assert_eq!(a.min, 4);
        assert_eq!(a.max, 9);
        assert_eq!(a.cognitive_sum, 13);
    }

    #[test]
    fn boolean_sequence_collapses_same_operator() {
        let mut s = CognitiveStats::default();
        s.observe_boolean("-and"); // first → +1
        s.observe_boolean("-and"); // same → no bump
        s.observe_boolean("-or"); // transition → +1
        assert_eq!(s.structural, 2);
    }
}
