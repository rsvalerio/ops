//! TQ-010: extreme column widths and over-long labels must not panic or
//! produce empty output. Also covers display-width invariants in
//! `render_separator`.
//!
//! TEST-11 / TASK-1980: every test here that renders at a given `columns`
//! value now asserts the produced line's *visible width* against that budget,
//! not only that the line is non-empty. Two of these cases rendered lines
//! three to ten times the requested width while the suite stayed green,
//! because nothing looked at the width — which is exactly the defect CL-3 /
//! TASK-1969 fixed. A budget of `0` means "unknown" (see the truncation
//! policy in `configurable.rs`) and is the one case that is not clamped.

use super::*;
use crate::style::{visible_width, ELLIPSIS};
use ops_core::output::StepLine;

/// Render `step` at `columns` and assert the crate's central layout
/// invariant: a rendered line never exceeds the budget it was given.
fn render_within_budget(theme: &ConfigurableTheme, step: &StepLine, columns: u16) -> String {
    let line = theme.render(step, columns);
    assert!(
        visible_width(&line) <= usize::from(columns),
        "line of {} columns exceeds the {columns}-column budget: {line:?}",
        visible_width(&line)
    );
    line
}

#[test]
fn render_with_zero_columns_does_not_panic() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let step = StepLine::new(StepStatus::Succeeded, "test".to_string(), Some(1.0));
    // Budget 0 means "unknown": the line is rendered unclamped rather than
    // reduced to nothing.
    let line = theme.render(&step, 0);
    assert!(!line.is_empty(), "should still produce output");
}

#[test]
fn render_with_one_column_does_not_panic() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let step = StepLine::new(StepStatus::Succeeded, "test".to_string(), Some(1.0));
    let line = render_within_budget(&theme, &step, 1);
    assert!(!line.is_empty(), "should still produce output");
}

#[test]
fn render_with_two_columns_does_not_panic() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let step = StepLine::new(StepStatus::Succeeded, "x".to_string(), None);
    let line = render_within_budget(&theme, &step, 2);
    assert!(!line.is_empty(), "should still produce output");
}

#[test]
fn render_with_very_small_columns_handles_gracefully() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let step = StepLine::new(
        StepStatus::Succeeded,
        "very long label that exceeds column width".to_string(),
        Some(1.0),
    );
    let line = render_within_budget(&theme, &step, 5);
    assert!(!line.is_empty(), "should handle small width");
}

#[test]
fn render_pending_with_zero_columns() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let step = StepLine::new(StepStatus::Pending, "pending".to_string(), None);
    let line = theme.render(&step, 0);
    assert!(!line.is_empty());
}

#[test]
fn render_failed_with_minimal_columns() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let step = StepLine::new(StepStatus::Failed, "f".to_string(), Some(0.01));
    let line = render_within_budget(&theme, &step, 3);
    assert!(!line.is_empty());
}

/// TQ-010 + TEST-11 / TASK-1980 AC#2: a label longer than the column width is
/// truncated to the budget and marked with the ellipsis, rather than echoed
/// back at three times the requested width.
#[test]
fn render_label_longer_than_columns() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let long_label = "a_very_long_command_name_that_exceeds_the_terminal_column_width_by_far";
    let step = StepLine::new(StepStatus::Succeeded, long_label.to_string(), Some(1.23));
    let line = render_within_budget(&theme, &step, 20);
    assert!(
        !line.is_empty(),
        "render should produce output even for long labels"
    );
    assert!(
        !line.contains(long_label),
        "the over-long label must not be echoed verbatim: {line:?}"
    );
    assert!(
        line.contains(ELLIPSIS),
        "truncation must be marked with the ellipsis: {line:?}"
    );
}

/// TEST-11 / TASK-1980 AC#3: the old bound here was `sep.len() <= 200`, a
/// byte-length assertion with no relationship to the contract. The contract
/// is that the separator never pushes the line past `columns`.
#[test]
fn render_separator_label_longer_than_columns() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let columns = 10usize;
    let long_label = "this_label_is_way_too_long_for_the_given_column_width";
    let sep = theme.render_separator(long_label, "1.23s", columns, false);
    // Nothing is left over for the separator, so it collapses to its floor of
    // three columns — measured in display columns and tied to `columns`.
    assert!(
        visible_width(&sep) <= columns.max(3),
        "separator of {} columns for a {columns}-column budget: {sep:?}",
        visible_width(&sep)
    );

    // And when the label does fit, the whole line lands exactly on the budget.
    let step = StepLine::new(StepStatus::Succeeded, "cargo build".to_string(), Some(1.23));
    let line = theme.render(&step, 80);
    assert_eq!(visible_width(&line), 80, "{line:?}");
}

