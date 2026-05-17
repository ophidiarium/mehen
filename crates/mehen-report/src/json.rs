use mehen_engine::{DiffReport, MetricsReport};

/// Render a `MetricsReport` as JSON. Pretty-printed when `pretty=true`.
pub fn render_metrics_json(report: &MetricsReport, pretty: bool) -> serde_json::Result<String> {
    if pretty {
        serde_json::to_string_pretty(report)
    } else {
        serde_json::to_string(report)
    }
}

/// Render a `DiffReport` as JSON. Pretty-printed when `pretty=true`.
pub fn render_diff_json(report: &DiffReport, pretty: bool) -> serde_json::Result<String> {
    if pretty {
        serde_json::to_string_pretty(report)
    } else {
        serde_json::to_string(report)
    }
}
