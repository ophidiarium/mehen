//! Tateishi simplified Japanese readability score (§35.1).
//!
//! ```text
//! RS = −0.12 * ls − 1.37 * la + 7.4 * lh − 23.18 * lc − 5.4 * lk
//!    − 4.67 * cp + 115.79
//! ```
//!
//! Where:
//!   ls = mean chars per sentence
//!   la = mean chars per alphabet run
//!   lh = mean chars per hiragana run
//!   lc = mean chars per kanji run
//!   lk = mean chars per katakana run
//!   cp = `、` per `。`
//!
//! Calibrated so mean=50, SD=10; higher = easier.

use super::scripts::{Class, Run, ScriptComposition};

pub(crate) fn tateishi_simplified_rs(
    runs: &[Run],
    sents: &[String],
    _composition: &ScriptComposition,
) -> f64 {
    let (la, lh, lc, lk) = run_means(runs);

    // Mean chars per sentence — only visible chars (ignore whitespace /
    // punctuation terminators).
    let mut total_visible = 0u64;
    for s in sents {
        for c in s.chars() {
            if !c.is_whitespace() && !is_sentence_end_punct(c) {
                total_visible += 1;
            }
        }
    }
    let ls = if !sents.is_empty() {
        total_visible as f64 / sents.len() as f64
    } else {
        0.0
    };

    // Comma/period ratio — count `、` and `。` across all sentences.
    let mut comma = 0u64;
    let mut period = 0u64;
    for s in sents {
        for c in s.chars() {
            if c == '、' {
                comma += 1;
            }
            if c == '。' {
                period += 1;
            }
        }
    }
    let cp = if period == 0 {
        0.0
    } else {
        comma as f64 / period as f64
    };

    let rs = -0.12 * ls - 1.37 * la + 7.4 * lh - 23.18 * lc - 5.4 * lk - 4.67 * cp + 115.79;
    round3(rs)
}

fn run_means(runs: &[Run]) -> (f64, f64, f64, f64) {
    let mut la_total = 0u64;
    let mut la_count = 0u64;
    let mut lh_total = 0u64;
    let mut lh_count = 0u64;
    let mut lc_total = 0u64;
    let mut lc_count = 0u64;
    let mut lk_total = 0u64;
    let mut lk_count = 0u64;

    for r in runs {
        let len = r.length as u64;
        match r.class {
            Class::Latin => {
                la_total += len;
                la_count += 1;
            }
            Class::Hiragana => {
                lh_total += len;
                lh_count += 1;
            }
            Class::Kanji => {
                lc_total += len;
                lc_count += 1;
            }
            Class::Katakana => {
                lk_total += len;
                lk_count += 1;
            }
            _ => {}
        }
    }
    let la = if la_count == 0 {
        0.0
    } else {
        la_total as f64 / la_count as f64
    };
    let lh = if lh_count == 0 {
        0.0
    } else {
        lh_total as f64 / lh_count as f64
    };
    let lc = if lc_count == 0 {
        0.0
    } else {
        lc_total as f64 / lc_count as f64
    };
    let lk = if lk_count == 0 {
        0.0
    } else {
        lk_total as f64 / lk_count as f64
    };
    (la, lh, lc, lk)
}

fn is_sentence_end_punct(c: char) -> bool {
    matches!(c, '。' | '！' | '？' | '!' | '?' | '.')
}

fn round3(x: f64) -> f64 {
    if !x.is_finite() {
        return 0.0;
    }
    (x * 1000.0).round() / 1000.0
}
