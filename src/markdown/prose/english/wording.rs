//! English wording and style heuristics (§33).
//!
//! Produces per-document density metrics and the §33.11 Wording Quality
//! Score. Every sub-score is emitted alongside the composite so writers can
//! see which axis drove a drop.

use std::collections::HashSet;
use std::sync::OnceLock;

use regex::Regex;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub(crate) struct WordingReport {
    pub(crate) passive_ratio: f64,
    pub(crate) hedge_density: f64,
    pub(crate) weasel_density: f64,
    pub(crate) wordy_density: f64,
    pub(crate) adverb_density: f64,
    pub(crate) nominalization_density: f64,
    pub(crate) expletive_count: u64,
    pub(crate) lexical_illusions: u64,
    pub(crate) cliche_density: f64,
    pub(crate) nonword_count: u64,
    pub(crate) long_sentence_count: u64,
    pub(crate) wording_quality_score: f64,
}

pub(crate) fn analyze(sents: &[String], words_per_sent: &[Vec<String>]) -> WordingReport {
    let sent_count = sents.iter().filter(|s| !s.trim().is_empty()).count() as f64;
    let words_flat: Vec<String> = words_per_sent.iter().flatten().cloned().collect();
    let words_total = words_flat.len() as f64;
    if words_total == 0.0 || sent_count == 0.0 {
        return WordingReport::default();
    }

    let passive_ratio = passive_sentence_ratio(sents);
    let hedge_density = hedge_density(&words_flat);
    let weasel_density = weasel_density(&words_flat, sents);
    let wordy_density = wordy_density(sents, words_total);
    let adverb_density = adverb_density(&words_flat);
    let nominalization_density = nominalization_density(&words_flat);
    let expletive_count = expletive_count(sents);
    let lexical_illusions = lexical_illusions(sents);
    let cliche_density = cliche_density(sents, words_total);
    let nonword_count = nonword_count(&words_flat);
    let long_sentence_count = words_per_sent.iter().filter(|s| s.len() > 30).count() as u64;
    let long_rate = if sent_count > 0.0 {
        long_sentence_count as f64 / sent_count
    } else {
        0.0
    };

    // Wording Quality Score §33.11.
    let wqs = clamp01(
        1.0 - 0.18 * sat(passive_ratio, 0.25, 0.60)
            - 0.15 * sat(hedge_density, 0.02, 0.08)
            - 0.12 * sat(weasel_density, 0.01, 0.05)
            - 0.12 * sat(wordy_density, 0.01, 0.05)
            - 0.10 * sat(adverb_density, 0.02, 0.06)
            - 0.08 * sat(nominalization_density, 0.08, 0.20)
            - 0.08 * sat(long_rate, 0.05, 0.30)
            - 0.07 * sat(cliche_density, 0.002, 0.02)
            - 0.05 * bool01(lexical_illusions > 0)
            - 0.05 * bool01(nonword_count > 0),
    );

    WordingReport {
        passive_ratio: round3(passive_ratio),
        hedge_density: round3(hedge_density),
        weasel_density: round3(weasel_density),
        wordy_density: round3(wordy_density),
        adverb_density: round3(adverb_density),
        nominalization_density: round3(nominalization_density),
        expletive_count: expletive_count as u64,
        lexical_illusions: lexical_illusions as u64,
        cliche_density: round3(cliche_density),
        nonword_count: nonword_count as u64,
        long_sentence_count,
        wording_quality_score: round3(wqs),
    }
}

fn round3(x: f64) -> f64 {
    if !x.is_finite() {
        return 0.0;
    }
    (x * 1000.0).round() / 1000.0
}

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

/// Saturates at `lo..=hi`. Maps `x ≤ lo` to 0 and `x ≥ hi` to 1 linearly.
fn sat(x: f64, lo: f64, hi: f64) -> f64 {
    if hi <= lo {
        return 0.0;
    }
    ((x - lo) / (hi - lo)).clamp(0.0, 1.0)
}

fn bool01(b: bool) -> f64 {
    if b { 1.0 } else { 0.0 }
}

// ---------- §33.1 Passive voice -----------------------------------------

fn irregular_past_participles() -> &'static HashSet<String> {
    static CELL: OnceLock<HashSet<String>> = OnceLock::new();
    CELL.get_or_init(|| {
        let raw = include_str!("../../data/passive_irregulars.txt");
        raw.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.to_ascii_lowercase())
            .collect()
    })
}

fn passive_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| {
        // (?i) case-insensitive; \b...\b word boundaries.
        Regex::new(r"(?i)\b(am|is|are|was|were|be|been|being)\s+(\w+)\b").unwrap()
    })
}

