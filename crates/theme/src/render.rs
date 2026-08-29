//! Error detail block rendering.

use ops_core::config::theme_types::ErrorBlockChars;
use ops_core::output::ErrorDetail;

use super::style::{apply_style_gated, color_enabled};

/// SEC-21 / TASK-1965: neutralise a string that came from a child process
/// before it is interpolated into a rendered line.
///
/// `detail.message` and every `detail.stderr_tail` entry are verbatim
/// subprocess stderr, and report row details are whatever the report
/// producer captured. Without this they can carry ESC sequences that clear
/// the operator's screen, move the cursor or open an OSC-8 hyperlink — and a
/// bare `\r` additionally corrupts the boxed frame, because the width
/// helpers score it as zero columns while the terminal acts on it.
///
/// Routes through the project's own SEC-21 defence,
/// [`ops_core::ui::sanitise_line`], so the theme escapes exactly what the
/// `ui::error` / dry-run audit channels escape.
pub fn sanitise(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    ops_core::ui::sanitise_line(s, &mut out);
    out
}

/// Shared helper for rendering error detail blocks with configurable box-drawing characters.
#[must_use]
pub fn render_error_block(
    detail: &ErrorDetail,
    icon_column_width: usize,
    chars: &ErrorBlockChars,
    left_pad: usize,
) -> Vec<String> {
    render_error_block_gated(detail, icon_column_width, chars, left_pad, color_enabled())
}

/// [`render_error_block`] with an explicit colour gate.
///
/// TEST-25 / TASK-1979: the colour behaviour of the error block used to be
/// covered by a local re-implementation in the test module, because
/// `apply_style` consults live stderr TTY state and a test harness never has
/// one. Injecting the gate here lets those tests exercise the shipped
/// renderer — including the `stderr_tail` branch — instead of a copy that
/// had already drifted from it.
#[must_use]
pub fn render_error_block_gated(
    detail: &ErrorDetail,
    icon_column_width: usize,
    chars: &ErrorBlockChars,
    left_pad: usize,
    color_enabled: bool,
) -> Vec<String> {
    if detail.message.is_empty() && detail.stderr_tail.is_empty() {
        return Vec::new();
    }
    let pad = " ".repeat(left_pad);
    let gutter = if chars.rail.is_empty() {
        " ".repeat(icon_column_width.saturating_add(3))
    } else {
        format!("{}   ", chars.rail)
    };
    let top = apply_style_gated(&chars.top, &chars.color, color_enabled);
    let mid = apply_style_gated(&chars.mid, &chars.color, color_enabled);
    let bottom = apply_style_gated(&chars.bottom, &chars.color, color_enabled);
    let mut lines = Vec::new();
    lines.push(format!("{pad}{gutter}{top}"));
    if !detail.message.is_empty() {
        lines.push(format!(
            "{}{}{} {}",
            pad,
            gutter,
            mid,
            sanitise(&detail.message)
        ));
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
            lines.push(format!("{pad}{gutter}{mid}   {}", sanitise(stderr_line)));
        }
    }
    lines.push(format!("{pad}{gutter}{bottom}"));
    lines
}
