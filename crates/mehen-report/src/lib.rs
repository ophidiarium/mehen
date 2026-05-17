//! `mehen-report` — rendering and serialization.
//!
//! Phase 1 scope: render shapes that downstream callers can rely on while
//! the orchestrators are filled in. The pre-1.0 Markdown documentation
//! diff renderer lives in `mehen-engine::legacy::diff_markdown` for the
//! duration of the v1 transition (it is a peer of the legacy `diff`
//! orchestrator and breaks the otherwise-cyclic dep on `mehen-engine`).

#![forbid(unsafe_code)]

mod json;
mod markdown;

pub use json::{render_diff_json, render_metrics_json};
pub use markdown::{render_diff_github_markdown, render_metrics_markdown};
