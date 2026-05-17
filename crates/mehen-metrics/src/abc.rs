use serde::Serialize;

/// ABC metric accumulator.
///
/// `magnitude = sqrt(A^2 + B^2 + C^2)`. Per plan §5.2 the language crate
/// recognizes assignments, branches, and conditions; the math is shared.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct AbcStats {
    pub assignments: u32,
    pub branches: u32,
    pub conditions: u32,
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

    pub fn merge(&mut self, other: &AbcStats) {
        self.assignments = self.assignments.saturating_add(other.assignments);
        self.branches = self.branches.saturating_add(other.branches);
        self.conditions = self.conditions.saturating_add(other.conditions);
    }

    pub fn magnitude(&self) -> f64 {
        let a = self.assignments as f64;
        let b = self.branches as f64;
        let c = self.conditions as f64;
        (a * a + b * b + c * c).sqrt()
    }
}
