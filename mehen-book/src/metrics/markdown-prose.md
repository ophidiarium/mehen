# Markdown Prose Metrics — Language-Aware Layer

The structural Markdown layer ([Markdown Metrics](./markdown.md)) is
deliberately language-opaque. The **prose layer** (§§29–38) adds
language-aware signals — readability formulas, lexical diversity, wording
quality, Japanese script composition and JTF conformance — on top of the
same AST.

Architectural constraints from §29.1 are strict:

1. **Layered, not folded.** Prose metrics are a separate top-level section
   in the output schema. They do not modify DMI, MCC, MRPC, or FillerLazyRisk
   weights silently.
2. **Per-block language tag.** Language detection runs per Markdown block
   (paragraph, heading, list item, blockquote), not per document.
3. **Structural artifacts stay excluded.** Code fences, inline code, link
   destinations, image alt-text, YAML/TOML/JSON front matter, HTML/MDX, and
   table delimiters are stripped before any readability or wording
   calculation.
4. **Short-text refusal.** Grade-level formulas are suppressed when
   `words < 100` OR `sentences < 5`. The tool reports raw counts and a
   `short_doc_warning` instead of a meaningless grade.
5. **Feature-gated dictionaries.** Dictionary-dependent features ship
   behind Cargo `--features` flags so the default binary stays small.
6. **Deterministic and reproducible.** No network, no cloud, no sampling.

As with the structural layer, every formula reference cites
`(§N.N)` in `docs/mehen_markdown_metrics_research_foundation.md`.

## Block-level language detection (§30)

Language identification happens once per Markdown block so metric dispatch
can choose the correct locale pipeline.

**Tier 0 default (§30.1)** is a zero-dependency Unicode-block heuristic.
For the English/Japanese split, Unicode-block ratios outperform trigram LMs
on short inputs because Chinese has no hiragana/katakana:

```text
let total = non_whitespace_non_punct_chars
let kana  = hiragana_chars + katakana_chars
let cjk   = kana + han_chars
let latin = ascii_letter_chars + fullwidth_latin_letter_chars

if kana / total >= 0.15:                language = ja
elif cjk / total >= 0.40 and kana == 0: language = zh  (treated as "other")
elif latin / total >= 0.80:             language = en
else:                                   language = other
```

**Opt-in trigram classifier (§30.2)** behind Cargo features:

- `whatlang` — pure Rust, 70 languages, MIT, reliable above ~120 characters.
- `lingua` — highest accuracy in published benchmarks; restricted to
  `[English, Japanese]` for a manageable binary size.

**Tagging rules (§30.3):** a block inherits its parent heading's language
when its own signal is inconclusive; code fences, inline code, link
targets, image targets, front matter, and HTML are tagged `none` and
excluded from prose metrics; a document with both English and Japanese
blocks is labelled `mixed` at the document level but each block keeps its
own tag for metric routing.

## English readability suite (§31)

Mehen emits **every formula's raw score with provenance** rather than
averaging. Two formulas on the same text routinely disagree by 2–4 grade
levels because they target different comprehension thresholds (SMOG ~100%,
FKGL ~75%, Dale-Chall in between). Averaging them is statistically wrong.

| Formula | Section | Syllables | Key notes |
|---|---|---|---|
| Flesch Reading Ease | §31.1 | yes | `206.835 − 1.015 * ASL − 84.6 * ASW`. Higher = easier. |
| Flesch-Kincaid Grade | §31.2 | yes | `0.39 * ASL + 11.8 * ASW − 15.59`. MIL-M-38784A standard. |
| Gunning Fog | §31.3 | yes | `0.4 * (ASL + 100 * P_complex)`. Target grade 7–12 for business writing. |
| SMOG | §31.4 | yes | `1.0430 * sqrt(poly * 30 / sentences) + 3.1291`. `null` below 30 sentences. |
| ARI | §31.5 | no | `4.71 * CPW + 0.5 * ASL − 21.43`. Syllable-free. |
| Coleman-Liau | §31.6 | no | `0.0588 * L − 0.296 * S − 15.8`. Syllable-free. |
| New Dale-Chall | §31.7 | no | `0.1579 * PDW + 0.0496 * ASL` (+ `3.6365` if PDW > 5%). |
| FORCAST | §31.8 | counts 1-syllable | `20 − (N / 10)`. **Non-narrative** text (manuals, specs). |
| LIX | §31.9 | no | `ASL + 100 * (long_words / words)`. Sanity check. |
| RIX | §31.9 | no | `long_words / sentences`. |

