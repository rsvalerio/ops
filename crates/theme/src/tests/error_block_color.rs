//! Error-block rendering: SGR wrapping rules around the rail/top/mid/bottom
//! glyphs and width invariance under unknown color specs.

use super::*;
use crate::style::{apply_style_gated, visible_width};
use ops_core::config::theme_types::ErrorBlockChars;

fn render_with(chars: ErrorBlockChars, enabled: bool) -> Vec<String> {
    // Mirror render_error_block's structure but with explicit styling gate,
    // since apply_style itself consults stderr TTY state at runtime.
    let detail = ErrorDetail::new("exit status: 1".to_string(), vec![]);
    let pad = String::new();
    let gutter = if chars.rail.is_empty() {
        "    ".to_string()
    } else {
        format!("{}   ", chars.rail)
    };
    let top = apply_style_gated(&chars.top, &chars.color, enabled);
    let mid = apply_style_gated(&chars.mid, &chars.color, enabled);
    let bottom = apply_style_gated(&chars.bottom, &chars.color, enabled);
    vec![
        format!("{}{}{}", pad, gutter, top),
        format!("{}{}{} {}", pad, gutter, mid, detail.message),
        format!("{}{}{}", pad, gutter, bottom),
    ]
}

#[test]
fn error_block_color_wraps_top_mid_bottom_with_sgr_when_enabled() {
    let chars = ErrorBlockChars {
        top: "┌─".into(),
        mid: "│".into(),
        bottom: "└─".into(),
        rail: "│".into(),
        color: "red dim".into(),
    };
    let lines = render_with(chars, true);
    for line in &lines {
        assert!(
            line.contains('\x1b'),
            "glyph should carry SGR when color enabled: {line}"
        );
    }
}

#[test]
fn error_block_rail_remains_unstyled_when_color_set() {
    let chars = ErrorBlockChars {
        top: "┌─".into(),
        mid: "│".into(),
        bottom: "└─".into(),
        rail: "│".into(),
        color: "red".into(),
    };
    let lines = render_with(chars, true);
    for line in &lines {
        assert!(
            line.starts_with('│'),
            "rail glyph must not be wrapped in SGR: {line}"
        );
    }
}

#[test]
fn error_block_unknown_color_does_not_change_display_width() {
    let base = ErrorBlockChars {
        top: "┌─".into(),
        mid: "│".into(),
        bottom: "└─".into(),
        rail: "│".into(),
        color: String::new(),
    };
    let colored = ErrorBlockChars {
        color: "not-a-color zzz".into(),
        ..base.clone()
    };
    let plain_lines = render_with(base, true);
    let colored_lines = render_with(colored, true);
    assert_eq!(plain_lines.len(), colored_lines.len());
    for (p, c) in plain_lines.iter().zip(colored_lines.iter()) {
        let pw = visible_width(p);
        let cw = visible_width(c);
        assert_eq!(pw, cw, "layout must be invariant: plain={p} colored={c}");
    }
}
