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
