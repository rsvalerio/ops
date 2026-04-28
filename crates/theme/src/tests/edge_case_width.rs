//! TQ-010: extreme column widths and over-long labels must not panic or
//! produce empty output. Also covers display-width invariants in
//! `render_separator`.

use super::*;
use ops_core::output::StepLine;

#[test]
fn render_with_zero_columns_does_not_panic() {
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let step = StepLine::new(StepStatus::Succeeded, "test".to_string(), Some(1.0));
    let line = theme.render(&step, 0);
    assert!(!line.is_empty(), "should still produce output");
}

#[test]
fn render_with_one_column_does_not_panic() {
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let step = StepLine::new(StepStatus::Succeeded, "test".to_string(), Some(1.0));
    let line = theme.render(&step, 1);
    assert!(!line.is_empty(), "should still produce output");
}

#[test]
fn render_with_two_columns_does_not_panic() {
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let step = StepLine::new(StepStatus::Succeeded, "x".to_string(), None);
    let line = theme.render(&step, 2);
    assert!(!line.is_empty(), "should still produce output");
}

#[test]
fn render_with_very_small_columns_handles_gracefully() {
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let step = StepLine::new(
        StepStatus::Succeeded,
        "very long label that exceeds column width".to_string(),
        Some(1.0),
    );
    let line = theme.render(&step, 5);
    assert!(!line.is_empty(), "should handle small width");
}

#[test]
fn render_pending_with_zero_columns() {
    let theme = ConfigurableTheme(ThemeConfig::classic());
    let step = StepLine::new(StepStatus::Pending, "pending".to_string(), None);
    let line = theme.render(&step, 0);
    assert!(!line.is_empty());
}

#[test]
fn render_failed_with_minimal_columns() {
    let theme = ConfigurableTheme(ThemeConfig::classic());
    let step = StepLine::new(StepStatus::Failed, "f".to_string(), Some(0.01));
    let line = theme.render(&step, 3);
    assert!(!line.is_empty());
}

/// TQ-010: Label longer than column width does not panic or produce empty output.
#[test]
fn render_label_longer_than_columns() {
    let theme = ConfigurableTheme(ThemeConfig::classic());
    let long_label = "a_very_long_command_name_that_exceeds_the_terminal_column_width_by_far";
    let step = StepLine::new(StepStatus::Succeeded, long_label.to_string(), Some(1.23));
    let line = theme.render(&step, 20);
    assert!(
        !line.is_empty(),
        "render should produce output even for long labels"
    );
}

#[test]
fn render_separator_label_longer_than_columns() {
    let theme = ConfigurableTheme(ThemeConfig::classic());
    let long_label = "this_label_is_way_too_long_for_the_given_column_width";
    let sep = theme.render_separator(long_label, "1.23s", 10, false);
    assert!(sep.len() <= 200, "separator should not be excessively long");
}

/// READ-5/TASK-0351: a custom theme returning a multi-byte duration string
/// (e.g. comma-decimal, leading wide glyph) must not cause the separator
/// math to over-reserve width. The fixed-inside cost is computed in
/// display columns, not UTF-8 bytes.
#[test]
fn render_separator_uses_display_width_for_multi_byte_duration() {
    use ops_core::output::display_width;
    let theme = ConfigurableTheme(ThemeConfig::classic());
    let ascii_dur = "1.23s"; // 5 bytes, 5 columns
    let wide_dur = "⏱ 1.2s"; // 6 columns, 8 bytes
    let prefix = "● cargo build";
    let cols = 80;
    let sep_ascii = theme.render_separator(prefix, ascii_dur, cols, false);
    let sep_wide = theme.render_separator(prefix, wide_dur, cols, false);
    let ascii_w = display_width(&sep_ascii);
    let wide_w = display_width(&sep_wide);
    assert_eq!(
        ascii_w.saturating_sub(wide_w),
        display_width(wide_dur) - display_width(ascii_dur),
        "separator length must scale with duration display width, not byte length",
    );
}

#[test]
fn icon_column_width_handles_all_statuses() {
    let theme = ConfigurableTheme(ThemeConfig::classic());
    let width = theme.icon_column_width();
    assert!(width > 0, "icon column width should be positive");
}