**Ensemble reporting (§31.10):**

1. Emit every formula with provenance.
2. Compute an **ensemble grade band** as `[min(FKGL, Fog, ARI, CLI), max(…)]`
   — the interval where those four "running-prose" formulas agree.
3. Emit FORCAST separately as the preferred single score for non-narrative
   docs (API references, parameter tables, forms).
4. Suppress SMOG when `sentences < 30`.
5. Report Dale-Chall only with an explicit `list: ngsl-1.2` or
   `list: dale-chall-new-1995` provenance tag. Because the Dale-Chall 3000
   list is not openly licensed, mehen defaults to the **NGSL 1.2** (Browne
   et al. 2013, CC BY) with 2,800 headwords.

**Syllable counting (§31.11):** Tier-0 default is a vowel-group heuristic
(~85% agreement with CMU on open-domain text). Behind `--features
syllables-cmu`, mehen links the CMU Pronouncing Dictionary for exact counts
on ~134k words with the heuristic as an OOV fallback.

**Sentence segmentation (§31.12):** UAX #29 (`unicode-segmentation`) with:

- A bundled ~150-entry English abbreviation list that suppresses
  sentence breaks after `Mr.`, `e.g.`, `i.e.`, `U.S.`, `v1.2.3`, `file.ext`.
- No split when the period is followed by a lowercase letter, a digit, or
  `<space><digit>`.
- Markdown block boundaries (blank line, heading, fence open/close, list
  item start) are **hard** terminators regardless of punctuation.
- Inline code, fenced blocks, URLs, image alt-text, front matter, and HTML
  are stripped before segmentation.

**Thresholds by doc type (§31.13)** are conventions synthesized from
Google, Microsoft, and 18F style guides. They are tunable profile defaults:

| Doc type | FKGL | Fog | Passive max | Max sentence words |
|---|---:|---:|---:|---:|
| README / overview | ≤ 10 | ≤ 12 | 15 % | 30 |
| Tutorial | ≤ 9 | ≤ 11 | 10 % | 25 |
| API reference | ≤ 12 | ≤ 14 | 20 % | 35 |
| ADR / design | ≤ 12 | ≤ 14 | 25 % | 40 |
| Error messages | ≤ 7 | ≤ 9 | 5 % | 15 |
| Release notes | ≤ 11 | ≤ 13 | 15 % | 30 |

## English lexical diversity (§32)

Formula-independent indicators of vocabulary richness and content-word
saturation. They do not depend on syllable counts and are robust across
document types.

- **MATTR₅₀ (§32.2)** — Moving-Average Type-Token Ratio over 50-token
  sliding windows (Covington & McFall 2010). Length-invariant by
  construction and cheap to compute. MTLD and HD-D are reported as
  alternative diversity measures behind `--features lexical-diversity`.
- **Hapax ratio / dis-legomena ratio (§32.4)** — `V_1 / V` and `V_2 / V`.
  Zipf's law predicts hapax ≈ 0.5 on natural prose; > 0.6 flags laundry-list
  reference dumps, extremely low values flag repetitive template content.
- **Lexical density (§32.1)** — content words / total words. Without POS
  tagging, approximated as `1 − stopwords / tokens` using the 175-entry
  NLTK English stopword list. Typical ranges: spoken ~0.40, written ~0.52,
  academic ~0.60. High LD is legitimate in technical prose; very low LD
  flags conversational or templatic text.
- **Yule's K (§32.3)** — optional; MATTR is usually sufficient.
- **Sentence/word length moments (§32.5)** — `avg_sentence_words`,
  `p90_sentence_words`, `max_sentence_words`, `stddev_sentence_words`,
  `avg_word_chars`, `p90_word_chars`. These drive the §31 formulas but are
  reported individually so writers see the levers directly.

## English wording quality (§33)

