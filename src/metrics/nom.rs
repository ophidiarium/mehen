use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::checker::Checker;
use crate::langs::{GoCode, PythonCode, RustCode, TsxCode, TypescriptCode};
use crate::macros::implement_metric_trait;
use crate::node::Node;

/// The `Nom` metric suite.
#[derive(Clone, Debug)]
pub(crate) struct Stats {
    functions: usize,
    closures: usize,
    functions_sum: usize,
    closures_sum: usize,
    functions_min: usize,
    functions_max: usize,
    closures_min: usize,
    closures_max: usize,
    space_count: usize,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            functions: 0,
            closures: 0,
            functions_sum: 0,
            closures_sum: 0,
            functions_min: usize::MAX,
            functions_max: 0,
            closures_min: usize::MAX,
            closures_max: 0,
            space_count: 1,
        }
    }
}

impl Serialize for Stats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("nom", 10)?;
        st.serialize_field("functions", &self.functions_sum())?;
        st.serialize_field("closures", &self.closures_sum())?;
        st.serialize_field("functions_average", &self.functions_average())?;
        st.serialize_field("closures_average", &self.closures_average())?;
        st.serialize_field("total", &self.total())?;
        st.serialize_field("average", &self.average())?;
        st.serialize_field("functions_min", &self.functions_min())?;
        st.serialize_field("functions_max", &self.functions_max())?;
        st.serialize_field("closures_min", &self.closures_min())?;
        st.serialize_field("closures_max", &self.closures_max())?;
        st.end()
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "functions: {}, \
             closures: {}, \
             functions_average: {}, \
             closures_average: {}, \
             total: {} \
             average: {} \
             functions_min: {} \
             functions_max: {} \
             closures_min: {} \
             closures_max: {}",
            self.functions_sum(),
            self.closures_sum(),
            self.functions_average(),
            self.closures_average(),
            self.total(),
            self.average(),
            self.functions_min(),
            self.functions_max(),
            self.closures_min(),
            self.closures_max(),
        )
    }
}

impl Stats {
    /// Merges a second `Nom` metric suite into the first one
    pub(crate) fn merge(&mut self, other: &Self) {
        self.functions_min = self.functions_min.min(other.functions_min);
        self.functions_max = self.functions_max.max(other.functions_max);
        self.closures_min = self.closures_min.min(other.closures_min);
        self.closures_max = self.closures_max.max(other.closures_max);
        self.functions_sum += other.functions_sum;
        self.closures_sum += other.closures_sum;
        self.space_count += other.space_count;
    }

    /// Counts the number of function definitions in a scope
    #[inline(always)]
    pub(crate) fn functions(&self) -> f64 {
        // Only function definitions are considered, not general declarations
        self.functions as f64
    }

    /// Counts the number of closures in a scope
    #[inline(always)]
    pub(crate) fn closures(&self) -> f64 {
        self.closures as f64
    }

    /// Return the sum metric for functions
    #[inline(always)]
    pub(crate) fn functions_sum(&self) -> f64 {
        // Only function definitions are considered, not general declarations
        self.functions_sum as f64
    }

    /// Return the sum metric for closures
    #[inline(always)]
    pub(crate) fn closures_sum(&self) -> f64 {
        self.closures_sum as f64
    }

    /// Returns the average number of function definitions over all spaces
    #[inline(always)]
    pub(crate) fn functions_average(&self) -> f64 {
        self.functions_sum() / self.space_count as f64
    }

    /// Returns the average number of closures over all spaces
    #[inline(always)]
    pub(crate) fn closures_average(&self) -> f64 {
        self.closures_sum() / self.space_count as f64
    }

    /// Returns the average number of function definitions and closures over all spaces
    #[inline(always)]
    pub(crate) fn average(&self) -> f64 {
        self.total() / self.space_count as f64
    }

    /// Counts the number of function definitions in a scope
    #[inline(always)]
    pub(crate) fn functions_min(&self) -> f64 {
        // Only function definitions are considered, not general declarations
        self.functions_min as f64
    }

    /// Counts the number of closures in a scope
    #[inline(always)]
    pub(crate) fn closures_min(&self) -> f64 {
        self.closures_min as f64
    }
    /// Counts the number of function definitions in a scope
    #[inline(always)]
    pub(crate) fn functions_max(&self) -> f64 {
        // Only function definitions are considered, not general declarations
        self.functions_max as f64
    }

    /// Counts the number of closures in a scope
    #[inline(always)]
    pub(crate) fn closures_max(&self) -> f64 {
        self.closures_max as f64
    }
    /// Returns the total number of function definitions and
    /// closures in a scope
    #[inline(always)]
    pub(crate) fn total(&self) -> f64 {
        self.functions_sum() + self.closures_sum()
    }
    #[inline(always)]
    pub(crate) fn compute_sum(&mut self) {
        self.functions_sum += self.functions;
        self.closures_sum += self.closures;
    }
    #[inline(always)]
    pub(crate) fn compute_minmax(&mut self) {
        self.functions_min = self.functions_min.min(self.functions);
        self.functions_max = self.functions_max.max(self.functions);
        self.closures_min = self.closures_min.min(self.closures);
        self.closures_max = self.closures_max.max(self.closures);
        self.compute_sum();
    }
}

pub(crate) trait Nom
where
    Self: Checker,
{
    fn compute(node: &Node, stats: &mut Stats) {
        if Self::is_func(node) {
            stats.functions += 1;
            return;
        }
        if Self::is_closure(node) {
            stats.closures += 1;
        }
    }
}

implement_metric_trait!([Nom], PythonCode, TypescriptCode, TsxCode, RustCode, GoCode);

#[cfg(test)]
mod tests {
    use crate::langs::{PythonParser, RustParser};
    use crate::tools::check_metrics;

    #[test]
    fn python_nom() {
        check_metrics::<PythonParser>(
            "def a():
                 pass
             def b():
                 pass
             def c():
                 pass
             x = lambda a : a + 42",
            "foo.py",
            |metric| {
                // Number of spaces = 4
                insta::assert_json_snapshot!(
                    metric.nom,
                    @r###"
                    {
                      "functions": 3.0,
                      "closures": 1.0,
                      "functions_average": 0.75,
                      "closures_average": 0.25,
                      "total": 4.0,
                      "average": 1.0,
                      "functions_min": 0.0,
                      "functions_max": 1.0,
                      "closures_min": 0.0,
                      "closures_max": 1.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn rust_nom() {
        check_metrics::<RustParser>(
            "mod A { fn foo() {}}
             mod B { fn foo() {}}
             let closure = |i: i32| -> i32 { i + 42 };",
            "foo.rs",
            |metric| {
                // Number of spaces = 4
                insta::assert_json_snapshot!(
                    metric.nom,
                    @r###"
                    {
                      "functions": 2.0,
                      "closures": 1.0,
                      "functions_average": 0.5,
                      "closures_average": 0.25,
                      "total": 3.0,
                      "average": 0.75,
                      "functions_min": 0.0,
                      "functions_max": 1.0,
                      "closures_min": 0.0,
                      "closures_max": 1.0
                    }"###
                );
            },
        );
    }
}