/// READ-5/TASK-0351: a custom theme returning a multi-byte duration string
/// (e.g. comma-decimal, leading wide glyph) must not cause the separator
/// math to over-reserve width. The fixed-inside cost is computed in
/// display columns, not UTF-8 bytes.
#[test]
fn render_separator_uses_display_width_for_multi_byte_duration() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let ascii_dur = "1.23s"; // 5 bytes, 5 columns
    let wide_dur = "⏱ 1.2s"; // 6 columns, 8 bytes
    let prefix = "● cargo build";
    let cols = 80;
    let sep_ascii = theme.render_separator(prefix, ascii_dur, cols, false);
    let sep_wide = theme.render_separator(prefix, wide_dur, cols, false);
    let ascii_w = visible_width(&sep_ascii);
    let wide_w = visible_width(&sep_wide);
    assert_eq!(
        ascii_w.saturating_sub(wide_w),
        visible_width(wide_dur) - visible_width(ascii_dur),
        "separator length must scale with duration display width, not byte length",
    );
}

/// READ-5 / TASK-1971 AC#1: the separator is budgeted in *columns*, so a
/// full-width separator glyph yields half as many repetitions and the same
/// total width — not a line twice as wide as the budget it came from.
#[test]
fn render_separator_budgets_wide_glyphs_in_columns() {
    let narrow = ConfigurableTheme::new(ThemeConfig {
        separator_char: '.',
        ..ThemeConfig::compact()
    });
    let wide = ConfigurableTheme::new(ThemeConfig {
        // U+FF0E FULLWIDTH FULL STOP: two terminal columns per glyph.
        separator_char: '\u{ff0e}',
        ..ThemeConfig::compact()
    });
    let prefix = "● cargo build";
    let narrow_sep = narrow.render_separator(prefix, "1.23s", 80, false);
    let wide_sep = wide.render_separator(prefix, "1.23s", 80, false);
    assert_eq!(
        wide_sep.chars().filter(|c| *c == '\u{ff0e}').count(),
        narrow_sep.chars().filter(|c| *c == '.').count() / 2,
        "a two-column separator must repeat half as often: {wide_sep:?}"
    );
    assert!(
        visible_width(&wide_sep) <= visible_width(&narrow_sep),
        "the wide separator must not overshoot the narrow one's width"
    );
}

/// READ-5 / TASK-1971 AC#4: a theme with a full-width separator *and* a
/// two-column spinner glyph still produces a boxed step line exactly as wide
/// as its border.
#[test]
fn wide_glyph_theme_boxed_line_matches_border_width() {
    use ops_core::config::theme_types::LayoutKind;
    let theme = ConfigurableTheme::new(ThemeConfig {
        layout_kind: LayoutKind::Boxed,
        left_pad: 0,
        separator_char: '\u{ff0e}',
        tick_chars: "🚀🛰 ".into(),
        ..ThemeConfig::compact()
    });
    let columns = 60u16;
    let effective = columns - theme.step_column_reserve();
    for status in [StepStatus::Succeeded, StepStatus::Running] {
        let step = StepLine::new(status, "cargo build".to_string(), Some(1.23));
        let inner = theme.render(&step, effective);
        let wrapped = theme.wrap_step_line(&inner, "█", columns);
        assert_eq!(
            visible_width(&wrapped),
            usize::from(columns),
            "{status:?} row must match the border width: {wrapped:?}"
        );
    }
}

#[test]
fn icon_column_width_handles_all_statuses() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let width = theme.icon_column_width();
    assert!(width > 0, "icon column width should be positive");
}

/// PERF-3 / TASK-1975 AC#3: the accessor now returns a value computed once in
/// the constructor — it must still report the widest `ALL_STATUSES` glyph,
/// including for a theme whose custom icon is multi-column.
#[test]
fn icon_column_width_tracks_widest_custom_icon() {
    let theme = ConfigurableTheme::new(ThemeConfig {
        icon_failed: "🚀".into(),
        ..ThemeConfig::compact()
    });
    assert_eq!(theme.icon_column_width(), 2);
    let narrow = ConfigurableTheme::new(ThemeConfig::compact());
    assert!(theme.icon_column_width() > narrow.icon_column_width());
}
