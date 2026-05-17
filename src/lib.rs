//! Pre-1.0 `mehen` library — the metric and markdown machinery the test
//! suite still exercises during the v1 rewrite.
//!
//! Per the rewrite plan §8, contents of this crate get redistributed into
//! the new `crates/mehen-*` workspace one module at a time. When the
//! redistribution is complete the entire crate is deleted; the published
//! `mehen` binary lives in `crates/mehen-cli/`.

#![allow(clippy::upper_case_acronyms)]

pub mod alterator;
pub mod checker;
pub mod ci;
pub mod concurrent_files;
pub mod diff;
#[cfg(feature = "markdown")]
pub mod diff_markdown;
pub mod formats;
pub mod getter;
pub mod git;
pub mod langs;
pub mod languages;
pub mod macros;
#[cfg(feature = "markdown")]
pub mod markdown;
pub mod metric_selector;
pub mod metrics;
pub mod node;
pub mod parser;
pub mod preproc;
pub mod rust_metric_helpers;
pub mod spaces;
pub mod tools;
pub mod top_offenders;
pub mod traits;

use globset::{Glob, GlobSet, GlobSetBuilder};

/// Build a `GlobSet` from a list of glob strings, ignoring empty entries.
///
/// Used by both the `diff` and `top-offenders` orchestrators to turn the
/// user's `--include` / `--exclude` flags into a usable matcher.
pub fn mk_globset(elems: Vec<String>) -> GlobSet {
    if elems.is_empty() {
        return GlobSet::empty();
    }
    let mut globset = GlobSetBuilder::new();
    elems.iter().filter(|e| !e.is_empty()).for_each(|e| {
        if let Ok(glob) = Glob::new(e) {
            globset.add(glob);
        }
    });
    globset.build().map_or(GlobSet::empty(), |globset| globset)
}
