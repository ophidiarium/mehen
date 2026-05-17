//! Section Balance Score per §20.
//!
//! ```text
//! section_word_counts = [W_s for each section s]
//! median_section_words = median(section_word_counts)
//! p95_section_words = percentile(section_word_counts, 95)
//! large_section_rate = count(W_s > 1200) / max(1, S)
//! tiny_section_rate = count(W_s < 40) / max(1, S)
//! heading_skip_rate = heading_skips / max(1, H)
//! ```
//!
//! ```text
//! SectionBalanceScore = clamp01(
//!     1
//!   - 0.30 * sat(p95_section_words; 900, 2000)
//!   - 0.25 * sat(large_section_rate; 0.05, 0.40)
//!   - 0.15 * sat(tiny_section_rate; 0.20, 0.70)
//!   - 0.20 * sat(heading_skip_rate; 0.02, 0.20)
//!   - 0.10 * sat(abs(max_heading_depth - expected_depth); 2, 5)
//! )
//! ```
//!
//! `expected_depth` is profile-specific. Phase D uses a single default of
//! `3` (typical for README / technical reference docs) until profile-aware
//! thresholds ship in a later phase. Document-type profiles live in §22.

use crate::markdown::mathops::{clamp01, sat};
use crate::markdown::types::Section;

/// Default `expected_depth` for §20 until profile-aware thresholds land.
const EXPECTED_DEPTH_DEFAULT: f64 = 3.0;

/// Computed §20 output plus the intermediate signals so §17.4 (lazy
/// sectioning) and the DMI (S_norm = 1 - SectionBalanceScore) can reuse
/// them.
///
/// Fields marked `#[allow(dead_code)]` are kept for Phase F's `mehen diff`
/// sticky comment even though the analyzer does not read them directly.
#[derive(Debug, Default, Clone)]
pub(crate) struct SectionBalance {
    pub(crate) section_balance_score: f64,
    #[allow(dead_code)]
    pub(crate) p95_section_words: f64,
    #[allow(dead_code)]
    pub(crate) median_section_words: f64,
    #[allow(dead_code)]
    pub(crate) large_section_rate: f64,
    #[allow(dead_code)]
    pub(crate) tiny_section_rate: f64,
    #[allow(dead_code)]
    pub(crate) heading_skip_rate: f64,
    #[allow(dead_code)]
    pub(crate) max_heading_depth: u8,
    pub(crate) long_section_rate: f64,
    #[allow(dead_code)]
    pub(crate) heading_count: u64,
    pub(crate) shallow_large_doc: bool,
}

/// Computes §20's Section Balance Score from the Phase-A section list.
pub(crate) fn analyze_section_balance(sections: &[Section], words: u64) -> SectionBalance {
    // A document with no sections is structurally balanced by definition
    // (there is nothing to imbalance). Return a perfect score instead of
    // computing abs(0 - expected_depth) which would spuriously penalize an
    // empty document.
    if sections.is_empty() {
        return SectionBalance {
            section_balance_score: 1.0,
            ..SectionBalance::default()
        };
    }

    let s = sections.len() as f64;
    let s_max = s.max(1.0);

    let mut word_counts: Vec<u64> = sections.iter().map(|sec| sec.word_count).collect();
    let large = word_counts.iter().filter(|w| **w > 1200).count() as f64;
    let tiny = word_counts.iter().filter(|w| **w < 40).count() as f64;

    let large_section_rate = large / s_max;
    let tiny_section_rate = tiny / s_max;
    let long_section_rate = large_section_rate;

    let p95 = percentile_u64(&mut word_counts, 0.95);
    let median = percentile_u64(&mut word_counts, 0.50);

    // §8.1-style heading skip count: Σ max(0, child_level - parent_level - 1)
    // over parent / child heading pairs; plus the count of top-level headings
    // that start at level > 1 (document opens with `###` etc. is a skip).
    let heading_skip_rate = heading_skip_rate(sections);
    let max_depth = max_heading_depth(sections);

    let shallow_large_doc = words > 2500 && max_depth <= 2;

    let raw = 1.0
        - 0.30 * sat(p95, 900.0, 2000.0)
        - 0.25 * sat(large_section_rate, 0.05, 0.40)
        - 0.15 * sat(tiny_section_rate, 0.20, 0.70)
        - 0.20 * sat(heading_skip_rate, 0.02, 0.20)
        - 0.10 * sat((max_depth as f64 - EXPECTED_DEPTH_DEFAULT).abs(), 2.0, 5.0);

    SectionBalance {
        section_balance_score: clamp01(raw),
        p95_section_words: p95,
        median_section_words: median,
        large_section_rate,
        tiny_section_rate,
        heading_skip_rate,
        max_heading_depth: max_depth,
        long_section_rate,
        heading_count: sections.len() as u64,
        shallow_large_doc,
    }
}

