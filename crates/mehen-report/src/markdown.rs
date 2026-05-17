use mehen_core::{DiffReport, MetricsReport};

/// Phase 1 placeholder for the single-file Markdown renderer. Phase 5 wires
/// real layout — heading, per-space tables, diagnostics callout — once the
/// language analyzers fill in the metric set.
pub fn render_metrics_markdown(report: &MetricsReport) -> String {
    format!(
        "# {}\n\n- language: `{}`\n- backend: `{}`\n",
        report.path,
        report.language,
        report.analysis_backend.label()
    )
}

/// Phase 1 placeholder for the GitHub Markdown diff comment. Phase 4 ports
/// the existing pre-1.0 documentation diff renderer here. The output must
/// be byte-identical after stable timestamp/version redaction (parity
/// contract — rewrite plan §12.3.1).
pub fn render_diff_github_markdown(report: &DiffReport) -> String {
    format!(
        "<!-- mehen-schema:1 -->\n<!-- mehen-source -->\n\n_base: `{}`_  _head: `{}`_\n",
        report.base, report.head
    )
}
