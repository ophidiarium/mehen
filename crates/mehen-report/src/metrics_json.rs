//! Per-metric JSON renderer.
//!
//! Per the rewrite plan §9.1, `mehen metrics --format json` emits a
//! report whose `metrics` field is an object keyed by the metric family
//! (`cyclomatic`, `cognitive`, `halstead`, `loc`, …). Each family is a
//! nested object with the family-specific roll-up fields (`sum`,
//! `average`, `min`, `max` for cyclomatic / cognitive; `n1` / `N1` /
//! `volume` / … for Halstead; etc.).
//!
//! The new `MetricSpace::metrics` map keeps each numeric value at its own
//! flat key (`cyclomatic.sum`, `cyclomatic.min`, `loc.sloc.avg`, …) so
//! selectors can reference any individual aggregator. This module pivots
//! that flat shape back into the documented per-family object so the JSON
//! report matches the published schema.
//!
//! Per family is added here as the corresponding analyzer crate reaches
//! parity with that metric, so each family becomes consumable from the
//! report layer in lockstep with the per-language port (rewrite plan
//! §8.2).

use mehen_core::{MetricKey, MetricSet, MetricValue};
use serde::Serialize;

/// Render the `cyclomatic` family object: `{ sum, average, min, max }`.
///
/// Reads the rolled-up values published by the shared walker
/// (`mehen-tree-sitter::walker::apply_state_to`) at
/// `cyclomatic.sum` / `.avg` / `.min` / `.max`. Integer counts surface
/// as integer-valued floats so the JSON shape is uniform regardless of
/// the underlying numeric variant.
///
/// Field declaration order on the typed struct is the JSON output order
/// — `sum`, `average`, `min`, `max` — matching the documented schema.
pub fn cyclomatic(metrics: &MetricSet) -> Cyclomatic {
    Cyclomatic {
        sum: as_f64(metrics, "cyclomatic.sum"),
        average: as_f64(metrics, "cyclomatic.avg"),
        min: as_f64(metrics, "cyclomatic.min"),
        max: as_f64(metrics, "cyclomatic.max"),
    }
}

#[derive(Serialize)]
pub struct Cyclomatic {
    pub sum: f64,
    pub average: f64,
    pub min: f64,
    pub max: f64,
}

fn as_f64(metrics: &MetricSet, key: &str) -> f64 {
    match metrics.get(&MetricKey::new(key)) {
        Some(MetricValue::Int(i)) => i as f64,
        Some(MetricValue::Float(f)) => f,
        None => 0.0,
    }
}
