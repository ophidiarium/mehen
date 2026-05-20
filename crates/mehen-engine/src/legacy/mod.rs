//! Pre-1.0 `diff` and `top-offenders` orchestrators retained verbatim.
//!
//! After the per-language migrations (Python/TS/PHP/Ruby/Rust/Go/C/Kotlin/
//! PowerShell), every analyzer flows through `mehen-engine::AnalyzerRegistry`
//! and the legacy tree-sitter walker is gone. What remains here is the
//! pre-1.0 CLI dispatch surface — `run_diff`, `run_top_offenders`, the
//! shared metric selector catalogue, the rayon-based file walker, and the
//! glob helper — kept until plan §7 Phase 5 ports the orchestrators to the
//! new `analyze_diff` / `rank_top_offenders` entry points and rewires
//! `mehen-cli` directly.

pub(crate) mod concurrent_files;
pub(crate) mod diff;
pub(crate) mod metric_selector;
pub(crate) mod top_offenders;

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
