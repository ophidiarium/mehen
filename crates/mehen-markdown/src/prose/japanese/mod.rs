// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Japanese prose pipeline (§§34–36).
//!
//! Tier-0 only. Works on concatenated block text. Sub-modules:
//!   - [`scripts`]: Unicode script classification, ratios, script-run stats.
//!   - [`sentences`]: bracket-aware `。！？` segmentation.
//!   - [`tateishi`]: Tateishi simplified RS (§35.1).
//!   - [`jouyou`]: Jōyō grade + hyōgai ratio (§35.2).
//!   - [`wording`]: politeness, comma/period ratio, jukugo density,
//!     long-kanji runs, weak-phrase / redundant / doubled-joshi heuristics.
//!   - [`jtf`]: JTF rules 1, 3, 5, 7, 8, 11.

pub mod jouyou;
pub mod jtf;
pub mod scripts;
pub mod sentences;
pub mod tateishi;
pub mod wording;

use serde::Serialize;

pub use self::jtf::JtfReport;
pub use self::scripts::ScriptComposition;
pub use self::wording::{JapaneseLexical, JapaneseWording};

#[derive(Debug, Clone, Serialize, Default)]
pub struct JapaneseReport {
    pub script_composition: ScriptComposition,
    pub readability: JapaneseReadability,
    pub lexical: JapaneseLexical,
    pub wording: JapaneseWording,
    pub style_conformance: JtfReport,
    pub short_doc_warning: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct JapaneseReadability {
    pub tateishi_rs: Option<f64>,
    pub jouyou_grade_mean: Option<f64>,
    pub hyougai_ratio: f64,
}

pub fn analyze(text: &str) -> JapaneseReport {
    // 1. Script composition + script runs — inputs for nearly everything.
    let (script_composition, runs) = scripts::analyze(text);

    // 2. Sentences (bracket-aware).
    let sents = sentences::split(text);
    let sent_count = sents.len();

    // Short-doc guard (§35.1): refuse readability when < 300 visible chars
    // or when hiragana_ratio > 0.90.
    let short = script_composition.visible_chars < 300
        || script_composition.hiragana_ratio > 0.90
        || sent_count < 5;

    // 3. Tateishi simplified RS (§35.1).
    let tateishi_rs = if short {
        None
    } else {
        Some(tateishi::tateishi_simplified_rs(
            &runs,
            &sents,
            &script_composition,
        ))
    };

    // 4. Jōyō grade stats (§35.2).
    let jouyou = jouyou::analyze(text);
    let jouyou_grade_mean = if jouyou.counted == 0 {
        None
    } else {
        Some(jouyou.grade_mean)
    };
    let hyougai_ratio = jouyou.hyougai_ratio;

    // 5. Lexical (comma/period ratio, avg sent chars, p90, jukugo).
    let lexical = wording::lexical(&sents, &script_composition, &runs);

    // 6. JTF mechanical rules — computed first so wording's WQS can consume
    //    the resulting violation density per §36.7.
    let style_conformance = jtf::analyze(text, &sents, &script_composition, &jouyou);

    // 7. Wording / politeness / weak phrases. Threads hyougai_ratio and
    //    the JTF violation density through so the composite Wording
    //    Quality Score (§36.7) covers those axes directly.
    let wording = wording::wording(
        text,
        &sents,
        &script_composition,
        &runs,
        &lexical,
        jouyou.hyougai_ratio,
        style_conformance.violation_density_per_1000,
    );

    JapaneseReport {
        script_composition,
        readability: JapaneseReadability {
            tateishi_rs,
            jouyou_grade_mean,
            hyougai_ratio,
        },
        lexical,
        wording,
        style_conformance,
        short_doc_warning: short,
    }
}
