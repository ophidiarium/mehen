use std::collections::HashSet;

use smol_str::SmolStr;

/// One operator token observed by a language analyzer.
///
/// Per the rewrite plan §5.1:
/// - language crates emit per-token operator/operand events,
/// - `mehen-metrics` owns set-based `n1`/`n2` deduplication and `N1`/`N2`
///   totals,
/// - language crates own classification (e.g. "Python `String` is operand
///   only when not a docstring"); they decide *what* to emit, the builder
///   decides *how to count it*.
///
/// The `kind` field is the language-side classification (the AST kind name
/// or a stable token category). The `text` field, when present, is used for
/// operand-text deduplication where the language wants it (variable names,
/// numeric literals normalized to a canonical form, …).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HalsteadOperator {
    pub kind: SmolStr,
    pub text: Option<SmolStr>,
}

/// One operand token observed by a language analyzer.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HalsteadOperand {
    pub kind: SmolStr,
    pub text: Option<SmolStr>,
}

/// Counts derived from a [`HalsteadBuilder`]: distinct operators (`n1`),
/// distinct operands (`n2`), total operators (`N1`), total operands (`N2`).
///
/// Volume / difficulty / effort live on `mehen_metrics::HalsteadStats`
/// once Phase 3 finalizes that struct; this builder only owns the
/// dedup/totalling step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HalsteadCounts {
    pub n1: u32,
    pub n2: u32,
    pub big_n1: u32,
    pub big_n2: u32,
}

/// Accumulates operator/operand events for one space.
///
/// The dedup key is the operator/operand value as emitted by the language
/// crate. The crate is responsible for normalizing token text where it
/// wants language-specific behavior (Python docstrings → not operands;
/// JavaScript numeric `0x10` and `16` should canonicalize to the same
/// operand if the language crate chooses to).
#[derive(Default, Debug, Clone)]
pub struct HalsteadBuilder {
    operators: HashSet<HalsteadOperator>,
    operands: HashSet<HalsteadOperand>,
    big_n1: u32,
    big_n2: u32,
}

impl HalsteadBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe_operator(&mut self, op: HalsteadOperator) {
        self.big_n1 = self.big_n1.saturating_add(1);
        self.operators.insert(op);
    }

    pub fn observe_operand(&mut self, op: HalsteadOperand) {
        self.big_n2 = self.big_n2.saturating_add(1);
        self.operands.insert(op);
    }

    pub fn counts(&self) -> HalsteadCounts {
        HalsteadCounts {
            n1: self.operators.len() as u32,
            n2: self.operands.len() as u32,
            big_n1: self.big_n1,
            big_n2: self.big_n2,
        }
    }

    /// Iterator over distinct operator entries — for tests / diagnostics.
    pub fn operators(&self) -> impl Iterator<Item = &HalsteadOperator> {
        self.operators.iter()
    }

    /// Iterator over distinct operand entries — for tests / diagnostics.
    pub fn operands(&self) -> impl Iterator<Item = &HalsteadOperand> {
        self.operands.iter()
    }

    /// Merge counts from a child space (post-finalize) into this one.
    pub fn merge(&mut self, other: &HalsteadBuilder) {
        for op in &other.operators {
            self.operators.insert(op.clone());
        }
        for op in &other.operands {
            self.operands.insert(op.clone());
        }
        self.big_n1 = self.big_n1.saturating_add(other.big_n1);
        self.big_n2 = self.big_n2.saturating_add(other.big_n2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op(kind: &str) -> HalsteadOperator {
        HalsteadOperator {
            kind: SmolStr::new(kind),
            text: None,
        }
    }

    fn operand(text: &str) -> HalsteadOperand {
        HalsteadOperand {
            kind: SmolStr::new("identifier"),
            text: Some(SmolStr::new(text)),
        }
    }

    #[test]
    fn dedups_operators() {
        let mut b = HalsteadBuilder::new();
        b.observe_operator(op("+"));
        b.observe_operator(op("+"));
        b.observe_operator(op("-"));
        let c = b.counts();
        assert_eq!(c.n1, 2);
        assert_eq!(c.big_n1, 3);
    }

    #[test]
    fn dedups_operands_by_text() {
        let mut b = HalsteadBuilder::new();
        b.observe_operand(operand("x"));
        b.observe_operand(operand("x"));
        b.observe_operand(operand("y"));
        let c = b.counts();
        assert_eq!(c.n2, 2);
        assert_eq!(c.big_n2, 3);
    }

    #[test]
    fn merge_unions_distinct_sets() {
        let mut a = HalsteadBuilder::new();
        a.observe_operator(op("+"));
        let mut b = HalsteadBuilder::new();
        b.observe_operator(op("-"));
        a.merge(&b);
        let c = a.counts();
        assert_eq!(c.n1, 2);
        assert_eq!(c.big_n1, 2);
    }
}
