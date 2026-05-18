use serde::Serialize;

/// Number of arguments accumulator (NArgs).
///
/// Mirrors the pre-1.0 `nargs::Stats`. Per-space, `fn_nargs` /
/// `closure_nargs` hold the arg count of that function or closure
/// space (set once when the space opens); `*_sum` are the rolled-up
/// totals across closed spaces; `*_min` / `*_max` are bounds. Averages
/// divide by the function / closure counts (NOM totals), set via
/// `finalize_average`.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct NargsStats {
    pub fn_nargs: u32,
    pub closure_nargs: u32,
    pub fn_nargs_sum: u32,
    pub closure_nargs_sum: u32,
    pub fn_nargs_min: u32,
    pub fn_nargs_max: u32,
    pub closure_nargs_min: u32,
    pub closure_nargs_max: u32,
    pub fn_nargs_average: f64,
    pub closure_nargs_average: f64,
    pub minmax_seen: bool,
}

impl NargsStats {
    /// Set the function arg count for this space. Called once when a
    /// `Function` space opens.
    pub fn record_function_args(&mut self, count: u32) {
        self.fn_nargs = count;
    }

    /// Set the closure arg count for this space. Called once when a
    /// `Closure` space opens.
    pub fn record_closure_args(&mut self, count: u32) {
        self.closure_nargs = count;
    }

    /// Snapshot the per-space `fn_nargs` / `closure_nargs` into `*_sum`,
    /// `*_min`, `*_max`. Mirrors the pre-1.0 `compute_minmax`.
    pub fn finalize_minmax(&mut self) {
        self.fn_nargs_sum = self.fn_nargs_sum.saturating_add(self.fn_nargs);
        self.closure_nargs_sum = self.closure_nargs_sum.saturating_add(self.closure_nargs);
        if self.minmax_seen {
            self.fn_nargs_min = self.fn_nargs_min.min(self.fn_nargs);
            self.closure_nargs_min = self.closure_nargs_min.min(self.closure_nargs);
        } else {
            self.fn_nargs_min = self.fn_nargs;
            self.closure_nargs_min = self.closure_nargs;
            self.minmax_seen = true;
        }
        self.fn_nargs_max = self.fn_nargs_max.max(self.fn_nargs);
        self.closure_nargs_max = self.closure_nargs_max.max(self.closure_nargs);
    }

    /// Compute averages once `*_sum` has been merged across all spaces.
    /// Divides by the NOM `functions_sum` and `closures_sum`
    /// respectively; both fall back to `1` when the count is zero
    /// (matching the pre-1.0 `total_functions.max(1)` guard).
    pub fn finalize_average(&mut self, function_count: u32, closure_count: u32) {
        let fn_denom = function_count.max(1);
        let cl_denom = closure_count.max(1);
        self.fn_nargs_average = f64::from(self.fn_nargs_sum) / f64::from(fn_denom);
        self.closure_nargs_average = f64::from(self.closure_nargs_sum) / f64::from(cl_denom);
    }

    pub fn merge(&mut self, other: &NargsStats) {
        self.fn_nargs_sum = self.fn_nargs_sum.saturating_add(other.fn_nargs_sum);
        self.closure_nargs_sum = self
            .closure_nargs_sum
            .saturating_add(other.closure_nargs_sum);
        if !other.minmax_seen {
            return;
        }
        if self.minmax_seen {
            self.fn_nargs_min = self.fn_nargs_min.min(other.fn_nargs_min);
            self.closure_nargs_min = self.closure_nargs_min.min(other.closure_nargs_min);
        } else {
            self.fn_nargs_min = other.fn_nargs_min;
            self.closure_nargs_min = other.closure_nargs_min;
            self.minmax_seen = true;
        }
        self.fn_nargs_max = self.fn_nargs_max.max(other.fn_nargs_max);
        self.closure_nargs_max = self.closure_nargs_max.max(other.closure_nargs_max);
    }

    pub fn total(&self) -> u32 {
        self.fn_nargs_sum.saturating_add(self.closure_nargs_sum)
    }

