//! `mehen-report` — rendering and serialization.
//!
//! Phase 1 scope: render shapes that downstream callers can rely on while
//! the orchestrators are filled in. Phase 4 ports the existing pre-1.0
//! Markdown documentation diff renderer here from `src/diff_markdown.rs`.

#![forbid(unsafe_code)]

#[cfg(feature = "docs-diff")]
pub mod github_markdown_docs;
mod json;
mod markdown;

pub use json::{render_diff_json, render_metrics_json};
pub use markdown::{render_diff_github_markdown, render_metrics_markdown};
