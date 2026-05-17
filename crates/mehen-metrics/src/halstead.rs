use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use crate::halstead_builder::HalsteadCounts;

/// Finalized Halstead measurements for one space.
///
/// All formulas are pure math. The pre-1.0 implementation lives at
/// `src/metrics/halstead.rs`; the formulas reproduced here come from the
/// classic Halstead definitions and match the existing reference output.
///
/// Built by feeding token-level events into `HalsteadBuilder`, then calling
/// `HalsteadStats::from_counts(builder.counts())`. Language crates own
/// what counts as an operator or operand; the math is shared.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct HalsteadStats {
    /// `n1` — distinct operators.
    pub u_operators: u64,
    /// `N1` — total operators.
    pub operators: u64,
    /// `n2` — distinct operands.
    pub u_operands: u64,
    /// `N2` — total operands.
    pub operands: u64,
}

impl HalsteadStats {
    pub fn from_counts(counts: HalsteadCounts) -> Self {
        Self {
            u_operators: counts.n1 as u64,
            operators: counts.big_n1 as u64,
            u_operands: counts.n2 as u64,
            operands: counts.big_n2 as u64,
        }
    }

    pub fn vocabulary(&self) -> f64 {
        (self.u_operators + self.u_operands) as f64
    }

    pub fn length(&self) -> f64 {
        (self.operators + self.operands) as f64
    }

    pub fn estimated_program_length(&self) -> f64 {
        let n1 = self.u_operators as f64;
        let n2 = self.u_operands as f64;
        n1.mul_add(n1.log2(), n2 * n2.log2())
    }

    pub fn purity_ratio(&self) -> f64 {
        let len = self.length();
        if len == 0.0 {
            0.0
        } else {
            self.estimated_program_length() / len
        }
    }

    pub fn volume(&self) -> f64 {
        let voc = self.vocabulary();
        if voc <= 0.0 {
            0.0
        } else {
            self.length() * voc.log2()
        }
    }

    pub fn difficulty(&self) -> f64 {
        let n2 = self.u_operands as f64;
        if n2 == 0.0 {
            0.0
        } else {
            (self.u_operators as f64) / 2.0 * (self.operands as f64) / n2
        }
    }

    pub fn level(&self) -> f64 {
        let d = self.difficulty();
        if d == 0.0 { 0.0 } else { 1.0 / d }
    }

    pub fn effort(&self) -> f64 {
        self.difficulty() * self.volume()
    }

    /// Time to write the program in seconds, per Halstead's heuristic.
    pub fn time(&self) -> f64 {
        self.effort() / 18.0
    }

    /// Estimated number of bugs.
    pub fn bugs(&self) -> f64 {
        self.volume() / 3000.0
    }
}

impl Serialize for HalsteadStats {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Field set kept in sync with the pre-1.0 output shape so parity
        // snapshots can compare directly.
        let mut st = serializer.serialize_struct("halstead", 14)?;
        st.serialize_field("n1", &(self.u_operators as f64))?;
        st.serialize_field("N1", &(self.operators as f64))?;
        st.serialize_field("n2", &(self.u_operands as f64))?;
        st.serialize_field("N2", &(self.operands as f64))?;
        st.serialize_field("length", &self.length())?;
        st.serialize_field("estimated_program_length", &self.estimated_program_length())?;
        st.serialize_field("purity_ratio", &self.purity_ratio())?;
        st.serialize_field("vocabulary", &self.vocabulary())?;
        st.serialize_field("volume", &self.volume())?;
        st.serialize_field("difficulty", &self.difficulty())?;
        st.serialize_field("level", &self.level())?;
        st.serialize_field("effort", &self.effort())?;
        st.serialize_field("time", &self.time())?;
        st.serialize_field("bugs", &self.bugs())?;
        st.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::halstead_builder::{HalsteadBuilder, HalsteadOperand, HalsteadOperator};
    use smol_str::SmolStr;

    #[test]
    fn empty_stats_have_zero_volume() {
        let s = HalsteadStats::default();
        assert_eq!(s.volume(), 0.0);
        assert_eq!(s.difficulty(), 0.0);
    }

    #[test]
    fn from_builder_round_trips() {
        let mut b = HalsteadBuilder::new();
        b.observe_operator(HalsteadOperator {
            kind: SmolStr::new("+"),
            text: None,
        });
        b.observe_operator(HalsteadOperator {
            kind: SmolStr::new("="),
            text: None,
        });
        b.observe_operand(HalsteadOperand {
            kind: SmolStr::new("ident"),
            text: Some(SmolStr::new("x")),
        });
        b.observe_operand(HalsteadOperand {
            kind: SmolStr::new("number"),
            text: Some(SmolStr::new("1")),
        });
        let stats = HalsteadStats::from_counts(b.counts());
        assert_eq!(stats.u_operators, 2);
        assert_eq!(stats.operators, 2);
        assert_eq!(stats.u_operands, 2);
        assert_eq!(stats.operands, 2);
        assert_eq!(stats.vocabulary(), 4.0);
        assert_eq!(stats.length(), 4.0);
    }
}
