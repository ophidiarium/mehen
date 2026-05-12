//! Language-aware prose metric layer (§§29–38).
//!
//! This module adds a *separate* top-level `prose` section to the Markdown
//! output schema. It never modifies the structural scores computed by Phase A
//! (LOC, words, sections, ECU) or later phases (MCC, DMI, FillerLazyRisk).
//!
//! Entry point: [`analyze_prose`] takes the parsed tree + the source buffer
//! and produces a [`ProseReport`] that the analyzer attaches to
//! [`crate::markdown::types::MarkdownMetrics`].
//!
//! Tier 0 scope (§38):
//!   - Unicode-script block-ratio language detection (no trigram model).
//!   - UAX #29 word + sentence segmentation (English), with abbreviation list.
//!   - Vowel-group English syllables (no CMU, no hyphenation).
//!   - Classical English readability: FRES, FKGL, Fog, SMOG, ARI, CLI,
//!     Dale-Chall (NGSL-backed), FORCAST, LIX, RIX.
//!   - English wording: passive, hedges, weasels, wordy phrases, adverbs,
//!     nominalizations, expletives, lexical illusions, clichés, nonwords,
//!     long sentences, WQS.
//!   - Inclusive-language flags.
//!   - Japanese script composition, Tateishi simplified RS, Jōyō grade,
//!     jukugo density, politeness, JTF mechanical rules, textlint subset.
//!
//! Tier 1/2 features (CMU syllables, Lindera, Vibrato, JLPT, Lingua) are
//! feature-gated and OFF by default. See `Cargo.toml`.

pub(crate) mod english;
pub(crate) mod japanese;
pub(crate) mod lang_detect;

use serde::Serialize;

use crate::node::Node;

use self::english::EnglishReport;
use self::japanese::JapaneseReport;
use self::lang_detect::{DetectedBlock, Language};

/// Whole-document prose output (§29.2).
#[derive(Debug, Clone, Serialize, Default)]
pub(crate) struct ProseReport {
    pub(crate) language_detection: LanguageDetection,
    pub(crate) english: Option<EnglishReport>,
    pub(crate) japanese: Option<JapaneseReport>,
    pub(crate) meta: ProseMeta,
}

