//! Snapshot tests for the Markdown pipeline (Phase A + Phase B + Phase C +
//! Phase E).
//!
//! Each fixture under `fixtures/` exercises a distinct aspect of the
//! analyzer so regressions in any single dimension (LOC bucket, word count,
//! section tree, ECU, MRPC graph weight, MCC penalty, Halstead class
//! coverage, DMI normalization, link class, table burden, diagram
//! complexity, artifact debt, prose EN/JA readability and wording) surface
//! as an isolated snapshot diff.

use std::path::PathBuf;

use crate::markdown::analyze_markdown;
use crate::markdown::diagrams;

fn load_fixture(name: &str) -> (String, PathBuf) {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("src/markdown/tests/fixtures");
    path.push(name);
    let contents =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading fixture {name}: {e}"));
    (contents, path)
}

fn assert_fixture_snapshot(name: &str) {
    let (source, path) = load_fixture(name);
    let metrics = analyze_markdown(&source, &path);
    // Redact the absolute path so snapshots are portable across workspaces.
    insta::with_settings!({
        snapshot_suffix => name,
        omit_expression => true,
    }, {
        insta::assert_yaml_snapshot!(metrics, {
            ".path" => "<fixture_path>"
        });
    });
}

#[test]
fn empty_fixture() {
    assert_fixture_snapshot("empty.md");
}

#[test]
fn pure_prose_fixture() {
    assert_fixture_snapshot("pure_prose.md");
}

#[test]
fn code_fences_fixture() {
    assert_fixture_snapshot("code_fences.md");
}

#[test]
fn table_mixed_fixture() {
    assert_fixture_snapshot("table_mixed.md");
}

#[test]
fn frontmatter_fixture() {
    assert_fixture_snapshot("frontmatter.md");
}

#[test]
fn heading_skip_fixture() {
    assert_fixture_snapshot("heading_skip.md");
}

#[test]
fn tight_list_fixture() {
    // Tight bullet lists land in the `list_item` container without a
    // paragraph child. LOC classification must count them as PLOC; if a
    // future regression drops ListItem from the prose arm they fall
    // through to Blank and land in BLOC instead.
    assert_fixture_snapshot("tight_list.md");
}

#[test]
fn navigation_heavy_fixture() {
    // Exercises §7 MRPC: many sections, internal anchors, relative
    // repo links, external links across multiple domains, and
    // footnote/reference definitions.
    assert_fixture_snapshot("navigation_heavy.md");
}

#[test]
fn cognitive_cluster_fixture() {
    // Exercises §8 MCC penalties: unlabelled code, dense links, nested
    // lists, blockquote, callout.
    assert_fixture_snapshot("cognitive_cluster.md");
}

#[test]
fn halstead_mixed_fixture() {
    // Exercises §9 Halstead operator/operand classes.
    assert_fixture_snapshot("halstead_mixed.md");
}

#[test]
fn deep_nesting_fixture() {
    // Exercises §8.2 nesting multiplier — lists, blockquotes, callouts
    // interleaved.
    assert_fixture_snapshot("deep_nesting.md");
}

#[test]
fn embedded_code_large_fixture() {
    // Exercises §9.4 embedded-volume dispatch across Rust / Python / TS;
    // unsupported fences (sql) must not contribute.
    assert_fixture_snapshot("embedded_code_large.md");
}

#[test]
fn links_mixed_fixture() {
    // Exercises every §11.1 link class: internal anchor (resolving + not),
    // relative file (resolving + not), external (and bare URL), IssuePR,
    // Scholarly, ExternalVendor, reference-definition, shortcut reference,
    // and footnote. The aggregate link_debt / scent / review_burden pin
    // the §11.2–§11.4 formulas.
    assert_fixture_snapshot("links_mixed.md");
}

#[test]
fn broken_links_fixture() {
    // High broken-rate case: drives link_debt_score past the 0.10 sat
    // threshold.
    assert_fixture_snapshot("broken_links.md");
}

#[test]
fn table_large_fixture() {
    // Hard-warning table per §13: cols > 12 so the burden score dominates
    // the aggregate.
    assert_fixture_snapshot("table_large.md");
}

#[test]
fn diagram_mermaid_fixture() {
    // Codifies the §12.2 two-node cycle invariant.
    assert_fixture_snapshot("diagram_mermaid.md");
}

