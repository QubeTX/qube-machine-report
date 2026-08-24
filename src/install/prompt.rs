//! Interactive prompts for installation/uninstallation
//!
//! Provides user-friendly prompts for uninstall options.

use std::io::{self, BufRead, Write};
use std::path::Path;

/// Uninstall options available to the user
#[non_exhaustive]
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
    #[cfg(unix)]
    println!("     Also removes its exact matching cargo-dist receipt, when present");
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
    confirm_complete_uninstall_paths(Some(binary_path), parent_dir, None)
}

/// Confirm Complete uninstall while naming every exact Unix ownership path.
pub fn confirm_complete_uninstall_paths(
    binary_path: Option<&Path>,
    parent_dir: Option<&Path>,
    receipt_path: Option<&Path>,
) -> bool {
    println!();
    if let Err(error) =
        write_complete_uninstall_paths(&mut io::stdout(), binary_path, parent_dir, receipt_path)
    {
        eprintln!("Could not display the exact Complete-uninstall paths: {error}");
        return false;
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

fn write_complete_uninstall_paths(
    output: &mut impl Write,
    binary_path: Option<&Path>,
    parent_dir: Option<&Path>,
    receipt_path: Option<&Path>,
) -> io::Result<()> {
    writeln!(output, "This will permanently remove:")?;
    writeln!(output, "  - Shell profile modifications")?;
    if let Some(binary_path) = binary_path {
        writeln!(output, "  - Binary: {}", binary_path.display())?;
    }
    if let Some(receipt_path) = receipt_path {
        writeln!(output, "  - cargo-dist receipt: {}", receipt_path.display())?;
    }
    if let Some(dir) = parent_dir {
        writeln!(output, "  - Directory: {} (if empty)", dir.display())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::write_complete_uninstall_paths;
    use std::path::Path;

    #[test]
    fn complete_confirmation_names_exact_binary_and_receipt_paths() {
        let mut output = Vec::new();
        write_complete_uninstall_paths(
            &mut output,
            Some(Path::new("/managed/bin/tr300")),
            None,
            Some(Path::new("/config/tr300/tr300-receipt.json")),
        )
        .unwrap();
        let output = String::from_utf8(output).unwrap();

        assert!(output.contains("Binary: /managed/bin/tr300"));
        assert!(output.contains("cargo-dist receipt: /config/tr300/tr300-receipt.json"));
        assert!(!output.contains("when present"));
    }
}
