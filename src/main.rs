//! TR-300: Cross-platform system information report
//!
//! A modern CLI tool for displaying system information
//! in a visually appealing Unicode box-drawing table format.

use clap::Parser;
use tr_300::{
    collectors::{CollectMode, SystemInfo},
    config::Config,
    error::Result,
    install, report,
};

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

    /// Fast mode: skip slow platform-specific collectors for quick auto-run
    #[arg(long)]
    fast: bool,
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

    // Determine collection mode
    let mode = if cli.fast {
        CollectMode::Fast
    } else {
        CollectMode::Full
    };

    // Run the report
    run_report(&config, mode)
}

/// Run the main system report
fn run_report(config: &Config, mode: CollectMode) -> Result<()> {
    let info = SystemInfo::collect_with_mode(mode)?;
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

/// Uninstall tr300 from shell profile (interactive)
fn run_uninstall() -> Result<()> {
    use install::{
        confirm_complete_uninstall, find_binary_location, get_binary_parent_dir,
        prompt_uninstall_option, UninstallOption,
    };

    let option = prompt_uninstall_option();

    match option {
        UninstallOption::Cancel => {
            println!();
            println!("Uninstall cancelled.");
            Ok(())
        }
        UninstallOption::ProfileOnly => {
            println!();
            println!("Removing shell profile modifications...");
            install::uninstall()?;
            println!();
            println!("TR-300 auto-run has been removed from your shell profile.");
            println!("The tr300 binary remains installed and can be run manually.");
            Ok(())
        }
        UninstallOption::Complete => {
            // Get binary location for confirmation prompt
            let binary_path = find_binary_location();
            let parent_dir = binary_path
                .as_ref()
                .and_then(|p| get_binary_parent_dir(p.as_path()));

            if let Some(ref path) = binary_path {
                if !confirm_complete_uninstall(path, parent_dir.as_deref()) {
                    println!();
                    println!("Uninstall cancelled.");
                    return Ok(());
                }
            }

            println!();
            println!("Performing complete uninstall...");
            install::uninstall_complete()?;
            println!();
            println!("TR-300 has been completely removed from your system.");
            Ok(())
        }
    }
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
