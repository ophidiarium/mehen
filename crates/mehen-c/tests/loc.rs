//! LOC tests for the C walker.
//!
//! Every legacy `check_metrics::<CParser>` LOC test from
//! `crates/mehen-engine/src/legacy/metrics/loc.rs` is ported here
//! byte-identical so the parity contract (plan §12.3.1) is visibly
//! maintained. The LLOC kind set spans 36 statement / preprocessor
//! variants — see `walker.rs::classify_loc`.

use mehen_c::CAnalyzer;
use mehen_core::{AnalysisConfig, Language, LanguageAnalyzer, SourceFile};

fn analyze(source: &str) -> mehen_core::LanguageAnalysis {
    let mut text = source.trim_end().trim_matches('\n').to_string();
    text.push('\n');
    let analyzer = CAnalyzer::new();
    let file = SourceFile::new("foo.c".into(), Language::C, text);
    analyzer.analyze(&file, &AnalysisConfig::default()).unwrap()
}

#[test]
fn c_typedef_counts_as_lloc() {
    // `typedef` is a declaration like `int x;` and must contribute one
    // logical line. Together with the `int x;` declaration this gives
    // an LLOC of 2.
    let a = analyze(
        "typedef unsigned int u32;
int x;",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(loc.lloc, 2.0);
}

#[test]
fn c_preproc_conditionals_count_as_lloc() {
    // `#ifdef FOO ... #else ... #endif` exposes two preprocessor
    // conditional containers (`preproc_ifdef` and a nested
    // `preproc_else`). Combined with the two inner `int x = …;`
    // declarations, LLOC must reach 4:
    //   +1 preproc_ifdef  (the `#ifdef FOO` branch)
    //   +1 declaration    (`int x = 1;`)
    //   +1 preproc_else   (the `#else` branch)
    //   +1 declaration    (`int y = 2;`)
    let a = analyze(
        "#ifdef FOO
int x = 1;
#else
int y = 2;
#endif",
    );
    let loc = mehen_report::metrics_json::loc(&a.root.metrics);
    assert_eq!(loc.lloc, 4.0);
}