| Sub-metric | Section | Default threshold |
|---|---|---|
| **Passive voice** (write-good / retext-passive pattern) | §33.1 | Doc-type ratio from §31.13. |
| **Hedge words** (Hyland 2005; proselint; ~165 entries) | §33.2 | Flag > 3 % in non-narrative docs. |
| **Weasel words** (write-good) | §33.3 | Count-based. |
| **Wordy phrases** (too-wordy, retext-simplify; ~240 entries) | §33.4 | Per-match count / 100 words. |
| **Adverb density** (-ly endings minus exceptions) | §33.5 | Hemingway budget ≤ 1 per 100 words. |
| **Nominalizations** (`-tion`, `-sion`, `-ment`, `-ence`, `-ance`, `-ity`, `-ness`, `-ism`) | §33.6 | Flag paragraph > 10 % of content words. |
| **Expletive constructions** (`^(there\|it)\s+(is\|are\|was\|were)`) | §33.7 | Per 100 sentences. |
| **Lexical illusions** (`lower(t[i-1]) == lower(t[i])`) | §33.8 | Zero-tolerance defect. |
| **Clichés** (~700 entries) | §33.9 | Per 1,000 words. |
| **Non-words** (`irregardless → regardless`, `thusly → thus`, …) | §33.9 | Error-level flag. |
| **Long sentences** | §33.10 | Warning > 30 words, error > 40. |

**Wording Quality Score (§33.11):**

```text
WordingQualityScore = clamp01(
    1
  − 0.18 * sat(passive_ratio;          0.25, 0.60)
  − 0.15 * sat(hedge_density;          0.02, 0.08)
  − 0.12 * sat(weasel_density;         0.01, 0.05)
  − 0.12 * sat(wordy_density;          0.01, 0.05)
  − 0.10 * sat(adverb_density;         0.02, 0.06)
  − 0.08 * sat(nominalization_density; 0.08, 0.20)
  − 0.08 * sat(long_sentence_rate;     0.05, 0.30)
  − 0.07 * sat(cliche_density;         0.002, 0.02)
  − 0.05 * (lexical_illusions > 0 ? 1 : 0)
  − 0.05 * (nonword_count     > 0 ? 1 : 0)
)
```

WQS is deliberately orthogonal to FillerLazyRisk (§17): §17 covers
repetition and specificity, §33 covers style and register.

## Inclusive Language Score (§33.12)

alex / retext-equality-style checks against a bundled list covering:

- **Gendered defaults** — `mankind → humanity`, `fireman → firefighter`,
  `manhole → maintenance-hole`.
- **Ableist idioms** — `crazy`, `insane`, `lame`, `dumb`, `blind to`,
  `tone deaf`.
- **Exclusionary tech terms** — `master/slave → primary/replica`,
  `whitelist/blacklist → allowlist/denylist`, `grandfather clause →
  legacy exception`, `sanity check → spot check`.
- **Condescending** — `obviously`, `just`, `simply`, `easy`, `of course`.

Output is a per-document `InclusiveLanguageScore` plus a list of flags with
source spans. Any new inclusive-language flag is a `🔴` regression in PR
reporting (§39.4).

## Japanese script composition (§34) — Tier 0

Japanese is unusual among major languages: script composition alone carries
enough information to produce defensible readability scores without a
tokenizer. This is the foundational insight of Tateishi, Ono & Yamada
(1988) and remains the basis for mehen's Tier-0 Japanese layer.

**Unicode script classification (§34.1):** each grapheme cluster classifies
into Hiragana, Katakana, Kanji (Han + Extensions A/B + Compatibility), CJK
punctuation, Latin (+ Fullwidth), or Digit.

**Primary ratios (§34.2):** `kanji_ratio`, `hiragana_ratio`,
`katakana_ratio`, `latin_ratio`, `digit_ratio`, `script_entropy` (Shannon
entropy over the five classes).

**Register bands (§34.3):**

| Kanji ratio | Register |
|---|---|
| < 20 % | Children's writing, conversation. |
| 20–30 % | Casual prose, novels, user-facing content. |
| 30–40 % | Newspaper, business writing, non-fiction. |
| 40–50 % | Technical, legal, academic. |
| > 50 % | Classical/literary, specialist text. |

Katakana > 15 % typically signals software documentation (loanwords like
`データベース`) or marketing copy. Hiragana > 75 % indicates text aimed at
small children or machine-translated output.

