//! Configuration management for TR-300
//!
//! Handles configuration constants and runtime settings
//! matching the TR-200 output format.

use crate::error::{AppError, Result};
use std::path::PathBuf;

/// Default title for the report header
pub const DEFAULT_TITLE: &str = "SHAUGHNESSY V DEVELOPMENT INC.";

/// Default subtitle for the report header
pub const DEFAULT_SUBTITLE: &str = "TR-300 MACHINE REPORT";

/// Minimum width for the label column
pub const MIN_LABEL_WIDTH: usize = 5;

/// Maximum width for the label column
pub const MAX_LABEL_WIDTH: usize = 13;

/// Minimum width for the data column
pub const MIN_DATA_WIDTH: usize = 20;

/// Maximum width for the data column
pub const MAX_DATA_WIDTH: usize = 32;

/// Borders and padding overhead (7 chars: "│ " + " │ " + " │")
pub const BORDERS_PADDING: usize = 7;

/// Box-drawing characters (Unicode)
pub mod chars {
    pub const TOP_LEFT: char = '┌';
    pub const TOP_RIGHT: char = '┐';
    pub const BOTTOM_LEFT: char = '└';
    pub const BOTTOM_RIGHT: char = '┘';
    pub const HORIZONTAL: char = '─';
    pub const VERTICAL: char = '│';
    pub const T_DOWN: char = '┬';
    pub const T_UP: char = '┴';
    pub const T_RIGHT: char = '├';
    pub const T_LEFT: char = '┤';
    pub const CROSS: char = '┼';

    /// Bar graph characters
    pub const BAR_FILLED: char = '█';
    pub const BAR_EMPTY: char = '░';
}

/// ASCII fallback characters
pub mod ascii_chars {
    pub const TOP_LEFT: char = '+';
    pub const TOP_RIGHT: char = '+';
    pub const BOTTOM_LEFT: char = '+';
    pub const BOTTOM_RIGHT: char = '+';
    pub const HORIZONTAL: char = '-';
    pub const VERTICAL: char = '|';
    pub const T_DOWN: char = '+';
    pub const T_UP: char = '+';
    pub const T_RIGHT: char = '+';
    pub const T_LEFT: char = '+';
    pub const CROSS: char = '+';

    /// Bar graph characters
    pub const BAR_FILLED: char = '#';
    pub const BAR_EMPTY: char = '.';
}

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Whether to use Unicode box-drawing characters (false = ASCII)
    pub use_unicode: bool,
    /// Whether to use colors in output
    pub use_colors: bool,
    /// Custom title (overrides DEFAULT_TITLE)
    pub title: Option<String>,
    /// Custom subtitle (overrides DEFAULT_SUBTITLE)
    pub subtitle: Option<String>,
    /// Whether to show network information
    pub show_network: bool,
    /// Whether to show disk information
    pub show_disks: bool,
    /// Output width (0 = auto-detect)
    pub width: usize,
    /// Whether to use compact mode
    pub compact: bool,
    /// Output format: "table" (default) or "json"
    pub format: OutputFormat,
}

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Table,
    Json,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            use_unicode: true,
            use_colors: true,
            title: None,
            subtitle: None,
            show_network: true,
            show_disks: true,
            width: 0,
            compact: false,
            format: OutputFormat::Table,
        }
    }
}

