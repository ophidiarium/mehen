use serde::Serialize;
use serde::ser::{SerializeStruct, Serializer};
use std::fmt;

use crate::legacy::checker::Checker;
use crate::legacy::langs::{CCode, GoCode, KotlinCode, LANG, RubyCode};
use crate::legacy::metrics::cyclomatic;
use crate::legacy::spaces::SpaceKind;

// FIX ME: New Java switches are not correctly recognised by tree-sitter-java version 0.19.0
// However, the issue has already been addressed and resolved upstream on the tree-sitter-java GitHub repository
// Upstream issue: https://github.com/tree-sitter/tree-sitter-java/issues/69
// Upstream PR which resolves the issue: https://github.com/tree-sitter/tree-sitter-java/pull/78

/// The `Wmc` metric.
///
/// This metric sums the cyclomatic complexities of all the methods defined in a class.
/// The `Wmc` (Weighted Methods per Class) is an object-oriented metric for classes.
///
/// Original paper and definition:
/// <https://www.researchgate.net/publication/3187649_Kemerer_CF_A_metric_suite_for_object_oriented_design_IEEE_Trans_Softw_Eng_206_476-493>
#[derive(Debug, Clone, Default)]
pub(crate) struct Stats {
    cyclomatic: f64,
    class_wmc: f64,
    interface_wmc: f64,
    class_wmc_sum: f64,
    interface_wmc_sum: f64,
    space_kind: SpaceKind,
    not_applicable: bool,
    /// True once any class-like or interface-like space has been observed
    /// anywhere in the subtree being aggregated. Tracked separately from the
    /// numeric sums so an *empty* class (no methods, sum = 0.0) still keeps
    /// unit-level `wmc` visible.
    has_class_like: bool,
}

impl Serialize for Stats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("wmc", 3)?;
        st.serialize_field("classes", &self.class_wmc_sum())?;
        st.serialize_field("interfaces", &self.interface_wmc_sum())?;
        st.serialize_field("total", &self.total_wmc())?;
        st.end()
    }
}

impl fmt::Display for Stats {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "classes: {}, interfaces: {}, total: {}",
            self.class_wmc_sum(),
            self.interface_wmc_sum(),
            self.total_wmc()
        )
    }
}

impl Stats {
    /// Merges a second `Wmc` metric into the first one
    pub(crate) fn merge(&mut self, other: &Self) {
        use SpaceKind::*;

        // Merges the cyclomatic complexity of a method
        // into the `Wmc` metric value of a class or interface.
        // Rust `impl` blocks are class-like for this purpose, and `trait`
        // blocks are interface-like.
        if other.space_kind == Function {
            match self.space_kind {
                Class | Impl => self.class_wmc += other.cyclomatic,
                Interface | Trait => self.interface_wmc += other.cyclomatic,
                _ => {}
            }
        }

        self.class_wmc_sum += other.class_wmc_sum;
        self.interface_wmc_sum += other.interface_wmc_sum;
        self.not_applicable |= other.not_applicable;
        self.has_class_like |= other.has_class_like;
    }

    /// Returns the sum of the `Wmc` metric values of the classes in a space.
    #[inline(always)]
    pub(crate) fn class_wmc_sum(&self) -> f64 {
        self.class_wmc_sum
    }

    /// Returns the sum of the `Wmc` metric values of the interfaces in a space.
    #[inline(always)]
    pub(crate) fn interface_wmc_sum(&self) -> f64 {
        self.interface_wmc_sum
    }

    /// Returns the total `Wmc` metric value in a space.
    #[inline(always)]
    pub(crate) fn total_wmc(&self) -> f64 {
        self.class_wmc_sum() + self.interface_wmc_sum()
    }

    // Accumulates the `Wmc` metric values
    // of classes and interfaces into the sums
    #[inline(always)]
    pub(crate) fn compute_sum(&mut self) {
        self.class_wmc_sum += self.class_wmc;
        self.interface_wmc_sum += self.interface_wmc;
    }

    // Checks if the `Wmc` metric is disabled
    #[inline(always)]
    pub(crate) fn is_disabled(&self) -> bool {
        if self.not_applicable {
            return true;
        }
        match self.space_kind {
            SpaceKind::Function | SpaceKind::Unknown => true,
            // A unit-level space only reports aggregated WMC if there are
            // class-like spaces inside. Use the explicit presence flag so an
            // empty class / trait / impl (sum = 0) is not hidden as "noise".
            SpaceKind::Unit => !self.has_class_like,
            _ => false,
        }
    }

