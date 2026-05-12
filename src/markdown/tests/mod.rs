//! Snapshot tests for the Markdown pipeline (Phase A + Phase C).
//!
//! Each fixture under `fixtures/` exercises a distinct aspect of the
//! analyzer so regressions in any single dimension (LOC bucket, word count,
//! section tree, ECU, link class, table burden, diagram complexity, artifact
//! debt) surface as an isolated snapshot diff.

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
    for src in ["", "a", "#", "#\n", "a\n"] {
        let path = PathBuf::from("tiny.md");
        let metrics = analyze_markdown(src, &path);
        assert!(
            metrics.loc.dloc <= 2,
            "dloc {}: input {src:?}",
            metrics.loc.dloc
        );
    }
}

#[test]
fn trailing_newlines_preserved_in_dloc() {
    // See Phase-A comment: trailing blanks must survive in DLOC/BLOC.
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
    let normalized = "Alpha.\n\nBeta.\n";
    let normalized_metrics = analyze_markdown(normalized, &path);
    assert_eq!(
        normalized_metrics.loc.dloc, 3,
        "sanity check: the normalized form undercounts lines — that is why \
         Markdown must receive raw bytes"
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
