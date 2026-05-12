//! Japanese wording heuristics (§§36.1–36.7).
//!
//! Tier-0: all checks run off static lists and regex-free substring matches
//! so the default binary needs no morphological analyzer. The Wording
//! Quality Score follows §36.7 verbatim.

use std::sync::OnceLock;

use serde::Serialize;

use super::scripts::{Class, Run, ScriptComposition};

#[derive(Debug, Clone, Serialize, Default)]
pub(crate) struct JapaneseLexical {
    pub(crate) avg_sentence_chars: f64,
    pub(crate) p90_sentence_chars: u64,
    pub(crate) max_sentence_chars: u64,
    pub(crate) comma_period_ratio: f64,
    pub(crate) jukugo_density: f64,
    pub(crate) sentence_count: u64,
    pub(crate) char_count: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub(crate) struct JapaneseWording {
    pub(crate) politeness_dominant: String,
    pub(crate) keitai_count: u64,
    pub(crate) jotai_count: u64,
    pub(crate) honorific_count: u64,
    pub(crate) keitai_jotai_mix_count: u64,
    pub(crate) weak_phrase_count: u64,
    pub(crate) redundant_expression_count: u64,
    pub(crate) doubled_joshi_count: u64,
    pub(crate) long_kanji_run_count: u64,
    pub(crate) max_comma_violation_count: u64,
    pub(crate) max_ten_violation_count: u64,
    pub(crate) long_sentence_count: u64,
    pub(crate) wording_quality_score: f64,
}

pub(crate) fn lexical(
    sents: &[String],
    _composition: &ScriptComposition,
    runs: &[Run],
) -> JapaneseLexical {
    let sent_lens: Vec<u64> = sents
        .iter()
        .map(|s| {
            s.chars()
                .filter(|c| {
                    !c.is_whitespace() && !matches!(*c, '。' | '！' | '？' | '!' | '?' | '.')
                })
                .count() as u64
        })
        .collect();

    let char_count: u64 = sent_lens.iter().sum();
    let avg_sent = if !sent_lens.is_empty() {
        sent_lens.iter().sum::<u64>() as f64 / sent_lens.len() as f64
    } else {
        0.0
    };
    let max_sent = sent_lens.iter().copied().max().unwrap_or(0);
    let p90_sent = percentile_u64(&sent_lens, 90);

    // comma/period ratio.
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
    let cpr = if period == 0 {
        0.0
    } else {
        comma as f64 / period as f64
    };

    // Jukugo density: kanji runs with length >= 2 divided by total kanji
    // runs (§36.2).
    let mut total_kanji_runs = 0u64;
    let mut jukugo_runs = 0u64;
    for r in runs {
        if r.class == Class::Kanji {
            total_kanji_runs += 1;
            if r.length >= 2 {
                jukugo_runs += 1;
            }
        }
    }
    let jukugo_density = if total_kanji_runs == 0 {
        0.0
    } else {
        jukugo_runs as f64 / total_kanji_runs as f64
    };

    JapaneseLexical {
        avg_sentence_chars: round3(avg_sent),
        p90_sentence_chars: p90_sent,
        max_sentence_chars: max_sent,
        comma_period_ratio: round3(cpr),
        jukugo_density: round3(jukugo_density),
        sentence_count: sents.len() as u64,
        char_count,
    }
}

pub(crate) fn wording(
    text: &str,
    sents: &[String],
    _composition: &ScriptComposition,
    runs: &[Run],
    lexical: &JapaneseLexical,
    hyougai_ratio: f64,
    jtf_violation_density_per_1000: f64,
) -> JapaneseWording {
    // Politeness classification.
    let (keitai, jotai, honorific) = classify_politeness(sents);
    let total_sents = (keitai + jotai + honorific) as f64;
    let politeness_dominant = if total_sents == 0.0 {
        "none".to_string()
    } else {
        // Honorific + keitai are both polite styles; aggregate as keitai.
        let polite = keitai + honorific;
        let plain = jotai;
        if polite > plain {
            "keitai".to_string()
        } else if plain > polite {
            "jotai".to_string()
        } else {
            "mixed".to_string()
        }
    };
    let keitai_jotai_mix_count = if politeness_dominant == "keitai" {
        jotai
    } else if politeness_dominant == "jotai" {
        keitai + honorific
    } else {
        // Mixed: count the smaller group.
        (keitai + honorific).min(jotai)
    };

    // Weak / redundant.
    let weak_phrase_count = count_phrase_occurrences(text, weak_phrases());
    let redundant_expression_count = count_phrase_occurrences(text, redundant_expressions());

    // Doubled joshi (simple pattern): any of `を・は・が・に` appearing twice
    // within the same sentence with at least 1 char separation.
    let doubled_joshi_count = count_doubled_joshi(sents);

    // Long kanji runs: run length >= 7 (§36.6 max-kanji-continuous-len ≤ 6).
    let long_kanji_run_count = runs
        .iter()
        .filter(|r| r.class == Class::Kanji && r.length >= 7)
        .count() as u64;

    // max-comma (,): > 3 per sentence violates (halfwidth and fullwidth).
    let max_comma_violation_count = sents
        .iter()
        .filter(|s| s.chars().filter(|&c| c == ',' || c == '，').count() > 3)
        .count() as u64;
    // max-ten (、): > 3 per sentence violates.
    let max_ten_violation_count = sents
        .iter()
        .filter(|s| s.chars().filter(|&c| c == '、').count() > 3)
        .count() as u64;
    // sentence-length: > 100 visible chars per sentence violates.
    let long_sentence_count = sents
        .iter()
        .filter(|s| {
            s.chars()
                .filter(|c| {
                    !c.is_whitespace() && !matches!(*c, '。' | '！' | '？' | '!' | '?' | '.')
                })
                .count()
                > 100
        })
        .count() as u64;

    let sent_n = lexical.sentence_count.max(1) as f64;
    let weak_rate = weak_phrase_count as f64 / sent_n;
    let redundant_rate = redundant_expression_count as f64 / sent_n;
    let long_rate = long_sentence_count as f64 / sent_n;
    let long_kanji_rate = long_kanji_run_count as f64 / sent_n;
    let max_comma_rate = max_comma_violation_count as f64 / sent_n;
    let mix_ratio = if total_sents > 0.0 {
        keitai_jotai_mix_count as f64 / total_sents
    } else {
        0.0
    };

    // Japanese Wording Quality Score §36.7.
    //
    // The §36.7 formula has explicit `hyougai_ratio` and
    // `jtf_violation_density` terms; earlier revisions reused
    // `long_kanji_rate` as a placeholder for both, which let hyōgai-heavy
    // or JTF-violating documents keep a clean WQS. The jouyou + JTF signals
    // are now threaded in directly so the score responds to those axes.
    let wqs = clamp01(
        1.0 - 0.15 * sat(long_rate, 0.05, 0.30)
            - 0.12 * sat(weak_rate, 0.01, 0.05)
            - 0.12 * sat(redundant_rate, 0.01, 0.05)
            - 0.10 * sat(doubled_joshi_count as f64 / sent_n, 0.02, 0.10)
            - 0.10 * sat(long_kanji_rate, 0.05, 0.25)
            - 0.10
                * if keitai_jotai_mix_count > 0 {
                    sat(mix_ratio, 0.02, 0.20)
                } else {
                    0.0
                }
            - 0.08 * sat(max_comma_rate, 0.02, 0.15)
            - 0.08 * sat(hyougai_ratio, 0.05, 0.25)
            - 0.07 * sat(jtf_violation_density_per_1000, 0.5, 5.0),
    );

    JapaneseWording {
        politeness_dominant,
        keitai_count: keitai,
        jotai_count: jotai,
        honorific_count: honorific,
        keitai_jotai_mix_count,
        weak_phrase_count,
        redundant_expression_count,
        doubled_joshi_count,
        long_kanji_run_count,
        max_comma_violation_count,
        max_ten_violation_count,
        long_sentence_count,
        wording_quality_score: round3(wqs),
    }
}

fn classify_politeness(sents: &[String]) -> (u64, u64, u64) {
    let mut keitai = 0u64;
    let mut jotai = 0u64;
    let mut honorific = 0u64;

    let honor_suffixes = [
        "いらっしゃる",
        "いらっしゃいます",
        "召し上がる",
        "おります",
        "ございます",
        "ございました",
    ];
    let keitai_suffixes = [
        "です",
        "ます",
        "でした",
        "ました",
        "ません",
        "でしょう",
        "ましょう",
        "ですか",
        "ますか",
    ];
    let jotai_suffixes = ["だ", "である", "だった", "であった", "なのだ"];

    for s in sents {
        // Work on the sentence after trimming trailing punctuation.
        let trimmed: String = s
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .trim_end_matches(['。', '！', '？', '!', '?', '.'])
            .to_string();
        if trimmed.is_empty() {
            continue;
        }

        let mut classified = false;
        for suf in honor_suffixes {
            if trimmed.ends_with(suf) {
                honorific += 1;
                classified = true;
                break;
            }
        }
        if classified {
            continue;
        }
        for suf in keitai_suffixes {
            if trimmed.ends_with(suf) {
                keitai += 1;
                classified = true;
                break;
            }
        }
        if classified {
            continue;
        }
        for suf in jotai_suffixes {
            if trimmed.ends_with(suf) {
                jotai += 1;
                break;
            }
        }
    }

    (keitai, jotai, honorific)
}

fn weak_phrases() -> &'static Vec<String> {
    static CELL: OnceLock<Vec<String>> = OnceLock::new();
    CELL.get_or_init(|| {
        include_str!("../../data/ja_weak_phrases.txt")
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(String::from)
            .collect()
    })
}

