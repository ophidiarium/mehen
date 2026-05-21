// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

use serde::{Deserialize, Serialize};

use crate::selector::MetricSelector;

/// Whether higher values of a metric are worse (`HigherIsWorse`) or better
/// (`HigherIsBetter`). Per the rewrite plan §5.1 this lives with the metric
/// contract because the same number means different things across metrics:
/// `cognitive` going up is bad, `mi.visual_studio` going up is good.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Polarity {
    HigherIsWorse,
    HigherIsBetter,
}

/// A user-supplied threshold rule.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Threshold {
    pub selector: MetricSelector,
    /// Limit value. The polarity decides which side of `value` is a
    /// violation.
    pub value: f64,
    pub polarity: Polarity,
}

impl Threshold {
    pub fn new(selector: MetricSelector, value: f64, polarity: Polarity) -> Self {
        Self {
            selector,
            value,
            polarity,
        }
    }

    /// True when `actual` violates this threshold.
    pub fn violated_by(&self, actual: f64) -> bool {
        match self.polarity {
            Polarity::HigherIsWorse => actual > self.value,
            Polarity::HigherIsBetter => actual < self.value,
        }
    }
}

/// Result of evaluating one threshold against an actual measurement.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThresholdEvaluation {
    pub selector: MetricSelector,
    pub actual: f64,
    pub limit: f64,
    pub polarity: Polarity,
    pub violated: bool,
}

/// Convenience violation envelope. Used in `mehen diff --format json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThresholdViolation {
    pub path: String,
    pub evaluation: ThresholdEvaluation,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel(s: &str) -> MetricSelector {
        s.parse().unwrap()
    }

    #[test]
    fn higher_is_worse_violation() {
        let t = Threshold::new(sel("cognitive"), 5.0, Polarity::HigherIsWorse);
        assert!(!t.violated_by(5.0));
        assert!(t.violated_by(5.1));
        assert!(!t.violated_by(0.0));
    }

    #[test]
    fn higher_is_better_violation() {
        let t = Threshold::new(sel("mi.visual_studio"), 50.0, Polarity::HigherIsBetter);
        assert!(!t.violated_by(50.0));
        assert!(t.violated_by(49.9));
    }
}
