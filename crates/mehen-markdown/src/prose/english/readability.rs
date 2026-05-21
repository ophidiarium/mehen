// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Classical English readability formulas (§31).
//!
//! Every formula is emitted with provenance — no averaging. Grade-level
//! refusal for SMOG when `sentences < 30` is baked in here.

use std::collections::HashSet;
use std::sync::OnceLock;

use serde::Serialize;

use super::lexical::EnglishLexical;
use super::sentences::count_letters;
use super::syllables::{count_fog_syllables, count_syllables};

/// Cap on letter count contribution per word (§37.5 anti-gaming): a
/// CamelCase / snake_case identifier is at most 20 letters in ARI / CLI.
pub const IDENTIFIER_LEN_CAP: usize = 20;

/// One complete readability report. Grade-level fields use `Option<f64>` so
/// they can be `null` on sub-threshold inputs (SMOG and short-doc guard).
#[derive(Debug, Clone, Serialize, Default)]
pub struct ReadabilityReport {
    pub flesch_reading_ease: Option<f64>,
    pub flesch_kincaid_grade: Option<f64>,
    pub gunning_fog: Option<f64>,
    pub smog: Option<f64>,
    pub ari: Option<f64>,
    pub coleman_liau: Option<f64>,
    pub dale_chall_new: Option<f64>,
    pub dale_chall_list: String,
    pub forcast: Option<f64>,
    pub lix: Option<f64>,
    pub rix: Option<f64>,
    pub ensemble_grade_band: [Option<f64>; 2],
}

/// Returns an all-null report for inputs below the short-doc threshold.
pub fn short_doc_report(_lex: &EnglishLexical) -> ReadabilityReport {
    ReadabilityReport {
        dale_chall_list: "ngsl-1.2".to_string(),
        ..ReadabilityReport::default()
    }
}

pub fn analyze(sents: &[String], words_per_sent: &[Vec<String>]) -> ReadabilityReport {
    let words: Vec<&str> = words_per_sent
        .iter()
        .flatten()
        .map(|s| s.as_str())
        .collect();
    let words_count = words.len() as f64;
    let sent_count = sents.iter().filter(|s| !s.trim().is_empty()).count() as f64;
    if words_count == 0.0 || sent_count == 0.0 {
        return ReadabilityReport {
            dale_chall_list: "ngsl-1.2".to_string(),
            ..ReadabilityReport::default()
        };
    }

    let syllables_total: usize = words.iter().map(|w| count_syllables(w)).sum();
    let polysyllables_total = words.iter().filter(|w| count_syllables(w) >= 3).count() as f64;
    let letters_total: usize = words
        .iter()
        .map(|w| count_letters(w, IDENTIFIER_LEN_CAP))
        .sum();

    // FRES §31.1
    let fres = 206.835
        - 1.015 * (words_count / sent_count)
        - 84.6 * (syllables_total as f64 / words_count);

    // FKGL §31.2
    let fkgl =
        0.39 * (words_count / sent_count) + 11.8 * (syllables_total as f64 / words_count) - 15.59;

    // Fog §31.3 — complex_word = >=3 syllables after stripping inflection +
    // not proper-noun mid-sentence.
    let fog = gunning_fog(sents, words_per_sent);

    // SMOG §31.4 — null if sentences < 30.
    let smog = if sent_count < 30.0 {
        None
    } else {
        Some(1.0430 * ((polysyllables_total * 30.0 / sent_count).sqrt()) + 3.1291)
    };

    // ARI §31.5 — cap word length at IDENTIFIER_LEN_CAP.
    let ari =
        4.71 * (letters_total as f64 / words_count) + 0.5 * (words_count / sent_count) - 21.43;

    // Coleman-Liau §31.6
    let l = 100.0 * letters_total as f64 / words_count;
    let s_per_100w = 100.0 * sent_count / words_count;
    let cli = 0.0588 * l - 0.296 * s_per_100w - 15.8;

    // New Dale-Chall §31.7 — NGSL-backed.
    let dc = dale_chall(&words, words_count, sent_count);

    // FORCAST §31.8 — 150-word sample, monosyllables count.
    let forcast = forcast_score(&words);

    // LIX and RIX §31.9
    let long_words = words.iter().filter(|w| w.chars().count() >= 7).count() as f64;
    let lix = (words_count / sent_count) + 100.0 * (long_words / words_count);
    let rix = long_words / sent_count;

    // Ensemble band (min/max over FKGL, Fog, ARI, CLI).
    let band_values = [fkgl, fog, ari, cli];
    let ensemble_lo = band_values.iter().copied().fold(f64::INFINITY, f64::min);
    let ensemble_hi = band_values
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);

    ReadabilityReport {
        flesch_reading_ease: Some(clamp_round(fres)),
        flesch_kincaid_grade: Some(clamp_round(fkgl)),
        gunning_fog: Some(clamp_round(fog)),
        smog: smog.map(clamp_round),
        ari: Some(clamp_round(ari)),
        coleman_liau: Some(clamp_round(cli)),
        dale_chall_new: Some(clamp_round(dc)),
        dale_chall_list: "ngsl-1.2".to_string(),
        forcast: forcast.map(clamp_round),
        lix: Some(clamp_round(lix)),
        rix: Some(clamp_round(rix)),
        ensemble_grade_band: [
            Some(clamp_round(ensemble_lo)),
            Some(clamp_round(ensemble_hi)),
        ],
    }
}

