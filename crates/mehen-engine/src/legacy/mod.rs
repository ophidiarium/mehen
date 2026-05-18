//! Pre-1.0 metric and analysis machinery, relocated wholesale from the
//! pre-1.0 `mehen/src/` tree per rewrite plan §8.
//!
//! Each item is re-exposed under `mehen_engine::legacy::*` so the
//! transitional thin wrapper at `mehen/src/lib.rs` can re-export them
//! through the original `crate::*` paths (`crate::legacy::langs::LANG`,
//! `crate::legacy::metrics::*`, `crate::legacy::diff::*`, `crate::legacy::top_offenders::*` …)
//! and keep every pre-1.0 test, snapshot and CLI command compiling
//! without modification.
//!
//! Phase 5 lives here unchanged from the pre-1.0 layout — the legacy
//! tree splits into `mehen-metrics`, the per-language analyzer crates
//! and `mehen-engine` proper as the new analyzers reach parity (plan
//! §8.2 / §8.3). The unsafe-code lint is relaxed because the legacy
//! `mehen-pre1` crate did not deny it, and a few macro-generated
//! tests rely on `unsafe` extern declarations for tree-sitter
//! grammars.
#![allow(unsafe_code)]
#![allow(clippy::upper_case_acronyms)]

pub(crate) mod alterator;
pub(crate) mod checker;
pub(crate) mod concurrent_files;
pub(crate) mod diff;
pub(crate) mod getter;
pub(crate) mod langs;
pub(crate) mod languages;
pub(crate) mod macros;
pub(crate) mod metric_selector;
pub(crate) mod metrics;
pub(crate) mod node;
pub(crate) mod parser;
pub(crate) mod preproc;
pub(crate) mod rust_metric_helpers;
pub(crate) mod spaces;
pub(crate) mod tools;
pub(crate) mod top_offenders;
pub(crate) mod traits;

use globset::{Glob, GlobSet, GlobSetBuilder};

/// Build a `GlobSet` from a list of glob strings, ignoring empty entries.
///
/// Used by both the `diff` and `top-offenders` orchestrators to turn the
/// user's `--include` / `--exclude` flags into a usable matcher.
pub(crate) fn mk_globset(elems: Vec<String>) -> GlobSet {
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
