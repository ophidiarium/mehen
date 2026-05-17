use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// LOC line classification.
///
/// Per the rewrite plan §5.1, *what counts as a comment* (heredoc, doc
/// string, template literal, preprocessor line) is language-specific and
/// stays inside language analyzer crates. The accounting helper here
/// records each classified line into the correct bucket.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineClass {
    Blank,
    /// Pure comment line — no source code.
    Comment,
    /// Source line — contains code; may also contain trailing comment.
    Code,
    /// Logical line — counts toward `lloc`. Distinct from `code` because
    /// not every code line is logically meaningful (e.g. closing braces
    /// alone).
    Logical,
}

/// Accumulator for the LOC family.
///
/// The pre-1.0 implementation lives at `src/metrics/loc.rs`; the field set
/// here matches it so parity snapshots compare directly. Min/max/avg
/// finalization is computed at finalize time, not on every observation.
#[derive(Default, Clone, Debug, PartialEq)]
pub struct LocStats {
    /// Source lines of code (sloc): non-blank source lines (includes
    /// comments).
    pub sloc: u32,
    /// Physical lines of code (ploc): non-blank, non-comment lines that
    /// contain code.
    pub ploc: u32,
    /// Logical lines of code (lloc).
    pub lloc: u32,
    /// Comment-only lines.
    pub cloc: u32,
    /// Blank lines.
    pub blank: u32,
    /// Total physical lines.
    pub total: u32,
}

impl LocStats {
    pub fn observe(&mut self, class: LineClass) {
        self.total = self.total.saturating_add(1);
        match class {
            LineClass::Blank => self.blank = self.blank.saturating_add(1),
            LineClass::Comment => {
                self.cloc = self.cloc.saturating_add(1);
                self.sloc = self.sloc.saturating_add(1);
            }
            LineClass::Code => {
                self.ploc = self.ploc.saturating_add(1);
                self.sloc = self.sloc.saturating_add(1);
            }
            LineClass::Logical => {
                self.lloc = self.lloc.saturating_add(1);
            }
        }
    }

    pub fn merge(&mut self, other: &LocStats) {
        self.sloc = self.sloc.saturating_add(other.sloc);
        self.ploc = self.ploc.saturating_add(other.ploc);
        self.lloc = self.lloc.saturating_add(other.lloc);
        self.cloc = self.cloc.saturating_add(other.cloc);
        self.blank = self.blank.saturating_add(other.blank);
        self.total = self.total.saturating_add(other.total);
    }

    /// Comments-as-fraction-of-sloc, used by the maintainability index.
    pub fn comments_percentage(&self) -> f64 {
        if self.sloc == 0 {
            0.0
        } else {
            (self.cloc as f64) / (self.sloc as f64)
        }
    }
}

impl Serialize for LocStats {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut st = serializer.serialize_struct("loc", 6)?;
        st.serialize_field("sloc", &self.sloc)?;
        st.serialize_field("ploc", &self.ploc)?;
        st.serialize_field("lloc", &self.lloc)?;
        st.serialize_field("cloc", &self.cloc)?;
        st.serialize_field("blank", &self.blank)?;
        st.serialize_field("total", &self.total)?;
        st.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_classifies_correctly() {
        let mut s = LocStats::default();
        s.observe(LineClass::Code);
        s.observe(LineClass::Comment);
        s.observe(LineClass::Blank);
        s.observe(LineClass::Logical);
        assert_eq!(s.sloc, 2);
        assert_eq!(s.ploc, 1);
        assert_eq!(s.cloc, 1);
        assert_eq!(s.lloc, 1);
        assert_eq!(s.blank, 1);
        assert_eq!(s.total, 4);
    }

    #[test]
    fn comments_percentage_when_empty() {
        let s = LocStats::default();
        assert_eq!(s.comments_percentage(), 0.0);
    }

    #[test]
    fn merge_sums_buckets() {
        let mut a = LocStats {
            sloc: 1,
            ploc: 1,
            lloc: 1,
            cloc: 0,
            blank: 0,
            total: 1,
        };
        let b = LocStats {
            sloc: 2,
            ploc: 1,
            lloc: 1,
            cloc: 1,
            blank: 1,
            total: 3,
        };
        a.merge(&b);
        assert_eq!(a.sloc, 3);
        assert_eq!(a.ploc, 2);
        assert_eq!(a.lloc, 2);
        assert_eq!(a.cloc, 1);
        assert_eq!(a.blank, 1);
        assert_eq!(a.total, 4);
    }
}
