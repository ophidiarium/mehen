//! JTF (Japan Translation Federation) Japanese Style Guide conformance
//! (§36.5). Tier-0 rules 1, 3, 5, 7, 8, 11 are mechanically checkable.
//!
//! Output is a list of `{rule, severity, count}` entries plus the density
//! per 1,000 characters used by the Japanese WQS §36.7.

use serde::Serialize;

use super::jouyou::JouyouStats;
use super::scripts::ScriptComposition;

#[derive(Debug, Clone, Serialize, Default)]
pub struct JtfReport {
    pub violations: Vec<JtfViolation>,
    pub total_violations: u64,
    pub violation_density_per_1000: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct JtfViolation {
    pub rule: String,
    pub severity: String,
    pub count: u64,
}

pub fn analyze(
    text: &str,
    sents: &[String],
    composition: &ScriptComposition,
    jouyou: &JouyouStats,
) -> JtfReport {
    let mut violations: Vec<JtfViolation> = Vec::new();

    // Rule 1: keitai/jōtai consistency (warn). Flagged by wording module; we
    // count mix cases here.
    let mix = keitai_jotai_mix_count(sents);
    if mix > 0 {
        violations.push(JtfViolation {
            rule: "rule-1-keitai-jotai-consistency".to_string(),
            severity: "warn".to_string(),
            count: mix,
        });
    }

    // Rule 3: stick to Jōyō kanji — hyōgai count (warn).
    if jouyou.hyougai_kanji > 0 {
        violations.push(JtfViolation {
            rule: "rule-3-jouyou-only".to_string(),
            severity: "warn".to_string(),
            count: jouyou.hyougai_kanji,
        });
    }

    // Rule 5: trailing long-vowel on katakana compound endings — warn.
    // Flag katakana compounds ending in certain chars where the long-vowel
    // mark `ー` should have been kept (`コンピュータ` vs `コンピューター`).
    let rule5 = count_missing_chouonpu(text);
    if rule5 > 0 {
        violations.push(JtfViolation {
            rule: "rule-5-trailing-chouonpu".to_string(),
            severity: "warn".to_string(),
            count: rule5,
        });
    }

    // Rule 7: kanji/hiragana/katakana must be full-width (error).
    // Detect halfwidth kana U+FF66–U+FF9F.
    let rule7 = text
        .chars()
        .filter(|c| {
            let u = *c as u32;
            (0xFF66..=0xFF9F).contains(&u)
        })
        .count() as u64;
    if rule7 > 0 {
        violations.push(JtfViolation {
            rule: "rule-7-fullwidth-kana".to_string(),
            severity: "error".to_string(),
            count: rule7,
        });
    }

    // Rule 8: digits and Latin alphabet must be halfwidth (warn).
    // Fullwidth digit / Latin inside Japanese text is a violation.
    let mut rule8 = 0u64;
    for c in text.chars() {
        let u = c as u32;
        if (0xFF10..=0xFF19).contains(&u)
            || (0xFF21..=0xFF3A).contains(&u)
            || (0xFF41..=0xFF5A).contains(&u)
        {
            rule8 += 1;
        }
    }
    if rule8 > 0 {
        violations.push(JtfViolation {
            rule: "rule-8-halfwidth-digits-latin".to_string(),
            severity: "warn".to_string(),
            count: rule8,
        });
    }

    // Rule 11: `.` `,` `<space>` should be halfwidth (info). Detect
    // fullwidth period `．`, fullwidth comma `，`, fullwidth space `　`.
    let rule11 = text
        .chars()
        .filter(|&c| matches!(c, '．' | '，' | '\u{3000}'))
        .count() as u64;
    if rule11 > 0 {
        violations.push(JtfViolation {
            rule: "rule-11-halfwidth-punct".to_string(),
            severity: "info".to_string(),
            count: rule11,
        });
    }

    let total: u64 = violations.iter().map(|v| v.count).sum();
    let density = if composition.visible_chars == 0 {
        0.0
    } else {
        (total as f64 * 1000.0) / composition.visible_chars as f64
    };

    JtfReport {
        violations,
        total_violations: total,
        violation_density_per_1000: (density * 1000.0).round() / 1000.0,
    }
}

fn keitai_jotai_mix_count(sents: &[String]) -> u64 {
    let mut keitai = 0u64;
    let mut jotai = 0u64;
    for s in sents {
        let t: String = s
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .trim_end_matches(['。', '！', '？', '!', '?', '.'])
            .to_string();
        if t.is_empty() {
            continue;
        }
        if t.ends_with("です")
            || t.ends_with("ます")
            || t.ends_with("でした")
            || t.ends_with("ました")
            || t.ends_with("ません")
            || t.ends_with("ですか")
            || t.ends_with("ますか")
        {
            keitai += 1;
        } else if t.ends_with("だ") || t.ends_with("である") || t.ends_with("だった") {
            jotai += 1;
        }
    }
    if keitai > 0 && jotai > 0 {
        keitai.min(jotai)
    } else {
        0
    }
}

/// Counts katakana compounds ending on specific characters where JTF
/// rule 5 prefers a trailing `ー`. Heuristic: a katakana run of length
/// ≥ 3 ending in one of the stem-ending vowels without a trailing `ー`.
///
/// This is intentionally conservative — false positives are preferred to
/// false negatives because the output is advisory.
fn count_missing_chouonpu(text: &str) -> u64 {
    let mut count = 0u64;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let katakana_range = |c: char| {
        let u = c as u32;
        (0x30A0..=0x30FF).contains(&u)
    };
    while i < chars.len() {
        if katakana_range(chars[i]) {
            let mut end = i;
            while end < chars.len() && katakana_range(chars[end]) {
                end += 1;
            }
            let len = end - i;
            if len >= 3 {
                // JTF rule 5: the run's final character must be one of the
                // stem-ending vowels AND must not already be a `ー`. We do
                // NOT skip runs that contain an internal `ー` — e.g.
                // `コンピュータ` (internal `ー`, missing trailing `ー`) is
                // still a rule-5 violation. The only exception is when the
                // final character is itself `ー`, which means the chōonpu
                // is already present.
                let last = chars[end - 1];
                if last != 'ー' {
                    let ends = ['タ', 'ラ', 'リ', 'ル', 'レ', 'ロ', 'サ', 'ザ', 'ダ', 'バ'];
                    if ends.contains(&last) {
                        count += 1;
                    }
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule5_flags_internal_chouonpu_missing_trailing() {
        // Codex P2 regression: `コンピュータ` contains an internal `ー` but
        // is missing the trailing `ー` (JTF rule 5 prefers `コンピューター`).
        // The previous `has_internal` gate skipped runs that contained any
        // `ー`, so this canonical violation was never counted. After the fix
        // the rule only looks at the final character — present `ー` ⇒ OK,
        // missing trailing stem-vowel ⇒ violation.
        assert_eq!(
            count_missing_chouonpu("コンピュータ"),
            1,
            "コンピュータ has trailing タ without closing ー: must be a rule-5 violation"
        );

        // Negative control: `コンピューター` already has the trailing `ー`,
        // so it must NOT be flagged.
        assert_eq!(
            count_missing_chouonpu("コンピューター"),
            0,
            "コンピューター already ends in ー: must not fire"
        );
    }

    #[test]
    fn rule5_ignores_runs_shorter_than_three() {
        // Short runs (< 3 katakana chars) are outside the heuristic band —
        // they're too ambiguous to flag safely.
        assert_eq!(count_missing_chouonpu("タラ"), 0);
    }
}
