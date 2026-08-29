//! Step-line rendering basics: icons, separators, plan headers, error
//! detail, summary separator, color gating, and very-small-column behavior.

use super::*;

/// TASK-0354: `render_prefix` and `render` compute the same indent/icon/pad
/// triple. With a multi-character icon (like "OK"), the displayed width of
/// the rendered prefix must equal the sum of indent + icon + pad widths
/// (plus the trailing label and space). Catches drift between the two
/// callers of `step_prefix_parts`.
#[test]
fn render_prefix_width_matches_helper_components_for_multi_char_icon() {
    use ops_core::output::display_width;
    let mut cfg = ThemeConfig::compact();
    cfg.icon_succeeded = "OK".into();
    let theme = ConfigurableTheme::new(cfg);
    let step = StepLine::new(StepStatus::Succeeded, "cargo build".to_string(), None);

    let plain_prefix = theme.render_prefix(&step, false);
    let parts = theme.step_prefix_parts(StepStatus::Succeeded, false);
    let expected_width = display_width(parts.indent)
        + display_width(parts.icon)
        + display_width(&parts.pad)
        + 1 // single space between pad and label
        + display_width(&step.label);
    assert_eq!(
        display_width(&plain_prefix),
        expected_width,
        "render_prefix width must equal sum of helper component widths; otherwise render_separator's layout math drifts (DUP-5)"
    );

    assert!(
        plain_prefix.contains("OK"),
        "rendered prefix should contain the configured multi-char icon"
    );
}

#[test]
fn classic_theme_success_with_duration() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let line = render_line(
        &theme,
        StepStatus::Succeeded,
        "cargo build --all-targets",
        Some(0.35),
    );
    assert!(line.starts_with(" ├── ◆ cargo build"));
    assert!(line.contains("0.35s"));
    assert!(line.contains('─'), "classic uses box-drawing separator");
    assert!(!line.contains(".."), "classic does not use dot separator");
}

#[test]
fn compact_theme_success_icon() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let line = render_line(&theme, StepStatus::Succeeded, "cargo test", Some(1.50));
    assert!(line.starts_with(' ' /* left_pad */));
    assert!(line.contains("✓ cargo test"));
    assert!(line.contains("1.50s"));
    assert!(line.contains('.'), "compact uses dot separator");
}

#[test]
fn classic_theme_failed() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let line = render_line(&theme, StepStatus::Failed, "cargo clippy", Some(0.10));
    assert!(line.starts_with(" ├── ✖ cargo clippy"));
    assert!(line.contains("0.10s"));
}

#[test]
fn classic_theme_pending_no_duration() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let line = render_line(&theme, StepStatus::Pending, "cargo build", None);
    assert!(line.starts_with(" ├── ◇ cargo build"));
    assert!(!line.contains('s'));
}

#[test]
fn classic_theme_running_status() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let line = render_line(&theme, StepStatus::Running, "cargo test", Some(0.5));
    assert!(line.starts_with("◆ cargo test") || line.contains("cargo test"));
    assert!(line.contains("0.50s"));
}

#[test]
fn compact_theme_running_status() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let line = render_line(&theme, StepStatus::Running, "cargo build", Some(1.0));
    assert!(line.contains("cargo build"));
    assert!(line.contains("1.00s"));
}

#[test]
fn classic_plan_header_tree() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let ids = vec!["build".into(), "clippy".into(), "test".into()];
    let lines = theme.render_plan_header(&ids);
    assert_eq!(lines.len(), 3);
    assert!(lines[0].is_empty(), "upper space");
    assert_eq!(lines[1], " ┌ Running: build, clippy, test");
    assert_eq!(lines[2], " │");
}

#[test]
fn plain_header_with_prefix_emits_prefix() {
    let mut cfg = ThemeConfig::compact();
    cfg.plan_header_prefix = "🚀 ".into();
    let theme = ConfigurableTheme::new(cfg);
    let lines = theme.render_plan_header(&["build".into(), "test".into()]);
    assert_eq!(lines[1], " 🚀 Running: build, test");
}

/// CL-3 / TASK-1976 AC#3: with stderr redirected and stdout a TTY —
/// `ops verify 2> build.log` from an interactive shell — the theme must emit
/// no SGR, because everything it renders goes to the redirected stream.
///
/// A test harness cannot make stdout a real terminal, so the scenario is
/// pinned in two halves that together cover it: the gate itself is asked
/// about exactly that stream combination, and a step line is rendered with
/// the stderr-bound gate forced off and asserted to be plain.
///
/// TEST-9: the rendered half used to depend on *this process's* stderr being
/// redirected. `ops_core::style::stderr_is_terminal()` caches its
/// `is_terminal()` answer in a `OnceLock`, so no amount of `#[serial]` can
/// make that true — run the suite from an interactive shell (`cargo nextest
/// run` without a pipe) and the theme legitimately emits SGR, failing a test
/// that is not about the harness's stdio at all. Force `NO_COLOR` for the
/// render, which drives the same gate to `false` deterministically, and keep
/// the un-forced assertion only when the cached probe agrees stderr really
/// is redirected.
#[test]
#[serial]
fn step_line_is_plain_when_stderr_is_redirected() {
    // stdout a TTY, stderr redirected: the shared stdout-bound resolver says
    // "colour", the theme's stderr-bound gate must not. Both are pure
    // functions of their arguments and hold whatever this process's stdio is.
    assert!(ops_core::style::color_enabled_from(true, false, false));
    assert!(!crate::style::color_enabled_for(false, false));

    let mut cfg = ThemeConfig::compact();
    cfg.label_color = "cyan".into();
    cfg.duration_color = "bold green".into();
    let theme = ConfigurableTheme::new(cfg);

    {
        let _g = EnvGuard::set("NO_COLOR", "1");
        let line = render_line(&theme, StepStatus::Succeeded, "cargo build", Some(0.5));
        assert!(
            !line.contains('\x1b'),
            "a step line must carry no SGR once the stderr-bound gate is off: {line:?}"
        );
    }

    // When the harness's own stderr really is redirected, the same must hold
    // with nothing forced — the production path.
    if !ops_core::style::stderr_is_terminal() {
        let line = render_line(&theme, StepStatus::Succeeded, "cargo build", Some(0.5));
        assert!(
            !line.contains('\x1b'),
            "a step line written to a redirected stderr must carry no SGR: {line:?}"
        );
    }
}

