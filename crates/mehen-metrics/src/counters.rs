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
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct NomStats {
    pub functions: u32,
    pub closures: u32,
    pub min: u32,
    pub max: u32,
}

impl NomStats {
    pub fn record_function(&mut self) {
        self.functions = self.functions.saturating_add(1);
    }

    pub fn record_closure(&mut self) {
        self.closures = self.closures.saturating_add(1);
    }

    /// Fold the current per-space totals into the min/max bounds. Should be
    /// called once per space before merging into the parent.
    pub fn finalize_minmax(&mut self) {
        let value = self.total();
        self.min = if self.min == 0 {
            value
        } else {
            self.min.min(value)
        };
        self.max = self.max.max(value);
    }

    pub fn merge(&mut self, other: &NomStats) {
        self.functions = self.functions.saturating_add(other.functions);
        self.closures = self.closures.saturating_add(other.closures);
        self.min = match (self.min, other.min) {
            (0, b) => b,
            (a, 0) => a,
            (a, b) => a.min(b),
        };
        self.max = self.max.max(other.max);
    }

    pub fn total(&self) -> u32 {
        self.functions.saturating_add(self.closures)
    }
}

/// Number of exits (return/throw/raise/break/continue) accumulator.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct NexitStats {
    pub exits: u32,
    pub min: u32,
    pub max: u32,
    pub average: f64,
    pub sum: u32,
}

impl NexitStats {
    /// Record one exit point. Increments both the per-space `exits` count
    /// and the rolling `sum` so a `merge` immediately after does not lose
    /// the latest exit.
    pub fn record_exit(&mut self) {
        self.exits = self.exits.saturating_add(1);
        self.sum = self.sum.saturating_add(1);
    }

    /// Fold the current per-space `exits` value into the min/max bounds.
    pub fn finalize_minmax(&mut self) {
        let value = self.exits;
        self.min = if self.min == 0 {
            value
        } else {
            self.min.min(value)
        };
        self.max = self.max.max(value);
    }

    /// Compute the average exits per function once `sum` has been merged
    /// across all spaces.
    pub fn finalize_average(&mut self, function_count: u32) {
        self.average = if function_count == 0 {
            0.0
        } else {
            f64::from(self.sum) / f64::from(function_count)
        };
    }

    pub fn merge(&mut self, other: &NexitStats) {
        self.sum = self.sum.saturating_add(other.sum);
        self.exits = self.exits.saturating_add(other.exits);
        self.min = match (self.min, other.min) {
            (0, b) => b,
            (a, 0) => a,
            (a, b) => a.min(b),
        };
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
    fn nexit_record_exit_updates_sum() {
        let mut s = NexitStats::default();
        s.record_exit();
        s.record_exit();
        assert_eq!(s.exits, 2);
        assert_eq!(s.sum, 2);
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