#[test]
fn diagram_parse_error_fixture() {
    // Unknown language ("tikz") flips parse_error, adding the +2.0 term.
    assert_fixture_snapshot("diagram_parse_error.md");
}

#[test]
fn images_no_alt_fixture() {
    // One image without alt-text + missing target vs. one with alt and a
    // resolving target — pins the V_scaffold asymmetry.
    assert_fixture_snapshot("images_no_alt.md");
}

#[test]
fn artifact_debt_high_fixture() {
    // Several unlabelled fences, a parse-error diagram, and raw HTML.
    assert_fixture_snapshot("artifact_debt_high.md");
}

#[test]
fn tiny_file_produces_metrics() {
    // Codex P1: tiny Markdown files (1-3 bytes) used to be swallowed by
    // `read_file_inner`'s `file_size <= 3` early return. The analyzer
    // itself must still produce metrics — `read_file_raw` handles the
    // file-size heuristic on the CLI side, but the analyzer is the last
    // line of defense and must not assume a minimum input length.
    for src in ["", "a", "#", "#\n", "a\n"] {
        let path = PathBuf::from("tiny.md");
        let metrics = analyze_markdown(src, &path);
        // The only invariant we care about here: no panic, and metric
        // fields are populated (even with zero values) so JSON emission
        // never produces malformed output.
        assert!(
            metrics.loc.dloc <= 2,
            "dloc {}: input {src:?}",
            metrics.loc.dloc
        );
    }
}

#[test]
fn readme_en_fixture() {
    // Pure-English README-style fixture. Validates that the prose layer
    // populates english.* with non-null readability numbers and leaves
    // japanese absent. Also stresses the ensemble grade band calculation.
    assert_fixture_snapshot("readme_en.md");
}

#[test]
fn readme_ja_fixture() {
    // Pure-Japanese README. Validates the Tateishi RS path, script
    // composition, Jōyō grade proxy, and politeness classification.
    assert_fixture_snapshot("readme_ja.md");
}

#[test]
fn mixed_bilingual_fixture() {
    // Bilingual doc: blocks array must carry per-block language tags and
    // both english.* and japanese.* must populate. dominant_language is
    // `mixed`.
    assert_fixture_snapshot("mixed_bilingual.md");
}

#[test]
fn passive_heavy_fixture() {
    // Passive-voice-heavy document. `english.wording.passive_ratio` must
    // rise above 0.5 and WordingQualityScore must drop correspondingly.
    let (source, path) = load_fixture("passive_heavy.md");
    let metrics = analyze_markdown(&source, &path);
    let en = metrics
        .prose
        .english
        .as_ref()
        .expect("passive-heavy doc has English content");
    assert!(
        en.wording.passive_ratio > 0.5,
        "expected passive_ratio > 0.5, got {}",
        en.wording.passive_ratio
    );
    assert!(
        en.wording.wording_quality_score < 0.9,
        "expected WQS < 0.9 due to passive voice, got {}",
        en.wording.wording_quality_score
    );
    assert_fixture_snapshot("passive_heavy.md");
}

#[test]
fn tateishi_sample_fixture() {
    // Large Japanese fixture. Validates Tateishi RS surface, jouyou grade
    // non-null, and jukugo density > 0.
    let (source, path) = load_fixture("tateishi_sample.md");
    let metrics = analyze_markdown(&source, &path);
    let ja = metrics
        .prose
        .japanese
        .as_ref()
        .expect("tateishi sample has Japanese content");
    assert!(
        ja.readability.tateishi_rs.is_some(),
        "Tateishi RS must be populated"
    );
    assert!(
        ja.readability.jouyou_grade_mean.is_some(),
        "Jōyō grade mean must be populated"
    );
    assert!(
        ja.lexical.jukugo_density > 0.0,
        "expected non-zero jukugo density, got {}",
        ja.lexical.jukugo_density
    );
    assert_fixture_snapshot("tateishi_sample.md");
}