/// Percentile of `u64` word counts using type-7 linear interpolation.
fn percentile_u64(values: &mut [u64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort();
    let n = values.len();
    if n == 1 {
        return values[0] as f64;
    }
    let pos = q * (n as f64 - 1.0);
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    if lo == hi {
        values[lo] as f64
    } else {
        let frac = pos - lo as f64;
        values[lo] as f64 * (1.0 - frac) + values[hi] as f64 * frac
    }
}

/// `heading_skip_rate` = (heading skips) / max(1, H). A heading skip is a
/// child heading that jumps more than one level below its parent (e.g. H1 →
/// H3). Top-level sections whose heading level is > 1 also count as a skip
/// because the document implicitly "jumps" past H1.
fn heading_skip_rate(sections: &[Section]) -> f64 {
    let h = sections.len();
    if h == 0 {
        return 0.0;
    }
    let mut skips = 0u64;
    for s in sections {
        let child_level = s.heading_level.unwrap_or(1);
        let parent_level = match s.parent_section_id {
            Some(p) => sections
                .iter()
                .find(|x| x.section_id == p)
                .and_then(|x| x.heading_level)
                .unwrap_or(0),
            None => 0, // top-level: "parent" is level 0 conceptually
        };
        let jump = (child_level as i32) - (parent_level as i32);
        // A top-level H1 (parent_level=0, child_level=1, jump=1) is NOT a
        // skip. A top-level H3 (parent_level=0, child_level=3, jump=3) IS.
        // Nested H3 under H1 (parent_level=1, child_level=3, jump=2) IS.
        let is_skip = if parent_level == 0 {
            child_level > 1 && jump >= 2
        } else {
            jump >= 2
        };
        if is_skip {
            skips += 1;
        }
    }
    skips as f64 / h as f64
}

fn max_heading_depth(sections: &[Section]) -> u8 {
    sections
        .iter()
        .filter_map(|s| s.heading_level)
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_section(id: usize, level: u8, parent: Option<usize>, words: u64) -> Section {
        Section {
            section_id: id,
            heading_level: Some(level),
            heading_text: None,
            start_line: id as u64 + 1,
            end_line: id as u64 + 2,
            parent_section_id: parent,
            child_section_ids: Vec::new(),
            word_count: words,
            block_count: 1,
        }
    }

    #[test]
    fn empty_sections_produce_perfect_balance() {
        let out = analyze_section_balance(&[], 0);
        assert_eq!(out.section_balance_score, 1.0);
        assert_eq!(out.heading_count, 0);
    }

    #[test]
    fn heading_skip_rate_counts_h1_to_h3_jump() {
        let sections = vec![mk_section(0, 1, None, 50), mk_section(1, 3, Some(0), 50)];
        let out = analyze_section_balance(&sections, 100);
        assert!(out.heading_skip_rate > 0.0);
    }

    #[test]
    fn large_sections_lower_the_score() {
        let sections = vec![mk_section(0, 1, None, 3000)];
        let out = analyze_section_balance(&sections, 3000);
        // p95 is 3000 → saturates to 1.0, losing 0.30. Large rate = 1.0 → -0.25.
        assert!(out.section_balance_score < 0.6);
    }

    #[test]
    fn shallow_large_doc_flag_fires() {
        let sections = vec![mk_section(0, 1, None, 3000)];
        let out = analyze_section_balance(&sections, 3000);
        assert!(out.shallow_large_doc);
    }
}
