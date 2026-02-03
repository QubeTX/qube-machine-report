//! Custom error types for tr-300
//!
//! Uses thiserror for ergonomic error definitions with
//! clear, actionable messages for end users.

use thiserror::Error;

/// Result type alias using AppError
pub type Result<T> = std::result::Result<T, AppError>;

/// Application-level errors for tr-300
#[derive(Error, Debug)]
pub enum AppError {
    /// Failed to retrieve system information
    #[error("Failed to retrieve system information: {message}")]
    SystemInfo { message: String },

    /// Platform-specific operation failed
    #[error("Platform operation failed: {message}")]
    Platform { message: String },

    /// I/O operation failed
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error
    #[error("Configuration error: {message}")]
    Config { message: String },

    /// Terminal/display error
    #[error("Display error: {message}")]
    Display { message: String },

    /// WMI error (Windows only)
    #[cfg(windows)]
    #[error("WMI query failed: {0}")]
    Wmi(#[from] wmi::WMIError),
}

impl AppError {
    /// Create a system info error
    pub fn system_info(message: impl Into<String>) -> Self {
        Self::SystemInfo {
            message: message.into(),
        }
    }

    /// Create a platform error
    pub fn platform(message: impl Into<String>) -> Self {
        Self::Platform {
            message: message.into(),
        }
    }

    /// Create a config error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a display error
    pub fn display(message: impl Into<String>) -> Self {
        Self::Display {
            message: message.into(),
        }
    }
}
