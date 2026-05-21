// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! English prose pipeline (§§31–33).
//!
//! Works on a concatenated-across-blocks plain-text string, post-stripping.
//! Sub-modules partition responsibility:
//!   - [`sentences`]: UAX #29 + abbreviation-aware sentence segmentation.
//!   - [`syllables`]: vowel-group heuristic.
//!   - [`readability`]: FRES, FKGL, Fog, SMOG, ARI, CLI, Dale-Chall
//!     (NGSL-backed), FORCAST, LIX, RIX, ensemble band.
//!   - [`lexical`]: MATTR₅₀, hapax ratio, lexical density, moments.
//!   - [`wording`]: passive, hedges, weasels, wordy phrases, adverbs,
//!     nominalizations, expletives, lexical illusions, clichés, nonwords,
//!     long sentences, WQS.
//!   - [`inclusive`]: alex-style inclusive-language flags.

pub mod inclusive;
pub mod lexical;
pub mod readability;
pub mod sentences;
pub mod syllables;
pub mod wording;

use serde::Serialize;

use self::inclusive::InclusiveReport;
pub use self::lexical::EnglishLexical;
use self::readability::ReadabilityReport;
use self::wording::WordingReport;

#[derive(Debug, Clone, Serialize, Default)]
pub struct EnglishReport {
    pub readability: ReadabilityReport,
    pub lexical: EnglishLexical,
    pub wording: WordingReport,
    pub inclusive_language: InclusiveReport,
    pub short_doc_warning: bool,
}

/// Runs the full English pipeline against `text`.
pub fn analyze(text: &str) -> EnglishReport {
    // 1. Tokenize into sentences + words. Later stages reuse these.
    let sents = sentences::split(text);
    let words_per_sent: Vec<Vec<String>> = sents
        .iter()
        .map(|s| sentences::words_in_sentence(s))
        .collect();
    let words_flat: Vec<String> = words_per_sent.iter().flatten().cloned().collect();

    let sentences_count = sents.iter().filter(|s| !s.trim().is_empty()).count();
    let words_count = words_flat.len();

    // 2. Short-doc refusal per §37.5 / §29.1.
    let short_doc = words_count < 100 || sentences_count < 5;

    let lexical = lexical::analyze(&words_per_sent, &words_flat);

    let readability = if short_doc {
        // Emit zeros + explicit null-grades via a short-doc-only report.
        readability::short_doc_report(&lexical)
    } else {
        readability::analyze(&sents, &words_per_sent)
    };

    let wording = wording::analyze(&sents, &words_per_sent);
    let inclusive = inclusive::analyze(&words_flat, text);

    EnglishReport {
        readability,
        lexical,
        wording,
        inclusive_language: inclusive,
        short_doc_warning: short_doc,
    }
}
