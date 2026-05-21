// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Konstantin Vyatkin <tino@vtkn.io>

//! Evidence Coverage Score per §16.
//!
//! §16.2 Per-section formula:
//!
//! ```text
//! anchor_density_s = evidence_anchors_s / max(1, W_s / 250)
//! section_evidence_s = sat(anchor_density_s; 0.2, 1.5)
//! ```
//!
//! §16.3 Aggregate: `0.5 * mean(section_evidence_s) + 0.5 * p25(section_evidence_s)`.
//!
//! The actual counting of per-section evidence anchors lives alongside the
//! §15 grounding pipeline in `grounding.rs`. This module re-exports the
//! values so the §23 schema can cleanly map `grounding.evidence_coverage_score`
//! and Phase-D consumers (filler, RCI) have a narrow dependency surface.
//!
//! §16.1 evidence anchors we count:
//!
//! - Resolved relative link.
//! - External link.
//! - Internal link to a non-trivial section (treated as `resolved Some(true)`).
//! - Labelled code fence.
//! - Table with header.
//! - Parseable diagram with caption.
//! - Image with alt/caption.
//! - Math block with nearby explanation.
//! - Issue/PR/Scholarly reference link.
//! - Path-like token resolved to repo (rolls into §15 counts).
//!
//! The implementation is in `grounding::compute_per_section_anchors` and
//! the aggregate is produced inline by `grounding::analyze_grounding`.
//! This module is intentionally empty: it exists to group the §16 design
//! notes alongside the source tree so future Phase-D consumers know
//! where the anchor-counting logic lives without adding a second cache
//! of the same values.
