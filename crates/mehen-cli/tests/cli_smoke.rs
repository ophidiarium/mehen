//! Smoke tests for the 1.0 `mehen` CLI.
//!
//! Replaces the pre-1.0 `tests/cli_smoke.rs`. The pre-1.0 commands
//! `--dump`, `--find`, `--count`, `--function`, root-level `-m -p` are
//! dropped per the rewrite plan §2.1; the new surface is `metrics`,
//! `diff`, and `top-offenders`.
use std::io::Write;
use std::process::Command;

fn write_python(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let mut f = std::fs::File::create(&path).expect("create py file");
    f.write_all(body.as_bytes()).expect("write py file");
    path
}

#[test]
fn version_prints_name_and_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_mehen-next"))
        .arg("--version")
        .output()
        .expect("failed to run mehen --version");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("mehen"));
}

#[test]
fn help_succeeds() {
    let output = Command::new(env!("CARGO_BIN_EXE_mehen-next"))
        .arg("--help")
        .output()
        .expect("failed to run mehen --help");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    assert!(stdout.contains("metrics"), "expected `metrics` in help");
    assert!(stdout.contains("diff"), "expected `diff` in help");
    assert!(
        stdout.contains("top-offenders"),
        "expected `top-offenders` in help"
    );
}

#[test]
fn metrics_emits_json_for_python_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_python(
        dir.path(),
        "sample.py",
        "def foo(x):\n    if x:\n        return 1\n    return 2\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_mehen-next"))
        .args(["metrics", path.to_str().unwrap(), "--pretty"])
        .output()
        .expect("failed to run mehen metrics");
    assert!(
        output.status.success(),
        "mehen metrics failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout utf8");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("metrics output must be valid JSON");
    assert_eq!(parsed["language"].as_str(), Some("python"));
    assert_eq!(parsed["analysis_backend"].as_str(), Some("tree-sitter"));
    let spaces = parsed["root"]["spaces"]
        .as_array()
        .expect("root must have spaces array");
    assert!(!spaces.is_empty(), "expected one function space");
    assert_eq!(spaces[0]["kind"].as_str(), Some("function"));
    assert_eq!(spaces[0]["name"].as_str(), Some("foo"));
}

#[test]
fn metrics_rejects_unknown_language() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = write_python(dir.path(), "sample.unknown", "def f(): pass\n");

    let output = Command::new(env!("CARGO_BIN_EXE_mehen-next"))
        .args(["metrics", path.to_str().unwrap()])
        .output()
        .expect("failed to run mehen metrics");
    assert!(
        !output.status.success(),
        "unknown language must fail; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn top_offenders_requires_paths() {
    let output = Command::new(env!("CARGO_BIN_EXE_mehen-next"))
        .args(["top-offenders"])
        .output()
        .expect("failed to run mehen top-offenders");
    assert!(
        !output.status.success(),
        "top-offenders without paths must fail"
    );
}
