use serde::Serialize;

/// Number of arguments accumulator (NArgs).
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct NargsStats {
    pub functions: u32,
    pub closures: u32,
    pub min: u32,
    pub max: u32,
}

impl NargsStats {
    pub fn record_function_args(&mut self, count: u32) {
        self.functions = self.functions.saturating_add(count);
        self.update_bounds(count);
    }

    pub fn record_closure_args(&mut self, count: u32) {
        self.closures = self.closures.saturating_add(count);
        self.update_bounds(count);
    }

    fn update_bounds(&mut self, count: u32) {
        // `min` is sentinel-encoded with 0 because there is no way for a
        // function to legitimately have "negative one args" — the first
        // observation seeds the bound.
        self.min = if self.min == 0 {
            count
        } else {
            self.min.min(count)
        };
        self.max = self.max.max(count);
    }

    pub fn merge(&mut self, other: &NargsStats) {
        self.functions = self.functions.saturating_add(other.functions);
        self.closures = self.closures.saturating_add(other.closures);
        self.min = match (self.min, other.min) {
            (0, b) => b,
            (a, 0) => a,
            (a, b) => a.min(b),
        };
        self.max = self.max.max(other.max);
    }
}

/// Number of methods/functions (NOM) accumulator.
///
/// `functions`/`closures` track the per-space count (number of nested
/// function/closure spaces directly opened from this one). `*_sum` are
/// the running totals across closed spaces; `finalize_minmax` snapshots
/// the per-space values into the bounds and adds them into `*_sum`.
/// `space_count` is bumped at the same time so averages divide by the
/// total number of spaces folded in.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct NomStats {
    pub functions: u32,
    pub closures: u32,
    pub functions_sum: u32,
    pub closures_sum: u32,
    pub functions_min: u32,
    pub functions_max: u32,
    pub closures_min: u32,
    pub closures_max: u32,
    pub space_count: u32,
    /// Sentinel — set on first finalize so 0-valued bounds don't get
    /// overwritten on subsequent finalizes.
    pub minmax_seen: bool,
}

impl NomStats {
    pub fn record_function(&mut self) {
        self.functions = self.functions.saturating_add(1);
    }

    pub fn record_closure(&mut self) {
        self.closures = self.closures.saturating_add(1);
    }

    /// Fold the current per-space `functions`/`closures` values into
    /// `*_sum`, `*_min`, `*_max` and bump `space_count`. Called once
    /// per space before merging into the parent.
    pub fn finalize_minmax(&mut self) {
        self.functions_sum = self.functions_sum.saturating_add(self.functions);
        self.closures_sum = self.closures_sum.saturating_add(self.closures);
        self.space_count = self.space_count.saturating_add(1);
        if self.minmax_seen {
            self.functions_min = self.functions_min.min(self.functions);
            self.closures_min = self.closures_min.min(self.closures);
        } else {
            self.functions_min = self.functions;
            self.closures_min = self.closures;
            self.minmax_seen = true;
        }
        self.functions_max = self.functions_max.max(self.functions);
        self.closures_max = self.closures_max.max(self.closures);
    }

    pub fn merge(&mut self, other: &NomStats) {
        self.functions_sum = self.functions_sum.saturating_add(other.functions_sum);
        self.closures_sum = self.closures_sum.saturating_add(other.closures_sum);
        self.space_count = self.space_count.saturating_add(other.space_count);
        if !other.minmax_seen {
            return;
        }
        if self.minmax_seen {
            self.functions_min = self.functions_min.min(other.functions_min);
            self.closures_min = self.closures_min.min(other.closures_min);
        } else {
            self.functions_min = other.functions_min;
            self.closures_min = other.closures_min;
            self.minmax_seen = true;
        }
        self.functions_max = self.functions_max.max(other.functions_max);
        self.closures_max = self.closures_max.max(other.closures_max);
    }

    /// `functions_sum + closures_sum` — the rolled-up total across all
    /// folded spaces. Used as the average denominator for cognitive,
    /// nexit, and nargs (per `mehen-engine::legacy::spaces::compute_averages`).
    pub fn total(&self) -> u32 {
        self.functions_sum.saturating_add(self.closures_sum)
    }

    pub fn functions_average(&self) -> f64 {
        average(self.functions_sum, self.space_count)
    }
    pub fn closures_average(&self) -> f64 {
        average(self.closures_sum, self.space_count)
    }
    pub fn average(&self) -> f64 {
        average(self.total(), self.space_count)
    }
}

fn average(numerator: u32, denominator: u32) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        f64::from(numerator) / f64::from(denominator)
    }
}

