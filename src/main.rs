//! TR-300: Cross-platform system information report
//!
//! A modern CLI tool for displaying system information
//! in a visually appealing Unicode box-drawing table format.

use clap::Parser;
use tr300::{
    cli::{Action, Cli},
    collectors::{CollectMode, SystemInfo},
    config::{Config, OutputFormat},
    error::Result,
    install, report, update,
};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let action = cli.action;

    // Build configuration from CLI args (needed by --update)
    let mut config = Config::new().with_colors(!cli.no_color);

    if cli.ascii || !is_utf8_locale() {
        config = config.with_ascii();
    }

    if cli.json {
        config = config.with_json();
    }

    if cli.no_elevation_hint {
        config = config.with_no_elevation_hint(true);
    }

    if let Some(title) = cli.title {
        config = config.with_title(title);
    }

    // Enable UTF-8 output on Windows. Bound for the lifetime of main() so the
    // prior console codepage is restored on normal exit. Early action exits
    // explicitly drop the guard before `process::exit` for the same guarantee.
    #[cfg(windows)]
    let _cp_guard = enable_utf8_console();

    // Handle action commands (early exit)
    if action == Some(Action::MigrateCleanup) {
        // Hidden, installer-internal: consolidate to a single install. Advisory —
        // never fails (exit 0 on partial/empty); only a true internal error is
        // nonzero. Mirrors `nd300 migrate-cleanup`.
        let mut opts = tr300::migrate::MigrateOptions::default();
        opts.cargo_copy = cli.cargo_copy;
        opts.other_edition = cli.other_edition;
        opts.quiet = cli.quiet;
        opts.dry_run = cli.dry_run;
        opts.json = cli.json;
        opts.user_profile = cli.user_profile.clone();
        opts.cargo_home = cli.cargo_home.clone();
        let exit_code = tr300::migrate::run(&config, &opts);
        #[cfg(windows)]
        drop(_cp_guard);
        std::process::exit(exit_code);
    }

    if action == Some(Action::UpdateCleanup) {
        let exit_code = cli
            .update_backup
            .as_deref()
            .map(update::cleanup_windows_update_backup)
            .unwrap_or(2);
        #[cfg(windows)]
        drop(_cp_guard);
        std::process::exit(exit_code);
    }

    if action == Some(Action::UpdateWorker) {
        let exit_code = match (
            cli.update_strategy.as_deref(),
            cli.update_version.as_deref(),
            cli.update_backup.as_deref(),
        ) {
            (Some(strategy), Some(version), Some(backup)) => {
                update::run_windows_update_worker(strategy, version, backup)
            }
            _ => 2,
        };
        #[cfg(windows)]
        drop(_cp_guard);
        std::process::exit(exit_code);
    }

    if cli.update || action == Some(Action::Update) {
        let exit_code = update::run(&config);
        #[cfg(windows)]
        drop(_cp_guard);
        std::process::exit(exit_code);
    }

    if cli.install || action == Some(Action::Install) {
        return run_install();
    }

    if cli.uninstall || action == Some(Action::Uninstall) {
        return run_uninstall();
    }

    // Determine collection mode
    let mode = if cli.fast {
        CollectMode::Fast
    } else {
        CollectMode::Full
    };

    // Run the report
    run_report(&config, mode, cli.save_report)
}

/// Run the main system report
fn run_report(config: &Config, mode: CollectMode, save_report: bool) -> Result<()> {
    use std::io::Write;

    let info = SystemInfo::collect_with_mode(mode)?;
    let output = report::generate(&info, config);
    print!("{}", output);
    std::io::stdout().flush()?;

    // Saving is deliberately opt-in. Ordinary report runs must not touch the
    // filesystem: managed Windows antivirus products can treat an unexpected
    // report-file write as suspicious and stall the host. Clap restricts the
    // explicit save aliases to full table mode; retain the defensive runtime
    // gate in case this function is later called from another entry point.
    if save_report && mode == CollectMode::Full && config.format == OutputFormat::Table {
        match report::save_markdown_report(&info) {
            Ok(outcome) if outcome.used_cwd_fallback => eprintln!(
                "Report saved: {} (Downloads folder not found — saved to the current directory)",
                outcome.path.display()
            ),
            Ok(outcome) => eprintln!("Report saved: {}", outcome.path.display()),
            Err(e) => eprintln!("Warning: could not save markdown report: {}", e),
        }
    }

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
    #[cfg(target_os = "macos")]
    println!("Please restart your shell or run 'source ~/.zshrc' (or the profile shown above)");
    #[cfg(not(target_os = "macos"))]
    println!("Please restart your shell or run 'source ~/.bashrc' (or the profile shown above)");
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
        _ => {
            println!();
            println!("Uninstall cancelled: unsupported option; no changes were made.");
            Ok(())
        }
    }
}

