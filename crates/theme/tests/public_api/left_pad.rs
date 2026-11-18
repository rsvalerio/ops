//! `left_pad` is applied to step lines, plan headers, and the summary.

use super::*;

fn theme_with_pad(pad: usize) -> ConfigurableTheme {
    ConfigurableTheme::new(ThemeConfig { left_pad: pad, ..ThemeConfig::compact() })
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
    let lines = theme.render_plan_header(&["build".into()], 0);
    assert_eq!(lines[1], "  Running: build");
}

#[test]
fn left_pad_prepends_spaces_to_summary() {
    let theme = theme_with_pad(2);
    let summary = theme.render_summary(RunOutcome::Success, 1.0);
    assert!(summary.starts_with("  Done"));
}

#[test]
fn left_pad_zero_produces_no_padding() {
    let theme = theme_with_pad(0);
    let line = render_line(&theme, StepStatus::Succeeded, "cargo test", Some(1.0));
    assert!(line.starts_with("  ✓"), "should start with step_indent, no extra pad");

    let padded = theme_with_pad(2);
    let padded_line = render_line(&padded, StepStatus::Succeeded, "cargo test", Some(1.0));
    assert!(padded_line.starts_with("    ✓"), "should have 2-space pad + step_indent");
}

/// A running row reserves the left pad too: the runner's running style
/// prepends it to `running_template`, so the row on screen is
/// `left_pad + running_template_overhead + rendered` columns and must fill
/// the terminal exactly, whatever the label length.
#[test]
fn running_row_fills_columns_with_left_pad_and_template_overhead() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    assert_eq!(theme.left_pad(), 1, "fixture assumes classic's one-column pad");
    let columns: u16 = 80;
    for label in ["build", &"x".repeat(200)] {
        let step = StepLine::new(StepStatus::Running, label.to_string(), None);
        let line = theme.render(&step, columns);
        let on_screen = theme
            .left_pad()
            .saturating_add(theme.running_template_overhead())
            .saturating_add(ops_theme::style::visible_width(&line));
        assert_eq!(on_screen, usize::from(columns), "label {} cols: {line:?}", label.len());
    }
}
