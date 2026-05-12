//! Dedicated Markdown document-metrics pipeline.
//!
//! The Markdown analyzer runs outside the generic
//! `spaces::metrics()` source-code pipeline because prose / tables /
//! diagrams / fences do not map onto function spaces. Its inputs are a raw
//! source buffer and path; its output is a serializable
//! [`types::MarkdownMetrics`] record matching §23's exported schema (Phase A
//! LOC/size surface plus Phase-E language-aware prose metrics).
//!
//! The high-level entry point is [`analyzer::analyze_markdown`]. It is
//! invoked from `main.rs` when the detected language is `LANG::Markdown`;
//! when the `markdown` Cargo feature is disabled this entire module
//! disappears and the routing falls through, so `default-features = false`
//! still produces a functional binary.

pub(crate) mod analyzer;
pub(crate) mod ecu;
pub(crate) mod loc;
pub(crate) mod prose;
pub(crate) mod sections;
pub(crate) mod types;
pub(crate) mod words;

pub(crate) use analyzer::analyze_markdown;

#[cfg(test)]
mod tests;
