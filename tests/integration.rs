//! Integration tests for TR-300
//
// These tests invoke the compiled `tr300` and `report` binaries via the
// `CARGO_BIN_EXE_*` environment variables that cargo sets automatically.
// We avoid `assert_cmd::Command::cargo_bin` because it was deprecated in
// assert_cmd 2.x as incompatible with custom build dirs.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

fn tr300() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tr300"))
}

fn report() -> Command {
    Command::new(env!("CARGO_BIN_EXE_report"))
}

fn help_stdout(mut command: Command, flag: &str) -> String {
    let assertion = command.arg(flag).assert().success();
    String::from_utf8(assertion.get_output().stdout.clone())
        .expect("command help should be valid UTF-8")
}

#[test]
fn tr300_short_and_long_help_cover_the_complete_visible_surface() {
    for flag in ["-h", "--help"] {
        let help = help_stdout(tr300(), flag);
        for expected in [
            "[ACTION]",
            "update",
            "install",
            "uninstall",
            "--ascii",
            "--json",
            "--install",
            "--uninstall",
            "--update",
            "--title",
            "--no-color",
            "--fast",
            "--full",
            "--no-elevation-hint",
            "--report",
            "--save",
            "--help",
            "--version",
        ] {
            assert!(
                help.contains(expected),
                "tr300 {flag} omitted visible surface {expected:?}\n{help}"
            );
        }
    }
}

#[test]
fn report_help_and_version_exactly_match_tr300() {
    for flag in ["-h", "--help", "-V", "--version"] {
        assert_eq!(help_stdout(report(), flag), help_stdout(tr300(), flag));
    }
}

