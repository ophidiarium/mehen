//! Documentation Maintainability Index (DMI) per §10.
//!
//! Phase B wires only the V, M, R terms from §10.1:
//!
//! ```text
//! V_norm = sat(ln(1 + MDH_volume_total); 8, 15)
//! M_norm = sat(MCC; 15, 80)
//! R_norm = sat(MRPC; 8, 40)
//! ```
//!
//! Phases C/D will add L (link debt), T (table burden), A (artifact debt),
//! S (section balance), F (filler risk), and G (good scaffold). Until then
//! they contribute zero and are explicitly commented as stubs.
//!
//! Final formula §10.2:
//!
//! ```text
//! DMI = clamp01(
//!       1
//!     - 0.18 * V_norm
//!     - 0.18 * M_norm
//!     - 0.10 * R_norm
//!     - 0.16 * L_norm   // TODO: Phase C stub
//!     - 0.10 * T_norm   // TODO: Phase C stub
//!     - 0.10 * A_norm   // TODO: Phase C stub
//!     - 0.10 * S_norm   // TODO: Phase D stub
//!     - 0.12 * F_norm   // TODO: Phase D stub
//!     + 0.10 * G_norm   // TODO: Phase D stub
//! ) * 100
//! ```

/// Inputs to §10.2's formula.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DmiInputs {
    /// Phase-B: MRPC weighted value (§7.3).
    pub(crate) mrpc: f64,
    /// Phase-B: MCC final value (§8.4).
    pub(crate) mcc: f64,
    /// Phase-B: Markdown Halstead total volume including §9.4 embedded term.
    pub(crate) total_volume: f64,
}

/// Computes the DMI value on the `[0, 100]` scale.
pub(crate) fn compute_dmi(inputs: DmiInputs) -> f64 {
    let v_norm = saturate(ln_1p(inputs.total_volume), 8.0, 15.0);
    let m_norm = saturate(inputs.mcc, 15.0, 80.0);
    let r_norm = saturate(inputs.mrpc, 8.0, 40.0);

    // TODO(Phase C): wire L_norm = LinkDebtScore.
    let l_norm = 0.0;
    // TODO(Phase C): wire T_norm = TableBurdenScore.
    let t_norm = 0.0;
    // TODO(Phase C): wire A_norm = ArtifactDebtScore.
    let a_norm = 0.0;
    // TODO(Phase D): wire S_norm = 1 - SectionBalanceScore.
    let s_norm = 0.0;
    // TODO(Phase D): wire F_norm = FillerLazyRisk.
    let f_norm = 0.0;
    // TODO(Phase D): wire G_norm = GoodScaffoldScore.
    let g_norm = 0.0;

    let raw = 1.0
        - 0.18 * v_norm
        - 0.18 * m_norm
        - 0.10 * r_norm
        - 0.16 * l_norm
        - 0.10 * t_norm
        - 0.10 * a_norm
        - 0.10 * s_norm
        - 0.12 * f_norm
        + 0.10 * g_norm;
    raw.clamp(0.0, 1.0) * 100.0
}

fn saturate(x: f64, lo: f64, hi: f64) -> f64 {
    if !x.is_finite() || hi <= lo {
        return 0.0;
    }
    ((x - lo) / (hi - lo)).clamp(0.0, 1.0)
}

fn ln_1p(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    x.ln_1p()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_inputs_produce_perfect_dmi() {
        let dmi = compute_dmi(DmiInputs {
            mrpc: 0.0,
            mcc: 0.0,
            total_volume: 0.0,
        });
        assert_eq!(dmi, 100.0);
    }

    #[test]
    fn extreme_inputs_clamp_to_zero() {
        let dmi = compute_dmi(DmiInputs {
            mrpc: 1000.0,
            mcc: 1000.0,
            total_volume: 1e18,
        });
        // 1 - 0.18 - 0.18 - 0.10 = 0.54 → × 100 = 54. Phase B without
        // L/T/A/S/F never reaches 0; 54 is the expected floor. Allow for
        // tiny IEEE-754 drift (the 0.54 computation is a sum of 64-bit
        // floats so the last mantissa bit can wobble).
        assert!((dmi - 54.0).abs() < 1e-9, "dmi = {dmi}");
    }

    #[test]
    fn intermediate_values_behave_monotonically() {
        let low = compute_dmi(DmiInputs {
            mrpc: 5.0,
            mcc: 5.0,
            total_volume: 10.0,
        });
        let high = compute_dmi(DmiInputs {
            mrpc: 30.0,
            mcc: 50.0,
            total_volume: 10_000.0,
        });
        assert!(low > high, "low={low}, high={high}");
    }
}
