// CLI argument definitions for TR-300
//
// Extracted to a separate module so both `main.rs` and `build.rs`
// can access the `Cli` struct (via `include!` in build.rs).

use clap::Parser;

/// TR-300: Cross-platform system information report
#[derive(Parser)]
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
    /// Use ASCII characters instead of Unicode box-drawing
    #[arg(long)]
    pub ascii: bool,

    /// Output in JSON format instead of table
    #[arg(long)]
    pub json: bool,

    /// Install tr300 to shell profile (adds 'report' alias and auto-run)
    #[arg(long)]
    pub install: bool,

    /// Remove tr300 from shell profile
    #[arg(long)]
    pub uninstall: bool,

    /// Custom title for the report header
    #[arg(short = 't', long)]
    pub title: Option<String>,

    /// Disable colored output
    #[arg(long)]
    pub no_color: bool,

    /// Fast mode: skip slow platform-specific collectors for quick auto-run
    #[arg(long)]
    pub fast: bool,
}
