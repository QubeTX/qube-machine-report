//! TR-200-style Unicode box-drawing table renderer
//!
//! Creates the exact table format used by TR-200 Machine Report
//! with proper dividers and centered headers.

use crate::config::BoxChars;

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
        let text_len = text.chars().count();
        let inner_width = self.total_width - 2; // Subtract the two │ borders

        let padding = if text_len >= inner_width {
            0
        } else {
            (inner_width - text_len) / 2
        };
        let extra = if text_len >= inner_width {
            0
        } else {
            (inner_width - text_len) % 2
        };

        let mut line = String::new();
        line.push(self.chars.vertical);
        line.push_str(&" ".repeat(padding));

        // Truncate if needed
        if text_len > inner_width {
            let truncated: String = text.chars().take(inner_width.saturating_sub(3)).collect();
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

    /// Fit a string to exact width, padding or truncating as needed
    fn fit_string(&self, s: &str, width: usize) -> String {
        let char_count = s.chars().count();
        if char_count > width {
            // Truncate with ellipsis
            if width <= 3 {
                s.chars().take(width).collect()
            } else {
                let truncated: String = s.chars().take(width - 3).collect();
                format!("{}...", truncated)
            }
        } else {
            // Pad with spaces
            format!("{:width$}", s, width = width)
        }
    }
}

/// Convenience function to create a full report table
pub struct ReportBuilder {
    renderer: TableRenderer,
    output: String,
    has_rows: bool,
}

impl ReportBuilder {
    /// Create a new report builder
    pub fn new(label_width: usize, data_width: usize, chars: BoxChars) -> Self {
        Self {
            renderer: TableRenderer::new(label_width, data_width, chars),
            output: String::new(),
            has_rows: false,
        }
    }

    /// Add the header section with title and subtitle
    pub fn header(mut self, title: &str, subtitle: &str) -> Self {
        self.output.push_str(&self.renderer.render_top_header());
        self.output.push_str(&self.renderer.render_header_bottom());
        self.output.push_str(&self.renderer.render_centered(title));
        self.output.push_str(&self.renderer.render_centered(subtitle));
        self.output.push_str(&self.renderer.render_top_divider());
        self
    }

    /// Add a data row
    pub fn row(mut self, label: &str, value: &str) -> Self {
        self.output.push_str(&self.renderer.render_row(label, value));
        self.has_rows = true;
        self
    }

    /// Add a section divider
    pub fn divider(mut self) -> Self {
        self.output.push_str(&self.renderer.render_middle_divider());
        self
    }

    /// Finish and return the complete output
    pub fn finish(mut self) -> String {
        self.output.push_str(&self.renderer.render_bottom_divider());
        self.output.push_str(&self.renderer.render_footer());
        self.output
    }

    /// Get the current output without footer (for adding more sections)
    pub fn build(self) -> String {
        self.output
    }
}
