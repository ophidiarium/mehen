//! Documentation Maintainability Index (DMI) per §10.
//!
//! After Phase D ships, every §10.1 term is wired:
//!
//! ```text
//! V_norm = sat(ln(1 + MDH_volume_total); 8, 15)
//! M_norm = sat(MCC; 15, 80)
//! R_norm = sat(MRPC; 8, 40)
//! L_norm = LinkDebtScore            (Phase C)
//! T_norm = TableBurdenScore         (Phase C)
//! A_norm = ArtifactDebtScore        (Phase C)
//! S_norm = 1 - SectionBalanceScore  (Phase D)
//! F_norm = FillerLazyRisk           (Phase D)
//! G_norm = GoodScaffoldScore        (Phase D)
//! ```
//!
//! Final formula §10.2:
//!
//! ```text
//! DMI = clamp01(
//!       1
//!     - 0.18 * V_norm
//!     - 0.18 * M_norm
//!     - 0.10 * R_norm
//!     - 0.16 * L_norm
//!     - 0.10 * T_norm
//!     - 0.10 * A_norm
//!     - 0.10 * S_norm
//!     - 0.12 * F_norm
//!     + 0.10 * G_norm
//! ) * 100
//! ```

/// Inputs to §10.2's formula.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DmiInputs {
    /// Phase-B: MRPC weighted value (§7.3).
    pub(crate) mrpc: f64,
    /// Phase-B: MCC final value (§8.4).
    pub(crate) mcc: f64,
    /// Phase-B: Markdown Halstead total volume including §9.4 embedded term.
    pub(crate) total_volume: f64,
    /// Phase-C: `links.link_debt_score` (§11.2).
    pub(crate) link_debt_score: f64,
    /// Phase-C: `tables.table_burden_score` (§13.3).
    pub(crate) table_burden_score: f64,
    /// Phase-C: `maintainability.artifact_debt_score` (§19).
    pub(crate) artifact_debt_score: f64,
    /// Phase-D: `1 - maintainability.section_balance_score` (§20).
    pub(crate) section_imbalance: f64,
    /// Phase-D: `ai_era.filler_lazy_structure_risk` (§17).
    pub(crate) filler_lazy_risk: f64,
    /// Phase-D: `maintainability.good_scaffold_score` (§21).
    pub(crate) good_scaffold_score: f64,
}

/// Computes the DMI value on the `[0, 100]` scale.
pub(crate) fn compute_dmi(inputs: DmiInputs) -> f64 {
    let v_norm = saturate(ln_1p(inputs.total_volume), 8.0, 15.0);
    let m_norm = saturate(inputs.mcc, 15.0, 80.0);
    let r_norm = saturate(inputs.mrpc, 8.0, 40.0);

    let l_norm = inputs.link_debt_score.clamp(0.0, 1.0);
    let t_norm = inputs.table_burden_score.clamp(0.0, 1.0);
    let a_norm = inputs.artifact_debt_score.clamp(0.0, 1.0);
    let s_norm = inputs.section_imbalance.clamp(0.0, 1.0);
    let f_norm = inputs.filler_lazy_risk.clamp(0.0, 1.0);
    let g_norm = inputs.good_scaffold_score.clamp(0.0, 1.0);

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
        let dmi = compute_dmi(DmiInputs::default());
        assert_eq!(dmi, 100.0);
    }

    #[test]
    fn extreme_inputs_clamp_to_zero() {
        let dmi = compute_dmi(DmiInputs {
            mrpc: 1000.0,
            mcc: 1000.0,
            total_volume: 1e18,
            link_debt_score: 1.0,
            table_burden_score: 1.0,
            artifact_debt_score: 1.0,
            section_imbalance: 1.0,
            filler_lazy_risk: 1.0,
            good_scaffold_score: 0.0,
        });
        // Sum of negative coefficients: 0.18+0.18+0.10+0.16+0.10+0.10+0.10+0.12 = 1.04.
        // 1.0 - 1.04 = -0.04 → clamped to 0. × 100 = 0.
        assert_eq!(dmi, 0.0);
    }

    #[test]
    fn good_scaffold_offsets_moderate_penalties() {
        let penalized = compute_dmi(DmiInputs {
            mrpc: 8.0,
            mcc: 15.0,
            total_volume: 0.0,
            link_debt_score: 0.5,
            table_burden_score: 0.0,
            artifact_debt_score: 0.0,
            section_imbalance: 0.0,
            filler_lazy_risk: 0.0,
            good_scaffold_score: 0.0,
        });
        let rewarded = compute_dmi(DmiInputs {
            mrpc: 8.0,
            mcc: 15.0,
            total_volume: 0.0,
            link_debt_score: 0.5,
            table_burden_score: 0.0,
            artifact_debt_score: 0.0,
            section_imbalance: 0.0,
            filler_lazy_risk: 0.0,
            good_scaffold_score: 1.0,
        });
        assert!(rewarded > penalized, "scaffold should offset");
    }

    #[test]
    fn intermediate_values_behave_monotonically() {
        let low = compute_dmi(DmiInputs {
            mrpc: 5.0,
            mcc: 5.0,
            total_volume: 10.0,
            ..DmiInputs::default()
        });
        let high = compute_dmi(DmiInputs {
            mrpc: 30.0,
            mcc: 50.0,
            total_volume: 10_000.0,
            link_debt_score: 0.8,
            table_burden_score: 0.5,
            artifact_debt_score: 0.7,
            section_imbalance: 0.4,
            filler_lazy_risk: 0.5,
            good_scaffold_score: 0.0,
        });
        assert!(low > high, "low={low}, high={high}");
    }
}