/// Proportion of sentences with at least one passive match.
fn passive_sentence_ratio(sents: &[String]) -> f64 {
    let irregs = irregular_past_participles();
    let re = passive_regex();
    let mut passive = 0usize;
    let mut total = 0usize;
    for s in sents {
        if s.trim().is_empty() {
            continue;
        }
        total += 1;
        for cap in re.captures_iter(s) {
            let Some(verb) = cap.get(2) else {
                continue;
            };
            let v = verb.as_str().to_ascii_lowercase();
            if v.ends_with("ed") || irregs.contains(&v) {
                passive += 1;
                break;
            }
        }
    }
    if total == 0 {
        0.0
    } else {
        passive as f64 / total as f64
    }
}

// ---------- §33.2 Hedges ------------------------------------------------

fn hedge_set() -> &'static HashSet<String> {
    static CELL: OnceLock<HashSet<String>> = OnceLock::new();
    CELL.get_or_init(|| load_phrase_set(include_str!("../../data/hedges.txt")))
}

fn hedge_density(words: &[String]) -> f64 {
    phrase_density(words, hedge_set())
}

// ---------- §33.3 Weasels -----------------------------------------------

fn weasel_set() -> &'static HashSet<String> {
    static CELL: OnceLock<HashSet<String>> = OnceLock::new();
    CELL.get_or_init(|| load_phrase_set(include_str!("../../data/weasels.txt")))
}

fn weasel_density(words: &[String], sents: &[String]) -> f64 {
    // Quoted-literal suppression (§37.5 item 3): skip sentences that carry
    // an inline-code token. Backticks are stripped upstream in
    // `extract_prose_text`, but `InlineCode` spans leave behind
    // `INLINE_CODE_SENTINEL` — the sentinel survives sentence splitting
    // and is filtered out of word tokenization, so it costs nothing at
    // the metric level while still flagging the original technical
    // context. Sentences containing it typically describe a
    // backtick-wrapped identifier and shouldn't contribute to weasel
    // density.
    let suppressed: HashSet<usize> = sents
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            if s.contains(crate::markdown::prose::lang_detect::INLINE_CODE_SENTINEL) {
                Some(i)
            } else {
                None
            }
        })
        .collect();
    let set = weasel_set();
    let mut matches = 0usize;
    // Multi-word phrases are handled by joining neighboring tokens.
    for (i, s) in sents.iter().enumerate() {
        if suppressed.contains(&i) {
            continue;
        }
        let toks: Vec<String> = s
            .split_whitespace()
            .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|t| !t.is_empty())
            .map(|t| t.to_ascii_lowercase())
            .collect();
        for start in 0..toks.len() {
            for end in (start + 1)..=(start + 4).min(toks.len()) {
                let phrase = toks[start..end].join(" ");
                if set.contains(&phrase) {
                    matches += 1;
                }
            }
        }
    }
    if words.is_empty() {
        0.0
    } else {
        matches as f64 / words.len() as f64
    }
}

// ---------- §33.4 Wordy phrases -----------------------------------------

fn wordy_set() -> &'static HashSet<String> {
    static CELL: OnceLock<HashSet<String>> = OnceLock::new();
    CELL.get_or_init(|| load_phrase_set(include_str!("../../data/wordy_phrases.txt")))
}

fn wordy_density(sents: &[String], words_total: f64) -> f64 {
    let set = wordy_set();
    let mut matches = 0usize;
    for s in sents {
        let lower = s.to_ascii_lowercase();
        for phrase in set.iter() {
            if lower.contains(phrase) {
                matches += 1;
            }
        }
    }
    if words_total == 0.0 {
        0.0
    } else {
        matches as f64 / words_total
    }
}

// ---------- §33.5 Adverbs -----------------------------------------------

fn adverb_exceptions() -> &'static HashSet<String> {
    static CELL: OnceLock<HashSet<String>> = OnceLock::new();
    CELL.get_or_init(|| {
        let raw = "only reply apply supply family early likely lovely silly holy \
                   daily weekly monthly yearly lonely ugly belly hourly \
                   ally rely ply imply comply multiply reply fly try dry";
        raw.split_whitespace()
            .map(|s| s.to_ascii_lowercase())
            .collect()
    })
}

fn adverb_density(words: &[String]) -> f64 {
    if words.is_empty() {
        return 0.0;
    }
    let exc = adverb_exceptions();
    let count = words
        .iter()
        .map(|w| w.to_ascii_lowercase())
        .filter(|w| w.ends_with("ly") && w.chars().count() > 3)
        .filter(|w| !exc.contains(w))
        .count() as f64;
    count / words.len() as f64
}

// ---------- §33.6 Nominalizations ---------------------------------------

fn nominalization_density(words: &[String]) -> f64 {
    if words.is_empty() {
        return 0.0;
    }
    let suffixes = ["tion", "sion", "ment", "ence", "ance", "ity", "ness", "ism"];
    let count = words
        .iter()
        .map(|w| w.to_ascii_lowercase())
        .filter(|w| w.chars().count() > 5 && suffixes.iter().any(|s| w.ends_with(s)))
        .count() as f64;
    count / words.len() as f64
}

// ---------- §33.7 Expletive constructions -------------------------------

