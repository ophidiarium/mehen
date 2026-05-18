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

    /// Min/max bounds across spaces in the rolled-up tree. Per-space LOC
    /// values are folded in at `finalize_minmax`. Initial value of 0 is a
    /// sentinel meaning "no value seen"; `finalize_minmax` upgrades from
    /// the sentinel on the first call.
    pub sloc_min: u32,
    pub sloc_max: u32,
    pub ploc_min: u32,
    pub ploc_max: u32,
    pub lloc_min: u32,
    pub lloc_max: u32,
    pub cloc_min: u32,
    pub cloc_max: u32,
    pub blank_min: u32,
    pub blank_max: u32,
    /// Number of spaces folded into the bounds — used to compute averages
    /// in `finalize_average`.
    pub space_count: u32,
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
        let prior = self.space_count;
        self.space_count = self.space_count.saturating_add(other.space_count);
        if other.space_count == 0 {
            return;
        }
        merge_min(&mut self.sloc_min, prior, other.sloc_min);
        self.sloc_max = self.sloc_max.max(other.sloc_max);
        merge_min(&mut self.ploc_min, prior, other.ploc_min);
        self.ploc_max = self.ploc_max.max(other.ploc_max);
        merge_min(&mut self.lloc_min, prior, other.lloc_min);
        self.lloc_max = self.lloc_max.max(other.lloc_max);
        merge_min(&mut self.cloc_min, prior, other.cloc_min);
        self.cloc_max = self.cloc_max.max(other.cloc_max);
        merge_min(&mut self.blank_min, prior, other.blank_min);
        self.blank_max = self.blank_max.max(other.blank_max);
    }

    /// Fold the current per-space sloc/ploc/lloc/cloc/blank values into
    /// the *_min / *_max bounds. Should be called once per space before
    /// merging into the parent. `space_count` is bumped here too.
    pub fn finalize_minmax(&mut self) {
        let prior = self.space_count;
        self.space_count = self.space_count.saturating_add(1);
        merge_min(&mut self.sloc_min, prior, self.sloc);
        self.sloc_max = self.sloc_max.max(self.sloc);
        merge_min(&mut self.ploc_min, prior, self.ploc);
        self.ploc_max = self.ploc_max.max(self.ploc);
        merge_min(&mut self.lloc_min, prior, self.lloc);
        self.lloc_max = self.lloc_max.max(self.lloc);
        merge_min(&mut self.cloc_min, prior, self.cloc);
        self.cloc_max = self.cloc_max.max(self.cloc);
        merge_min(&mut self.blank_min, prior, self.blank);
        self.blank_max = self.blank_max.max(self.blank);
    }

    /// Comments-as-fraction-of-sloc, used by the maintainability index.
    pub fn comments_percentage(&self) -> f64 {
        if self.sloc == 0 {
            0.0
        } else {
            (self.cloc as f64) / (self.sloc as f64)
        }
    }

    pub fn sloc_average(&self) -> f64 {
        average(self.sloc, self.space_count)
    }
    pub fn ploc_average(&self) -> f64 {
        average(self.ploc, self.space_count)
    }
    pub fn lloc_average(&self) -> f64 {
        average(self.lloc, self.space_count)
    }
    pub fn cloc_average(&self) -> f64 {
        average(self.cloc, self.space_count)
    }
    pub fn blank_average(&self) -> f64 {
        average(self.blank, self.space_count)
    }
}

/// `prior_count` is the `space_count` *before* merging the new candidate.
/// When prior_count is 0 the target is uninitialized — adopt candidate
/// directly. Otherwise take the smaller of the two.
fn merge_min(target: &mut u32, prior_count: u32, candidate: u32) {
    if prior_count == 0 {
        *target = candidate;
    } else {
        *target = (*target).min(candidate);
    }
}

fn average(numerator: u32, denominator: u32) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        f64::from(numerator) / f64::from(denominator)
    }
}

impl Serialize for LocStats {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut st = serializer.serialize_struct("loc", 20)?;
        st.serialize_field("sloc", &self.sloc)?;
        st.serialize_field("ploc", &self.ploc)?;
        st.serialize_field("lloc", &self.lloc)?;
        st.serialize_field("cloc", &self.cloc)?;
        st.serialize_field("blank", &self.blank)?;
        st.serialize_field("sloc_average", &self.sloc_average())?;
        st.serialize_field("ploc_average", &self.ploc_average())?;
        st.serialize_field("lloc_average", &self.lloc_average())?;
        st.serialize_field("cloc_average", &self.cloc_average())?;
        st.serialize_field("blank_average", &self.blank_average())?;
        st.serialize_field("sloc_min", &self.sloc_min)?;
        st.serialize_field("sloc_max", &self.sloc_max)?;
        st.serialize_field("cloc_min", &self.cloc_min)?;
        st.serialize_field("cloc_max", &self.cloc_max)?;
        st.serialize_field("ploc_min", &self.ploc_min)?;
        st.serialize_field("ploc_max", &self.ploc_max)?;
        st.serialize_field("lloc_min", &self.lloc_min)?;
        st.serialize_field("lloc_max", &self.lloc_max)?;
        st.serialize_field("blank_min", &self.blank_min)?;
        st.serialize_field("blank_max", &self.blank_max)?;
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
            ..Default::default()
        };
        let b = LocStats {
            sloc: 2,
            ploc: 1,
            lloc: 1,
            cloc: 1,
            blank: 1,
            total: 3,
            ..Default::default()
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
