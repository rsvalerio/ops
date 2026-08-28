//! Error-block rendering: SGR wrapping rules around the rail/top/mid/bottom
//! glyphs, gutter geometry, and width invariance under unknown color specs.
//!
//! TEST-25 / TASK-1979: these tests used to assert against a local copy of
//! `render_error_block` — the function under test was never called, and the
//! copy had already drifted (a hardcoded four-space gutter, no `left_pad`, no
//! `stderr_tail` branch). They now drive the shipped renderer through its
//! injected colour gate, so the properties they claim to pin are pinned on
//! the code that ships.

use super::*;
use crate::render::render_error_block_gated;
use crate::style::visible_width;
use ops_core::config::theme_types::ErrorBlockChars;

/// Icon column width of the theme the assertions are written against.
fn icon_column_width() -> usize {
    ConfigurableTheme::new(ThemeConfig::compact()).icon_column_width()
}

fn render_with(chars: &ErrorBlockChars, enabled: bool) -> Vec<String> {
    let detail = ErrorDetail::new("exit status: 1".to_string(), vec![]);
    render_error_block_gated(&detail, icon_column_width(), chars, 0, enabled)
}

fn boxed_chars(color: &str) -> ErrorBlockChars {
    ErrorBlockChars {
        top: "┌─".into(),
        mid: "│".into(),
        bottom: "└─".into(),
        rail: "│".into(),
        color: color.into(),
    }
}

#[test]
fn error_block_color_wraps_top_mid_bottom_with_sgr_when_enabled() {
    let lines = render_with(&boxed_chars("red dim"), true);
    assert_eq!(lines.len(), 3, "top + message + bottom: {lines:?}");
    for line in &lines {
        assert!(
            line.contains('\x1b'),
            "glyph should carry SGR when color enabled: {line}"
        );
    }
}

#[test]
fn error_block_rail_remains_unstyled_when_color_set() {
    let lines = render_with(&boxed_chars("red"), true);
    for line in &lines {
        assert!(
            line.starts_with('│'),
            "rail glyph must not be wrapped in SGR: {line}"
        );
    }
}

#[test]
fn error_block_unknown_color_does_not_change_display_width() {
    let plain_lines = render_with(&boxed_chars(""), true);
    let colored_lines = render_with(&boxed_chars("not-a-color zzz"), true);
    assert_eq!(plain_lines.len(), colored_lines.len());
    for (p, c) in plain_lines.iter().zip(colored_lines.iter()) {
        assert_eq!(
            visible_width(p),
            visible_width(c),
            "layout must be invariant: plain={p} colored={c}"
        );
    }
}

/// TEST-25 / TASK-1979 AC#3: the railless gutter is `icon_column_width + 3`
/// spaces, not the four spaces the old copy hardcoded — so a theme with wider
/// icons is covered by the same assertion.
#[test]
fn error_block_gutter_width_tracks_icon_column_width() {
    let chars = ErrorBlockChars {
        rail: String::new(),
        ..boxed_chars("red")
    };
    let detail = ErrorDetail::new("exit status: 1".to_string(), vec![]);
    for icon_width in [1usize, 2, 4] {
        let lines = render_error_block_gated(&detail, icon_width, &chars, 0, true);
        let top = &lines[0];
        let leading = top.chars().take_while(|c| *c == ' ').count();
        assert_eq!(
            leading,
            icon_width + 3,
            "gutter must be icon_column_width + 3 spaces: {top:?}"
        );
    }
}

/// TEST-25 / TASK-1979 AC#4: the `stderr_tail` branch — the header line plus
/// one line per captured entry — is part of the shipped renderer, and no
/// colour test reached it while the suite asserted against a three-line copy.
#[test]
fn error_block_stderr_tail_branch_is_colored_and_indented() {
    let detail = ErrorDetail::new(
        "exit status: 1".to_string(),
        vec!["boom".to_string(), "kaboom".to_string()],
    );
    let chars = boxed_chars("red");
    let lines = render_error_block_gated(&detail, icon_column_width(), &chars, 0, true);
    // top + message + "stderr (last 2 lines):" + 2 entries + bottom
    assert_eq!(lines.len(), 6, "{lines:?}");
    assert!(
        crate::strip_ansi(&lines[2]).ends_with("stderr (last 2 lines):"),
        "{:?}",
        lines[2]
    );
    for line in &lines[2..5] {
        assert!(
            line.contains('\x1b'),
            "mid glyph on the stderr_tail branch must carry SGR: {line}"
        );
    }
    assert!(
        crate::strip_ansi(&lines[4]).ends_with("   kaboom"),
        "{:?}",
        lines[4]
    );

    // With the gate off the same lines carry no SGR at all.
    let plain = render_error_block_gated(&detail, icon_column_width(), &chars, 0, false);
    for line in &plain {
        assert!(!line.contains('\x1b'), "gate off must be plain: {line}");
    }
}
