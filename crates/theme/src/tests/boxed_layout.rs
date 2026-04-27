//! Boxed-layout borders (top/bottom), error-detail framing, and step-line
//! wrapping for the boxed theme.

use super::*;
use crate::step_line_theme::BoxSnapshot;
use ops_core::config::theme_types::LayoutKind;

fn snap(
    completed: usize,
    total: usize,
    elapsed: f64,
    success: bool,
    columns: u16,
) -> BoxSnapshot<'static> {
    BoxSnapshot {
        completed,
        total,
        elapsed_secs: elapsed,
        success,
        columns,
        command_ids: &[],
    }
}

fn boxed_theme() -> ConfigurableTheme {
    ConfigurableTheme(ThemeConfig {
        layout_kind: LayoutKind::Boxed,
        left_pad: 0,
        ..ThemeConfig::compact()
    })
}

#[test]
fn flat_theme_returns_no_borders() {
    let theme = ConfigurableTheme(ThemeConfig::compact());
    assert!(theme.box_top_border(snap(0, 5, 0.0, true, 80)).is_none());
    assert!(theme.box_bottom_border(snap(5, 5, 1.0, true, 80)).is_none());
    assert_eq!(theme.step_column_reserve(), 0);
    assert_eq!(theme.wrap_step_line("hello", "█", 80), "hello");
}

#[test]
fn boxed_top_border_spans_columns() {
    let theme = boxed_theme();
    let ids = vec!["build".to_string(), "test".to_string()];
    let top = theme
        .box_top_border(BoxSnapshot {
            command_ids: &ids,
            ..snap(2, 5, 12.4, true, 60)
        })
        .expect("boxed theme returns top");
    let plain = strip_ansi(&top);
    assert_eq!(ops_core::output::display_width(&plain), 60, "got: {top}");
    assert!(plain.starts_with("╭─"), "top corner: {plain}");
    assert!(plain.ends_with("╮"), "top corner end: {plain}");
    assert!(
        plain.contains("Running: build, test"),
        "contains command IDs: {plain}"
    );
}

#[test]
fn boxed_bottom_border_shows_done_when_success() {
    let theme = boxed_theme();
    let bottom = theme
        .box_bottom_border(snap(5, 5, 94.0, true, 60))
        .expect("boxed theme returns bottom");
    let plain = strip_ansi(&bottom);
    assert!(plain.contains("Done 5/5"), "got: {plain}");
    assert!(plain.starts_with("╰─"));
    assert!(plain.ends_with("╯"));
}

#[test]
fn boxed_bottom_border_shows_failed_when_not_success() {
    let theme = boxed_theme();
    let bottom = theme
        .box_bottom_border(snap(3, 5, 2.0, false, 50))
        .expect("boxed theme returns bottom");
    let plain = strip_ansi(&bottom);
    assert!(plain.contains("Failed 3/5"), "got: {plain}");
}

#[test]
fn boxed_top_border_lists_command_ids_when_provided() {
    let theme = boxed_theme();
    let ids = vec!["build".to_string(), "test".to_string()];
    let s = BoxSnapshot {
        command_ids: &ids,
        ..snap(0, 2, 0.0, true, 60)
    };
    let top = strip_ansi(&theme.box_top_border(s).expect("top"));
    assert!(top.contains("Running: build, test"), "got: {top}");
}

#[test]
fn boxed_step_with_duration_matches_border_width() {
    // Regression: render() overshot by one column when a duration was present,
    // causing the right `│` of a completed step to land past the border `╮`/`╯`.
    let theme = boxed_theme();
    let columns = 60u16;
    let reserve = theme.step_column_reserve();
    let effective = columns - reserve;
    let step = StepLine {
        status: StepStatus::Succeeded,
        label: "cargo build".to_string(),
        elapsed: Some(1.23),
    };
    let inner = theme.render(&step, effective);
    let wrapped = theme.wrap_step_line(&inner, "█", columns);
    let plain = strip_ansi(&wrapped);
    assert_eq!(
        ops_core::output::display_width(&plain),
        columns as usize,
        "wrapped width: {plain}"
    );

    let ids = vec!["build".to_string()];
    let top = strip_ansi(
        &theme
            .box_top_border(BoxSnapshot {
                command_ids: &ids,
                ..snap(1, 1, 1.23, true, columns)
            })
            .unwrap(),
    );
    assert_eq!(
        ops_core::output::display_width(&top),
        ops_core::output::display_width(&plain),
        "step width must equal border width"
    );
}

#[test]
fn boxed_error_detail_has_right_border() {
    let theme = boxed_theme();
    let detail = ErrorDetail {
        message: "exit status: 1".to_string(),
        stderr_tail: vec!["boom".to_string()],
    };
    let lines = theme.render_error_detail(&detail, 60);
    for line in &lines {
        let plain = strip_ansi(line);
        assert_eq!(
            ops_core::output::display_width(&plain),
            60,
            "error detail line width: {plain}"
        );
        assert!(plain.ends_with(" │"), "right border: {plain}");
    }
}

#[test]
fn boxed_error_detail_aligns_mid_with_label_column() {
    // Studio-like theme: rail mirrors the frame's left border, and the
    // top/mid/bottom glyphs must land under the step-label column
    // (box prefix `│ █  ` + step_indent).
    let theme = ConfigurableTheme(ThemeConfig {
        layout_kind: LayoutKind::Boxed,
        left_pad: 0,
        error_block: ops_core::config::theme_types::ErrorBlockChars {
            top: "├─".into(),
            mid: "│".into(),
            bottom: "└─".into(),
            rail: "│".into(),
            color: String::new(),
        },
        ..ThemeConfig::compact()
    });
    let detail = ErrorDetail {
        message: "exit status: 1".to_string(),
        stderr_tail: vec![],
    };
    let lines = theme.render_error_detail(&detail, 80);
    let plain_top = strip_ansi(&lines[0]);
    let plain_mid = strip_ansi(&lines[1]);
    assert!(plain_top.starts_with("│      ├─"), "got: {plain_top}");
    assert!(
        plain_mid.starts_with("│      │ exit status: 1"),
        "got: {plain_mid}"
    );
}

#[test]
fn wrap_step_line_reserves_seven_columns_and_pads_to_width() {
    let theme = boxed_theme();
    assert_eq!(theme.step_column_reserve(), 7);
    let wrapped = theme.wrap_step_line("  ✓ cargo build 1.23s", "█", 60);
    let plain = strip_ansi(&wrapped);
    assert_eq!(
        ops_core::output::display_width(&plain),
        60,
        "wrap width: {plain}"
    );
    assert!(plain.starts_with("│ █  "), "prefix: {plain}");
    assert!(plain.ends_with(" │"), "suffix: {plain}");
}
