//! Interactive prompts for installation/uninstallation
//!
//! Provides user-friendly prompts for uninstall options.

use std::io::{self, BufRead, Write};
use std::path::Path;

/// Uninstall options available to the user
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UninstallOption {
    /// Remove shell profile modifications only (keep binary)
    ProfileOnly,
    /// Complete uninstall (profile + binary)
    Complete,
    /// Cancel the operation
    Cancel,
}

/// Prompt the user to choose an uninstall option
pub fn prompt_uninstall_option() -> UninstallOption {
    println!();
    println!("TR-300 Uninstall Options:");
    println!();
    println!("  1. Remove auto-run only");
    println!("     Removes shell profile modifications (alias and auto-run)");
    println!("     The tr300 binary will remain installed");
    println!();
    println!("  2. Uninstall TR300 entirely");
    println!("     Removes shell profile modifications AND the tr300 binary");
    println!();
    println!("  0. Cancel");
    println!();

    loop {
        print!("Enter your choice [0-2]: ");
        io::stdout().flush().ok();

        let stdin = io::stdin();
        let mut input = String::new();
        if stdin.lock().read_line(&mut input).is_err() {
            return UninstallOption::Cancel;
        }

        match input.trim() {
            "0" => return UninstallOption::Cancel,
            "1" => return UninstallOption::ProfileOnly,
            "2" => return UninstallOption::Complete,
            _ => {
                println!("Invalid choice. Please enter 0, 1, or 2.");
            }
        }
    }
}

/// Confirm complete uninstall with the user
/// Returns true if user confirms, false otherwise
pub fn confirm_complete_uninstall(binary_path: &Path, parent_dir: Option<&Path>) -> bool {
    println!();
    println!("This will permanently remove:");
    println!("  - Shell profile modifications");
    println!("  - Binary: {}", binary_path.display());
    if let Some(dir) = parent_dir {
        println!("  - Directory: {} (if empty)", dir.display());
    }
    println!();

    loop {
        print!("Are you sure? [y/N]: ");
        io::stdout().flush().ok();

        let stdin = io::stdin();
        let mut input = String::new();
        if stdin.lock().read_line(&mut input).is_err() {
            return false;
        }

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => return true,
            "n" | "no" | "" => return false,
            _ => {
                println!("Please enter 'y' or 'n'.");
            }
        }
    }
}
