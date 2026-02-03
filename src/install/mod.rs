//! Self-installation utilities
//!
//! Provides commands to install/uninstall tr300 to system paths.

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

pub mod prompt;

use crate::error::Result;
use std::path::PathBuf;

pub use prompt::{confirm_complete_uninstall, prompt_uninstall_option, UninstallOption};

/// Install tr300 to the system
pub fn install() -> Result<()> {
    #[cfg(unix)]
    {
        unix::install()
    }

    #[cfg(windows)]
    {
        windows::install()
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err(crate::error::AppError::platform(
            "Self-installation not supported on this platform",
        ))
    }
}

/// Uninstall tr300 from the system
pub fn uninstall() -> Result<()> {
    #[cfg(unix)]
    {
        unix::uninstall()
    }

    #[cfg(windows)]
    {
        windows::uninstall()
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err(crate::error::AppError::platform(
            "Self-uninstallation not supported on this platform",
        ))
    }
}

/// Get the installation path
pub fn install_path() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        Some(unix::install_path())
    }

    #[cfg(windows)]
    {
        Some(windows::install_path())
    }

    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

/// Find the location of the currently running binary
pub fn find_binary_location() -> Option<PathBuf> {
    #[cfg(unix)]
    {
        unix::find_binary_location()
    }

    #[cfg(windows)]
    {
        windows::find_binary_location()
    }

    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

/// Get the parent directory of the binary (for cleanup on Windows)
pub fn get_binary_parent_dir(binary_path: &std::path::Path) -> Option<PathBuf> {
    #[cfg(unix)]
    {
        let _ = binary_path;
        None // Unix doesn't need directory cleanup
    }

    #[cfg(windows)]
    {
        windows::get_binary_parent_dir(binary_path)
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = binary_path;
        None
    }
}

/// Perform complete uninstall (profile + binary)
pub fn uninstall_complete() -> Result<()> {
    #[cfg(unix)]
    {
        unix::uninstall_complete()
    }

    #[cfg(windows)]
    {
        windows::uninstall_complete()
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err(crate::error::AppError::platform(
            "Complete uninstallation not supported on this platform",
        ))
    }
}
