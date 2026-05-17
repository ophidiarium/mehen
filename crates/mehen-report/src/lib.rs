//! `mehen-report` — rendering and serialization.
//!
//! Phase 1 scope: render shapes that downstream callers can rely on while
//! the orchestrators are filled in. Per the rewrite plan §8.1 the pre-1.0
//! Markdown documentation diff renderer lives here under
//! `github_markdown_docs` (gated by the `docs-diff` feature so consumers
//! that don't need the Markdown analyzer don't pay for it).

#![forbid(unsafe_code)]

#[cfg(feature = "docs-diff")]
pub mod github_markdown_docs;
mod json;
mod markdown;

pub use json::{render_diff_json, render_metrics_json};
pub use markdown::{render_diff_github_markdown, render_metrics_markdown};
