// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Shared §4 helpers used across Phase C modules.
//!
//! `clamp01` and `sat` appear in nearly every formula from §§11–19; rather
//! than duplicate them in every submodule we expose them here. All metric
//! code should prefer these over inline implementations so behavior stays
//! consistent if we ever revisit NaN handling.

/// `clamp01(x) = min(1, max(0, x))` per §4.
#[inline]
pub(crate) fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

/// `sat(x; lo, hi) = clamp01((x - lo) / (hi - lo))` per §4. Degenerate
/// ranges (`hi <= lo`) snap to 1.0 once `x` crosses the threshold.
#[inline]
pub(crate) fn sat(x: f64, lo: f64, hi: f64) -> f64 {
    if hi <= lo {
        return if x >= hi { 1.0 } else { 0.0 };
    }
    clamp01((x - lo) / (hi - lo))
}

/// Normalizes `-0.0` to `+0.0` so YAML / JSON emitters never emit the
/// negative-zero variant in snapshots. IEEE 754 allows `-0.0 + 0.0` to
/// collapse to `+0.0`, but we guard explicitly for determinism on every
/// platform.
#[inline]
pub(crate) fn normalize_zero(x: f64) -> f64 {
    if x == 0.0 { 0.0 } else { x }
}