#[derive(Debug, Clone, Serialize, Default)]
pub(crate) struct LanguageDetection {
    pub(crate) dominant_language: String,
    pub(crate) blocks: Vec<BlockLangReport>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BlockLangReport {
    pub(crate) start_line: u64,
    pub(crate) end_line: u64,
    pub(crate) language: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub(crate) struct ProseMeta {
    pub(crate) short_doc_warning: bool,
    pub(crate) words_counted: u64,
    pub(crate) sentences_counted: u64,
    /// The kinds of blocks stripped from prose input.
    pub(crate) blocks_stripped: Vec<String>,
}

/// Analyzes prose layers over the Markdown parse tree. `source` is the raw
/// file bytes so we can slice text without re-walking the tree repeatedly.
pub(crate) fn analyze_prose(root: &Node<'_>, source: &[u8]) -> ProseReport {
    // 1. Enumerate prose-eligible blocks with per-block text spans.
    let blocks = lang_detect::collect_prose_blocks(root, source);

    // 2. Per-block language tagging.
    let detected: Vec<DetectedBlock> = blocks
        .iter()
        .map(|b| lang_detect::classify_block(b))
        .collect();

    // Inherit parent-heading language for short blocks that classify as
    // Other. This is a second pass over `detected` because a block's
    // inheritance depends on the preceding heading context.
    let detected = lang_detect::propagate_heading_inheritance(detected);

    // 3. Document-level dominant language: majority among {en, ja}, falling
    //    back to `mixed` when both appear, `other` otherwise.
    let dominant = lang_detect::dominant_language(&detected);

    // 4. Run per-language pipelines on the concatenated text of their blocks.
    let en_text = lang_detect::concat_lang_text(&detected, Language::En);
    let ja_text = lang_detect::concat_lang_text(&detected, Language::Ja);

    let english = if !en_text.trim().is_empty() {
        Some(english::analyze(&en_text))
    } else {
        None
    };
    let japanese = if !ja_text.trim().is_empty() {
        Some(japanese::analyze(&ja_text))
    } else {
        None
    };

    // 5. Meta: word / sentence totals, short-doc warning, blocks-stripped.
    let (words_counted, sentences_counted) = match (english.as_ref(), japanese.as_ref()) {
        (Some(en), Some(ja)) => (
            en.lexical.words_total + ja.lexical.char_count,
            en.lexical.sentence_count + ja.lexical.sentence_count,
        ),
        (Some(en), None) => (en.lexical.words_total, en.lexical.sentence_count),
        (None, Some(ja)) => (ja.lexical.char_count, ja.lexical.sentence_count),
        (None, None) => (0, 0),
    };

    let short_doc_warning = match (english.as_ref(), japanese.as_ref()) {
        (Some(en), _) => en.short_doc_warning,
        (None, Some(ja)) => ja.short_doc_warning,
        (None, None) => true,
    };

    let blocks_stripped = blocks_stripped_kinds(root);

    let blocks_out: Vec<BlockLangReport> = detected
        .iter()
        .filter_map(|b| {
            if matches!(b.language, Language::None) {
                None
            } else {
                Some(BlockLangReport {
                    start_line: b.start_line,
                    end_line: b.end_line,
                    language: b.language.as_str().to_string(),
                })
            }
        })
        .collect();

    ProseReport {
        language_detection: LanguageDetection {
            dominant_language: dominant.as_str().to_string(),
            blocks: blocks_out,
        },
        english,
        japanese,
        meta: ProseMeta {
            short_doc_warning,
            words_counted,
            sentences_counted,
            blocks_stripped,
        },
    }
}

/// Enumerates the kinds of blocks that were excluded from prose analysis.
/// Deterministic: always emitted in a fixed order so snapshots are stable.
fn blocks_stripped_kinds(root: &Node<'_>) -> Vec<String> {
    use crate::languages::Markdown;

    let mut has_code = false;
    let mut has_frontmatter = false;
    let mut has_html = false;
    let mut has_math = false;
    let mut has_table = false;
    let mut has_mdx = false;

    fn walk(
        node: &Node<'_>,
        code: &mut bool,
        fm: &mut bool,
        html: &mut bool,
        math: &mut bool,
        table: &mut bool,
        mdx: &mut bool,
    ) {
        let kind: Markdown = node.kind_id().into();
        match kind {
            Markdown::FencedCodeBlock | Markdown::IndentedCodeBlock | Markdown::InlineCode => {
                *code = true;
            }
            Markdown::MinusMetadata | Markdown::PlusMetadata => {
                *fm = true;
            }
            Markdown::HtmlBlock
            | Markdown::HtmlBlock1
            | Markdown::HtmlBlock3
            | Markdown::HtmlBlock4
            | Markdown::HtmlBlock5
            | Markdown::HtmlBlock6
            | Markdown::HtmlBlock7
            | Markdown::HtmlCommentBlock
            | Markdown::HtmlInline => {
                *html = true;
            }
            Markdown::MdxJsxBlock | Markdown::MdxJsxInline => {
                *mdx = true;
            }
            Markdown::MathBlock | Markdown::MathInline => {
                *math = true;
            }
            Markdown::PipeTable => {
                *table = true;
            }
            _ => {}
        }
        let mut cursor = node.cursor();
        if cursor.goto_first_child() {
            loop {
                walk(&cursor.node(), code, fm, html, math, table, mdx);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    walk(
        root,
        &mut has_code,
        &mut has_frontmatter,
        &mut has_html,
        &mut has_math,
        &mut has_table,
        &mut has_mdx,
    );

    let mut out = Vec::new();
    if has_code {
        out.push("code".to_string());
    }
    if has_frontmatter {
        out.push("frontmatter".to_string());
    }
    if has_html {
        out.push("html".to_string());
    }
    if has_mdx {
        out.push("mdx".to_string());
    }
    if has_math {
        out.push("math".to_string());
    }
    if has_table {
        out.push("table".to_string());
    }
    out
}
