use serde::Serialize;

use crate::cyclomatic::CyclomaticStats;
use crate::halstead::HalsteadStats;
use crate::loc::LocStats;

/// Maintainability index variants. All three flavors are reported because
/// downstream tooling depends on different conventions; the pre-1.0 output
/// shape (Visual Studio is the headline number) is preserved.
#[derive(Default, Clone, Debug, PartialEq, Serialize)]
pub struct MiStats {
    pub mi_original: f64,
    pub mi_sei: f64,
    pub mi_visual_studio: f64,
}

impl MiStats {
    /// Compute the MI variants from the underlying LOC, cyclomatic, and
    /// Halstead measurements. All three formulas are pure math; the
    /// Visual Studio variant clamps at zero.
    pub fn compute(loc: &LocStats, cyclomatic: &CyclomaticStats, halstead: &HalsteadStats) -> Self {
        let halstead_volume = halstead.volume();
        let cy = cyclomatic.cyclomatic_sum as f64;
        let sloc = f64::from(loc.sloc());
        let comments_percentage = loc.comments_percentage();

        let original = if sloc > 0.0 && halstead_volume > 0.0 {
            16.2_f64.mul_add(
                -sloc.ln(),
                0.23_f64.mul_add(-cy, 5.2_f64.mul_add(-halstead_volume.ln(), 171.0)),
            )
        } else {
            0.0
        };

        let sei = if sloc > 0.0 && halstead_volume > 0.0 {
            50.0_f64.mul_add(
                (comments_percentage * 2.4).sqrt().sin(),
                16.2_f64.mul_add(
                    -sloc.log2(),
                    0.23_f64.mul_add(-cy, 5.2_f64.mul_add(-halstead_volume.log2(), 171.0)),
                ),
            )
        } else {
            0.0
        };

        let visual_studio = (original * 100.0 / 171.0).max(0.0);

        Self {
            mi_original: original,
            mi_sei: sei,
            mi_visual_studio: visual_studio,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_inputs_yield_zero() {
        let mi = MiStats::compute(
            &LocStats::default(),
            &CyclomaticStats::default(),
            &HalsteadStats::default(),
        );
        assert_eq!(mi.mi_original, 0.0);
        assert_eq!(mi.mi_visual_studio, 0.0);
    }
}