impl Config {
    /// Create a new config with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the config file path
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| AppError::config("Could not determine config directory"))?;

        Ok(config_dir.join("tr-300").join("config.toml"))
    }

    /// Get the title to display
    pub fn title(&self) -> &str {
        self.title.as_deref().unwrap_or(DEFAULT_TITLE)
    }

    /// Get the subtitle to display
    pub fn subtitle(&self) -> &str {
        self.subtitle.as_deref().unwrap_or(DEFAULT_SUBTITLE)
    }

    /// Get the box-drawing characters based on unicode setting
    pub fn box_chars(&self) -> BoxChars {
        if self.use_unicode {
            BoxChars::unicode()
        } else {
            BoxChars::ascii()
        }
    }

    /// Get bar graph characters
    pub fn bar_chars(&self) -> (char, char) {
        if self.use_unicode {
            (chars::BAR_FILLED, chars::BAR_EMPTY)
        } else {
            (ascii_chars::BAR_FILLED, ascii_chars::BAR_EMPTY)
        }
    }

    /// Get effective terminal width
    pub fn effective_width(&self) -> usize {
        if self.width > 0 {
            self.width
        } else {
            // Auto-detect terminal width
            crossterm::terminal::size()
                .map(|(w, _)| w as usize)
                .unwrap_or(80)
        }
    }

    /// Calculate column widths based on content
    pub fn calculate_widths(&self, max_label: usize, max_data: usize) -> (usize, usize) {
        let label_width = max_label.clamp(MIN_LABEL_WIDTH, MAX_LABEL_WIDTH);
        let data_width = max_data.clamp(MIN_DATA_WIDTH, MAX_DATA_WIDTH);
        (label_width, data_width)
    }

    /// Get total table width
    pub fn table_width(&self, label_width: usize, data_width: usize) -> usize {
        label_width + data_width + BORDERS_PADDING
    }

    // Builder methods

    /// Set ASCII mode (disable Unicode)
    pub fn with_ascii(mut self) -> Self {
        self.use_unicode = false;
        self
    }

    /// Set output width
    pub fn with_width(mut self, width: Option<usize>) -> Self {
        if let Some(w) = width {
            self.width = w;
        }
        self
    }

    /// Set compact mode
    pub fn with_compact(mut self, compact: bool) -> Self {
        self.compact = compact;
        self
    }

    /// Set color mode
    pub fn with_colors(mut self, colors: bool) -> Self {
        self.use_colors = colors;
        self
    }

    /// Set custom title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Disable network display
    pub fn without_network(mut self) -> Self {
        self.show_network = false;
        self
    }

    /// Disable disk display
    pub fn without_disks(mut self) -> Self {
        self.show_disks = false;
        self
    }

    /// Set JSON output format
    pub fn with_json(mut self) -> Self {
        self.format = OutputFormat::Json;
        self
    }
}

/// Box-drawing character set
#[derive(Debug, Clone, Copy)]
pub struct BoxChars {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub horizontal: char,
    pub vertical: char,
    pub t_down: char,
    pub t_up: char,
    pub t_right: char,
    pub t_left: char,
    pub cross: char,
}

impl BoxChars {
    /// Unicode box-drawing characters
    pub fn unicode() -> Self {
        Self {
            top_left: chars::TOP_LEFT,
            top_right: chars::TOP_RIGHT,
            bottom_left: chars::BOTTOM_LEFT,
            bottom_right: chars::BOTTOM_RIGHT,
            horizontal: chars::HORIZONTAL,
            vertical: chars::VERTICAL,
            t_down: chars::T_DOWN,
            t_up: chars::T_UP,
            t_right: chars::T_RIGHT,
            t_left: chars::T_LEFT,
            cross: chars::CROSS,
        }
    }

    /// ASCII fallback characters
    pub fn ascii() -> Self {
        Self {
            top_left: ascii_chars::TOP_LEFT,
            top_right: ascii_chars::TOP_RIGHT,
            bottom_left: ascii_chars::BOTTOM_LEFT,
            bottom_right: ascii_chars::BOTTOM_RIGHT,
            horizontal: ascii_chars::HORIZONTAL,
            vertical: ascii_chars::VERTICAL,
            t_down: ascii_chars::T_DOWN,
            t_up: ascii_chars::T_UP,
            t_right: ascii_chars::T_RIGHT,
            t_left: ascii_chars::T_LEFT,
            cross: ascii_chars::CROSS,
        }
    }
}