    pub fn nargs_average(&self, function_count: u32, closure_count: u32) -> f64 {
        let denom = function_count.saturating_add(closure_count).max(1);
        f64::from(self.total()) / f64::from(denom)
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
///
/// Mirrors the pre-1.0 `npa::Stats`. Tracks per-class and per-interface
/// public-attribute counts plus the totals; the rolled-up CDA (Class
/// Data Accessibility) is `class_npa_sum / class_na_sum` and similarly
/// for interfaces. `class_*` increment when the enclosing space is
/// `Class` / `Impl`; `interface_*` increment when the enclosing space
/// is `Interface` / `Trait`. Languages without class-like constructs
/// flip `not_applicable` so the metric is omitted from output.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct NpaStats {
    pub class_npa: u32,
    pub interface_npa: u32,
    pub class_na: u32,
    pub interface_na: u32,
    pub class_npa_sum: u32,
    pub interface_npa_sum: u32,
    pub class_na_sum: u32,
    pub interface_na_sum: u32,
    pub not_applicable: bool,
    pub has_class_like: bool,
}

impl NpaStats {
    /// Record one attribute observation. `container` is the kind of
    /// the enclosing class-like or interface-like space; pass other
    /// kinds to skip recording.
    pub fn record_attribute(&mut self, container: ContainerKind, is_public: bool) {
        match container {
            ContainerKind::Class => {
                self.class_na = self.class_na.saturating_add(1);
                if is_public {
                    self.class_npa = self.class_npa.saturating_add(1);
                }
            }
            ContainerKind::Interface => {
                self.interface_na = self.interface_na.saturating_add(1);
                if is_public {
                    self.interface_npa = self.interface_npa.saturating_add(1);
                }
            }
            ContainerKind::Other => {}
        }
    }

    pub fn record_class_like(&mut self) {
        self.has_class_like = true;
    }

    pub fn finalize_minmax(&mut self) {
        self.class_npa_sum = self.class_npa_sum.saturating_add(self.class_npa);
        self.interface_npa_sum = self.interface_npa_sum.saturating_add(self.interface_npa);
        self.class_na_sum = self.class_na_sum.saturating_add(self.class_na);
        self.interface_na_sum = self.interface_na_sum.saturating_add(self.interface_na);
    }

    pub fn merge(&mut self, other: &NpaStats) {
        self.class_npa_sum = self.class_npa_sum.saturating_add(other.class_npa_sum);
        self.interface_npa_sum = self
            .interface_npa_sum
            .saturating_add(other.interface_npa_sum);
        self.class_na_sum = self.class_na_sum.saturating_add(other.class_na_sum);
        self.interface_na_sum = self.interface_na_sum.saturating_add(other.interface_na_sum);
        self.not_applicable |= other.not_applicable;
        self.has_class_like |= other.has_class_like;
    }

    pub fn class_cda(&self) -> f64 {
        if self.class_na_sum == 0 {
            f64::NAN
        } else {
            f64::from(self.class_npa_sum) / f64::from(self.class_na_sum)
        }
    }

    pub fn interface_cda(&self) -> f64 {
        if self.interface_npa_sum == self.interface_na_sum && self.interface_npa_sum != 0 {
            1.0
        } else if self.interface_na_sum == 0 {
            f64::NAN
        } else {
            f64::from(self.interface_npa_sum) / f64::from(self.interface_na_sum)
        }
    }

    pub fn total_npa(&self) -> u32 {
        self.class_npa_sum.saturating_add(self.interface_npa_sum)
    }

    pub fn total_na(&self) -> u32 {
        self.class_na_sum.saturating_add(self.interface_na_sum)
    }

    pub fn total_cda(&self) -> f64 {
        let na = self.total_na();
        if na == 0 {
            f64::NAN
        } else {
            f64::from(self.total_npa()) / f64::from(na)
        }
    }

    pub fn is_disabled(&self) -> bool {
        self.not_applicable || !self.has_class_like
    }
}

/// Number of public methods accumulator (NPM).
///
/// Same shape as [`NpaStats`] but counts methods rather than
/// attributes.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct NpmStats {
    pub class_npm: u32,
    pub interface_npm: u32,
    pub class_nm: u32,
    pub interface_nm: u32,
    pub class_npm_sum: u32,
    pub interface_npm_sum: u32,
    pub class_nm_sum: u32,
    pub interface_nm_sum: u32,
    pub not_applicable: bool,
    pub has_class_like: bool,
}

impl NpmStats {
    pub fn record_method(&mut self, container: ContainerKind, is_public: bool) {
        match container {
            ContainerKind::Class => {
                self.class_nm = self.class_nm.saturating_add(1);
                if is_public {
                    self.class_npm = self.class_npm.saturating_add(1);
                }
            }
            ContainerKind::Interface => {
                self.interface_nm = self.interface_nm.saturating_add(1);
                if is_public {
                    self.interface_npm = self.interface_npm.saturating_add(1);
                }
            }
            ContainerKind::Other => {}
        }
    }

    pub fn record_class_like(&mut self) {
        self.has_class_like = true;
    }

    pub fn finalize_minmax(&mut self) {
        self.class_npm_sum = self.class_npm_sum.saturating_add(self.class_npm);
        self.interface_npm_sum = self.interface_npm_sum.saturating_add(self.interface_npm);
        self.class_nm_sum = self.class_nm_sum.saturating_add(self.class_nm);
        self.interface_nm_sum = self.interface_nm_sum.saturating_add(self.interface_nm);
    }