**Script-run features (§34.4):** a "run" is a maximal substring of
same-script characters. Per document: mean chars per alphabet run (`la`),
hiragana run (`lh`), kanji run (`lc`), katakana run (`lk`); percentages of
each run type (`pa`, `ph`, `pc`, `pk`); mean chars per sentence (`ls`);
`、` per `。` (`cp`). These are the exact inputs the Tateishi formula needs.

**Sentence segmentation (§34.5):** primary terminators `。`, `！`, `？` plus
half-width equivalents. Do not split inside `「…」`, `『…』`, `（…）`. Treat
blank-line paragraph boundaries and Markdown block boundaries as hard
terminators. Ellipsis `…` / `‥` / `...` is not a terminator.

**Sentence-length thresholds (§34.6):** default flag points are > 60 chars
(warning), > 90 (hard-to-read), > 120 (error). Mean sentence length > 60
triggers a document-level warning.

## Tateishi simplified RS (§35.1) + Jōyō grade (§35.2)

**Tateishi, Ono & Yamada (1988) simplified 6-variable form (§35.1):**

```text
RS = −0.12 * ls − 1.37 * la + 7.4 * lh − 23.18 * lc − 5.4 * lk
     − 4.67 * cp + 115.79
```

Calibrated so mean ≈ 50, SD ≈ 10, **higher = easier**. Mehen emits this as
`tateishi_rs` with sanity guards: refuse when `hiragana_ratio > 0.90` (the
formula is gamed upward) or when character count < 300.

Tateishi is mehen's Tier-0 Japanese readability score because it needs
sentence boundaries and script runs — both computable without a tokenizer.

**Jōyō grade proxy (§35.2):** the 2,136-character 2010 Jōyō list maps each
character to a grade 1–8 (1–6 elementary, 7 = secondary Jōyō, 8 = non-Jōyō
`hyōgai`). Ships as a ~6 KB static table behind `--features
japanese-jouyou`.

```text
jouyou_grade_mean = mean(grade(c) for each kanji c)
hyougai_ratio     = non_jouyou_kanji_chars / kanji_chars
```

`jouyou_grade_mean` is a direct school-grade analogue to Flesch-Kincaid:
< 3 indicates elementary reading, > 6 indicates high-school+ technical
prose.

Higher-tier formulas — **Shibasaki & Hara (§35.4)**, **Lee & Hasebe
jReadability (§35.5)**, **Obi/Obi2 (§35.6)**, **Mizuno/Goda (§35.7)** —
require morphological analysis and are available behind `--features
japanese-morph` (Lindera + IPADIC) or `--features japanese-unidic` (Vibrato
+ UniDic). **JLPT bands (§35.3)** are optional behind `--features
japanese-jlpt` (~300 KB).

## Japanese wording quality (§36.7)

```text
WordingQualityScore_ja = clamp01(
    1
  − 0.15 * sat(long_sentence_rate;          0.05, 0.30)
  − 0.12 * sat(weak_phrase_density;         0.01, 0.05)
  − 0.12 * sat(redundant_expression_rate;   0.01, 0.05)
  − 0.10 * sat(doubled_joshi_count / sentences; 0.02, 0.10)
  − 0.10 * sat(long_kanji_run_rate;         0.05, 0.25)
  − 0.10 * (keitai_jotai_mix_count > 0
            ? sat(mix_ratio; 0.02, 0.20)
            : 0)
  − 0.08 * sat(max_comma_violation_rate;    0.02, 0.15)
  − 0.08 * sat(hyougai_ratio;               0.02, 0.10)
  − 0.07 * sat(jtf_violation_density;       0.5,  5.0)
  − 0.08 * sat(gairaigo_excess;             0.30, 0.60)
)
```

Reported as a 0–1 score with sub-score breakdown.

## JTF rule conformance (§36.5)

The Japan Translation Federation's 12 rules are mechanically checkable:

| Rule | Check | Severity |
|---|---|---|
| 1 | keitai/jōtai consistency | warn |
| 2 | `、` / `。` used as punctuation | info |
| 3 | Stick to Jōyō kanji (flag `hyōgai`) | warn |
| 4 | Okurigana per official rules | info |
| 5 | Trailing long-vowel mark on katakana compound endings (`コンピューター` not `コンピュータ`) | warn |
| 6 | Long katakana compounds broken with `・` or half-width space | info |
| 7 | Kanji / hiragana / katakana full-width | error |
| 8 | Digits and Latin alphabet half-width | warn |
| 9 | Symbols full-width | info |
| 10 | No space between full-width and half-width | info |
| 11 | `.`, `,`, spaces half-width | info |
| 12 | Standardize unit notation | info |