fn clamp_round(x: f64) -> f64 {
    // Guard against NaN / infinities; round to 3 decimals for stable
    // snapshots.
    if !x.is_finite() {
        return 0.0;
    }
    (x * 1000.0).round() / 1000.0
}

/// Gunning Fog with the §31.3 proper-noun filter (skip capitalize-mid-sentence
/// tokens) and inflection-suffix stripping before syllable counting.
fn gunning_fog(sents: &[String], words_per_sent: &[Vec<String>]) -> f64 {
    let mut total_words = 0usize;
    let mut complex = 0usize;
    let mut total_sents = 0usize;

    for (sent, words) in sents.iter().zip(words_per_sent.iter()) {
        if sent.trim().is_empty() {
            continue;
        }
        total_sents += 1;
        for (i, w) in words.iter().enumerate() {
            total_words += 1;
            // Skip mid-sentence proper nouns: capitalized first char but not
            // the first word of the sentence.
            let first_char = w.chars().next();
            let is_cap = first_char.map(|c| c.is_ascii_uppercase()).unwrap_or(false);
            if is_cap && i > 0 {
                continue;
            }
            let syl = count_fog_syllables(w);
            if syl >= 3 {
                complex += 1;
            }
        }
    }
    if total_words == 0 || total_sents == 0 {
        return 0.0;
    }
    0.4 * ((total_words as f64 / total_sents as f64)
        + 100.0 * (complex as f64 / total_words as f64))
}

/// Returns the NGSL 1.2 headword set, lazy-initialised.
fn ngsl_set() -> &'static HashSet<String> {
    static CELL: OnceLock<HashSet<String>> = OnceLock::new();
    CELL.get_or_init(|| {
        let raw = include_str!("../../data/ngsl_1_2.txt");
        raw.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.to_ascii_lowercase())
            .collect()
    })
}

/// New Dale-Chall score using NGSL 1.2 as the familiar-word list.
fn dale_chall(words: &[&str], words_count: f64, sent_count: f64) -> f64 {
    let ngsl = ngsl_set();
    let difficult = words.iter().filter(|w| !is_familiar(w, ngsl)).count() as f64;
    let pdw = 100.0 * difficult / words_count;
    let asl = words_count / sent_count;
    let raw = 0.1579 * pdw + 0.0496 * asl;
    if pdw > 5.0 { raw + 3.6365 } else { raw }
}

/// Strips common inflectional suffixes before NGSL lookup, matching the
/// "inflectional stripping" rule in §31.7.
fn is_familiar(word: &str, set: &HashSet<String>) -> bool {
    let w = word.to_ascii_lowercase();
    if set.contains(&w) {
        return true;
    }
    for suf in ["es", "ed", "ing", "ly", "s"] {
        if let Some(base) = w.strip_suffix(suf)
            && set.contains(base)
        {
            return true;
        }
    }
    // Adjective -> adverb (`quick` -> `quickly`).
    // Plural / possessive (`runner's`) — drop non-alpha tail.
    let clean: String = w.chars().filter(|c| c.is_alphabetic()).collect();
    if clean != w && set.contains(&clean) {
        return true;
    }
    false
}

/// FORCAST §31.8 — `20 − N/10` where `N` = single-syllable words in a 150-word
/// sample. Returns `None` if `words.len() < 150`.
fn forcast_score(words: &[&str]) -> Option<f64> {
    if words.len() < 150 {
        return None;
    }
    let sample = &words[..150];
    let monosyllables = sample.iter().filter(|w| count_syllables(w) == 1).count() as f64;
    Some(20.0 - (monosyllables / 10.0))
}

#[cfg(test)]
mod tests {
    use super::super::sentences;
    use super::*;

    #[test]
    fn fres_reasonable_for_simple_text() {
        let text = "The cat sat. It looked out. A bird flew. The sun was warm. \
                    The grass was green. It played with a toy. It ran around. \
                    Then it took a nap. It felt happy. It was a good day for the cat.";
        let sents = sentences::split(text);
        let wps: Vec<Vec<String>> = sents
            .iter()
            .map(|s| sentences::words_in_sentence(s))
            .collect();
        let r = analyze(&sents, &wps);
        // Easy text: FRES should be high.
        let fres = r.flesch_reading_ease.unwrap();
        assert!(fres > 70.0, "expected easy text FRES > 70, got {fres}");
    }
}