/// Check whether the current locale supports UTF-8.
///
/// Implements strict POSIX precedence: `LC_ALL` wins outright when set
/// and non-empty; otherwise `LC_CTYPE` (the category that governs
/// character classification, the relevant one for terminal rendering);
/// otherwise `LANG`. Empty-string values for the higher-precedence
/// variables mean "fall through to the next category" per the POSIX
/// locale spec (see `locale(7)`).
///
/// Pre-v3.15.5 the function had an asymmetric skip: `LC_ALL=C` /
/// `LC_ALL=POSIX` fell through to `LANG` (correct), but
/// `LC_ALL=de_DE.ISO-8859-1` short-circuited to `false` (correct per
/// POSIX precedence — LC_ALL trumps — but the asymmetry was a smell).
/// The strict reading is now consistent: any non-empty value in the
/// higher-precedence variable is authoritative, including `C` and
/// `POSIX` (which both correctly return `false` for "not UTF-8").
fn is_utf8_locale() -> bool {
    // On Windows, we handle UTF-8 via SetConsoleOutputCP
    #[cfg(windows)]
    {
        true
    }

    #[cfg(not(windows))]
    {
        for var in ["LC_ALL", "LC_CTYPE", "LANG"] {
            if let Ok(val) = std::env::var(var) {
                if val.is_empty() {
                    // POSIX: empty value falls through to the next
                    // category in precedence order.
                    continue;
                }
                let upper = val.to_uppercase();
                return upper.contains("UTF-8") || upper.contains("UTF8");
            }
        }
        false
    }
}

/// RAII guard that restores the prior Windows console output code page
/// when dropped.
///
/// Before v3.15.5, `enable_utf8_console` set `CP_UTF8` (65001) for the
/// current console window and never restored it — the change survived
/// the `tr300` process exit and affected every subsequent program
/// running in the same `cmd.exe` / Windows Terminal tab, occasionally
/// breaking legacy CP-437-dependent batch scripts. Wrapping the change
/// in a guard whose `Drop` puts the prior CP back makes the side effect
/// scoped to TR-300's lifetime in main().
///
/// Note: `std::process::exit` does not run `Drop`; callers using it must drop
/// this guard explicitly first.
#[cfg(windows)]
struct ConsoleCpGuard {
    prev: u32,
}

#[cfg(windows)]
impl Drop for ConsoleCpGuard {
    fn drop(&mut self) {
        unsafe {
            winapi::um::wincon::SetConsoleOutputCP(self.prev);
        }
    }
}

/// Switch the Windows console to UTF-8 output for the lifetime of the
/// returned guard.
///
/// Returns `None` (no guard, no change) when neither stdout nor stderr
/// is a terminal (a fully-piped invocation) or when the console is
/// already in UTF-8 mode. Returns `Some(guard)` otherwise; the caller
/// must keep the guard alive for as long as box-drawing output is
/// emitted.
#[cfg(windows)]
fn enable_utf8_console() -> Option<ConsoleCpGuard> {
    use std::io::IsTerminal;

    if !std::io::stdout().is_terminal() && !std::io::stderr().is_terminal() {
        return None;
    }

    // GetConsoleOutputCP lives in `consoleapi`, not `wincon` — the
    // pairing with `SetConsoleOutputCP` (in `wincon`) is one of winapi-rs's
    // mildly inconsistent module placements.
    let prev = unsafe { winapi::um::consoleapi::GetConsoleOutputCP() };
    if prev == 65001 {
        // Already UTF-8 — Windows Terminal's default, or some earlier
        // tool already switched. Nothing to restore.
        return None;
    }
    unsafe {
        winapi::um::wincon::SetConsoleOutputCP(65001);
    }
    Some(ConsoleCpGuard { prev })
}
