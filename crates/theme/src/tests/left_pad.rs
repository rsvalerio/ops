//! `left_pad` is applied to step lines, plan headers, and the summary.

use super::*;

fn theme_with_pad(pad: usize) -> ConfigurableTheme {
    ConfigurableTheme(ThemeConfig {
        left_pad: pad,
        ..ThemeConfig::compact()
    })
}

#[test]
fn left_pad_prepends_spaces_to_step_line() {
    let theme = theme_with_pad(3);
    let line = render_line(&theme, StepStatus::Succeeded, "cargo build", Some(0.5));
    assert!(line.starts_with("   "), "should have 3-space left pad");
    assert!(line.contains("✓ cargo build"));
}

#[test]
fn left_pad_prepends_spaces_to_plan_header() {
    let theme = theme_with_pad(2);
    let lines = theme.render_plan_header(&["build".into()]);
    assert_eq!(lines[1], "  Running: build");
}

#[test]
fn left_pad_prepends_spaces_to_summary() {
    let theme = theme_with_pad(2);
    let summary = theme.render_summary(true, 1.0);
    assert!(summary.starts_with("  Done"));
}

#[test]
fn left_pad_zero_produces_no_padding() {
    let theme = theme_with_pad(0);
    let line = render_line(&theme, StepStatus::Succeeded, "cargo test", Some(1.0));
    assert!(
        line.starts_with("  ✓"),
        "should start with step_indent, no extra pad"
    );

    let padded = theme_with_pad(2);
    let padded_line = render_line(&padded, StepStatus::Succeeded, "cargo test", Some(1.0));
    assert!(
        padded_line.starts_with("    ✓"),
        "should have 2-space pad + step_indent"
    );
}
