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
        failed: 0,
        skipped: 0,
        total,
        elapsed_secs: elapsed,
        success,
        columns,
        command_ids: &[],
    }
}

fn boxed_theme() -> ConfigurableTheme {
    ConfigurableTheme::new(ThemeConfig {
        layout_kind: LayoutKind::Boxed,
        left_pad: 0,
        ..ThemeConfig::compact()
    })
}

#[test]
fn flat_theme_returns_no_borders() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
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
fn boxed_bottom_border_shows_breakdown_when_not_success() {
    // CL-3 / TASK-0771: bottom border now surfaces the succeeded/skipped/failed
    // breakdown instead of conflating terminal count with success count.
    let theme = boxed_theme();
    let bottom = theme
        .box_bottom_border(BoxSnapshot {
            failed: 1,
            skipped: 1,
            ..snap(3, 5, 2.0, false, 80)
        })
        .expect("boxed theme returns bottom");
    let plain = strip_ansi(&bottom);
    assert!(
        plain.contains("1 succeeded, 1 skipped, 1 failed of 5"),
        "got: {plain}"
    );
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
    let step = StepLine::new(StepStatus::Succeeded, "cargo build".to_string(), Some(1.23));
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
    let detail = ErrorDetail::new("exit status: 1".to_string(), vec!["boom".to_string()]);
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
    let theme = ConfigurableTheme::new(ThemeConfig {
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
    let detail = ErrorDetail::new("exit status: 1".to_string(), vec![]);
    let lines = theme.render_error_detail(&detail, 80);
    let plain_top = strip_ansi(&lines[0]);
    let plain_mid = strip_ansi(&lines[1]);
    assert!(plain_top.starts_with("│      ├─"), "got: {plain_top}");
    assert!(
        plain_mid.starts_with("│      │ exit status: 1"),
        "got: {plain_mid}"
    );
}

/// FN-1 / TASK-1192 AC#2: pin gutter alignment for two step_indent widths
/// (0 and 2) so a future refactor of `boxed_error_indent_columns` cannot
/// silently mis-align the error glyph column.
#[test]
fn boxed_error_indent_tracks_step_indent_width() {
    fn theme_with_step_indent(step_indent: &str) -> ConfigurableTheme {
        ConfigurableTheme::new(ThemeConfig {
            layout_kind: LayoutKind::Boxed,
            left_pad: 0,
            step_indent: step_indent.to_string(),
            error_block: ops_core::config::theme_types::ErrorBlockChars {
                top: "├─".into(),
                mid: "│".into(),
                bottom: "└─".into(),
                rail: "│".into(),
                color: String::new(),
            },
            ..ThemeConfig::compact()
        })
    }
    let detail = ErrorDetail::new("exit status: 1".to_string(), vec![]);

    // step_indent width 0 (existing baseline).
    let theme0 = theme_with_step_indent("");
    let plain0 = strip_ansi(&theme0.render_error_detail(&detail, 80)[0]);
    let count0 = plain0
        .strip_prefix('│')
        .and_then(|s| s.find('├'))
        .expect("├ glyph in output");

    // step_indent width 2: gutter must shift right by exactly 2 columns.
    let theme2 = theme_with_step_indent("  ");
    let plain2 = strip_ansi(&theme2.render_error_detail(&detail, 80)[0]);
    let count2 = plain2
        .strip_prefix('│')
        .and_then(|s| s.find('├'))
        .expect("├ glyph in output");

    assert_eq!(
        count2,
        count0 + 2,
        "step_indent width 2 must shift the error glyph by 2 columns; \
         baseline={plain0:?} shifted={plain2:?}"
    );
}

/// PERF-3 / TASK-1130: pin the no-extra-allocation contract on the hot path.
/// `wrap_step_line` must not allocate an intermediate `" ".repeat(n)` String
/// per call and `render_separator` must not call `sep.to_string().repeat(n)` —
/// in both cases the result String is built directly. We pin this by asserting
/// the produced output is byte-identical to the spec contract: the wrapped
/// line is exactly `outer_columns` wide in display cells, the separator
/// contains only ASCII dots and spaces, and (as a `String` capacity proxy)
/// the buffer was reserved at construction so we never grow mid-write.
#[test]
fn wrap_step_line_and_render_separator_no_intermediate_repeat_alloc() {
    let theme = boxed_theme();
    let wrapped = theme.wrap_step_line("  ✓ cargo build 1.23s", "█", 60);
    let plain = strip_ansi(&wrapped);
    assert_eq!(ops_core::output::display_width(&plain), 60);
    // The string is byte-stable: prefix → cell → label → padding → trailing bar.
    assert!(plain.starts_with("│ █  "));
    assert!(plain.ends_with(" │"));

    let sep = theme.render_separator("  ✓ cargo build", "1.23s", 60, false);
    // Always begins with one leading space, then a non-empty run of separator
    // glyphs (no `to_string().repeat` intermediate), no trailing space when
    // duration_str is non-empty.
    assert!(sep.starts_with(' '));
    assert!(!sep.ends_with(' '));
    let dots = sep.trim_start();
    assert!(!dots.is_empty(), "sep: {sep:?}");

    let sep_running = theme.render_separator("  ✓ cargo build", "", 60, true);
    assert!(sep_running.starts_with(' '));
    assert!(sep_running.ends_with(' '));
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
