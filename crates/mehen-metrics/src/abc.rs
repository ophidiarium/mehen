use serde::Serialize;

/// ABC metric accumulator.
///
/// Mirrors the pre-1.0 `abc::Stats`. Per-space `assignments` /
/// `branches` / `conditions` are the running counts for the current
/// space. `*_sum` are the rolled-up totals across closed spaces
/// (snapshotted by `finalize_minmax`). Min/max bounds track per-space
/// values across the rolled-up tree. Averages divide by `space_count`.
/// `magnitude` follows Fitzpatrick (1997): sqrt(A² + B² + C²) over the
/// rolled-up sums.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct AbcStats {
    pub assignments: u32,
    pub branches: u32,
    pub conditions: u32,
    pub assignments_sum: u32,
    pub branches_sum: u32,
    pub conditions_sum: u32,
    pub assignments_min: u32,
    pub assignments_max: u32,
    pub branches_min: u32,
    pub branches_max: u32,
    pub conditions_min: u32,
    pub conditions_max: u32,
    pub space_count: u32,
    pub minmax_seen: bool,
}

impl AbcStats {
    pub fn record_assignment(&mut self) {
        self.assignments = self.assignments.saturating_add(1);
    }

    pub fn record_branch(&mut self) {
        self.branches = self.branches.saturating_add(1);
    }

    pub fn record_condition(&mut self) {
        self.conditions = self.conditions.saturating_add(1);
    }

    /// Snapshot the per-space `assignments` / `branches` / `conditions`
    /// into `*_sum`, `*_min`, `*_max` and bump `space_count`. Mirrors
    /// the pre-1.0 `compute_minmax`.
    pub fn finalize_minmax(&mut self) {
        self.assignments_sum = self.assignments_sum.saturating_add(self.assignments);
        self.branches_sum = self.branches_sum.saturating_add(self.branches);
        self.conditions_sum = self.conditions_sum.saturating_add(self.conditions);
        self.space_count = self.space_count.saturating_add(1);
        if self.minmax_seen {
            self.assignments_min = self.assignments_min.min(self.assignments);
            self.branches_min = self.branches_min.min(self.branches);
            self.conditions_min = self.conditions_min.min(self.conditions);
        } else {
            self.assignments_min = self.assignments;
            self.branches_min = self.branches;
            self.conditions_min = self.conditions;
            self.minmax_seen = true;
        }
        self.assignments_max = self.assignments_max.max(self.assignments);
        self.branches_max = self.branches_max.max(self.branches);
        self.conditions_max = self.conditions_max.max(self.conditions);
    }

    pub fn merge(&mut self, other: &AbcStats) {
        self.assignments_sum = self.assignments_sum.saturating_add(other.assignments_sum);
        self.branches_sum = self.branches_sum.saturating_add(other.branches_sum);
        self.conditions_sum = self.conditions_sum.saturating_add(other.conditions_sum);
        self.space_count = self.space_count.saturating_add(other.space_count);
        if !other.minmax_seen {
            return;
        }
        if self.minmax_seen {
            self.assignments_min = self.assignments_min.min(other.assignments_min);
            self.branches_min = self.branches_min.min(other.branches_min);
            self.conditions_min = self.conditions_min.min(other.conditions_min);
        } else {
            self.assignments_min = other.assignments_min;
            self.branches_min = other.branches_min;
            self.conditions_min = other.conditions_min;
            self.minmax_seen = true;
        }
        self.assignments_max = self.assignments_max.max(other.assignments_max);
        self.branches_max = self.branches_max.max(other.branches_max);
        self.conditions_max = self.conditions_max.max(other.conditions_max);
    }

    /// Magnitude over the rolled-up sums: `sqrt(A² + B² + C²)`.
    pub fn magnitude(&self) -> f64 {
        let a = f64::from(self.assignments_sum);
        let b = f64::from(self.branches_sum);
        let c = f64::from(self.conditions_sum);
        (a.mul_add(a, b.mul_add(b, c * c))).sqrt()
    }

    pub fn assignments_average(&self) -> f64 {
        average(self.assignments_sum, self.space_count)
    }
    pub fn branches_average(&self) -> f64 {
        average(self.branches_sum, self.space_count)
    }
    pub fn conditions_average(&self) -> f64 {
        average(self.conditions_sum, self.space_count)
    }
}

fn average(numerator: u32, denominator: u32) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        f64::from(numerator) / f64::from(denominator)
    }
}
