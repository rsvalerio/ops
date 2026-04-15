//! Error detail block rendering.

use ops_core::config::theme_types::ErrorBlockChars;
use ops_core::output::ErrorDetail;

/// Shared helper for rendering error detail blocks with configurable box-drawing characters.
pub fn render_error_block(
    detail: &ErrorDetail,
    icon_column_width: usize,
    chars: &ErrorBlockChars,
    left_pad: usize,
) -> Vec<String> {
    if detail.message.is_empty() && detail.stderr_tail.is_empty() {
        return Vec::new();
    }
    let pad = " ".repeat(left_pad);
    let gutter = if chars.rail.is_empty() {
        " ".repeat(icon_column_width + 3)
    } else {
        format!("{}   ", chars.rail)
    };
    let mut lines = Vec::new();
    lines.push(format!("{}{}{}", pad, gutter, chars.top));
    if !detail.message.is_empty() {
        lines.push(format!("{}{}{} {}", pad, gutter, chars.mid, detail.message));
    }
    if !detail.stderr_tail.is_empty() {
        lines.push(format!(
            "{}{}{} stderr (last {} lines):",
            pad,
            gutter,
            chars.mid,
            detail.stderr_tail.len()
        ));
        for stderr_line in &detail.stderr_tail {
            lines.push(format!("{}{}{}   {}", pad, gutter, chars.mid, stderr_line));
        }
    }
    lines.push(format!("{}{}{}", pad, gutter, chars.bottom));
    lines
}