/// Number of exits (return/throw/raise/exit) accumulator.
///
/// Per the pre-1.0 `src/metrics/exit.rs` and the rewrite plan §5.2:
/// language crates decide which constructs are exits. The accumulator
/// keeps the per-space `exits` count (raw, not McCabe-style); on space
/// close `finalize_minmax` snapshots that into `sum`/`min`/`max`. The
/// `average` denominator is the function count (NOM total), not the
/// space count — set externally via `finalize_average(function_count)`.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct NexitStats {
    pub exits: u32,
    pub min: u32,
    pub max: u32,
    pub average: f64,
    pub sum: u32,
    /// `true` once `finalize_minmax` has snapshotted at least one space
    /// — used as the "min initialized" sentinel so the first close sets
    /// `min`, even when its value is 0.
    pub minmax_seen: bool,
}

impl NexitStats {
    /// Record one exit point. The `+0` constant for sum aggregation is
    /// added at finalize time; `sum` stays 0 until `finalize_minmax`
    /// snapshots a closed space.
    pub fn record_exit(&mut self) {
        self.exits = self.exits.saturating_add(1);
    }

    /// Fold the current per-space `exits` value into `sum`, `min`,
    /// `max`. Should be called once per space before merging into the
    /// parent.
    pub fn finalize_minmax(&mut self) {
        let value = self.exits;
        self.sum = self.sum.saturating_add(value);
        if self.minmax_seen {
            self.min = self.min.min(value);
        } else {
            self.min = value;
            self.minmax_seen = true;
        }
        self.max = self.max.max(value);
    }

    /// Compute the average exits per function once `sum` has been
    /// merged across all spaces. The denominator is the **NOM total**
    /// (functions + closures), not the space count.
    pub fn finalize_average(&mut self, function_count: u32) {
        self.average = if function_count == 0 {
            0.0
        } else {
            f64::from(self.sum) / f64::from(function_count)
        };
    }

    pub fn merge(&mut self, other: &NexitStats) {
        self.sum = self.sum.saturating_add(other.sum);
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
}

/// Number of public attributes accumulator (NPA).
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct NpaStats {
    pub public: u32,
    pub total: u32,
}

impl NpaStats {
    pub fn record_attribute(&mut self, public: bool) {
        self.total = self.total.saturating_add(1);
        if public {
            self.public = self.public.saturating_add(1);
        }
    }

    pub fn merge(&mut self, other: &NpaStats) {
        self.public = self.public.saturating_add(other.public);
        self.total = self.total.saturating_add(other.total);
    }
}

/// Number of public methods accumulator (NPM).
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct NpmStats {
    pub public: u32,
    pub total: u32,
}

impl NpmStats {
    pub fn record_method(&mut self, public: bool) {
        self.total = self.total.saturating_add(1);
        if public {
            self.public = self.public.saturating_add(1);
        }
    }

    pub fn merge(&mut self, other: &NpmStats) {
        self.public = self.public.saturating_add(other.public);
        self.total = self.total.saturating_add(other.total);
    }
}

/// Weighted methods per class accumulator (WMC).
///
/// WMC sums the cyclomatic complexity of every method on a class. Stored
/// as a u32 so language crates can fold per-method cyclomatic into one
/// per-class counter without re-walking the tree.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct WmcStats {
    pub wmc: u32,
}

impl WmcStats {
    pub fn record_method_cyclomatic(&mut self, cyclomatic: u32) {
        self.wmc = self.wmc.saturating_add(cyclomatic);
    }

    pub fn merge(&mut self, other: &WmcStats) {
        self.wmc = self.wmc.saturating_add(other.wmc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nargs_record_updates_bounds() {
        let mut s = NargsStats::default();
        s.record_function_args(3);
        s.record_function_args(1);
        s.record_function_args(5);
        assert_eq!(s.min, 1);
        assert_eq!(s.max, 5);
    }

    #[test]
    fn nargs_merge_combines_bounds() {
        let mut a = NargsStats {
            functions: 3,
            closures: 0,
            min: 1,
            max: 3,
        };
        let b = NargsStats {
            functions: 5,
            closures: 0,
            min: 5,
            max: 5,
        };
        a.merge(&b);
        assert_eq!(a.min, 1);
        assert_eq!(a.max, 5);
        assert_eq!(a.functions, 8);
    }

    #[test]
    fn nexit_record_exit_only_bumps_per_space_count() {
        let mut s = NexitStats::default();
        s.record_exit();
        s.record_exit();
        assert_eq!(s.exits, 2);
        // sum stays 0 until finalize_minmax snapshots a closed space.
        assert_eq!(s.sum, 0);
    }

    #[test]
    fn nexit_finalize_minmax_snapshots_per_space_count_into_sum() {
        let mut s = NexitStats::default();
        s.record_exit();
        s.record_exit();
        s.finalize_minmax();
        assert_eq!(s.sum, 2);
        assert_eq!(s.min, 2);
        assert_eq!(s.max, 2);
    }

    #[test]
    fn nexit_finalize_average_handles_zero() {
        let mut s = NexitStats {
            sum: 6,
            ..Default::default()
        };
        s.finalize_average(0);
        assert_eq!(s.average, 0.0);
        s.finalize_average(3);
        assert_eq!(s.average, 2.0);
    }
}
