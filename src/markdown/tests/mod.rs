//! Snapshot tests for the Phase-A Markdown pipeline.
//!
//! Each fixture under `fixtures/` exercises a distinct aspect of the
//! analyzer so regressions in any single dimension (LOC bucket, word count,
//! section tree, ECU coefficient) surface as an isolated snapshot diff.

use std::path::PathBuf;

use crate::markdown::analyze_markdown;

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