#[test]
#[serial]
fn label_color_does_not_affect_non_tty_output() {
    // Force color disabled via NO_COLOR so the test is robust regardless of
    // whether cargo test's stdio is a TTY (it is under `ops --raw test`).
    let _g = EnvGuard::set("NO_COLOR", "1");
    let mut cfg = ThemeConfig::compact();
    cfg.label_color = "cyan".into();
    let theme = ConfigurableTheme::new(cfg);
    let line = render_line(&theme, StepStatus::Succeeded, "cargo build", Some(0.5));
    assert!(
        !line.contains('\x1b'),
        "color-disabled output must not contain ANSI escapes"
    );
    assert!(line.contains("cargo build"));
}

#[test]
#[serial]
fn summary_color_does_not_affect_non_tty_output() {
    let _g = EnvGuard::set("NO_COLOR", "1");
    let mut cfg = ThemeConfig::compact();
    cfg.summary_color = "bold green".into();
    let theme = ConfigurableTheme::new(cfg);
    let s = theme.render_summary(true, 1.0);
    assert!(!s.contains('\x1b'));
    assert!(s.contains("Done"));
}

#[test]
fn compact_plan_header_plain() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let ids = vec!["build".into(), "test".into()];
    let lines = theme.render_plan_header(&ids);
    assert_eq!(lines.len(), 3);
    assert!(lines[0].is_empty());
    assert_eq!(lines[1], " Running: build, test");
    assert!(lines[2].is_empty());
}

#[test]
fn classic_error_detail_with_stderr() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let detail = ErrorDetail::new(
        "exit status: 101".to_string(),
        vec![
            "thread 'main' panicked at ...".to_string(),
            "error: test failed".to_string(),
        ],
    );
    let lines = theme.render_error_detail(&detail, 80);
    assert_eq!(lines[0], " │   ┌─");
    assert_eq!(lines[1], " │   │ exit status: 101");
    assert_eq!(lines[2], " │   │ stderr (last 2 lines):");
    assert_eq!(lines[3], " │   │   thread 'main' panicked at ...");
    assert_eq!(lines[4], " │   │   error: test failed");
    assert_eq!(lines[5], " │   └─");
    assert_eq!(lines.len(), 6);
}

#[test]
fn compact_error_detail_gutter_width() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let detail = ErrorDetail::new("exit status: 1".to_string(), vec![]);
    let lines = theme.render_error_detail(&detail, 80);
    assert_eq!(lines[0], "     ╭─");
    assert_eq!(lines[1], "     │ exit status: 1");
    assert_eq!(lines[2], "     ╰─");
    assert_eq!(lines.len(), 3);
}

#[test]
fn classic_summary_separator_is_rail() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let sep = theme.render_summary_separator(80);
    assert_eq!(sep, " │");
}

#[test]
fn compact_summary_separator_is_empty() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let sep = theme.render_summary_separator(80);
    assert!(sep.is_empty());
}

#[test]
fn error_detail_empty_returns_nothing() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let detail = ErrorDetail::new(String::new(), vec![]);
    let lines = theme.render_error_detail(&detail, 80);
    assert!(lines.is_empty());
}

/// TEST-11 / TASK-1980: these two used to assert only that the label was
/// echoed back. At these widths the label does not fit — the chrome, the
/// duration and the minimum separator run already consume the whole budget —
/// so under the CL-3 / TASK-1969 truncation policy the label is cut. What is
/// worth pinning is that the line still renders and still fits its budget.
#[test]
fn classic_theme_very_small_columns() {
    use crate::style::visible_width;
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let step = StepLine::new(StepStatus::Succeeded, "cmd".to_string(), Some(0.5));
    let line = theme.render(&step, 10);
    assert!(!line.is_empty());
    assert!(visible_width(&line) <= 10, "{line:?}");
    // Widen the terminal and the label comes back in full.
    let wide = theme.render(&step, 40);
    assert!(wide.contains("cmd"), "{wide:?}");
    assert_eq!(visible_width(&wide), 40, "{wide:?}");
}

#[test]
fn compact_theme_very_small_columns() {
    use crate::style::visible_width;
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let step = StepLine::new(StepStatus::Succeeded, "x".to_string(), Some(0.5));
    let line = theme.render(&step, 5);
    assert!(!line.is_empty());
    assert!(visible_width(&line) <= 5, "{line:?}");
    let wide = theme.render(&step, 40);
    assert!(wide.contains('x'), "{wide:?}");
    assert_eq!(visible_width(&wide), 40, "{wide:?}");
}
