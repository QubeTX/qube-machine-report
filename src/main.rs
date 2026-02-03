//! TR-300: Cross-platform system information report
//!
//! A modern CLI tool for displaying system information
//! in a visually appealing Unicode box-drawing table format.

use clap::Parser;
use tr_300::{collectors::SystemInfo, config::Config, error::Result, install, report};

/// TR-300: Cross-platform system information report
#[derive(Parser)]
#[command(name = "tr300")]
#[command(author, version, about = "TR-300 Machine Report - Cross-platform system information")]
#[command(long_about = "TR-300 is the successor to TR-200 Machine Report.\n\n\
    It displays comprehensive system information including OS, network, CPU, \n\
    memory, disk usage, and session details in a formatted table.\n\n\
    After installation with --install, you can also use the 'report' alias.")]
struct Cli {
    /// Use ASCII characters instead of Unicode box-drawing
    #[arg(long)]
    ascii: bool,

    /// Output in JSON format instead of table
    #[arg(long)]
    json: bool,

    /// Install tr300 to shell profile (adds 'report' alias and auto-run)
    #[arg(long)]
    install: bool,

    /// Remove tr300 from shell profile
    #[arg(long)]
    uninstall: bool,

    /// Custom title for the report header
    #[arg(short = 't', long)]
    title: Option<String>,

    /// Disable colored output
    #[arg(long)]
    no_color: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle install/uninstall commands
    if cli.install {
        return run_install();
    }

    if cli.uninstall {
        return run_uninstall();
    }

    // Build configuration from CLI args
    let mut config = Config::new().with_colors(!cli.no_color);

    if cli.ascii {
        config = config.with_ascii();
    }

    if cli.json {
        config = config.with_json();
    }

    if let Some(title) = cli.title {
        config = config.with_title(title);
    }

    // Enable UTF-8 output on Windows
    #[cfg(windows)]
    {
        enable_utf8_console();
    }

    // Run the report
    run_report(&config)
}

/// Run the main system report
fn run_report(config: &Config) -> Result<()> {
    let info = SystemInfo::collect()?;
    let output = report::generate(&info, config);
    print!("{}", output);
    Ok(())
}

/// Install tr300 to shell profile
fn run_install() -> Result<()> {
    println!("Installing TR-300...");
    install::install()?;
    println!("Installation complete!");
    println!();
    println!("The following changes were made:");
    println!("  - Added 'report' alias for tr300");
    println!("  - Added auto-run on new interactive shells");
    println!();
    println!("Please restart your shell or run 'source ~/.bashrc' (or equivalent)");
    println!("to activate the changes.");
    Ok(())
}

/// Uninstall tr300 from shell profile
fn run_uninstall() -> Result<()> {
    println!("Uninstalling TR-300...");
    install::uninstall()?;
    println!("Uninstallation complete!");
    println!();
    println!("TR-300 configuration has been removed from your shell profile.");
    println!("The binary itself was not removed. To fully remove, delete the tr300 executable.");
    Ok(())
}

/// Enable UTF-8 console output on Windows
#[cfg(windows)]
fn enable_utf8_console() {
    use std::io::IsTerminal;

    // Only set console mode if we're actually in a terminal
    if std::io::stdout().is_terminal() {
        unsafe {
            // Set console output code page to UTF-8
            winapi::um::wincon::SetConsoleOutputCP(65001);
        }
    }
}

/// Print version information
#[allow(dead_code)]
fn print_version() {
    println!("tr300 {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Cross-platform system information report");
    println!("Repository: {}", env!("CARGO_PKG_REPOSITORY"));
    println!("License: {}", env!("CARGO_PKG_LICENSE"));
}
