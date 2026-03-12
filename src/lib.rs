//! TR-300: Cross-platform system information report
//!
//! This library provides the core functionality for collecting
//! and displaying system information in the TR-200 format.
//!
//! # Example
//!
//! ```no_run
//! use tr_300::{collectors::{CollectMode, SystemInfo}, config::Config, report};
//!
//! // Full report (default)
//! let info = SystemInfo::collect().unwrap();
//! let config = Config::default();
//! let report = report::generate(&info, &config);
//! println!("{}", report);
//!
//! // Fast report (skips slow collectors)
//! let info = SystemInfo::collect_with_mode(CollectMode::Fast).unwrap();
//! let report = report::generate(&info, &config);
//! println!("{}", report);
//! ```

pub mod cli;
pub mod collectors;
pub mod config;
pub mod error;
pub mod install;
pub mod render;
pub mod report;

pub use collectors::{CollectMode, SystemInfo};
pub use config::Config;
pub use error::{AppError, Result};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Format bytes as a human-readable string (B, KB, MB, GB, TB)
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Generate a system report with default configuration (full mode)
pub fn generate_report() -> Result<String> {
    let info = SystemInfo::collect()?;
    let config = Config::default();
    Ok(report::generate(&info, &config))
}

/// Generate a system report with custom configuration (full mode)
pub fn generate_report_with_config(config: &Config) -> Result<String> {
    let info = SystemInfo::collect()?;
    Ok(report::generate(&info, config))
}
