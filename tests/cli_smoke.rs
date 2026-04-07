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
