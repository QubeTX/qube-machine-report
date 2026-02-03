//! Rendering modules for TR-300
//!
//! Provides table and bar graph rendering matching the TR-200 format.

pub mod bar;
pub mod table;

pub use bar::{render_bar, render_bar_ascii, render_bar_unicode};
pub use table::{ReportBuilder, TableRenderer};