## textlint preset subset (§36.6)

Mehen ports a subset of `textlint-rule-preset-ja-technical-writing` with
their documented defaults:

| Rule | Default | Check |
|---|---|---|
| `sentence-length` | ≤ 100 chars | Long-sentence flag |
| `max-comma` | ≤ 3 `,` / sentence | Over-comma'd sentences |
| `max-ten` | ≤ 3 `、` / sentence | Over-reading-marked sentences |
| `max-kanji-continuous-len` | ≤ 6 | Hard-to-read kanji runs |
| `no-mix-dearu-desumasu` | zone-aware | JTF rule 1 |
| `ja-no-mixed-period` | `。` | Sentence terminator consistency |
| `no-double-negative-ja` | — | `ないではない` |
| `no-doubled-joshi` | `min_interval: 1` | Repeated particles (`を…を`) |
| `no-doubled-conjunctive-particle-ga` | — | Repeated `が` |
| `no-doubled-conjunction` | — | `しかし…しかし` |
| `no-dropping-the-ra` | — | Colloquial `見れる` for `見られる` |
| `no-hankaku-kana` | — | Halfwidth kana forbidden |
| `no-exclamation-question-mark` | — | `!` / `?` in technical docs |
| `ja-no-weak-phrase` | — | `かもしれない`, `と思います` |
| `ja-no-successive-word` | — | Repeated words |
| `ja-no-abusage` | — | Misused kanji |
| `ja-no-redundant-expression` | — | `することができる` → `できる` |
| `ja-unnatural-alphabet` | — | IME miscarriages |

All thresholds are user-tunable via profile configuration.

## Tier model (§37.1)

The prose layer ships in tiers so the default binary stays small:

| Tier | Cargo features | What you get | Binary cost |
|---|---|---|---|
| **0 (default)** | none | Unicode-block language detection; UAX #29 sentence/word segmentation; vowel-group English syllables; Tateishi simplified RS; all wording heuristics off static lists; JTF mechanical checks 1, 3, 5, 7, 8, 11 | ~100–300 KB |
| **1a** | `syllables-cmu` | CMU Pronouncing Dictionary for English syllables | +1–2 MB |
| **1b** | `japanese-jouyou` | Jōyō grade proxy, hyōgai ratio | +10 KB |
| **1c** | `japanese-jlpt` | JLPT N5–N1 word and kanji bands | +300 KB |
| **1d** | `lingua` | High-accuracy trigram language detection (EN + JA only) | +2–5 MB |
| **2a** | `japanese-morph` (Lindera + embedded IPADIC) | Bunsetsu counts, POS tags, Shibasaki grade, jukugo morphological refinement | +50 MB |
| **2b** | `japanese-unidic` (Vibrato + external UniDic) | jReadability replica, kango/wago/gairaigo split | external dict |
| **2c** | `lexical-diversity` | MTLD, HD-D, Yule's K | +50 KB |
| **2d** | `vale-rules` | Parse vale-compatible YAML rule packs (existence, substitution, occurrence, repetition, capitalization, consistency, conditional, metric, readability) | +200 KB |

Tier 0 delivers approximately 80 % of the observable readability and
wording signal with zero large-dictionary dependencies. Tiers 1–2 are
strictly additive: enabling them never changes Tier 0 outputs.

**Interaction with structural scores (§29.3):**

- **DMI (§10):** unaffected by default. With `--with-prose-penalty`, a
  bounded `0.05 * (1 − WordingQualityScore)` term subtracts. Default DMI
  weights (§10.2) are unchanged.
- **FillerLazyRisk (§17):** can optionally consume `specificity_density_en`
  that uses stopword ratios instead of the purely character-class default.
  Opt-in behind a Cargo feature.
- **Review Criticality Index (§18):** the ensemble readability grade of
  changed paragraphs contributes as an explicit sub-term only when the
  prose layer is enabled.

## Anti-gaming defenses (§37.5)

The prose layer resists trivial metric games:

