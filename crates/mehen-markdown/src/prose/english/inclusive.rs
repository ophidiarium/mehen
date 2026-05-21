// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Inclusive-language flags (§33.12) — alex / retext-equality style.
//!
//! The bundled data file `inclusive_flags.txt` carries one entry per line:
//!   `<category>\t<surface>\t<preferred>`
//! where `surface` is matched case-insensitively against word-boundaries
//! in the prose text. Preferred is informational.

use std::sync::OnceLock;

use regex::Regex;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct InclusiveReport {
    pub flags: Vec<Flag>,
    pub inclusive_language_score: f64,
    /// Total distinct surfaces flagged.
    pub flag_count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Flag {
    pub category: String,
    pub surface: String,
    pub preferred: String,
    pub count: u64,
}

struct Entry {
    category: String,
    surface: String,
    preferred: String,
    // Pre-compiled regex with `\b` boundaries.
    re: Regex,
}

fn entries() -> &'static Vec<Entry> {
    static CELL: OnceLock<Vec<Entry>> = OnceLock::new();
    CELL.get_or_init(|| {
        let raw = include_str!("../../data/inclusive_flags.txt");
        let mut out = Vec::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() < 3 {
                continue;
            }
            let surface = parts[1].trim();
            if surface.is_empty() {
                continue;
            }
            let pattern = format!(r"(?i)\b{}\b", regex::escape(surface));
            if let Ok(re) = Regex::new(&pattern) {
                out.push(Entry {
                    category: parts[0].trim().to_string(),
                    surface: surface.to_string(),
                    preferred: parts[2].trim().to_string(),
                    re,
                });
            }
        }
        out
    })
}

pub fn analyze(_words: &[String], raw_text: &str) -> InclusiveReport {
    let mut flags: Vec<Flag> = Vec::new();
    let mut total = 0u64;
    for entry in entries() {
        let count = entry.re.find_iter(raw_text).count() as u64;
        if count > 0 {
            total += count;
            flags.push(Flag {
                category: entry.category.clone(),
                surface: entry.surface.clone(),
                preferred: entry.preferred.clone(),
                count,
            });
        }
    }

    // Score: start from 1.0, subtract 0.05 per hit, clamp to 0.
    let score = (1.0 - 0.05 * total as f64).clamp(0.0, 1.0);

    InclusiveReport {
        flags,
        inclusive_language_score: (score * 1000.0).round() / 1000.0,
        flag_count: total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_whitelist() {
        let text = "Please add IP to the whitelist.";
        let r = analyze(&[], text);
        assert!(r.flag_count >= 1);
        assert!(r.flags.iter().any(|f| f.surface == "whitelist"));
    }

    #[test]
    fn no_flags_clean_text() {
        let text = "Please add IP to the allowlist.";
        let r = analyze(&[], text);
        assert_eq!(r.flag_count, 0);
        assert!((r.inclusive_language_score - 1.0).abs() < 0.01);
    }
}
