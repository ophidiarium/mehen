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
//! The primary public surface in 1.0 is the set of typed accumulator
//! structs — `LocStats`, `CyclomaticStats`, `CognitiveStats`,
//! `HalsteadStats`, `AbcStats`, `NargsStats`, `NomStats`, `NexitStats`,
//! `MiStats`, `WmcStats`, `NpaStats`, `NpmStats` — plus the
//! `HalsteadBuilder` event sink that language crates use to emit
//! token-level operator/operand events.
//!
//! Phase-2 deliverable. The 1.0 first phase wires up the namespace and the
//! Halstead event protocol; per-stat structs land as language crates need
//! them, in [Phase 3 of the rewrite plan].

#![forbid(unsafe_code)]

mod halstead_builder;
mod selector;
mod threshold;

pub use halstead_builder::{HalsteadBuilder, HalsteadCounts, HalsteadOperand, HalsteadOperator};
pub use selector::{MetricSelector, SelectorParseError};
pub use threshold::{Polarity, Threshold, ThresholdEvaluation, ThresholdViolation};

// Re-export the metric key namespace so language crates only need
// `mehen_metrics::keys::*`.
pub use mehen_core::{MetricKey, keys};