- **Code-block exfiltration.** Prose heuristics never count content of
  `fenced_code_block`, `inline_code`, link destinations, `image_block`
  targets, `html_block`, `mdx_jsx_block`, `front_matter`, or table-cell
  delimiters.
- **Identifier inflation.** A long CamelCase or snake_case identifier is
  one word; its character contribution to ARI / Coleman-Liau is capped
  (`min(identifier_len, 20)`) when it appears in running prose.
- **Citation padding.** Quoted-literal blockquotes count toward structural
  metrics but not toward weasel/hedge detection.
- **Short-doc gaming.** Grade-level scores are suppressed when `words < 100`
  OR `sentences < 5`; a `short_doc_warning` is emitted instead of a
  manipulable grade.
- **Abbreviation splitting.** The bundled abbreviation list suppresses
  sentence breaks after `Mr.`, `e.g.`, `i.e.`, `U.S.`, `vs.`, `approx.`,
  `fig.`, `ver.`, `ch.`, etc.

These defenses are normative: the validation plan (§37.6 / §26.4) requires
that none of the gaming attacks above shift overall `WordingQualityScore`
by more than 0.05.

## Exported schema

The prose layer extends the Markdown schema (§23) with a dedicated
`prose:` block (§29.2, abbreviated):

```yaml
markdown:
  prose:
    language_detection:
      dominant_language: en | ja | other | mixed
      blocks:
        - {range: [start, end], language: en, confidence: 0.97}
    english:
      readability:
        flesch_reading_ease:  0.0
        flesch_kincaid_grade: 0.0
        gunning_fog:          0.0
        smog:                 0.0   # null if sentences < 30
        ari:                  0.0
        coleman_liau:         0.0
        dale_chall_new:       0.0   # with list-provenance tag
        forcast:              0.0
        ensemble_grade_band:  [low, high]
      lexical:
        mattr_50: 0.0
        hdd_42:   0.0
        mtld:     0.0   # --features lexical-diversity
        yule_k:   0.0   # --features lexical-diversity
        hapax_ratio:         0.0
        lexical_density:     0.0
        avg_sentence_words:  0.0
        p90_sentence_words:  0
        avg_word_chars:      0.0
      wording:
        passive_ratio:         0.0
        hedge_density:         0.0
        weasel_density:        0.0
        wordy_density:         0.0
        adverb_density:        0.0
        nominalization_density: 0.0
        expletive_count:       0
        lexical_illusions:     0
        cliche_density:        0.0
        nonword_count:         0
        long_sentence_count:   0
      inclusive_language:
        flags: []
    japanese:
      script_composition:
        kanji_ratio:    0.0
        hiragana_ratio: 0.0
        katakana_ratio: 0.0
        latin_ratio:    0.0
        digit_ratio:    0.0
        script_entropy: 0.0
      readability:
        tateishi_rs:       0.0
        jouyou_grade_mean: 0.0   # --features japanese-jouyou
        hyougai_ratio:     0.0   # --features japanese-jouyou
        jreadability:      0.0   # --features japanese-unidic
        shibasaki_grade:   0.0   # --features japanese-morph
      lexical:
        avg_sentence_chars: 0.0
        p90_sentence_chars: 0
        comma_period_ratio: 0.0
        jukugo_density:     0.0
      wording:
        politeness_dominant:      desumasu | dearu | mixed
        keitai_jotai_mix_count:   0
        weak_phrase_count:        0
        redundant_expression_count: 0
        doubled_joshi_count:      0
        long_kanji_run_count:     0
      style_conformance:
        jtf_violations: []
  meta:
    short_doc_warning: false
    words_counted:     0
    sentences_counted: 0
    blocks_stripped:   [code, frontmatter, html, mdx, math, table]
```

## See also

- [Markdown Metrics](./markdown.md) — structural layer (LOC family, MCC,
  MRPC, DMI, link/table/visual/grounding/filler/RCI).
- [`mehen diff` PR comment](../commands/pr-comment.md) — how prose deltas
  surface in the sticky GitHub comment, including per-profile
  threshold breaches.
- `docs/mehen_markdown_metrics_research_foundation.md` §§29–38 for the
  full derivations, citation graph, and tool compatibility surface
  (vale, retext, write-good, proselint, alex, textlint-ja, harper,
  cargo-spellcheck).
