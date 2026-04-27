//! Rendering with Unicode labels (CJK, emoji, RTL, mixed-width).

use super::*;

#[test]
fn render_handles_unicode_labels() {
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let step = StepLine {
        status: StepStatus::Succeeded,
        label: "构建项目".to_string(),
        elapsed: Some(1.0),
    };
    let line = theme.render(&step, 80);
    assert!(line.contains("构建项目"));
}

#[test]
fn render_handles_emoji_in_label() {
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let step = StepLine {
        status: StepStatus::Succeeded,
        label: "✅ Build successful 🎉".to_string(),
        elapsed: Some(1.0),
    };
    let line = theme.render(&step, 80);
    assert!(line.contains("✅"));
    assert!(line.contains("🎉"));
}

#[test]
fn render_handles_mixed_width_unicode() {
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let step = StepLine {
        status: StepStatus::Succeeded,
        label: "测试 🚀 😀 test".to_string(),
        elapsed: Some(1.0),
    };
    let line = theme.render(&step, 80);
    assert!(line.contains("测试"));
    assert!(line.contains("🚀"));
}

#[test]
fn render_handles_very_long_unicode_label() {
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let step = StepLine {
        status: StepStatus::Succeeded,
        label: "构建".repeat(50),
        elapsed: Some(1.0),
    };
    let line = theme.render(&step, 80);
    assert!(line.contains("构建"));
}

#[test]
fn render_handles_right_to_left_text() {
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let step = StepLine {
        status: StepStatus::Succeeded,
        label: "مرحبا".to_string(),
        elapsed: Some(1.0),
    };
    let line = theme.render(&step, 80);
    assert!(line.contains("مرحبا"));
}
