//! Error detail block rendering.

use ops_core::config::theme_types::ErrorBlockChars;
use ops_core::output::ErrorDetail;

use super::style::apply_style;

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
    let top = apply_style(&chars.top, &chars.color);
    let mid = apply_style(&chars.mid, &chars.color);
    let bottom = apply_style(&chars.bottom, &chars.color);
    let mut lines = Vec::new();
    lines.push(format!("{}{}{}", pad, gutter, top));
    if !detail.message.is_empty() {
        lines.push(format!("{}{}{} {}", pad, gutter, mid, detail.message));
    }
    if !detail.stderr_tail.is_empty() {
        lines.push(format!(
            "{}{}{} stderr (last {} lines):",
            pad,
            gutter,
            mid,
            detail.stderr_tail.len()
        ));
        for stderr_line in &detail.stderr_tail {
            lines.push(format!("{}{}{}   {}", pad, gutter, mid, stderr_line));
        }
    }
    lines.push(format!("{}{}{}", pad, gutter, bottom));
    lines
}
