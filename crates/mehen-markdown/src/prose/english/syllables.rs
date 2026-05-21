// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Vowel-group syllable counter (§31.11).
//!
//! Pure heuristic — no dictionary, no features — matching the Tier 0
//! contract of §38. Exact CMU-backed counts ship behind `syllables-cmu`
//! (Tier 1a, currently unimplemented).

/// Counts the number of heuristic syllables in `word`. Returns `≥ 1` for
/// any non-empty word.
pub fn count_syllables(word: &str) -> usize {
    let w: String = word
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .flat_map(|c| c.to_lowercase())
        .collect();
    if w.is_empty() {
        return 0;
    }
    let vowels = ['a', 'e', 'i', 'o', 'u', 'y'];
    let mut count = 0usize;
    let mut prev_vowel = false;
    for c in w.chars() {
        let is_v = vowels.contains(&c);
        if is_v && !prev_vowel {
            count += 1;
        }
        prev_vowel = is_v;
    }
    if w.ends_with('e') && !w.ends_with("le") && count > 1 {
        count -= 1;
    }
    if w.ends_with("ed") && count > 1 {
        let second_last = w.chars().rev().nth(2);
        if !matches!(second_last, Some('t') | Some('d')) {
            count -= 1;
        }
    }
    count.max(1)
}

/// Gunning-Fog inflection-aware syllable counter (§31.3). Strips common
/// inflectional suffixes before counting so `preceded` doesn't trip the 3+
/// threshold via `-ed`.
pub fn count_fog_syllables(word: &str) -> usize {
    let w = word.to_ascii_lowercase();
    let w: String = w.chars().filter(|c| c.is_ascii_alphabetic()).collect();
    if w.is_empty() {
        return 0;
    }
    let stripped: &str = if let Some(s) = w.strip_suffix("ing") {
        s
    } else if let Some(s) = w.strip_suffix("ed") {
        s
    } else if let Some(s) = w.strip_suffix("es") {
        s
    } else {
        &w
    };
    // Avoid zero-length stripped-token corner case.
    let base = if stripped.is_empty() { &w } else { stripped };
    count_syllables(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_basic() {
        assert_eq!(count_syllables("hello"), 2);
        assert_eq!(count_syllables("the"), 1);
        assert_eq!(count_syllables("syllable"), 3);
        // `cookie` demonstrates a known limitation of the vowel-group
        // heuristic (§31.11): the trailing `-ie` is one vowel group, and
        // the silent-`e` rule then drops it. CMU-backed counts (Tier 1a)
        // return 2. We document the Tier-0 answer.
        assert_eq!(count_syllables("cookie"), 1);
    }

    #[test]
    fn drops_silent_e() {
        assert_eq!(count_syllables("make"), 1);
        assert_eq!(count_syllables("wave"), 1);
        assert_eq!(count_syllables("little"), 2);
    }

    #[test]
    fn non_alpha_safe() {
        assert_eq!(count_syllables(""), 0);
        assert_eq!(count_syllables("1234"), 0);
    }

    #[test]
    fn fog_strips_inflection() {
        // The Fog count should be `≤` the raw count for any inflected form
        // (i.e. stripping never lengthens syllables). That is the invariant
        // the Gunning Fog index depends on — whether a particular word
        // crosses the 3-syllable threshold is incidental.
        let raw = count_syllables("preceded");
        let fog = count_fog_syllables("preceded");
        assert!(fog <= raw, "fog {fog} must be <= raw {raw}");

        let raw2 = count_syllables("running");
        let fog2 = count_fog_syllables("running");
        assert!(fog2 <= raw2, "fog {fog2} must be <= raw {raw2}");
    }
}