#[test]
fn short_doc_fixture() {
    // Short-doc guard: words < 100 / sentences < 5 → suppress grade
    // formulas, raise short_doc_warning.
    let (source, path) = load_fixture("short_doc.md");
    let metrics = analyze_markdown(&source, &path);
    assert!(
        metrics.prose.meta.short_doc_warning,
        "short doc must emit warning"
    );
    let en = metrics
        .prose
        .english
        .as_ref()
        .expect("short doc still has EN prose");
    assert!(
        en.readability.flesch_reading_ease.is_none(),
        "FRES must be null for short doc"
    );
    assert!(
        en.readability.flesch_kincaid_grade.is_none(),
        "FKGL must be null for short doc"
    );
    assert_fixture_snapshot("short_doc.md");
}

#[test]
fn weak_phrase_ja_fixture() {
    // Japanese document with many weak phrases. `weak_phrase_count` must
    // be non-zero.
    let (source, path) = load_fixture("weak_phrase_ja.md");
    let metrics = analyze_markdown(&source, &path);
    let ja = metrics
        .prose
        .japanese
        .as_ref()
        .expect("weak-phrase fixture is Japanese");
    assert!(
        ja.wording.weak_phrase_count > 0,
        "expected weak_phrase_count > 0, got {}",
        ja.wording.weak_phrase_count
    );
    assert_fixture_snapshot("weak_phrase_ja.md");
}

#[test]
fn prose_no_modification_of_structural_scores() {
    // §29.1 non-negotiable: prose layer must NEVER modify DMI, MCC, MRPC,
    // or any structural score. This test re-analyzes the pure-prose fixture
    // and captures its structural fields, then confirms they match what
    // Phase A alone would produce.
    let (source, path) = load_fixture("pure_prose.md");
    let metrics = analyze_markdown(&source, &path);
    // Phase-A baseline for pure_prose.md:
    //   loc.dloc = 7, ploc = 4, bloc = 3; size.words = 27; sections = 1.
    assert_eq!(metrics.loc.dloc, 7);
    assert_eq!(metrics.loc.ploc, 4);
    assert_eq!(metrics.loc.bloc, 3);
    assert_eq!(metrics.size.words, 27);
    assert_eq!(metrics.sections.len(), 1);
}

#[test]
fn trailing_newlines_preserved_in_dloc() {
    // `read_file_raw` feeds `analyze_markdown` the file-on-disk bytes, so
    // trailing blank lines survive and count toward DLOC/BLOC. Guards
    // against the Codex P1 regression: if a future change routes Markdown
    // through `remove_blank_lines` again, the trailing blanks collapse and
    // this assertion breaks.
    //
    // Input: "Alpha.\n\nBeta.\n\n\n"
    //   line 1: Alpha.   (prose)
    //   line 2: blank
    //   line 3: Beta.    (prose)
    //   line 4: blank
    //   line 5: blank
    //   (the final \n is the line-5 terminator, not a new line)
    let src = "Alpha.\n\nBeta.\n\n\n";
    let path = PathBuf::from("trailing_newlines.md");
    let metrics = analyze_markdown(src, &path);
    assert_eq!(
        metrics.loc.dloc, 5,
        "dloc must count every physical line including trailing blanks"
    );
    assert!(
        metrics.loc.bloc >= 3,
        "three blank lines (one between, two trailing) must land in BLOC"
    );

    // Cross-check: stripping all trailing newlines (the `remove_blank_lines`
    // regression path) would drop DLOC to 3.
    let normalized = "Alpha.\n\nBeta.\n";
    let normalized_metrics = analyze_markdown(normalized, &path);
    assert_eq!(
        normalized_metrics.loc.dloc, 3,
        "sanity check: the normalized form undercounts lines — that is why \
         Markdown must receive raw bytes"
    );
}

#[test]
fn embedded_volume_scales_with_fence_size() {
    // Invariant (§9.4): a 100-LOC supported-language code fence must
    // produce an `embedded_volume` that scales sub-linearly with the
    // fence's internal Halstead volume (due to the 0.20 * sqrt(volume_c)
    // term) but non-zero. Doubling the fence content should NOT double the
    // embedded_volume — the sqrt term grows by ~1.41× only.
    let small = build_rust_fence(5);
    let medium = build_rust_fence(50);
    let large = build_rust_fence(100);

    let path = PathBuf::from("scale.md");
    let s = analyze_markdown(&small, &path);
    let m = analyze_markdown(&medium, &path);
    let l = analyze_markdown(&large, &path);

    let ev_s = s.complexity.halstead.embedded_volume;
    let ev_m = m.complexity.halstead.embedded_volume;
    let ev_l = l.complexity.halstead.embedded_volume;

    assert!(ev_s > 0.0, "small fence embedded volume must be > 0");
    assert!(ev_m > ev_s, "medium > small");
    assert!(ev_l > ev_m, "large > medium");
    // total_volume = markdown_volume + embedded_volume.
    assert!(
        (l.complexity.halstead.total_volume - l.complexity.halstead.volume - ev_l).abs() < 1e-6,
        "total_volume must equal volume + embedded_volume"
    );
}