#[test]
fn report_forwards_all_actions_and_preserves_exit_status() {
    let directory = tempfile::tempdir().unwrap();
    let source = std::path::Path::new(env!("CARGO_BIN_EXE_report"));
    let launcher = directory.path().join(source.file_name().unwrap());
    std::fs::copy(source, &launcher).unwrap();
    let stub_source = directory.path().join("stub.rs");
    std::fs::write(
        &stub_source,
        r#"
fn main() {
    if let Ok(directory) = std::env::var("TR300_ALIAS_TEST_WAIT") {
        let directory = std::path::Path::new(&directory);
        std::fs::write(directory.join("ready"), b"ready").unwrap();
        for _ in 0..1000 {
            if directory.join("release").exists() { break; }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    for arg in std::env::args_os().skip(1) {
        println!("{}", arg.to_string_lossy());
    }
    std::process::exit(37);
}
"#,
    )
    .unwrap();
    let sibling = directory.path().join(
        std::path::Path::new(env!("CARGO_BIN_EXE_tr300"))
            .file_name()
            .unwrap(),
    );
    let compile = std::process::Command::new("rustc")
        .arg(&stub_source)
        .arg("-o")
        .arg(&sibling)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    for action in [
        "install",
        "update",
        "uninstall",
        "--install",
        "--update",
        "--uninstall",
        "migrate-cleanup",
        "update-worker",
    ] {
        Command::new(&launcher)
            .args([action, "--title", "Lab & %PATH% '$()'"])
            .assert()
            .code(37)
            .stdout(format!("{action}\n--title\nLab & %PATH% '$()'\n"));
    }
    #[cfg(windows)]
    {
        // The real launcher remains loaded while its child waits. Staging the
        // loaded image must free the original filename for the new package.
        let mut child = std::process::Command::new(&launcher)
            .arg("update")
            .env("TR300_ALIAS_TEST_WAIT", directory.path())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let started = std::time::Instant::now();
        while !directory.path().join("ready").exists() && started.elapsed().as_secs() < 5 {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(directory.path().join("ready").exists());
        std::fs::rename(
            &launcher,
            directory.path().join(".tr300-report-update-backup-1-2.exe"),
        )
        .unwrap();
        std::fs::copy(source, &launcher).unwrap();
        std::fs::write(directory.path().join("release"), b"release").unwrap();
        assert_eq!(child.wait().unwrap().code(), Some(37));
    }
}

#[test]
fn report_fast_json_delegates_to_the_adjacent_tr300() {
    let output = report()
        .args(["--fast", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value =
        serde_json::from_slice(&output).expect("report --fast --json output should parse");
    assert_eq!(value["collection_mode"], "fast");
}

#[test]
fn isolated_report_never_falls_back_to_tr300_on_path() {
    let directory = tempfile::tempdir().unwrap();
    let source = std::path::Path::new(env!("CARGO_BIN_EXE_report"));
    let launcher = directory.path().join(source.file_name().unwrap());
    std::fs::copy(source, &launcher).unwrap();
    let tr300_directory = std::path::Path::new(env!("CARGO_BIN_EXE_tr300"))
        .parent()
        .unwrap();

    Command::new(&launcher)
        .env("PATH", tr300_directory)
        .args(["--fast", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "adjacent TR-300 executable is unavailable",
        ));
}

#[test]
fn report_forwards_title_as_one_literal_argument() {
    let title = "Lab & %PATH% '$()'";
    report()
        .args(["--fast", "--ascii", "--title", title])
        .assert()
        .success()
        .stdout(predicate::str::contains(title));
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
fn test_default_report_has_no_markdown_side_effect_message() {
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
        .stdout(predicate::str::contains("Possible values:"))
        .stdout(predicate::str::contains("- update"))
        .stdout(predicate::str::contains("- install"))
        .stdout(predicate::str::contains(
            "- uninstall: Open the uninstall menu for profile-only or Complete removal",
        ));
}

#[test]
fn generated_man_pages_match_their_command_boundaries() {
    let tr300_man = include_str!("../man/tr300.1");
    let report_man = include_str!("../man/report.1");

    assert!(tr300_man.contains(".TH tr300 1"));
    assert!(tr300_man.contains("\\-\\-install"));
    assert!(tr300_man.contains("\\-\\-update"));
    assert!(tr300_man.contains("\\-\\-uninstall"));
    assert!(tr300_man.contains("aliases: \\-s, \\-\\-save"));
    assert!(tr300_man.contains(".SH SEE ALSO\nreport(1)"));

    assert!(report_man.contains(".TH report 1"));
    for expected in ["\\-\\-fast", "\\-\\-full", "\\-\\-json", "\\-\\-report"] {
        assert!(report_man.contains(expected));
    }
    assert!(report_man.contains("aliases: \\-s, \\-\\-save"));
    assert!(report_man.contains(".SH SEE ALSO\ntr300(1)"));
    for expected in ["\\-\\-install", "\\-\\-update", "\\-\\-uninstall"] {
        assert!(report_man.contains(expected));
    }
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

// --- v4.3.0 additions ---

#[test]
fn test_json_includes_thermal_keys() {
    // Thermal keys are additive schema-v1 members: always present, null when
    // no trusted sensor answered (most CI runners), a number when one did.
    let output = tr300()
        .arg("--json")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).expect("--json output should parse");
    let cpu = value
        .get("cpu")
        .and_then(Value::as_object)
        .expect("cpu should be a JSON object");
    for key in ["temperature_c", "gpu_temperature_c"] {
        let reading = cpu
            .get(key)
            .unwrap_or_else(|| panic!("cpu.{key} should always be present"));
        assert!(
            reading.is_null() || reading.is_number(),
            "cpu.{key} should be null or a number, got {reading}"
        );
    }
    #[cfg(windows)]
    assert_eq!(
        cpu.get("temperature_c"),
        Some(&Value::Null),
        "Windows CPU temperature must remain explicitly null"
    );
}

#[test]
fn test_full_flag_is_accepted_and_conflicts_with_fast() {
    tr300().arg("--full").assert().success();
    tr300().args(["--full", "--fast"]).assert().failure();
}

#[test]
fn test_ascii_mode_never_emits_degree_sign() {
    // Live CLI smoke for the final output contract. Deterministic fixture tests
    // in report.rs exercise populated thermal rows on sensorless CI runners.
    tr300()
        .args(["--ascii", "--no-elevation-hint"])
        .assert()
        .success()
        .stdout(predicate::str::contains("°").not());
}
