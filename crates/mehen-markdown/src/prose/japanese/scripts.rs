// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Japanese Unicode-script classification and script-run statistics
//! (§§34.1–34.4).
//!
//! Each grapheme cluster is bucketed into one of five visible classes:
//! hiragana, katakana, kanji (Han), latin, digit. Whitespace and CJK/ASCII
//! punctuation are excluded from ratios (§34.2).
//!
//! Script-run statistics feed Tateishi's formula (§35.1). A "run" is a
//! maximal substring of the same script class.

use serde::Serialize;
use unicode_script::{Script, UnicodeScript};

#[derive(Debug, Clone, Serialize, Default)]
pub struct ScriptComposition {
    pub kanji_ratio: f64,
    pub hiragana_ratio: f64,
    pub katakana_ratio: f64,
    pub latin_ratio: f64,
    pub digit_ratio: f64,
    pub script_entropy: f64,
    pub visible_chars: u64,
}

/// A run of characters in a single script class.
#[derive(Debug, Clone, Copy)]
pub struct Run {
    pub class: Class,
    pub length: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Hiragana,
    Katakana,
    Kanji,
    Latin,
    Digit,
    Other, // whitespace / punctuation / symbol
}

/// Computes both the visible composition and the run list in a single pass.
pub fn analyze(text: &str) -> (ScriptComposition, Vec<Run>) {
    let mut hira = 0u64;
    let mut kata = 0u64;
    let mut han = 0u64;
    let mut lat = 0u64;
    let mut dig = 0u64;
    let mut visible = 0u64;

    let mut runs: Vec<Run> = Vec::new();
    let mut current_class: Option<Class> = None;
    let mut current_len: u32 = 0;

    for c in text.chars() {
        let class = classify(c);

        // Count visible chars and their category shares.
        match class {
            Class::Hiragana => {
                hira += 1;
                visible += 1;
            }
            Class::Katakana => {
                kata += 1;
                visible += 1;
            }
            Class::Kanji => {
                han += 1;
                visible += 1;
            }
            Class::Latin => {
                lat += 1;
                visible += 1;
            }
            Class::Digit => {
                dig += 1;
                visible += 1;
            }
            Class::Other => {
                // Don't count toward visible or ratios.
            }
        }

        // Update run list — `Other` breaks a run but is not itself a run.
        if class == Class::Other {
            if let Some(cl) = current_class.take() {
                if current_len > 0 {
                    runs.push(Run {
                        class: cl,
                        length: current_len,
                    });
                }
                current_len = 0;
            }
            continue;
        }
        match current_class {
            Some(cl) if cl == class => {
                current_len += 1;
            }
            Some(_) => {
                runs.push(Run {
                    class: current_class.unwrap(),
                    length: current_len,
                });
                current_class = Some(class);
                current_len = 1;
            }
            None => {
                current_class = Some(class);
                current_len = 1;
            }
        }
    }
    if let Some(cl) = current_class {
        runs.push(Run {
            class: cl,
            length: current_len,
        });
    }

    let total = visible.max(1) as f64;
    let hir_r = hira as f64 / total;
    let kat_r = kata as f64 / total;
    let kan_r = han as f64 / total;
    let lat_r = lat as f64 / total;
    let dig_r = dig as f64 / total;
    let entropy = shannon_entropy(&[hir_r, kat_r, kan_r, lat_r, dig_r]);

    let composition = ScriptComposition {
        kanji_ratio: round3(kan_r),
        hiragana_ratio: round3(hir_r),
        katakana_ratio: round3(kat_r),
        latin_ratio: round3(lat_r),
        digit_ratio: round3(dig_r),
        script_entropy: round3(entropy),
        visible_chars: visible,
    };
    (composition, runs)
}

fn classify(c: char) -> Class {
    let u = c as u32;
    if c.is_whitespace() {
        return Class::Other;
    }
    // CJK punctuation / full-width punctuation: Other.
    if (0x3000..=0x303F).contains(&u) || (0xFF00..=0xFF0F).contains(&u) {
        return Class::Other;
    }
    if c.is_ascii_punctuation() {
        return Class::Other;
    }
    if (0x3040..=0x309F).contains(&u) || (0x1B130..=0x1B16F).contains(&u) {
        return Class::Hiragana;
    }
    if (0x30A0..=0x30FF).contains(&u)
        || (0x31F0..=0x31FF).contains(&u)
        || (0xFF65..=0xFF9F).contains(&u)
    {
        return Class::Katakana;
    }
    if matches!(c.script(), Script::Han) {
        return Class::Kanji;
    }
    if c.is_ascii_alphabetic() || (0xFF21..=0xFF3A).contains(&u) || (0xFF41..=0xFF5A).contains(&u) {
        return Class::Latin;
    }
    if c.is_ascii_digit() || (0xFF10..=0xFF19).contains(&u) {
        return Class::Digit;
    }
    Class::Other
}

fn shannon_entropy(probs: &[f64]) -> f64 {
    let mut e = 0.0;
    for &p in probs {
        if p > 0.0 {
            e -= p * p.log2();
        }
    }
    e
}

fn round3(x: f64) -> f64 {
    if !x.is_finite() {
        return 0.0;
    }
    (x * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_hiragana() {
        let (c, runs) = analyze("あいうえお");
        assert_eq!(c.visible_chars, 5);
        assert!((c.hiragana_ratio - 1.0).abs() < 0.01);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].length, 5);
    }

    #[test]
    fn classify_mixed() {
        let (c, _) = analyze("日本語は hello と ABC123 です");
        assert!(c.kanji_ratio > 0.0);
        assert!(c.hiragana_ratio > 0.0);
        assert!(c.latin_ratio > 0.0);
        assert!(c.digit_ratio > 0.0);
    }
}