    /// Marks this metric as not applicable to the current language so it is
    /// omitted from output rather than serialized as a measured zero.
    #[inline(always)]
    pub(crate) fn mark_not_applicable(&mut self) {
        self.not_applicable = true;
    }

    /// Returns whether the `Wmc` metric is meaningful for the given language.
    /// Languages without class-like constructs opt out. Markdown is a
    /// documentation language with no classes or methods and likewise opts
    /// out of `Wmc`.
    #[inline(always)]
    pub(crate) fn applies_to(lang: LANG) -> bool {
        #[cfg(feature = "markdown")]
        if matches!(lang, LANG::Markdown) {
            return false;
        }
        !matches!(lang, LANG::Go | LANG::C)
    }

    /// Records the kind of the enclosing space. Also flags the stats as
    /// having observed a class-like space if applicable, so unit-level
    /// aggregation can distinguish "no classes" from "classes with no
    /// counted methods".
    #[inline(always)]
    pub(crate) fn set_space_kind(&mut self, kind: SpaceKind) {
        self.space_kind = kind;
        if matches!(
            kind,
            SpaceKind::Class | SpaceKind::Interface | SpaceKind::Impl | SpaceKind::Trait
        ) {
            self.has_class_like = true;
        }
    }
}

pub(crate) trait Wmc
where
    Self: Checker,
{
    fn compute(space_kind: SpaceKind, cyclomatic: &cyclomatic::Stats, stats: &mut Stats);
}

macro_rules! impl_wmc {
    ($($code:ident),+) => (
        $(
           impl Wmc for $code {
               fn compute(
                   space_kind: SpaceKind,
                   cyclomatic: &cyclomatic::Stats,
                   stats: &mut Stats,
               ) {
                   stats.set_space_kind(space_kind);
                   if matches!(space_kind, SpaceKind::Function) {
                       stats.cyclomatic = cyclomatic.cyclomatic();
                   }
               }
           }
        )+
    );
}

impl_wmc!(RubyCode, KotlinCode);

// Go has no class-like constructs; WMC is not applicable.
impl Wmc for GoCode {
    fn compute(_space_kind: SpaceKind, _cyclomatic: &cyclomatic::Stats, _stats: &mut Stats) {}
}

// C has no class-like constructs; WMC is not applicable.
impl Wmc for CCode {
    fn compute(_space_kind: SpaceKind, _cyclomatic: &cyclomatic::Stats, _stats: &mut Stats) {}
}

// Markdown is a documentation language; WMC is a code-metric and does not
// apply. The metric is also disabled for the language via `applies_to`.
#[cfg(feature = "markdown")]
impl Wmc for crate::legacy::langs::MarkdownCode {
    fn compute(_space_kind: SpaceKind, _cyclomatic: &cyclomatic::Stats, _stats: &mut Stats) {}
}

#[cfg(test)]
mod tests {
    use crate::legacy::langs::{KotlinParser, RubyParser};
    use crate::legacy::tools::check_metrics;

    #[test]
    fn kotlin_wmc_class_sums_method_cyclomatics() {
        check_metrics::<KotlinParser>(
            "class C {
                 fun a(x: Int): Int {
                     return if (x > 0) 1 else 0
                 }
                 fun b(): Int { return 1 }
             }",
            "foo.kt",
            |metric| {
                // class C -> a cyc = 2 (if), b cyc = 1 -> 3
                insta::assert_json_snapshot!(
                    metric.wmc,
                    @r###"
                    {
                      "classes": 3.0,
                      "interfaces": 0.0,
                      "total": 3.0
                    }"###
                );
            },
        );
    }

    #[test]
    fn ruby_wmc_class_sums_method_cyclomatics() {
        check_metrics::<RubyParser>(
            "class C
                 def a(x)
                     return 1 if x
                     return 0
                 end
                 def b
                     1
                 end
             end",
            "foo.rb",
            |metric| {
                insta::assert_json_snapshot!(
                    metric.wmc,
                    @r###"
                    {
                      "classes": 3.0,
                      "interfaces": 0.0,
                      "total": 3.0
                    }"###
                );
            },
        );
    }
}
