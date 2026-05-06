use std::process::Command;

#[test]
fn help_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_mehen"))
        .arg("--help")
        .output()
        .expect("failed to run mehen --help");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout was not UTF-8");
    assert!(stdout.contains("Analyze source code."));
}

#[test]
fn metrics_succeeds_for_rust_file() {
    let output = Command::new(env!("CARGO_BIN_EXE_mehen"))
        .args(["--metrics", "-p", "src/main.rs"])
        .output()
        .expect("failed to run mehen --metrics");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout was not UTF-8");
    assert!(!stdout.trim().is_empty());
}

fn run_metrics_format(format: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_mehen"))
        .args(["--metrics", "-O", format, "-p", "src/main.rs"])
        .output()
        .unwrap_or_else(|e| panic!("failed to run mehen -O {format}: {e}"));

    assert!(output.status.success(), "mehen -O {format} failed");
    String::from_utf8(output.stdout).expect("stdout was not UTF-8")
}

#[test]
fn metrics_json_output_is_valid() {
    let stdout = run_metrics_format("json");
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON output: {e}\n---\n{stdout}"));

    assert!(parsed.is_array() || parsed.is_object());
}

#[test]
fn metrics_toml_output_is_valid() {
    let stdout = run_metrics_format("toml");
    let parsed: toml::Value = toml::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid TOML output: {e}\n---\n{stdout}"));

    assert!(parsed.is_table());
}

#[test]
fn metrics_yaml_output_is_valid() {
    let stdout = run_metrics_format("yaml");
    let parsed: serde_norway::Value = serde_norway::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid YAML output: {e}\n---\n{stdout}"));

    assert!(parsed.is_mapping() || parsed.is_sequence());
}

#[test]
fn metrics_file_output_writes_valid_json() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let output = Command::new(env!("CARGO_BIN_EXE_mehen"))
        .args([
            "--metrics",
            "-O",
            "json",
            "-o",
            dir.path().to_str().unwrap(),
            "-p",
            "src/main.rs",
        ])
        .output()
        .expect("failed to run mehen with -o");

    assert!(output.status.success(), "mehen -O json -o dir failed");

    let json_path = dir.path().join("src/main.rs.json");
    assert!(json_path.exists(), "expected output file {json_path:?}");

    let content = std::fs::read_to_string(&json_path).expect("failed to read output file");
    let parsed: serde_json::Value = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("invalid JSON in file output: {e}\n---\n{content}"));

    assert!(parsed.is_array() || parsed.is_object());
}

#[test]
fn top_offenders_json_ranks_files_by_metric() {
    // Run against the mehen source tree itself: guaranteed to contain several
    // Rust files with measurable LOC, so ranking will produce a non-empty list.
    let output = Command::new(env!("CARGO_BIN_EXE_mehen"))
        .args([
            "top-offenders",
            "--metric",
            "loc.lloc",
            "--metric",
            "cognitive",
            "--max-results",
            "3",
            "--output-format",
            "json",
            "src",
        ])
        .output()
        .expect("failed to run mehen top-offenders");

    assert!(
        output.status.success(),
        "mehen top-offenders failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout was not UTF-8");
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid JSON output: {e}\n---\n{stdout}"));

    let arr = parsed
        .as_array()
        .expect("top-offenders JSON must be an array");
    assert!(!arr.is_empty(), "expected at least one offender from src/");
    assert!(
        arr.len() <= 3,
        "expected at most 3 offenders, got {}",
        arr.len()
    );

    // Each entry carries `path` and a non-empty `metrics` array whose names
    // match the `--metric` order the CLI was given.
    for entry in arr {
        assert!(entry.get("path").and_then(|p| p.as_str()).is_some());
        let metrics = entry
            .get("metrics")
            .and_then(|m| m.as_array())
            .expect("each offender has a metrics array");
        assert_eq!(metrics.len(), 2);
        assert_eq!(
            metrics[0].get("name").and_then(|n| n.as_str()),
            Some("loc.lloc")
        );
        assert_eq!(
            metrics[1].get("name").and_then(|n| n.as_str()),
            Some("cognitive")
        );
    }

    // Primary metric is lower-is-better: LLOC must be non-increasing down the list.
    let lloc_values: Vec<f64> = arr
        .iter()
        .map(|e| {
            e["metrics"][0]["value"]
                .as_f64()
                .expect("loc.lloc value must be numeric")
        })
        .collect();
    for pair in lloc_values.windows(2) {
        assert!(
            pair[0] >= pair[1],
            "ranking must be non-increasing on primary metric: {lloc_values:?}"
        );
    }
}

#[test]
fn top_offenders_requires_metric() {
    // No --metric provided: clap should reject the command.
    let output = Command::new(env!("CARGO_BIN_EXE_mehen"))
        .args(["top-offenders", "src"])
        .output()
        .expect("failed to run mehen top-offenders");

    assert!(
        !output.status.success(),
        "top-offenders without --metric should fail"
    );
}
