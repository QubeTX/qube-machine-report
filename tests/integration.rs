//! Integration tests for TR-300
//
// These tests invoke the compiled `tr300` binary via the `CARGO_BIN_EXE_tr300`
// environment variable that cargo sets automatically for integration tests.
// We avoid `assert_cmd::Command::cargo_bin` because it was deprecated in
// assert_cmd 2.x as incompatible with custom build dirs.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

fn tr300() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tr300"));
    command.arg("--no-save");
    command
}

#[test]
fn test_help_flag() {
    tr300()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("TR-300"));
}

#[test]
fn test_version_flag() {
    tr300()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("tr300"));
}

#[test]
fn test_no_save_suppresses_markdown_side_effect_message() {
    tr300()
        .assert()
        .success()
        .stderr(predicate::str::contains("Report saved:").not());
}

#[test]
fn test_default_report() {
    tr300()
        .assert()
        .success()
        .stdout(predicate::str::contains("QUBETX DEVELOPER TOOLS"))
        .stdout(predicate::str::contains("TR-300 MACHINE REPORT"));
}

#[test]
fn test_ascii_flag() {
    tr300()
        .arg("--ascii")
        .assert()
        .success()
        // ASCII mode should not have Unicode box chars
        .stdout(predicate::str::contains("+"));
}

#[test]
fn test_json_flag() {
    tr300()
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"os\":"))
        .stdout(predicate::str::contains("\"cpu\":"))
        .stdout(predicate::str::contains("\"memory\":"));
}

#[test]
fn test_json_output_parses() {
    let output = tr300()
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).expect("--json output should parse");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["collection_mode"], "full");
    assert!(value["os"].is_object());
    assert!(value["system"].is_object());
    assert_eq!(
        value["network"]["machine_ip_scope"].is_null(),
        value["network"]["machine_ip"].is_null()
    );
    assert_eq!(value["cpu"]["load_unit"], "percent_of_logical_cpu_capacity");
    assert_eq!(value["disk"]["used_definition"], "allocated_bytes");
    assert!(value["memory"]["available_bytes"].is_u64());
}

#[test]
fn test_custom_title() {
    tr300()
        .args(["--title", "CUSTOM TITLE"])
        .assert()
        .success()
        .stdout(predicate::str::contains("CUSTOM TITLE"));
}

#[test]
fn test_no_color_flag() {
    tr300().arg("--no-color").assert().success();
}

#[test]
fn test_output_contains_expected_fields() {
    tr300()
        .assert()
        .success()
        .stdout(predicate::str::contains("OS"))
        .stdout(predicate::str::contains("KERNEL"))
        .stdout(predicate::str::contains("HOSTNAME"))
        .stdout(predicate::str::contains("SSH CLIENT"))
        .stdout(predicate::str::contains("PROCESSOR"))
        .stdout(predicate::str::contains("CORES"))
        .stdout(predicate::str::contains("VOLUME"))
        .stdout(predicate::str::contains("DISK USAGE"))
        .stdout(predicate::str::contains("MEMORY"))
        .stdout(predicate::str::contains("AVAILABLE"))
        .stdout(predicate::str::contains("UPTIME"));
}

// --- v3.10.0 additions ---

#[test]
fn test_json_includes_schema_version() {
    tr300()
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"schema_version\": 1"));
}

#[test]
fn test_json_includes_elevation_keys() {
    tr300()
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"elevated\":"))
        .stdout(predicate::str::contains("\"elevation_unlocks_more\":"));
}

#[test]
fn test_no_elevation_hint_flag_accepted() {
    // Should not error and should not contain the hint text in output.
    tr300()
        .args(["--no-elevation-hint", "--ascii"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run with sudo").not())
        .stdout(predicate::str::contains("Run as Administrator").not());
}

#[test]
fn test_fast_mode_no_elevation_footer() {
    // --fast must never emit the elevation footer (auto-run safety).
    tr300()
        .args(["--fast", "--ascii"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run with sudo").not())
        .stdout(predicate::str::contains("Run as Administrator").not());
}

#[test]
fn test_fast_mode_omits_slow_conditional_rows() {
    tr300()
        .args(["--fast", "--ascii"])
        .assert()
        .success()
        .stdout(predicate::str::contains("ZFS HEALTH").not())
        .stdout(predicate::str::contains("RAM SLOTS").not());
}

#[test]
fn test_ascii_table_lines_keep_fixed_width() {
    let output = tr300()
        .args(["--ascii", "--no-elevation-hint"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output = String::from_utf8(output).expect("stdout should be utf-8");
    for line in output.lines().filter(|line| line.starts_with(['+', '|'])) {
        assert_eq!(
            line.chars().count(),
            51,
            "line has unexpected width: {line}"
        );
    }
}

#[test]
fn test_help_documents_positional_actions() {
    tr300()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("[ACTION]"))
        .stdout(predicate::str::contains(
            "[possible values: update, install, uninstall]",
        ));
}

// --- v3.11.0 additions ---

#[test]
fn test_json_includes_encryption_key() {
    // The `encryption` key is always present in JSON (nullable). On Windows
    // hosts where BitLocker is readable it'll be a string; otherwise null.
    tr300()
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"encryption\":"));
}

// --- v3.12.0 additions ---

#[test]
fn test_json_includes_session_uptime_seconds_key() {
    // `os.session_uptime_seconds` remains present and nullable for schema-v1
    // compatibility. No platform currently fabricates a second uptime value.
    tr300()
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"session_uptime_seconds\":"));
}
