use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::checker::Checker;
use crate::langs::*;
use crate::macros::implement_metric_trait;
use crate::node::Node;

/// The `Npm` metric.
///
/// This metric counts the number of public methods
/// of classes/interfaces.
#[derive(Clone, Debug, Default)]
pub struct Stats {
    class_npm: usize,
    interface_npm: usize,
    class_nm: usize,
    interface_nm: usize,
    class_npm_sum: usize,
    interface_npm_sum: usize,
    class_nm_sum: usize,
    interface_nm_sum: usize,
    is_class_space: bool,
}

impl Serialize for Stats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("npm", 9)?;
        st.serialize_field("classes", &self.class_npm_sum())?;
        st.serialize_field("interfaces", &self.interface_npm_sum())?;
        st.serialize_field("class_methods", &self.class_nm_sum())?;
        st.serialize_field("interface_methods", &self.interface_nm_sum())?;
        st.serialize_field("classes_average", &self.class_coa())?;
        st.serialize_field("interfaces_average", &self.interface_coa())?;
        st.serialize_field("total", &self.total_npm())?;
        st.serialize_field("total_methods", &self.total_nm())?;
        st.serialize_field("average", &self.total_coa())?;
        st.end()
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "classes: {}, interfaces: {}, class_methods: {}, interface_methods: {}, classes_average: {}, interfaces_average: {}, total: {}, total_methods: {}, average: {}",
            self.class_npm_sum(),
            self.interface_npm_sum(),
            self.class_nm_sum(),
            self.interface_nm_sum(),
            self.class_coa(),
            self.interface_coa(),
            self.total_npm(),
            self.total_nm(),
            self.total_coa()
        )
    }
}

impl Stats {
    /// Merges a second `Npm` metric into the first one
    pub fn merge(&mut self, other: &Self) {
        self.class_npm_sum += other.class_npm_sum;
        self.interface_npm_sum += other.interface_npm_sum;
        self.class_nm_sum += other.class_nm_sum;
        self.interface_nm_sum += other.interface_nm_sum;
    }

    /// Returns the number of class public methods in a space.
    #[inline(always)]
    pub fn class_npm(&self) -> f64 {
        self.class_npm as f64
    }

    /// Returns the number of interface public methods in a space.
    #[inline(always)]
    pub fn interface_npm(&self) -> f64 {
        self.interface_npm as f64
    }

    /// Returns the number of class methods in a space.
    #[inline(always)]
    pub fn class_nm(&self) -> f64 {
        self.class_nm as f64
    }

    /// Returns the number of interface methods in a space.
    #[inline(always)]
    pub fn interface_nm(&self) -> f64 {
        self.interface_nm as f64
    }

    /// Returns the number of class public methods sum in a space.
    #[inline(always)]
    pub fn class_npm_sum(&self) -> f64 {
        self.class_npm_sum as f64
    }

    /// Returns the number of interface public methods sum in a space.
    #[inline(always)]
    pub fn interface_npm_sum(&self) -> f64 {
        self.interface_npm_sum as f64
    }

    /// Returns the number of class methods sum in a space.
    #[inline(always)]
    pub fn class_nm_sum(&self) -> f64 {
        self.class_nm_sum as f64
    }

    /// Returns the number of interface methods sum in a space.
    #[inline(always)]
    pub fn interface_nm_sum(&self) -> f64 {
        self.interface_nm_sum as f64
    }

    /// Returns the class `Coa` metric value
    ///
    /// The `Class Operation Accessibility` metric value for a class
    /// is computed by dividing the `Npm` value of the class
    /// by the total number of methods defined in the class.
    ///
    /// This metric is an adaptation of the `Classified Operation Accessibility` (`COA`)
    /// security metric for not classified methods.
    /// Paper: <https://ieeexplore.ieee.org/abstract/document/5381538>
    #[inline(always)]
    pub fn class_coa(&self) -> f64 {
        self.class_npm_sum() / self.class_nm_sum()
    }

    /// Returns the interface `Coa` metric value
    ///
    /// The `Class Operation Accessibility` metric value for an interface
    /// is computed by dividing the `Npm` value of the interface
    /// by the total number of methods defined in the interface.
    ///
    /// This metric is an adaptation of the `Classified Operation Accessibility` (`COA`)
    /// security metric for not classified methods.
    /// Paper: <https://ieeexplore.ieee.org/abstract/document/5381538>
    #[inline(always)]
    pub fn interface_coa(&self) -> f64 {
        // For the Java language it's not necessary to compute the metric value
        // The metric value in Java can only be 1.0 or f64:NAN
        if self.interface_npm_sum == self.interface_nm_sum && self.interface_npm_sum != 0 {
            1.0
        } else {
            self.interface_npm_sum() / self.interface_nm_sum()
        }
    }

    /// Returns the total `Coa` metric value
    ///
    /// The total `Class Operation Accessibility` metric value
    /// is computed by dividing the total `Npm` value
    /// by the total number of methods.
    ///
    /// This metric is an adaptation of the `Classified Operation Accessibility` (`COA`)
    /// security metric for not classified methods.
    /// Paper: <https://ieeexplore.ieee.org/abstract/document/5381538>
    #[inline(always)]
    pub fn total_coa(&self) -> f64 {
        self.total_npm() / self.total_nm()
    }

    /// Returns the total number of public methods in a space.
    #[inline(always)]
    pub fn total_npm(&self) -> f64 {
        self.class_npm_sum() + self.interface_npm_sum()
    }

    /// Returns the total number of methods in a space.
    #[inline(always)]
    pub fn total_nm(&self) -> f64 {
        self.class_nm_sum() + self.interface_nm_sum()
    }

    // Accumulates the number of class and interface
    // public and not public methods into the sums
    #[inline(always)]
    pub(crate) fn compute_sum(&mut self) {
        self.class_npm_sum += self.class_npm;
        self.interface_npm_sum += self.interface_npm;
        self.class_nm_sum += self.class_nm;
        self.interface_nm_sum += self.interface_nm;
    }

    // Checks if the `Npm` metric is disabled
    #[inline(always)]
    pub(crate) fn is_disabled(&self) -> bool {
        !self.is_class_space
    }
}

pub trait Npm
where
    Self: Checker,
{
    fn compute(node: &Node, stats: &mut Stats);
}

implement_metric_trait!(Npm, PythonCode, TypescriptCode, TsxCode, RustCode, GoCode);
