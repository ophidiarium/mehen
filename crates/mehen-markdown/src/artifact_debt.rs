//! Artifact Debt Score per §19.
//!
//! ```text
//! ArtifactDebtScore = clamp01(
//!     0.25 * sat(unlabelled_code_fences / max(1, code_fences); 0.05, 0.50)
//!   + 0.20 * sat(artifact_parse_errors / max(1, artifacts); 0.00, 0.20)
//!   + 0.15 * sat(oversized_artifacts / max(1, artifacts); 0.05, 0.30)
//!   + 0.15 * sat(unexplained_artifacts / max(1, artifacts); 0.10, 0.60)
//!   + 0.15 * sat(raw_html_or_mdx_lines / max(1, DLOC); 0.05, 0.25)
//!   + 0.10 * sat(external_artifact_links / max(1, artifacts); 0.10, 0.60)
//! )
//! ```

use crate::mathops::{clamp01, sat};
use crate::types::{ArtifactKind, ArtifactRecord, LinkClass, LinkRecord, LocFamily};

/// Inputs the score needs beyond what's embedded in `ArtifactRecord`.
pub(crate) struct DebtInputs<'a> {
    pub(crate) artifacts: &'a [ArtifactRecord],
    pub(crate) links: &'a [LinkRecord],
    pub(crate) loc: &'a LocFamily,
    pub(crate) raw_html_or_mdx_lines: u64,
    pub(crate) diagram_parse_errors: u64,
}

pub(crate) fn artifact_debt_score(inputs: &DebtInputs<'_>) -> f64 {
    let artifacts = inputs.artifacts;
    // §19 is a per-artifact metric. A prose-only document with no
    // artifacts has zero artifact debt by definition — counting stray
    // prose external links as "artifact debt" creates false positives
    // on snippet-free markdown (Codex P1 on PR #84). The only §19
    // component that is not strictly artifact-bound is
    // `raw_html_or_mdx_lines / DLOC`; we keep that contribution so
    // raw-HTML-heavy prose still produces debt, but every per-artifact
    // ratio resolves to 0 when `artifacts.is_empty()`.
    if artifacts.is_empty() {
        let raw_html_lines = inputs.raw_html_or_mdx_lines as f64;
        let dloc = inputs.loc.dloc.max(1) as f64;
        return clamp01(0.15 * sat(raw_html_lines / dloc, 0.05, 0.25));
    }

    let total_artifacts = artifacts.len() as f64;

    let code_fences = artifacts
        .iter()
        .filter(|a| a.kind == ArtifactKind::Code)
        .count() as f64;
    let unlabelled = artifacts
        .iter()
        .filter(|a| a.kind == ArtifactKind::Code && a.language_tag.is_none())
        .count() as f64;

    let oversized = artifacts.iter().filter(|a| a.oversized).count() as f64;
    let unexplained = artifacts.iter().filter(|a| !a.has_explanation).count() as f64;

    // §19: artifact_parse_errors currently = diagram parse errors. If
    // Phase B wires code-fence parser errors these would be added here.
    let parse_errors = inputs.diagram_parse_errors as f64;
    let raw_html_lines = inputs.raw_html_or_mdx_lines as f64;
    let dloc = inputs.loc.dloc.max(1) as f64;

    // External artifact links = links pointed at from inside an artifact.
    // As a conservative approximation we use the count of External /
    // ExternalVendor / Scholarly / IssuePR link destinations document-wide.
    // The proper "inside artifact" restriction needs per-link artifact
    // attribution which is Phase D territory.
    let external_artifact_links = inputs
        .links
        .iter()
        .filter(|l| {
            matches!(
                l.class,
                LinkClass::External
                    | LinkClass::ExternalVendor
                    | LinkClass::Scholarly
                    | LinkClass::IssuePr
            )
        })
        .count() as f64;

    let score = 0.25 * sat(unlabelled / code_fences.max(1.0), 0.05, 0.50)
        + 0.20 * sat(parse_errors / total_artifacts, 0.00, 0.20)
        + 0.15 * sat(oversized / total_artifacts, 0.05, 0.30)
        + 0.15 * sat(unexplained / total_artifacts, 0.10, 0.60)
        + 0.15 * sat(raw_html_lines / dloc, 0.05, 0.25)
        + 0.10 * sat(external_artifact_links / total_artifacts, 0.10, 0.60);

    clamp01(score)
}
