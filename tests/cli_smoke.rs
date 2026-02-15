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
