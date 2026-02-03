//! Integration tests for TR-300

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("tr300").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("TR-300"));
}

#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("tr300").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("tr300"));
}

#[test]
fn test_default_report() {
    let mut cmd = Command::cargo_bin("tr300").unwrap();
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("QUBETX DEVELOPER TOOLS"))
        .stdout(predicate::str::contains("TR-300 MACHINE REPORT"));
}

#[test]
fn test_ascii_flag() {
    let mut cmd = Command::cargo_bin("tr300").unwrap();
    cmd.arg("--ascii")
        .assert()
        .success()
        // ASCII mode should not have Unicode box chars
        .stdout(predicate::str::contains("+"));
}

#[test]
fn test_json_flag() {
    let mut cmd = Command::cargo_bin("tr300").unwrap();
    cmd.arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"os\":"))
        .stdout(predicate::str::contains("\"cpu\":"))
        .stdout(predicate::str::contains("\"memory\":"));
}

#[test]
fn test_custom_title() {
    let mut cmd = Command::cargo_bin("tr300").unwrap();
    cmd.args(["--title", "CUSTOM TITLE"])
        .assert()
        .success()
        .stdout(predicate::str::contains("CUSTOM TITLE"));
}

#[test]
fn test_no_color_flag() {
    let mut cmd = Command::cargo_bin("tr300").unwrap();
    cmd.arg("--no-color")
        .assert()
        .success();
}

#[test]
fn test_output_contains_expected_fields() {
    let mut cmd = Command::cargo_bin("tr300").unwrap();
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("OS"))
        .stdout(predicate::str::contains("KERNEL"))
        .stdout(predicate::str::contains("HOSTNAME"))
        .stdout(predicate::str::contains("MACHINE IP"))
        .stdout(predicate::str::contains("PROCESSOR"))
        .stdout(predicate::str::contains("CORES"))
        .stdout(predicate::str::contains("CPU FREQ"))
        .stdout(predicate::str::contains("VOLUME"))
        .stdout(predicate::str::contains("DISK USAGE"))
        .stdout(predicate::str::contains("MEMORY"))
        .stdout(predicate::str::contains("UPTIME"));
}
