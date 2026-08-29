//! Rendering with Unicode labels (CJK, emoji, RTL, mixed-width).
//!
//! TEST-11 / TASK-1980: `assert!(line.contains(<the input>))` is true for any
//! function that echoes its argument, so these tests said nothing about the
//! layout they exist to cover. Each now also pins the rendered width against
//! the budget it was rendered at.

use super::*;
use crate::style::visible_width;

/// Render at `columns` and assert the line fits the budget it was given.
fn render_within_budget(
    theme: &ConfigurableTheme,
    status: StepStatus,
    label: &str,
    elapsed: Option<f64>,
    columns: u16,
) -> String {
    let step = StepLine::new(status, label.to_string(), elapsed);
    let line = theme.render(&step, columns);
    assert!(
        visible_width(&line) <= usize::from(columns),
        "line of {} columns exceeds the {columns}-column budget: {line:?}",
        visible_width(&line)
    );
    line
}

#[test]
fn render_handles_unicode_labels() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let line = render_within_budget(&theme, StepStatus::Succeeded, "构建项目", Some(1.0), 80);
    assert!(line.contains("构建项目"));
}

#[test]
fn render_handles_emoji_in_label() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let line = render_within_budget(
        &theme,
        StepStatus::Succeeded,
        "✅ Build successful 🎉",
        Some(1.0),
        80,
    );
    assert!(line.contains("✅"));
    assert!(line.contains("🎉"));
}

#[test]
fn render_handles_mixed_width_unicode() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let line = render_within_budget(
        &theme,
        StepStatus::Succeeded,
        "测试 🚀 😀 test",
        Some(1.0),
        80,
    );
    assert!(line.contains("测试"));
    assert!(line.contains("🚀"));
}

/// TEST-11 / TASK-1980 AC#2: 200 display columns of CJK rendered at
/// `columns = 80` used to pass on a containment check alone, while the
/// produced line was two and a half times the requested width.
#[test]
fn render_handles_very_long_unicode_label() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let label = "构建".repeat(50);
    let line = render_within_budget(&theme, StepStatus::Succeeded, &label, Some(1.0), 80);
    assert!(line.contains("构建"));
    assert!(
        !line.contains(&label),
        "the 200-column label must not be echoed verbatim: {line:?}"
    );
    assert!(
        line.contains(crate::style::ELLIPSIS),
        "truncation must be marked: {line:?}"
    );
}

#[test]
fn render_handles_right_to_left_text() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let line = render_within_budget(&theme, StepStatus::Succeeded, "مرحبا", Some(1.0), 80);
    assert!(line.contains("مرحبا"));
}

/// TEST-11 / TASK-1980 AC#4: a wide (CJK / emoji) label at a narrow width in
/// boxed layout must produce a line exactly as wide as the frame border — the
/// case where a half-truncated wide glyph would otherwise leave the closing
/// bar one column off.
#[test]
fn boxed_wide_label_at_narrow_width_matches_border_width() {
    use crate::step_line_theme::BoxSnapshot;
    use ops_core::config::theme_types::LayoutKind;
    let theme = ConfigurableTheme::new(ThemeConfig {
        layout_kind: LayoutKind::Boxed,
        left_pad: 0,
        ..ThemeConfig::compact()
    });
    for columns in [24u16, 25, 30, 40] {
        for label in ["构建项目构建项目构建项目", "🚀🚀🚀 deploy everything now"]
        {
            let effective = columns - theme.step_column_reserve();
            let step = StepLine::new(StepStatus::Succeeded, label.to_string(), Some(1.23));
            let inner = theme.render(&step, effective);
            let wrapped = theme.wrap_step_line(&inner, "█", columns);
            let border = theme
                .box_top_border(BoxSnapshot {
                    completed: 0,
                    failed: 0,
                    skipped: 0,
                    total: 1,
                    elapsed_secs: 0.0,
                    success: true,
                    columns,
                    command_ids: &[],
                })
                .expect("boxed theme returns a top border");
            assert_eq!(
                visible_width(&wrapped),
                visible_width(&border),
                "step line must match the border width at {columns} columns: {wrapped:?}"
            );
        }
    }
}
