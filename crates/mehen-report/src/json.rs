use mehen_core::{DiffReport, MetricsReport};

use crate::metrics_json::MetricsFamilies;

/// Render a `MetricsReport` as JSON. Pretty-printed when `pretty=true`.
///
/// Emits the documented per-family shape (`metrics: { cyclomatic, … }`,
/// rewrite plan §9.1) pivoted from the flat keys the analyzer publishes
/// into `root.metrics`. The full `MetricSpace` tree remains available
/// under `root` so consumers that reference individual aggregator keys
/// (e.g. `cyclomatic.max`) keep working alongside the published
/// schema.
pub fn render_metrics_json(report: &MetricsReport, pretty: bool) -> serde_json::Result<String> {
    let mut value = serde_json::to_value(report)?;
    let families = serde_json::to_value(MetricsFamilies::from_metrics(&report.root.metrics))?;
    if let serde_json::Value::Object(map) = &mut value {
        map.insert("metrics".to_string(), families);
    }
    if pretty {
        serde_json::to_string_pretty(&value)
    } else {
        serde_json::to_string(&value)
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
