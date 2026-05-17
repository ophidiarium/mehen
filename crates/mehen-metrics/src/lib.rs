//! `mehen-metrics` — shared metric contracts, formulas, accumulators, and
//! aggregation helpers.
//!
//! This crate must not become a central all-language calculator. Per the
//! rewrite plan §4.3:
//! - It owns the metric *math* (Halstead volume, MI formulas, min/max/avg
//!   finalization, set-based n1/n2 dedup, …).
//! - It does not own language interpretation (which Python AST nodes are
//!   decisions, whether a Ruby rescue modifier counts toward cognitive,
//!   etc.).
//!
//! Phase 1 ships the typed accumulator surface — `LocStats`,
//! `CyclomaticStats`, `CognitiveStats`, `HalsteadStats`, `AbcStats`,
//! `NargsStats`, `NomStats`, `NexitStats`, `MiStats`, `WmcStats`,
//! `NpaStats`, `NpmStats` — plus the `HalsteadBuilder` event sink and the
//! `MetricTreeBuilder` helper that language crates use to assemble a
//! `MetricSpace` tree without each crate re-implementing id allocation.

#![forbid(unsafe_code)]

mod abc;
mod cognitive;
mod counters;
mod cyclomatic;
mod halstead;
mod halstead_builder;
mod loc;
mod mi;
mod selector;
mod threshold;
mod tree_builder;

pub use abc::AbcStats;
pub use cognitive::CognitiveStats;
pub use counters::{NargsStats, NexitStats, NomStats, NpaStats, NpmStats, WmcStats};
pub use cyclomatic::CyclomaticStats;
pub use halstead::HalsteadStats;
pub use halstead_builder::{HalsteadBuilder, HalsteadCounts, HalsteadOperand, HalsteadOperator};
pub use loc::{LineClass, LocStats};
pub use mi::MiStats;
pub use selector::{MetricSelector, SelectorAggregator, SelectorParseError};
pub use threshold::{Polarity, Threshold, ThresholdEvaluation, ThresholdViolation};
pub use tree_builder::MetricTreeBuilder;

// Re-export the metric key namespace so language crates only need
// `mehen_metrics::keys::*`.
pub use mehen_core::{MetricKey, keys};
