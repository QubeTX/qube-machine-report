// CLI argument definitions for TR-300
//
// Extracted to a separate module so both `main.rs` and `build.rs`
// can access the `Cli` struct (via `include!` in build.rs).

use clap::{Parser, ValueEnum};

/// Positional action commands.
///
/// These mirror the legacy action flags (`--update`, `--install`,
/// `--uninstall`) so users can run `tr300 update` or the installed
/// `report update` alias without a double-dash flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Action {
    Update,
    Install,
    Uninstall,
}

/// TR-300: Cross-platform system information report
#[derive(Debug, Parser)]
#[command(name = "tr300")]
#[command(
    author,
    version,
    about = "TR-300 Machine Report - Cross-platform system information"
)]
#[command(long_about = "TR-300 is the successor to TR-200 Machine Report.\n\n\
    It displays comprehensive system information including OS, network, CPU, \n\
    memory, disk usage, and session details in a formatted table.\n\n\
    After installation with --install, you can also use the 'report' alias.")]
pub struct Cli {
    /// Optional action command: update, install, or uninstall
    #[arg(value_enum, conflicts_with_all = ["update", "install", "uninstall"])]
    pub action: Option<Action>,

    /// Use ASCII characters instead of Unicode box-drawing
    #[arg(long)]
    pub ascii: bool,

    /// Output in JSON format instead of table
    #[arg(long)]
    pub json: bool,

    /// Install tr300 to shell profile (adds 'report' alias and auto-run)
    #[arg(long, conflicts_with_all = ["update", "uninstall", "action"])]
    pub install: bool,

    /// Remove tr300 from shell profile
    #[arg(long, conflicts_with_all = ["update", "install", "action"])]
    pub uninstall: bool,

    /// Check for updates and install the latest version
    #[arg(long, conflicts_with_all = ["install", "uninstall", "action"])]
    pub update: bool,

    /// Custom title for the report header
    #[arg(short = 't', long)]
    pub title: Option<String>,

    /// Disable colored output
    #[arg(long)]
    pub no_color: bool,

    /// Fast mode: skip slow platform-specific collectors for quick auto-run
    #[arg(long)]
    pub fast: bool,

    /// Suppress the "Run with sudo / Administrator for more details" footer hint
    #[arg(long)]
    pub no_elevation_hint: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_positional_update_action() {
        let cli = Cli::try_parse_from(["tr300", "update"]).expect("update action should parse");
        assert_eq!(cli.action, Some(Action::Update));
    }

    #[test]
    fn parses_legacy_update_flag() {
        let cli = Cli::try_parse_from(["tr300", "--update"]).expect("--update should parse");
        assert!(cli.update);
        assert_eq!(cli.action, None);
    }

    #[test]
    fn parses_json_before_positional_update_action() {
        let cli =
            Cli::try_parse_from(["tr300", "--json", "update"]).expect("--json update should parse");
        assert!(cli.json);
        assert_eq!(cli.action, Some(Action::Update));
    }

    #[test]
    fn parses_json_after_positional_update_action() {
        let cli =
            Cli::try_parse_from(["tr300", "update", "--json"]).expect("update --json should parse");
        assert!(cli.json);
        assert_eq!(cli.action, Some(Action::Update));
    }

    #[test]
    fn parses_install_and_uninstall_actions() {
        let install =
            Cli::try_parse_from(["tr300", "install"]).expect("install action should parse");
        let uninstall =
            Cli::try_parse_from(["tr300", "uninstall"]).expect("uninstall action should parse");
        assert_eq!(install.action, Some(Action::Install));
        assert_eq!(uninstall.action, Some(Action::Uninstall));
    }

    #[test]
    fn rejects_positional_and_flag_action_conflict() {
        let err = Cli::try_parse_from(["tr300", "update", "--install"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }
}