fn redundant_expressions() -> &'static Vec<String> {
    static CELL: OnceLock<Vec<String>> = OnceLock::new();
    CELL.get_or_init(|| {
        include_str!("../../data/ja_redundant.txt")
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(String::from)
            .collect()
    })
}

fn count_phrase_occurrences(text: &str, phrases: &[String]) -> u64 {
    let mut total = 0u64;
    for p in phrases {
        // Walk the text, counting non-overlapping matches.
        let mut haystack = text;
        while let Some(idx) = haystack.find(p.as_str()) {
            total += 1;
            haystack = &haystack[idx + p.len()..];
        }
    }
    total
}

fn count_doubled_joshi(sents: &[String]) -> u64 {
    let mut total = 0u64;
    let joshi = ['を', 'は', 'が', 'に', 'で', 'と', 'も'];
    for s in sents {
        for j in joshi {
            let occurrences: Vec<usize> = s
                .chars()
                .enumerate()
                .filter_map(|(i, c)| if c == j { Some(i) } else { None })
                .collect();
            // Count pairs separated by at least 1 char (min_interval=1).
            for w in occurrences.windows(2) {
                if w[1] - w[0] > 1 {
                    total += 1;
                }
            }
        }
    }
    total
}

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

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

fn sat(x: f64, lo: f64, hi: f64) -> f64 {
    if hi <= lo {
        return 0.0;
    }
    ((x - lo) / (hi - lo)).clamp(0.0, 1.0)
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
    fn politeness_keitai() {
        let s = vec!["これはテストです。".to_string(), "動作します。".to_string()];
        let (k, j, _) = classify_politeness(&s);
        assert_eq!(k, 2);
        assert_eq!(j, 0);
    }

    #[test]
    fn politeness_jotai() {
        let s = vec!["これはテストだ。".to_string()];
        let (_, j, _) = classify_politeness(&s);
        assert_eq!(j, 1);
    }

    #[test]
    fn weak_phrase_detected() {
        let text = "このバージョンは動くかもしれない。";
        assert!(count_phrase_occurrences(text, weak_phrases()) > 0);
    }

    #[test]
    fn wqs_responds_to_hyougai_and_jtf_signals() {
        // Codex P1 regression: §36.7 has explicit hyougai_ratio and
        // jtf_violation_density terms. Earlier revisions reused
        // long_kanji_rate as a stand-in, which left the composite WQS
        // blind to both axes. After the fix, the same document scored
        // with clean jouyou/JTF signals must score HIGHER than one with
        // hyougai_ratio=0.30 and jtf_violation_density=3.0.
        use super::super::scripts::ScriptComposition;
        let text = "これはテストです。動作します。";
        let sents = vec!["これはテストです。".to_string(), "動作します。".to_string()];
        let composition = ScriptComposition::default();
        let runs: Vec<Run> = Vec::new();
        let lexical = JapaneseLexical {
            avg_sentence_chars: 10.0,
            p90_sentence_chars: 10,
            max_sentence_chars: 10,
            comma_period_ratio: 0.0,
            jukugo_density: 0.0,
            sentence_count: 2,
            char_count: 20,
        };

        let clean = wording(text, &sents, &composition, &runs, &lexical, 0.0, 0.0);
        let dirty = wording(text, &sents, &composition, &runs, &lexical, 0.30, 3.0);

        assert!(
            dirty.wording_quality_score < clean.wording_quality_score,
            "WQS must drop when hyougai/JTF signals are present: clean={}, dirty={}",
            clean.wording_quality_score,
            dirty.wording_quality_score
        );
    }
}
