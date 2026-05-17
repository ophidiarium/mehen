//! Pre-1.0 `mehen` library — transitional thin re-export wrapper.
//!
//! Per rewrite plan §8, every module previously living here has been
//! physically relocated:
//!
//! - `markdown` → `crates/mehen-markdown/`
//! - `git` → `crates/mehen-git/`
//! - `ci`, `diff`, `top_offenders`, `concurrent_files`,
//!   `metric_selector`, `tools`, `formats`, `langs`, `languages`,
//!   `macros`, `traits`, `parser`, `node`, `checker`, `getter`,
//!   `spaces`, `alterator`, `preproc`, `rust_metric_helpers`,
//!   `metrics` → `crates/mehen-engine/src/legacy/`
//! - `diff_markdown` → `crates/mehen-report/src/github_markdown_docs.rs`
//!
//! The re-exports below preserve the pre-1.0 public path
//! (`crate::diff::*`, `crate::langs::LANG`, `crate::metrics::*`, etc.)
//! so existing tests and the `crates/mehen-cli/src/args.rs` flatten
//! continue working while each module is gradually rehomed into its
//! plan-defined destination crate.

#![allow(clippy::upper_case_acronyms)]

pub use mehen_engine::ci;
pub use mehen_engine::legacy::{
    alterator, checker, concurrent_files, diff, formats, getter, langs, languages, macros,
    metric_selector, metrics, mk_globset, node, parser, preproc, rust_metric_helpers, spaces,
    tools, top_offenders, traits,
};
pub use mehen_git as git;

#[cfg(feature = "markdown")]
pub use mehen_markdown as markdown;

#[cfg(feature = "markdown")]
pub use mehen_engine::legacy::diff_markdown;

/// Register the embedded-code dispatch callback the moved
/// [`mehen_markdown::analyze_markdown`] uses to fold fenced source
/// snippets into Markdown metrics.
#[cfg(feature = "markdown")]
pub fn init_markdown() {
    use mehen_markdown::{EmbeddedFenceMetrics, FenceLanguage};

    fn legacy_dispatch(lang: FenceLanguage, body: String) -> Option<EmbeddedFenceMetrics> {
        let bytes = body.into_bytes();
        let path = synthetic_path(lang);
        let legacy_lang = legacy_lang_for(lang);
        let space = mehen_engine::legacy::langs::get_function_spaces(
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

    fn legacy_lang_for(lang: FenceLanguage) -> mehen_engine::legacy::langs::LANG {
        use mehen_engine::legacy::langs::LANG;
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