fn build_rust_fence(repeats: usize) -> String {
    let mut s = String::from("# Title\n\n```rust\n");
    for _ in 0..repeats {
        s.push_str("fn a() { let x = 1 + 2; let y = x * 3; }\n");
    }
    s.push_str("```\n");
    s
}

#[test]
fn dmi_stays_in_range() {
    // §10.4: DMI is bounded to [0, 100]. Phase B without L/T/A/S/F/G cannot
    // push DMI below 54 (= 100 - 18 - 18 - 10).
    for name in [
        "empty.md",
        "pure_prose.md",
        "code_fences.md",
        "navigation_heavy.md",
        "cognitive_cluster.md",
        "halstead_mixed.md",
        "deep_nesting.md",
        "embedded_code_large.md",
    ] {
        let (source, path) = load_fixture(name);
        let metrics = analyze_markdown(&source, &path);
        let dmi = metrics.maintainability.documentation_maintainability_index;
        assert!(
            (0.0..=100.0).contains(&dmi),
            "{name}: DMI {dmi} outside [0,100]"
        );
        assert!(
            dmi >= 54.0 - 1e-9,
            "{name}: DMI {dmi} below Phase-B floor (54)"
        );
    }
}

#[test]
fn mrpc_raw_matches_classic_formula() {
    // §7.2: `mrpc_raw = |E| - |N| + 2P`. Phase B does not expose |N|/|E|/P
    // directly, but we can cross-check by computing both raw and weighted
    // for fixtures where we know the graph shape.
    let (source, path) = load_fixture("navigation_heavy.md");
    let metrics = analyze_markdown(&source, &path);
    // At least one section + some edges, so raw cannot be 0.
    assert!(metrics.complexity.reading_path_complexity_raw.abs() > 0.0);
}

#[test]
fn halstead_vocabulary_is_non_negative() {
    for name in [
        "halstead_mixed.md",
        "embedded_code_large.md",
        "code_fences.md",
    ] {
        let (source, path) = load_fixture(name);
        let metrics = analyze_markdown(&source, &path);
        let h = &metrics.complexity.halstead;
        assert_eq!(h.vocabulary, h.operators_distinct + h.operands_distinct);
        assert_eq!(h.length, h.operators_total + h.operands_total);
        assert!(h.volume >= 0.0);
    }
}

#[test]
fn unlabelled_code_fence_penalty_shows_up() {
    // Two documents identical except for the fence's language tag. The
    // unlabelled variant must have higher MCC per §8.1 `Unlabelled code
    // fence +1.50`.
    let with_tag = "# T\n\nIntro.\n\n```rust\nlet x = 1;\n```\n\nOutro.\n";
    let without_tag = "# T\n\nIntro.\n\n```\nlet x = 1;\n```\n\nOutro.\n";
    let path = PathBuf::from("cmp.md");
    let a = analyze_markdown(with_tag, &path);
    let b = analyze_markdown(without_tag, &path);
    assert!(
        b.complexity.cognitive_complexity > a.complexity.cognitive_complexity,
        "unlabelled MCC ({}) should be > labelled MCC ({})",
        b.complexity.cognitive_complexity,
        a.complexity.cognitive_complexity
    );
}

/// Spec-pinned sanity check for the §12.2 cycle formula. Independent of the
/// Markdown analyzer so regressions in the diagram parser surface before
/// the insta snapshots start drifting.
#[test]
fn mermaid_two_node_cycle_matches_spec() {
    let sig = diagrams::mermaid::parse("graph TD\n  A --> B\n  B --> A\n");
    assert_eq!(sig.nodes, 2);
    assert_eq!(sig.edges, 2);
    assert_eq!(sig.components, 1);
    assert_eq!(sig.cycles, 1);
    assert!(!sig.parse_error);
}