fn expletive_regex() -> &'static Regex {
    static CELL: OnceLock<Regex> = OnceLock::new();
    CELL.get_or_init(|| Regex::new(r"(?i)^\s*(there|it)\s+(is|are|was|were)\b").unwrap())
}

fn expletive_count(sents: &[String]) -> usize {
    let re = expletive_regex();
    sents.iter().filter(|s| re.is_match(s)).count()
}

// ---------- §33.8 Lexical illusions (doubled words) ---------------------

fn lexical_illusions(sents: &[String]) -> usize {
    let mut total = 0usize;
    for s in sents {
        let toks: Vec<String> = s
            .split_whitespace()
            .map(|t| {
                t.trim_matches(|c: char| !c.is_alphanumeric())
                    .to_ascii_lowercase()
            })
            .filter(|t| !t.is_empty() && t.chars().any(|c| c.is_alphabetic()))
            .collect();
        for i in 1..toks.len() {
            if toks[i] == toks[i - 1] {
                total += 1;
            }
        }
    }
    total
}

// ---------- §33.9 Clichés & non-words -----------------------------------

fn cliche_set() -> &'static HashSet<String> {
    static CELL: OnceLock<HashSet<String>> = OnceLock::new();
    CELL.get_or_init(|| load_phrase_set(include_str!("../../data/cliches.txt")))
}

fn cliche_density(sents: &[String], words_total: f64) -> f64 {
    let set = cliche_set();
    let mut matches = 0usize;
    for s in sents {
        let lower = s.to_ascii_lowercase();
        for phrase in set.iter() {
            if lower.contains(phrase) {
                matches += 1;
            }
        }
    }
    if words_total == 0.0 {
        0.0
    } else {
        matches as f64 / (words_total / 1000.0).max(1.0)
    }
}

fn nonword_set() -> &'static HashSet<String> {
    static CELL: OnceLock<HashSet<String>> = OnceLock::new();
    CELL.get_or_init(|| load_phrase_set(include_str!("../../data/nonwords.txt")))
}

fn nonword_count(words: &[String]) -> usize {
    let set = nonword_set();
    words
        .iter()
        .filter(|w| set.contains(&w.to_ascii_lowercase()))
        .count()
}

// ---------- Helpers -----------------------------------------------------

fn load_phrase_set(raw: &str) -> HashSet<String> {
    raw.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_ascii_lowercase())
        .collect()
}

fn phrase_density(words: &[String], set: &HashSet<String>) -> f64 {
    if words.is_empty() {
        return 0.0;
    }
    let mut matches = 0usize;
    for start in 0..words.len() {
        for end in (start + 1)..=(start + 4).min(words.len()) {
            let phrase = words[start..end]
                .iter()
                .map(|w| w.to_ascii_lowercase())
                .collect::<Vec<_>>()
                .join(" ");
            if set.contains(&phrase) {
                matches += 1;
            }
        }
    }
    matches as f64 / words.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passive_detects_simple_case() {
        let sents = vec![
            "The ball was thrown by the boy.".to_string(),
            "The cat sleeps on the mat.".to_string(),
        ];
        let r = passive_sentence_ratio(&sents);
        assert!((r - 0.5).abs() < 0.01);
    }

    #[test]
    fn expletive_detects_there_is() {
        let sents = vec!["There is no doubt.".to_string(), "The cat sat.".to_string()];
        assert_eq!(expletive_count(&sents), 1);
    }

    #[test]
    fn weasel_density_suppresses_sentinel_sentences() {
        // Codex P2 regression: a sentence like `` `foo` is very fast ``
        // used to bypass backtick-suppression because `InlineCode` spans
        // are stripped upstream of sentence splitting, leaving no
        // backtick in the sentence for `weasel_density` to detect. The
        // fix substitutes `InlineCode` spans with `INLINE_CODE_SENTINEL`
        // (U+FFFC), which survives sentence splitting and word
        // tokenization. `weasel_density` now suppresses any sentence that
        // carries the sentinel.
        //
        // Construct the post-strip sentence directly so this test
        // doesn't depend on the tree-sitter pipeline.
        let sentinel = crate::markdown::prose::lang_detect::INLINE_CODE_SENTINEL.to_string();
        // "very" is in the bundled weasel list — ensures the control
        // case below actually fires.
        let sent_sentinel = format!("{sentinel} is very fast");
        let words_sentinel: Vec<String> = sent_sentinel
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        let with_density = weasel_density(&words_sentinel, &[sent_sentinel]);
        assert_eq!(
            with_density, 0.0,
            "sentinel-carrying sentence must not contribute to weasel density, got {with_density}"
        );

        // Control: same weasel word in a sentence without the sentinel
        // still registers.
        let plain = "this is very fast".to_string();
        let words_plain: Vec<String> = plain.split_whitespace().map(|s| s.to_string()).collect();
        let plain_density = weasel_density(&words_plain, &[plain]);
        assert!(
            plain_density > 0.0,
            "sanity: weasel `very` must still register without sentinel, got {plain_density}"
        );
    }
}