    pub fn merge(&mut self, other: &NpmStats) {
        self.class_npm_sum = self.class_npm_sum.saturating_add(other.class_npm_sum);
        self.interface_npm_sum = self
            .interface_npm_sum
            .saturating_add(other.interface_npm_sum);
        self.class_nm_sum = self.class_nm_sum.saturating_add(other.class_nm_sum);
        self.interface_nm_sum = self.interface_nm_sum.saturating_add(other.interface_nm_sum);
        self.not_applicable |= other.not_applicable;
        self.has_class_like |= other.has_class_like;
    }

    pub fn class_avg(&self) -> f64 {
        if self.class_nm_sum == 0 {
            f64::NAN
        } else {
            f64::from(self.class_npm_sum) / f64::from(self.class_nm_sum)
        }
    }

    pub fn interface_avg(&self) -> f64 {
        if self.interface_npm_sum == self.interface_nm_sum && self.interface_npm_sum != 0 {
            1.0
        } else if self.interface_nm_sum == 0 {
            f64::NAN
        } else {
            f64::from(self.interface_npm_sum) / f64::from(self.interface_nm_sum)
        }
    }

    pub fn total_npm(&self) -> u32 {
        self.class_npm_sum.saturating_add(self.interface_npm_sum)
    }

    pub fn total_nm(&self) -> u32 {
        self.class_nm_sum.saturating_add(self.interface_nm_sum)
    }

    pub fn total_avg(&self) -> f64 {
        let nm = self.total_nm();
        if nm == 0 {
            f64::NAN
        } else {
            f64::from(self.total_npm()) / f64::from(nm)
        }
    }

    pub fn is_disabled(&self) -> bool {
        self.not_applicable || !self.has_class_like
    }
}

/// Container kind for NPA / NPM accounting. Class-like (`Class` /
/// `Impl`) and interface-like (`Interface` / `Trait`) are tracked in
/// separate buckets per the pre-1.0 distinction; everything else is
/// ignored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerKind {
    Class,
    Interface,
    Other,
}

/// Weighted methods per class accumulator (WMC).
///
/// WMC sums the cyclomatic complexity of every method on a class.
/// Per-space `*_sum` are the rolled-up totals; the unit publishes the
/// total. `not_applicable` lets languages without class-like constructs
/// (Go, C, Markdown) opt out so the metric is omitted from output.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct WmcStats {
    /// Per-space cyclomatic value (snapshotted from the function/method
    /// space's cyclomatic at finalize time).
    pub wmc: u32,
    pub class_wmc_sum: u32,
    pub interface_wmc_sum: u32,
    pub not_applicable: bool,
    pub has_class_like: bool,
}

impl WmcStats {
    /// Set the per-space cyclomatic value. Called by the walker at
    /// space close on function/method spaces — pass the finalized
    /// `cyclomatic` count for that space.
    pub fn set_cyclomatic(&mut self, cyclomatic: u32) {
        self.wmc = cyclomatic;
    }

    pub fn record_class_like(&mut self) {
        self.has_class_like = true;
    }

    /// Snapshot this method-space's cyclomatic into the parent's
    /// `class_wmc_sum` / `interface_wmc_sum`. The walker calls this
    /// when merging a function/method space into its enclosing class
    /// or interface.
    pub fn finalize_method_into(&self, container: ContainerKind, parent: &mut WmcStats) {
        match container {
            ContainerKind::Class => {
                parent.class_wmc_sum = parent.class_wmc_sum.saturating_add(self.wmc);
            }
            ContainerKind::Interface => {
                parent.interface_wmc_sum = parent.interface_wmc_sum.saturating_add(self.wmc);
            }
            ContainerKind::Other => {}
        }
    }

    pub fn merge(&mut self, other: &WmcStats) {
        self.class_wmc_sum = self.class_wmc_sum.saturating_add(other.class_wmc_sum);
        self.interface_wmc_sum = self
            .interface_wmc_sum
            .saturating_add(other.interface_wmc_sum);
        self.not_applicable |= other.not_applicable;
        self.has_class_like |= other.has_class_like;
    }

    pub fn total(&self) -> u32 {
        self.class_wmc_sum.saturating_add(self.interface_wmc_sum)
    }

    pub fn is_disabled(&self) -> bool {
        self.not_applicable || !self.has_class_like
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nargs_record_then_finalize_snapshots_per_space_value() {
        let mut s = NargsStats::default();
        s.record_function_args(3);
        s.finalize_minmax();
        assert_eq!(s.fn_nargs_sum, 3);
        assert_eq!(s.fn_nargs_min, 3);
        assert_eq!(s.fn_nargs_max, 3);
    }

    #[test]
    fn nargs_merge_combines_bounds() {
        let mut a = NargsStats::default();
        a.record_function_args(3);
        a.finalize_minmax();
        let mut b = NargsStats::default();
        b.record_function_args(5);
        b.finalize_minmax();
        a.merge(&b);
        assert_eq!(a.fn_nargs_sum, 8);
        assert_eq!(a.fn_nargs_min, 3);
        assert_eq!(a.fn_nargs_max, 5);
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
