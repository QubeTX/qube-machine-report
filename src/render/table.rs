//! TR-200-style Unicode box-drawing table renderer
//!
//! Creates the exact table format used by TR-200 Machine Report
//! with proper dividers and centered headers.

use crate::config::BoxChars;
use unicode_width::UnicodeWidthStr;

/// Table renderer matching TR-200 format
pub struct TableRenderer {
    /// Label column width
    label_width: usize,
    /// Data column width
    data_width: usize,
    /// Box-drawing characters
    chars: BoxChars,
    /// Total table width
    total_width: usize,
}

impl TableRenderer {
    /// Create a new table renderer with specified column widths
    pub fn new(label_width: usize, data_width: usize, chars: BoxChars) -> Self {
        // Total = │ + space + label + space + │ + space + data + space + │
        // = 1 + 1 + label + 1 + 1 + 1 + data + 1 + 1 = label + data + 7
        let total_width = label_width + data_width + 7;
        Self {
            label_width,
            data_width,
            chars,
            total_width,
        }
    }

    /// Render the top header with ┌┬┬┬...┬┐ pattern
    pub fn render_top_header(&self) -> String {
        let mut line = String::new();
        line.push(self.chars.top_left);
        for _ in 0..(self.total_width - 2) {
            line.push(self.chars.t_down);
        }
        line.push(self.chars.top_right);
        line.push('\n');
        line
    }

    /// Render line below top header with ├┴┴┴...┴┤ pattern
    pub fn render_header_bottom(&self) -> String {
        let mut line = String::new();
        line.push(self.chars.t_right);
        for _ in 0..(self.total_width - 2) {
            line.push(self.chars.t_up);
        }
        line.push(self.chars.t_left);
        line.push('\n');
        line
    }

    /// Render a centered text line (for title/subtitle)
    pub fn render_centered(&self, text: &str) -> String {
        let text_width = text.width();
        let inner_width = self.total_width - 2; // Subtract the two │ borders

        let padding = if text_width >= inner_width {
            0
        } else {
            (inner_width - text_width) / 2
        };
        let extra = if text_width >= inner_width {
            0
        } else {
            (inner_width - text_width) % 2
        };

        let mut line = String::new();
        line.push(self.chars.vertical);
        line.push_str(&" ".repeat(padding));

        // Truncate if needed
        if text_width > inner_width {
            let mut truncated = String::new();
            let mut w = 0;
            for c in text.chars() {
                let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                if w + cw > inner_width.saturating_sub(3) {
                    break;
                }
                truncated.push(c);
                w += cw;
            }
            line.push_str(&truncated);
            line.push_str("...");
        } else {
            line.push_str(text);
        }

        line.push_str(&" ".repeat(padding + extra));
        line.push(self.chars.vertical);
        line.push('\n');
        line
    }

    /// Render top divider (after title section): ├────────────┬────────────────────┤
    pub fn render_top_divider(&self) -> String {
        let mut line = String::new();
        line.push(self.chars.t_right);

        // Label section: space + label + space
        for _ in 0..(self.label_width + 2) {
            line.push(self.chars.horizontal);
        }

        // Column divider
        line.push(self.chars.t_down);

        // Data section: space + data + space
        for _ in 0..(self.data_width + 2) {
            line.push(self.chars.horizontal);
        }

        line.push(self.chars.t_left);
        line.push('\n');
        line
    }

    /// Render middle divider (between sections): ├────────────┼────────────────────┤
    pub fn render_middle_divider(&self) -> String {
        let mut line = String::new();
        line.push(self.chars.t_right);

        // Label section
        for _ in 0..(self.label_width + 2) {
            line.push(self.chars.horizontal);
        }

        // Column divider (cross for middle)
        line.push(self.chars.cross);

        // Data section
        for _ in 0..(self.data_width + 2) {
            line.push(self.chars.horizontal);
        }

        line.push(self.chars.t_left);
        line.push('\n');
        line
    }

    /// Render bottom divider (before footer): ├────────────┴────────────────────┤
    pub fn render_bottom_divider(&self) -> String {
        let mut line = String::new();
        line.push(self.chars.t_right);

        // Label section
        for _ in 0..(self.label_width + 2) {
            line.push(self.chars.horizontal);
        }

        // Column divider (T-up for bottom)
        line.push(self.chars.t_up);

        // Data section
        for _ in 0..(self.data_width + 2) {
            line.push(self.chars.horizontal);
        }

        line.push(self.chars.t_left);
        line.push('\n');
        line
    }

    /// Render footer: └────────────┴────────────────────┘
    pub fn render_footer(&self) -> String {
        let mut line = String::new();
        line.push(self.chars.bottom_left);

        // Label section
        for _ in 0..(self.label_width + 2) {
            line.push(self.chars.horizontal);
        }

        // Column divider
        line.push(self.chars.t_up);

        // Data section
        for _ in 0..(self.data_width + 2) {
            line.push(self.chars.horizontal);
        }

        line.push(self.chars.bottom_right);
        line.push('\n');
        line
    }

    /// Render a data row: │ LABEL      │ VALUE                │
    pub fn render_row(&self, label: &str, value: &str) -> String {
        let label_display = self.fit_string(label, self.label_width);
        let value_display = self.fit_string(value, self.data_width);

        let mut line = String::new();
        line.push(self.chars.vertical);
        line.push(' ');
        line.push_str(&label_display);
        line.push(' ');
        line.push(self.chars.vertical);
        line.push(' ');
        line.push_str(&value_display);
        line.push(' ');
        line.push(self.chars.vertical);
        line.push('\n');
        line
    }

    /// Fit a string to exact width (display columns), padding or truncating as needed
    fn fit_string(&self, s: &str, width: usize) -> String {
        let display_width = s.width();
        if display_width > width {
            // Truncate with ellipsis
            if width <= 3 {
                let mut result = String::new();
                let mut w = 0;
                for c in s.chars() {
                    let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                    if w + cw > width {
                        break;
                    }
                    result.push(c);
                    w += cw;
                }
                while w < width {
                    result.push(' ');
                    w += 1;
                }
                result
            } else {
                let mut result = String::new();
                let mut w = 0;
                for c in s.chars() {
                    let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                    if w + cw > width - 3 {
                        break;
                    }
                    result.push(c);
                    w += cw;
                }
                result.push_str("...");
                w += 3;
                while w < width {
                    result.push(' ');
                    w += 1;
                }
                result
            }
        } else {
            // Pad with spaces
            let padding = width - display_width;
            format!("{}{}", s, " ".repeat(padding))
        }
    }
}
