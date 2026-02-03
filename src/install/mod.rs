//! Self-installation utilities
//!
//! Provides commands to install/uninstall tr300 to system paths.

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

use crate::error::Result;

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
pub fn install_path() -> Option<std::path::PathBuf> {
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
