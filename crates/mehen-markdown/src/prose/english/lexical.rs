// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! English lexical diversity and sentence/word moments (§32).
//!
//! Tier 0 scope: MATTR₅₀, hapax ratio, dis-legomena ratio, lexical density
//! (via NLTK stopwords), sentence/word-length moments. MTLD / HD-D / Yule's K
//! are Tier 2 and live behind `--features lexical-diversity` (not implemented
//! here).

use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::OnceLock;

use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct EnglishLexical {
    pub mattr_50: f64,
    pub hapax_ratio: f64,
    pub dis_ratio: f64,
    pub lexical_density: f64,
    pub avg_sentence_words: f64,
    pub p90_sentence_words: u64,
    pub max_sentence_words: u64,
    pub stddev_sentence_words: f64,
    pub avg_word_chars: f64,
    pub p90_word_chars: u64,
    pub sentence_count: u64,
    pub words_total: u64,
}

fn stopwords_set() -> &'static HashSet<String> {
    static CELL: OnceLock<HashSet<String>> = OnceLock::new();
    CELL.get_or_init(|| {
        let raw = include_str!("../../data/nltk_stopwords_en.txt");
        raw.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.to_ascii_lowercase())
            .collect()
    })
}

pub fn analyze(words_per_sent: &[Vec<String>], words_flat: &[String]) -> EnglishLexical {
    let words_total = words_flat.len() as u64;
    let sentence_lengths: Vec<u64> = words_per_sent
        .iter()
        .filter(|s| !s.is_empty())
        .map(|s| s.len() as u64)
        .collect();
    let sentence_count = sentence_lengths.len() as u64;

    // Sentence-length moments.
    let avg_sent = if !sentence_lengths.is_empty() {
        sentence_lengths.iter().sum::<u64>() as f64 / sentence_lengths.len() as f64
    } else {
        0.0
    };
    let p90_sent = percentile_u64(&sentence_lengths, 90);
    let max_sent = sentence_lengths.iter().copied().max().unwrap_or(0);
    let stddev_sent = stddev(&sentence_lengths, avg_sent);

    // Word-char moments. `chars().count()` is the Unicode scalar length.
    let word_lens: Vec<u64> = words_flat
        .iter()
        .map(|w| w.chars().count() as u64)
        .collect();
    let avg_word = if !word_lens.is_empty() {
        word_lens.iter().sum::<u64>() as f64 / word_lens.len() as f64
    } else {
        0.0
    };
    let p90_word = percentile_u64(&word_lens, 90);

    // Diversity: MATTR, hapax, dis. Lowercase the tokens so case doesn't
    // double-count types.
    let norm: Vec<String> = words_flat.iter().map(|w| w.to_ascii_lowercase()).collect();
    let types_total = {
        let s: HashSet<&String> = norm.iter().collect();
        s.len() as f64
    };

    let mattr_50 = mattr(&norm, 50);
    let (hapax, dis) = hapax_and_dis_ratio(&norm);

    // Lexical density ≈ 1 − stopwords/tokens.
    let stop_set = stopwords_set();
    let stop_count = norm.iter().filter(|w| stop_set.contains(*w)).count() as f64;
    let lexical_density = if !norm.is_empty() {
        1.0 - (stop_count / norm.len() as f64)
    } else {
        0.0
    };
    // Unused variable suppression: types_total is returned implicitly via
    // hapax/dis denominators; we drop it.
    let _ = types_total;

    EnglishLexical {
        mattr_50: round3(mattr_50),
        hapax_ratio: round3(hapax),
        dis_ratio: round3(dis),
        lexical_density: round3(lexical_density),
        avg_sentence_words: round3(avg_sent),
        p90_sentence_words: p90_sent,
        max_sentence_words: max_sent,
        stddev_sentence_words: round3(stddev_sent),
        avg_word_chars: round3(avg_word),
        p90_word_chars: p90_word,
        sentence_count,
        words_total,
    }
}

fn round3(x: f64) -> f64 {
    if !x.is_finite() {
        return 0.0;
    }
    (x * 1000.0).round() / 1000.0
}

/// Returns the p-th percentile (0..100) of a `u64` vector using
/// nearest-rank. Deterministic; no interpolation.
fn percentile_u64(values: &[u64], p: u8) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort();
    let rank = ((p as f64 / 100.0) * sorted.len() as f64).ceil() as usize;
    let idx = rank.saturating_sub(1).min(sorted.len() - 1);
    sorted[idx]
}

fn stddev(values: &[u64], mean: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let var: f64 = values
        .iter()
        .map(|&v| {
            let d = v as f64 - mean;
            d * d
        })
        .sum::<f64>()
        / values.len() as f64;
    var.sqrt()
}

/// Moving-average type-token ratio. Window size `w` (§32.2). If fewer than
/// `w` tokens are available, returns the single TTR over the full corpus.
fn mattr(tokens: &[String], w: usize) -> f64 {
    if tokens.is_empty() {
        return 0.0;
    }
    if tokens.len() < w {
        let types: HashSet<&String> = tokens.iter().collect();
        return types.len() as f64 / tokens.len() as f64;
    }
    let mut sum = 0.0f64;
    let mut windows = 0usize;
    for start in 0..=(tokens.len() - w) {
        let window = &tokens[start..start + w];
        let types: HashSet<&String> = window.iter().collect();
        sum += types.len() as f64 / w as f64;
        windows += 1;
    }
    if windows == 0 {
        return 0.0;
    }
    sum / windows as f64
}

/// Returns (`V1/V`, `V2/V`) — hapax ratio and dis-legomena ratio.
fn hapax_and_dis_ratio(tokens: &[String]) -> (f64, f64) {
    if tokens.is_empty() {
        return (0.0, 0.0);
    }
    let mut counts: HashMap<&String, u64> = HashMap::new();
    for t in tokens {
        *counts.entry(t).or_insert(0) += 1;
    }
    let v = counts.len() as f64;
    if v == 0.0 {
        return (0.0, 0.0);
    }
    let v1 = counts.values().filter(|&&c| c == 1).count() as f64;
    let v2 = counts.values().filter(|&&c| c == 2).count() as f64;
    (v1 / v, v2 / v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mattr_single_window() {
        let t: Vec<String> = vec!["a", "b", "c", "a", "b", "c"]
            .into_iter()
            .map(String::from)
            .collect();
        let m = mattr(&t, 3);
        assert!((m - 1.0).abs() < 0.01);
    }

    #[test]
    fn hapax_all_unique() {
        let t: Vec<String> = vec!["a", "b", "c"].into_iter().map(String::from).collect();
        let (h, _) = hapax_and_dis_ratio(&t);
        assert!((h - 1.0).abs() < 0.01);
    }
}
