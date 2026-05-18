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

/// Render the `nexits` family object: `{ sum, average, min, max }`.
///
/// `sum` is the total number of exit points across the rolled-up
/// spaces, `average` divides by the function count (NOM total) — not
/// the space count. `min` and `max` bound the per-space counts.
pub fn nexits(metrics: &MetricSet) -> Nexits {
    Nexits {
        sum: as_f64(metrics, "nexit.sum"),
        average: as_f64(metrics, "nexit.average"),
        min: as_f64(metrics, "nexit.min"),
        max: as_f64(metrics, "nexit.max"),
    }
}

#[derive(Serialize)]
pub struct Nexits {
    pub sum: f64,
    pub average: f64,
    pub min: f64,
    pub max: f64,
}

/// Render the `halstead` family object: 14 fields covering n1/N1/n2/N2,
/// length, estimated_program_length, purity_ratio, vocabulary, volume,
/// difficulty, level, effort, time, and bugs. Field ordering matches
/// the pre-1.0 `halstead::Stats::serialize`.
pub fn halstead(metrics: &MetricSet) -> Halstead {
    Halstead {
        n1: as_f64(metrics, "halstead.n1"),
        big_n1: as_f64(metrics, "halstead.N1"),
        n2: as_f64(metrics, "halstead.n2"),
        big_n2: as_f64(metrics, "halstead.N2"),
        length: as_f64(metrics, "halstead.length"),
        estimated_program_length: as_f64(metrics, "halstead.estimated_program_length"),
        purity_ratio: as_f64(metrics, "halstead.purity_ratio"),
        vocabulary: as_f64(metrics, "halstead.vocabulary"),
        volume: as_f64(metrics, "halstead.volume"),
        difficulty: as_f64(metrics, "halstead.difficulty"),
        level: as_f64(metrics, "halstead.level"),
        effort: as_f64(metrics, "halstead.effort"),
        time: as_f64(metrics, "halstead.time"),
        bugs: as_f64(metrics, "halstead.bugs"),
    }
}

#[derive(Serialize)]
pub struct Halstead {
    pub n1: f64,
    #[serde(rename = "N1")]
    pub big_n1: f64,
    pub n2: f64,
    #[serde(rename = "N2")]
    pub big_n2: f64,
    pub length: f64,
    pub estimated_program_length: f64,
    pub purity_ratio: f64,
    pub vocabulary: f64,
    pub volume: f64,
    pub difficulty: f64,
    pub level: f64,
    pub effort: f64,
    pub time: f64,
    pub bugs: f64,
}

/// Render the `abc` family object: 13 fields covering A/B/C totals,
/// the magnitude formula `sqrt(A² + B² + C²)`, per-class averages, and
/// per-class min/max bounds. Matches the pre-1.0
/// `abc::Stats::serialize` field order.
pub fn abc(metrics: &MetricSet) -> Abc {
    Abc {
        assignments: as_f64(metrics, "abc.assignments"),
        branches: as_f64(metrics, "abc.branches"),
        conditions: as_f64(metrics, "abc.conditions"),
        magnitude: as_f64(metrics, "abc"),
        assignments_average: as_f64(metrics, "abc.assignments_average"),
        branches_average: as_f64(metrics, "abc.branches_average"),
        conditions_average: as_f64(metrics, "abc.conditions_average"),
        assignments_min: as_f64(metrics, "abc.assignments_min"),
        assignments_max: as_f64(metrics, "abc.assignments_max"),
        branches_min: as_f64(metrics, "abc.branches_min"),
        branches_max: as_f64(metrics, "abc.branches_max"),
        conditions_min: as_f64(metrics, "abc.conditions_min"),
        conditions_max: as_f64(metrics, "abc.conditions_max"),
    }
}

#[derive(Serialize)]
pub struct Abc {
    pub assignments: f64,
    pub branches: f64,
    pub conditions: f64,
    pub magnitude: f64,
    pub assignments_average: f64,
    pub branches_average: f64,
    pub conditions_average: f64,
    pub assignments_min: f64,
    pub assignments_max: f64,
    pub branches_min: f64,
    pub branches_max: f64,
    pub conditions_min: f64,
    pub conditions_max: f64,
}

/// Render the `nargs` family object: 10 fields covering per-class
/// argument totals, averages, total, and min/max bounds. Field
/// ordering matches the pre-1.0 `nargs::Stats::serialize`.
pub fn nargs(metrics: &MetricSet) -> Nargs {
    Nargs {
        total_functions: as_f64(metrics, "nargs.total_functions"),
        total_closures: as_f64(metrics, "nargs.total_closures"),
        average_functions: as_f64(metrics, "nargs.average_functions"),
        average_closures: as_f64(metrics, "nargs.average_closures"),
        total: as_f64(metrics, "nargs"),
        average: as_f64(metrics, "nargs.average"),
        functions_min: as_f64(metrics, "nargs.functions_min"),
        functions_max: as_f64(metrics, "nargs.functions_max"),
        closures_min: as_f64(metrics, "nargs.closures_min"),
        closures_max: as_f64(metrics, "nargs.closures_max"),
    }
}

