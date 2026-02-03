//! Bar graph rendering
//!
//! Creates visual bar graphs using Unicode block characters
//! matching the TR-200 format.

/// Render a bar graph with the given percentage and width
///
/// # Arguments
/// * `percent` - Value from 0.0 to 100.0
/// * `width` - Width of the bar in characters
/// * `filled_char` - Character for filled portion (e.g., '█')
/// * `empty_char` - Character for empty portion (e.g., '░')
pub fn render_bar(percent: f64, width: usize, filled_char: char, empty_char: char) -> String {
    let percent = percent.clamp(0.0, 100.0);
    let filled_count = ((percent / 100.0) * width as f64).round() as usize;
    let empty_count = width.saturating_sub(filled_count);

    format!(
        "{}{}",
        filled_char.to_string().repeat(filled_count),
        empty_char.to_string().repeat(empty_count)
    )
}

/// Render a bar graph with default Unicode characters (█░)
pub fn render_bar_unicode(percent: f64, width: usize) -> String {
    render_bar(percent, width, '█', '░')
}

/// Render a bar graph with ASCII characters (#.)
pub fn render_bar_ascii(percent: f64, width: usize) -> String {
    render_bar(percent, width, '#', '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_empty() {
        let bar = render_bar_unicode(0.0, 10);
        assert_eq!(bar, "░░░░░░░░░░");
    }

    #[test]
    fn test_bar_full() {
        let bar = render_bar_unicode(100.0, 10);
        assert_eq!(bar, "██████████");
    }

    #[test]
    fn test_bar_half() {
        let bar = render_bar_unicode(50.0, 10);
        assert_eq!(bar, "█████░░░░░");
    }

    #[test]
    fn test_bar_clamp_over() {
        let bar = render_bar_unicode(150.0, 10);
        assert_eq!(bar, "██████████");
    }

    #[test]
    fn test_bar_clamp_under() {
        let bar = render_bar_unicode(-50.0, 10);
        assert_eq!(bar, "░░░░░░░░░░");
    }
}
