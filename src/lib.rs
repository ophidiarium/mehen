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
/// CI environment detection — physically relocated to
/// `crates/mehen-engine/src/ci.rs` per plan §8.1. Re-exported under the
/// original `crate::ci` path so the still-in-place `src/diff.rs` keeps
/// compiling unchanged during the rest of the move.
pub use mehen_engine::ci;
pub mod concurrent_files;
pub mod diff;
/// Markdown documentation diff renderer was relocated to
/// `crates/mehen-report/src/github_markdown_docs.rs` per plan §8.1.
/// Re-exported under the original `crate::diff_markdown` path so
/// `src/diff.rs` keeps compiling unchanged.
#[cfg(feature = "markdown")]
pub use mehen_report::github_markdown_docs as diff_markdown;
pub mod formats;
pub mod getter;
/// Git operations were physically relocated to `crates/mehen-git/`
/// per plan §8.1. Re-exported under the original `crate::git` path so
/// `src/diff.rs` keeps compiling unchanged.
pub use mehen_git as git;
pub mod langs;
pub mod languages;
pub mod macros;
/// Markdown analyzer was physically relocated to
/// `crates/mehen-markdown/` per plan §8.1. Re-exported under the
/// original `crate::markdown` path so the still-in-place `src/diff.rs`
/// and `src/diff_markdown.rs` keep compiling unchanged during the
/// rest of the move. The legacy `embedded_volume` dispatch is wired
/// in [`init_markdown`].
#[cfg(feature = "markdown")]
pub use mehen_markdown as markdown;

/// Register the embedded-code dispatch callback the moved
/// [`mehen_markdown::analyze_markdown`] uses to fold fenced source
/// snippets into Markdown metrics.
///
/// Called once at process startup from `crates/mehen-cli/src/main.rs`
/// and the `mehen` library's test setup; idempotent — subsequent
/// calls are silent no-ops by design of the underlying `OnceLock`.
#[cfg(feature = "markdown")]
pub fn init_markdown() {
    use mehen_markdown::{EmbeddedFenceMetrics, FenceLanguage};

    fn legacy_dispatch(lang: FenceLanguage, body: String) -> Option<EmbeddedFenceMetrics> {
        let bytes = body.into_bytes();
        let path = synthetic_path(lang);
        let legacy_lang = legacy_lang_for(lang);
        let space = crate::langs::get_function_spaces(
            &legacy_lang,
            bytes,
            std::path::Path::new(&path),
            None,
        )?;
        Some(EmbeddedFenceMetrics {
            volume: space.metrics.halstead.volume(),
            cognitive_sum: space.metrics.cognitive.cognitive_sum(),
            sloc: space.metrics.loc.sloc(),
        })
    }

    fn synthetic_path(lang: FenceLanguage) -> std::path::PathBuf {
        let name = match lang {
            FenceLanguage::Rust => "fence.rs",
            FenceLanguage::Python => "fence.py",
            FenceLanguage::Typescript => "fence.ts",
            FenceLanguage::Tsx => "fence.tsx",
            FenceLanguage::Go => "fence.go",
            FenceLanguage::Ruby => "fence.rb",
            FenceLanguage::Kotlin => "fence.kt",
            FenceLanguage::Powershell => "fence.ps1",
            FenceLanguage::C => "fence.c",
            FenceLanguage::Php => "fence.php",
        };
        std::path::PathBuf::from(name)
    }

    fn legacy_lang_for(lang: FenceLanguage) -> crate::langs::LANG {
        use crate::langs::LANG;
        match lang {
            FenceLanguage::Rust => LANG::Rust,
            FenceLanguage::Python => LANG::Python,
            FenceLanguage::Typescript => LANG::Typescript,
            FenceLanguage::Tsx => LANG::Tsx,
            FenceLanguage::Go => LANG::Go,
            FenceLanguage::Ruby => LANG::Ruby,
            FenceLanguage::Kotlin => LANG::Kotlin,
            FenceLanguage::Powershell => LANG::Powershell,
            FenceLanguage::C => LANG::C,
            FenceLanguage::Php => LANG::Php,
        }
    }

    mehen_markdown::set_legacy_dispatch(legacy_dispatch);
}
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
