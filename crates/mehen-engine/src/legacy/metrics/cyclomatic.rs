use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::legacy::checker::Checker;
use crate::legacy::langs::CCode;
use crate::legacy::languages::C;
use crate::legacy::node::Node;

/// The `Cyclomatic` metric.
#[derive(Debug, Clone)]
pub(crate) struct Stats {
    cyclomatic_sum: f64,
    cyclomatic: f64,
    n: usize,
    cyclomatic_max: f64,
    cyclomatic_min: f64,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            cyclomatic_sum: 0.,
            cyclomatic: 1.,
            n: 1,
            cyclomatic_max: 0.,
            cyclomatic_min: f64::MAX,
        }
    }
}

impl Serialize for Stats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("cyclomatic", 4)?;
        st.serialize_field("sum", &self.cyclomatic_sum())?;
        st.serialize_field("average", &self.cyclomatic_average())?;
        st.serialize_field("min", &self.cyclomatic_min())?;
        st.serialize_field("max", &self.cyclomatic_max())?;
        st.end()
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "sum: {}, average: {}, min: {}, max: {}",
            self.cyclomatic_sum(),
            self.cyclomatic_average(),
            self.cyclomatic_min(),
            self.cyclomatic_max()
        )
    }
}

impl Stats {
    /// Merges a second `Cyclomatic` metric into the first one
    pub(crate) fn merge(&mut self, other: &Self) {
        //Calculate minimum and maximum values
        self.cyclomatic_max = self.cyclomatic_max.max(other.cyclomatic_max);
        self.cyclomatic_min = self.cyclomatic_min.min(other.cyclomatic_min);

        self.cyclomatic_sum += other.cyclomatic_sum;
        self.n += other.n;
    }

    /// Returns the sum
    pub(crate) fn cyclomatic_sum(&self) -> f64 {
        self.cyclomatic_sum
    }

    /// Returns the `Cyclomatic` metric average value
    ///
    /// This value is computed dividing the `Cyclomatic` value for the
    /// number of spaces.
    pub(crate) fn cyclomatic_average(&self) -> f64 {
        self.cyclomatic_sum() / self.n as f64
    }
    /// Returns the `Cyclomatic` maximum value
    pub(crate) fn cyclomatic_max(&self) -> f64 {
        self.cyclomatic_max
    }
    /// Returns the `Cyclomatic` minimum value
    pub(crate) fn cyclomatic_min(&self) -> f64 {
        self.cyclomatic_min
    }
    #[inline(always)]
    pub(crate) fn compute_sum(&mut self) {
        self.cyclomatic_sum += self.cyclomatic;
    }
    #[inline(always)]
    pub(crate) fn compute_minmax(&mut self) {
        self.cyclomatic_max = self.cyclomatic_max.max(self.cyclomatic);
        self.cyclomatic_min = self.cyclomatic_min.min(self.cyclomatic);
        self.compute_sum();
    }
}

pub(crate) trait Cyclomatic
where
    Self: Checker,
{
    fn compute(node: &Node, stats: &mut Stats);
}

// No languages require empty Cyclomatic implementations
// implement_metric_trait!(Cyclomatic);

impl Cyclomatic for CCode {
    fn compute(node: &Node, stats: &mut Stats) {
        use C::*;

        match node.kind_id().into() {
            // Decision-point set aligned with Sonar's cyclomatic rule for C:
            // `if`, every `case`, loops, ternary (`conditional_expression`),
            // and each short-circuit boolean operator (`&&` / `||`).
            // `switch` itself is not a decision (cases are); `default` is
            // fallthrough.
            IfStatement
            | CaseStatement
            | ForStatement
            | WhileStatement
            | DoStatement
            | ConditionalExpression
            | AMPAMP
            | PIPEPIPE => {
                stats.cyclomatic += 1.;
            }
            _ => {}
        }
    }
}

// Markdown is a documentation language; cyclomatic is a code metric. The
// dedicated Markdown pipeline computes its own MRPC analogue (Phase B).
#[cfg(feature = "markdown")]
impl Cyclomatic for crate::legacy::langs::MarkdownCode {
    fn compute(_node: &Node, _stats: &mut Stats) {}
}