#[derive(Serialize)]
pub struct Nargs {
    pub total_functions: f64,
    pub total_closures: f64,
    pub average_functions: f64,
    pub average_closures: f64,
    pub total: f64,
    pub average: f64,
    pub functions_min: f64,
    pub functions_max: f64,
    pub closures_min: f64,
    pub closures_max: f64,
}

/// Render the `nom` family object: 10 fields covering function /
/// closure counts, per-class averages, total, and per-class min/max
/// bounds. Field ordering matches the pre-1.0 `Nom::Stats::serialize`.
pub fn nom(metrics: &MetricSet) -> Nom {
    Nom {
        functions: as_f64(metrics, "nom.functions"),
        closures: as_f64(metrics, "nom.closures"),
        functions_average: as_f64(metrics, "nom.functions_average"),
        closures_average: as_f64(metrics, "nom.closures_average"),
        total: as_f64(metrics, "nom"),
        average: as_f64(metrics, "nom.average"),
        functions_min: as_f64(metrics, "nom.functions_min"),
        functions_max: as_f64(metrics, "nom.functions_max"),
        closures_min: as_f64(metrics, "nom.closures_min"),
        closures_max: as_f64(metrics, "nom.closures_max"),
    }
}

#[derive(Serialize)]
pub struct Nom {
    pub functions: f64,
    pub closures: f64,
    pub functions_average: f64,
    pub closures_average: f64,
    pub total: f64,
    pub average: f64,
    pub functions_min: f64,
    pub functions_max: f64,
    pub closures_min: f64,
    pub closures_max: f64,
}

/// Render the `loc` family object: 20 fields covering SLOC / PLOC /
/// LLOC / CLOC / blank with rolled-up totals, per-line-class
/// averages, and per-line-class min/max bounds. The ordering matches
/// the pre-1.0 `Loc::Stats::serialize` field order.
pub fn loc(metrics: &MetricSet) -> Loc {
    Loc {
        sloc: as_f64(metrics, "loc.sloc"),
        ploc: as_f64(metrics, "loc.ploc"),
        lloc: as_f64(metrics, "loc.lloc"),
        cloc: as_f64(metrics, "loc.cloc"),
        blank: as_f64(metrics, "loc.blank"),
        sloc_average: as_f64(metrics, "loc.sloc.avg"),
        ploc_average: as_f64(metrics, "loc.ploc.avg"),
        lloc_average: as_f64(metrics, "loc.lloc.avg"),
        cloc_average: as_f64(metrics, "loc.cloc.avg"),
        blank_average: as_f64(metrics, "loc.blank.avg"),
        sloc_min: as_f64(metrics, "loc.sloc.min"),
        sloc_max: as_f64(metrics, "loc.sloc.max"),
        cloc_min: as_f64(metrics, "loc.cloc.min"),
        cloc_max: as_f64(metrics, "loc.cloc.max"),
        ploc_min: as_f64(metrics, "loc.ploc.min"),
        ploc_max: as_f64(metrics, "loc.ploc.max"),
        lloc_min: as_f64(metrics, "loc.lloc.min"),
        lloc_max: as_f64(metrics, "loc.lloc.max"),
        blank_min: as_f64(metrics, "loc.blank.min"),
        blank_max: as_f64(metrics, "loc.blank.max"),
    }
}

#[derive(Serialize)]
pub struct Loc {
    pub sloc: f64,
    pub ploc: f64,
    pub lloc: f64,
    pub cloc: f64,
    pub blank: f64,
    pub sloc_average: f64,
    pub ploc_average: f64,
    pub lloc_average: f64,
    pub cloc_average: f64,
    pub blank_average: f64,
    pub sloc_min: f64,
    pub sloc_max: f64,
    pub cloc_min: f64,
    pub cloc_max: f64,
    pub ploc_min: f64,
    pub ploc_max: f64,
    pub lloc_min: f64,
    pub lloc_max: f64,
    pub blank_min: f64,
    pub blank_max: f64,
}

fn as_f64(metrics: &MetricSet, key: &str) -> f64 {
    match metrics.get(&MetricKey::new(key)) {
        Some(MetricValue::Int(i)) => i as f64,
        Some(MetricValue::Float(f)) => f,
        None => 0.0,
    }
}
