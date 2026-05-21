// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Jōyō grade proxy (§35.2).
//!
//! Bundled list: `data/jouyou_kanji.txt` maps each Jōyō kanji to a grade
//! 1..=7 (1–6 elementary Kyōiku, 7 secondary Jōyō). Every kanji not in the
//! list is treated as grade 8 (hyōgai / 表外).

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Serialize;
use unicode_script::{Script, UnicodeScript};

#[derive(Debug, Clone, Serialize)]
pub struct JouyouStats {
    pub grade_mean: f64,
    pub hyougai_ratio: f64,
    pub counted: u64,
    /// Number of kanji classified as Jōyō.
    pub jouyou_kanji: u64,
    /// Number of kanji outside the Jōyō list.
    pub hyougai_kanji: u64,
}

impl Default for JouyouStats {
    fn default() -> Self {
        Self {
            grade_mean: 0.0,
            hyougai_ratio: 0.0,
            counted: 0,
            jouyou_kanji: 0,
            hyougai_kanji: 0,
        }
    }
}

fn grade_table() -> &'static HashMap<char, u8> {
    static CELL: OnceLock<HashMap<char, u8>> = OnceLock::new();
    CELL.get_or_init(|| {
        let raw = include_str!("../../data/jouyou_kanji.txt");
        let mut map = HashMap::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut parts = line.split('\t');
            let kanji = parts.next().unwrap_or("").trim();
            let grade = parts.next().unwrap_or("").trim();
            if kanji.is_empty() || grade.is_empty() {
                continue;
            }
            let c = match kanji.chars().next() {
                Some(c) => c,
                None => continue,
            };
            let g: u8 = match grade.parse() {
                Ok(g) => g,
                Err(_) => continue,
            };
            map.insert(c, g);
        }
        map
    })
}

pub fn analyze(text: &str) -> JouyouStats {
    let table = grade_table();
    let mut total_kanji: u64 = 0;
    let mut in_jouyou: u64 = 0;
    let mut hyougai: u64 = 0;
    let mut grade_sum: u64 = 0;

    for c in text.chars() {
        if !matches!(c.script(), Script::Han) {
            continue;
        }
        total_kanji += 1;
        if let Some(&g) = table.get(&c) {
            in_jouyou += 1;
            grade_sum += g as u64;
        } else {
            hyougai += 1;
            // Grade 8 contributes to the mean — high-weight penalty.
            grade_sum += 8;
        }
    }

    let grade_mean = if total_kanji == 0 {
        0.0
    } else {
        grade_sum as f64 / total_kanji as f64
    };
    let hyougai_ratio = if total_kanji == 0 {
        0.0
    } else {
        hyougai as f64 / total_kanji as f64
    };

    JouyouStats {
        grade_mean: (grade_mean * 1000.0).round() / 1000.0,
        hyougai_ratio: (hyougai_ratio * 1000.0).round() / 1000.0,
        counted: total_kanji,
        jouyou_kanji: in_jouyou,
        hyougai_kanji: hyougai,
    }
}
