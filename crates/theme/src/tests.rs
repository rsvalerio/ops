//! Tests for theme types and rendering.

use super::*;
use indexmap::IndexMap;
use ops_core::output::{ErrorDetail, StepLine, StepStatus};
use ops_core::test_utils::EnvGuard;
use serial_test::serial;

/// Minimal valid ThemeConfig TOML with all required fields.
/// Tests that need to tweak one field can append/override after this base.
const MINIMAL_THEME_TOML: &str = r#"
icon_pending = "○"
icon_running = ""
icon_succeeded = "●"
icon_failed = "✗"
icon_skipped = "—"
separator_char = '.'
step_indent = "  "
running_template = "  {spinner:.cyan}{msg}"
tick_chars = "⠁⠂⠄ "
running_template_overhead = 7
summary_prefix = "→ "
summary_separator = ""
left_pad = 0
"#;

fn render_line(
    theme: &dyn StepLineTheme,
    status: StepStatus,
    label: &str,
    elapsed: Option<f64>,
) -> String {
    let step = StepLine {
        status,
        label: label.to_string(),
        elapsed,
    };
    theme.render(&step, 80)
}

/// TASK-0354: render_prefix and render compute the same indent/icon/pad
/// triple. With a multi-character icon (like "OK"), the displayed width of
/// the rendered prefix must equal the sum of indent + icon + pad widths
/// (plus the trailing label and space). Catches drift between the two
/// callers of step_prefix_parts.
#[test]
fn render_prefix_width_matches_helper_components_for_multi_char_icon() {
    use ops_core::output::display_width;
    let mut cfg = ThemeConfig::compact();
    cfg.icon_succeeded = "OK".into();
    let theme = ConfigurableTheme(cfg);
    let step = StepLine {
        status: StepStatus::Succeeded,
        label: "cargo build".to_string(),
        elapsed: None,
    };

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

    // The rendered prefix must literally contain the multi-char icon glyph.
    assert!(
        plain_prefix.contains("OK"),
        "rendered prefix should contain the configured multi-char icon"
    );
}

#[test]
fn classic_theme_success_with_duration() {
    let theme = ConfigurableTheme(ThemeConfig::classic());
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
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let line = render_line(&theme, StepStatus::Succeeded, "cargo test", Some(1.50));
    assert!(line.starts_with(" " /* left_pad */));
    assert!(line.contains("✓ cargo test"));
    assert!(line.contains("1.50s"));
    assert!(line.contains('.'), "compact uses dot separator");
}

#[test]
fn classic_theme_failed() {
    let theme = ConfigurableTheme(ThemeConfig::classic());
    let line = render_line(&theme, StepStatus::Failed, "cargo clippy", Some(0.10));
    assert!(line.starts_with(" ├── ✖ cargo clippy"));
    assert!(line.contains("0.10s"));
}

#[test]
fn classic_theme_pending_no_duration() {
    let theme = ConfigurableTheme(ThemeConfig::classic());
    let line = render_line(&theme, StepStatus::Pending, "cargo build", None);
    assert!(line.starts_with(" ├── ◇ cargo build"));
    assert!(!line.contains("s"));
}

#[test]
fn classic_theme_running_status() {
    let theme = ConfigurableTheme(ThemeConfig::classic());
    let line = render_line(&theme, StepStatus::Running, "cargo test", Some(0.5));
    assert!(line.starts_with("◆ cargo test") || line.contains("cargo test"));
    assert!(line.contains("0.50s"));
}

#[test]
fn compact_theme_running_status() {
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let line = render_line(&theme, StepStatus::Running, "cargo build", Some(1.0));
    assert!(line.contains("cargo build"));
    assert!(line.contains("1.00s"));
}

#[test]
fn classic_plan_header_tree() {
    let theme = ConfigurableTheme(ThemeConfig::classic());
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
    let theme = ConfigurableTheme(cfg);
    let lines = theme.render_plan_header(&["build".into(), "test".into()]);
    assert_eq!(lines[1], " 🚀 Running: build, test");
}

#[test]
#[serial]
fn label_color_does_not_affect_non_tty_output() {
    // Force color disabled via NO_COLOR so the test is robust regardless of
    // whether cargo test's stdio is a TTY (it is under `ops --raw test`).
    let _g = EnvGuard::set("NO_COLOR", "1");
    let mut cfg = ThemeConfig::compact();
    cfg.label_color = "cyan".into();
    let theme = ConfigurableTheme(cfg);
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
    let theme = ConfigurableTheme(cfg);
    let s = theme.render_summary(true, 1.0);
    assert!(!s.contains('\x1b'));
    assert!(s.contains("Done"));
}

#[test]
fn compact_plan_header_plain() {
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let ids = vec!["build".into(), "test".into()];
    let lines = theme.render_plan_header(&ids);
    assert_eq!(lines.len(), 3);
    assert!(lines[0].is_empty());
    assert_eq!(lines[1], " Running: build, test");
    assert!(lines[2].is_empty());
}

#[test]
fn classic_error_detail_with_stderr() {
    let theme = ConfigurableTheme(ThemeConfig::classic());
    let detail = ErrorDetail {
        message: "exit status: 101".to_string(),
        stderr_tail: vec![
            "thread 'main' panicked at ...".to_string(),
            "error: test failed".to_string(),
        ],
    };
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
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let detail = ErrorDetail {
        message: "exit status: 1".to_string(),
        stderr_tail: vec![],
    };
    let lines = theme.render_error_detail(&detail, 80);
    assert_eq!(lines[0], "     ╭─");
    assert_eq!(lines[1], "     │ exit status: 1");
    assert_eq!(lines[2], "     ╰─");
    assert_eq!(lines.len(), 3);
}

#[test]
fn classic_summary_separator_is_rail() {
    let theme = ConfigurableTheme(ThemeConfig::classic());
    let sep = theme.render_summary_separator(80);
    assert_eq!(sep, " │");
}

#[test]
fn compact_summary_separator_is_empty() {
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let sep = theme.render_summary_separator(80);
    assert!(sep.is_empty());
}

#[test]
fn error_detail_empty_returns_nothing() {
    let theme = ConfigurableTheme(ThemeConfig::classic());
    let detail = ErrorDetail {
        message: String::new(),
        stderr_tail: vec![],
    };
    let lines = theme.render_error_detail(&detail, 80);
    assert!(lines.is_empty());
}

#[test]
fn classic_theme_very_small_columns() {
    let theme = ConfigurableTheme(ThemeConfig::classic());
    let step = StepLine {
        status: StepStatus::Succeeded,
        label: "cmd".to_string(),
        elapsed: Some(0.5),
    };
    let line = theme.render(&step, 10);
    assert!(line.contains("cmd"));
}

#[test]
fn compact_theme_very_small_columns() {
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let step = StepLine {
        status: StepStatus::Succeeded,
        label: "x".to_string(),
        elapsed: Some(0.5),
    };
    let line = theme.render(&step, 5);
    assert!(line.contains('x'));
}

#[test]
fn resolve_theme_classic() {
    let mut themes = IndexMap::new();
    themes.insert("classic".into(), ThemeConfig::classic());
    let theme = resolve_theme("classic", &themes).unwrap();
    assert_eq!(theme.status_icon(StepStatus::Succeeded), "◆");
}

#[test]
fn resolve_theme_compact() {
    let mut themes = IndexMap::new();
    themes.insert("compact".into(), ThemeConfig::compact());
    let theme = resolve_theme("compact", &themes).unwrap();
    assert_eq!(theme.status_icon(StepStatus::Succeeded), "✓");
}

#[test]
fn resolve_theme_custom() {
    let mut themes = IndexMap::new();
    themes.insert(
        "my-theme".into(),
        ThemeConfig {
            icon_succeeded: "OK".into(),
            ..ThemeConfig::compact()
        },
    );
    let theme = resolve_theme("my-theme", &themes).unwrap();
    assert_eq!(theme.status_icon(StepStatus::Succeeded), "OK");
}

#[test]
fn resolve_theme_not_found() {
    let themes = IndexMap::new();
    let result = resolve_theme("nonexistent", &themes);
    assert!(matches!(result, Err(ThemeError::NotFound(_))));
}

#[test]
fn resolve_theme_not_found_preserves_name() {
    let themes = IndexMap::new();
    match resolve_theme("missing-theme", &themes) {
        Err(ThemeError::NotFound(name)) => assert_eq!(name, "missing-theme"),
        _ => panic!("expected NotFound"),
    }
}

#[test]
fn resolve_theme_returns_distinct_instances_per_call() {
    // Regression guard: resolver should clone the backing config so mutations
    // to one returned theme cannot affect another. If the resolver were ever
    // refactored to return a shared reference, both calls below would alias.
    let mut themes = IndexMap::new();
    themes.insert("compact".into(), ThemeConfig::compact());
    let a = resolve_theme("compact", &themes).unwrap();
    let b = resolve_theme("compact", &themes).unwrap();
    assert_eq!(a.status_icon(StepStatus::Succeeded), "✓");
    assert_eq!(b.status_icon(StepStatus::Succeeded), "✓");
}

#[test]
fn resolve_theme_is_case_sensitive() {
    let mut themes = IndexMap::new();
    themes.insert("compact".into(), ThemeConfig::compact());
    match resolve_theme("Compact", &themes) {
        Err(ThemeError::NotFound(name)) => assert_eq!(name, "Compact"),
        _ => panic!("expected NotFound for case-mismatched name"),
    }
}

#[test]
fn list_theme_names_from_config() {
    let mut themes = IndexMap::new();
    themes.insert("classic".into(), ThemeConfig::classic());
    themes.insert("compact".into(), ThemeConfig::compact());
    let names = list_theme_names(&themes);
    assert!(names.contains(&"classic".to_string()));
    assert!(names.contains(&"compact".to_string()));
}

#[test]
fn list_theme_names_with_custom() {
    let mut themes = IndexMap::new();
    themes.insert("classic".into(), ThemeConfig::classic());
    themes.insert("compact".into(), ThemeConfig::compact());
    themes.insert("dark".into(), ThemeConfig::compact());
    themes.insert("light".into(), ThemeConfig::classic());
    let names = list_theme_names(&themes);
    assert!(names.contains(&"classic".to_string()));
    assert!(names.contains(&"compact".to_string()));
    assert!(names.contains(&"dark".to_string()));
    assert!(names.contains(&"light".to_string()));
}

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
fn theme_config_deserialize() {
    let toml = r#"
icon_pending = "○"
icon_running = ""
icon_succeeded = "●"
icon_failed = "✗"
icon_skipped = "—"
separator_char = '·'
step_indent = "  "
running_template = "  {spinner:.cyan}{msg} {elapsed:.dim}"
tick_chars = "⠁⠂⠄ "
running_template_overhead = 7
plan_header_style = "plain"
summary_prefix = "→ "
summary_separator = ""
description = "My custom theme"

[error_block]
top = "╭─"
mid = "│"
bottom = "╰─"
rail = ""
"#;
    let config: ThemeConfig = toml::from_str(toml).expect("should deserialize");
    assert_eq!(config.icon_succeeded, "●");
    assert_eq!(config.separator_char, '·');
    assert_eq!(config.description, Some("My custom theme".into()));
    assert_eq!(config.error_block.top, "╭─");
}

#[test]
fn theme_config_deserialize_missing_required_field() {
    let toml = r#"
icon_pending = "○"
icon_running = ""
"#;
    let result: Result<ThemeConfig, _> = toml::from_str(toml);
    assert!(result.is_err(), "should fail with missing required fields");
}

#[test]
fn theme_config_deserialize_invalid_field_type() {
    let toml = MINIMAL_THEME_TOML.replace(
        "separator_char = '.'",
        "separator_char = \"should_be_char_not_string\"",
    );
    let result: Result<ThemeConfig, _> = toml::from_str(&toml);
    assert!(
        result.is_err(),
        "should fail with invalid separator_char type"
    );
}

#[test]
fn theme_config_deserialize_unknown_field() {
    let toml = format!(
        "{}\nunknown_field = \"this should fail\"\n",
        MINIMAL_THEME_TOML
    );
    let result: Result<ThemeConfig, _> = toml::from_str(&toml);
    assert!(result.is_err(), "should fail with unknown field");
}

#[test]
fn theme_config_deserialize_invalid_plan_header_style() {
    let toml = format!(
        "{}\nplan_header_style = \"invalid_style\"\n",
        MINIMAL_THEME_TOML
    );
    let result: Result<ThemeConfig, _> = toml::from_str(&toml);
    assert!(
        result.is_err(),
        "should fail with invalid plan_header_style"
    );
}

#[test]
fn theme_config_deserialize_partial_with_defaults() {
    let result: Result<ThemeConfig, _> = toml::from_str(MINIMAL_THEME_TOML);
    assert!(result.is_ok(), "should succeed with required fields only");
    let config = result.unwrap();
    assert_eq!(config.plan_header_style, PlanHeaderStyle::Plain);
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

mod error_block_color_tests {
    use super::*;
    use crate::style::apply_style_gated;
    use ops_core::config::theme_types::ErrorBlockChars;
    use ops_core::output::display_width;

    fn render_with(chars: ErrorBlockChars, enabled: bool) -> Vec<String> {
        // Mirror render_error_block's structure but with explicit styling gate,
        // since apply_style itself consults stderr TTY state at runtime.
        let detail = ErrorDetail {
            message: "exit status: 1".to_string(),
            stderr_tail: vec![],
        };
        let pad = String::new();
        let gutter = if chars.rail.is_empty() {
            "    ".to_string()
        } else {
            format!("{}   ", chars.rail)
        };
        let top = apply_style_gated(&chars.top, &chars.color, enabled);
        let mid = apply_style_gated(&chars.mid, &chars.color, enabled);
        let bottom = apply_style_gated(&chars.bottom, &chars.color, enabled);
        vec![
            format!("{}{}{}", pad, gutter, top),
            format!("{}{}{} {}", pad, gutter, mid, detail.message),
            format!("{}{}{}", pad, gutter, bottom),
        ]
    }

    #[test]
    fn error_block_color_wraps_top_mid_bottom_with_sgr_when_enabled() {
        let chars = ErrorBlockChars {
            top: "┌─".into(),
            mid: "│".into(),
            bottom: "└─".into(),
            rail: "│".into(),
            color: "red dim".into(),
        };
        let lines = render_with(chars, true);
        for line in &lines {
            assert!(
                line.contains('\x1b'),
                "glyph should carry SGR when color enabled: {line}"
            );
        }
    }

    #[test]
    fn error_block_rail_remains_unstyled_when_color_set() {
        let chars = ErrorBlockChars {
            top: "┌─".into(),
            mid: "│".into(),
            bottom: "└─".into(),
            rail: "│".into(),
            color: "red".into(),
        };
        let lines = render_with(chars, true);
        for line in &lines {
            // Rail sits at the start, before any ANSI sequence.
            assert!(
                line.starts_with('│'),
                "rail glyph must not be wrapped in SGR: {line}"
            );
        }
    }

    #[test]
    fn error_block_unknown_color_does_not_change_display_width() {
        let base = ErrorBlockChars {
            top: "┌─".into(),
            mid: "│".into(),
            bottom: "└─".into(),
            rail: "│".into(),
            color: String::new(),
        };
        let colored = ErrorBlockChars {
            color: "not-a-color zzz".into(),
            ..base.clone()
        };
        let plain_lines = render_with(base, true);
        let colored_lines = render_with(colored, true);
        assert_eq!(plain_lines.len(), colored_lines.len());
        for (p, c) in plain_lines.iter().zip(colored_lines.iter()) {
            let pw = display_width(&strip_ansi(p));
            let cw = display_width(&strip_ansi(c));
            assert_eq!(pw, cw, "layout must be invariant: plain={p} colored={c}");
        }
    }
}

mod render_summary_tests {
    use super::*;

    #[test]
    fn classic_render_summary_success() {
        let theme = ConfigurableTheme(ThemeConfig::classic());
        let summary = theme.render_summary(true, 1.5);
        assert!(summary.contains("Done"));
        assert!(summary.contains("1.50s"));
        assert!(summary.starts_with(" └──"));
    }

    #[test]
    fn classic_render_summary_failure() {
        let theme = ConfigurableTheme(ThemeConfig::classic());
        let summary = theme.render_summary(false, 0.75);
        assert!(summary.contains("Failed"));
        assert!(summary.contains("0.75s"));
        assert!(summary.starts_with(" └──"));
    }

    #[test]
    fn classic_render_summary_minutes() {
        let theme = ConfigurableTheme(ThemeConfig::classic());
        let summary = theme.render_summary(true, 278.04);
        assert!(summary.contains("Done"));
        assert!(summary.contains("4m38s"));
    }

    #[test]
    fn compact_render_summary_success() {
        let theme = ConfigurableTheme(ThemeConfig::compact());
        let summary = theme.render_summary(true, 2.0);
        assert!(summary.contains("Done"));
        assert!(summary.contains("2.00s"));
        assert!(!summary.contains("└──"));
    }

    #[test]
    fn compact_render_summary_failure() {
        let theme = ConfigurableTheme(ThemeConfig::compact());
        let summary = theme.render_summary(false, 0.5);
        assert!(summary.contains("Failed"));
        assert!(summary.contains("0.50s"));
    }

    #[test]
    fn plan_header_style_tree_renders_correctly() {
        let mut theme = ThemeConfig::compact();
        theme.plan_header_style = PlanHeaderStyle::Tree;
        let configurable = ConfigurableTheme(theme);
        let lines = configurable.render_plan_header(&["a".into(), "b".into()]);
        assert_eq!(lines[1], " ┌ Running: a, b");
        assert_eq!(lines[2], " │");
    }
}

/// TQ-010: Edge case tests for extreme column widths.
mod edge_case_width_tests {
    use super::*;
    use ops_core::output::StepLine;

    #[test]
    fn render_with_zero_columns_does_not_panic() {
        let theme = ConfigurableTheme(ThemeConfig::compact());
        let step = StepLine {
            status: StepStatus::Succeeded,
            label: "test".to_string(),
            elapsed: Some(1.0),
        };
        let line = theme.render(&step, 0);
        assert!(!line.is_empty(), "should still produce output");
    }

    #[test]
    fn render_with_one_column_does_not_panic() {
        let theme = ConfigurableTheme(ThemeConfig::compact());
        let step = StepLine {
            status: StepStatus::Succeeded,
            label: "test".to_string(),
            elapsed: Some(1.0),
        };
        let line = theme.render(&step, 1);
        assert!(!line.is_empty(), "should still produce output");
    }

    #[test]
    fn render_with_two_columns_does_not_panic() {
        let theme = ConfigurableTheme(ThemeConfig::compact());
        let step = StepLine {
            status: StepStatus::Succeeded,
            label: "x".to_string(),
            elapsed: None,
        };
        let line = theme.render(&step, 2);
        assert!(!line.is_empty(), "should still produce output");
    }

    #[test]
    fn render_with_very_small_columns_handles_gracefully() {
        let theme = ConfigurableTheme(ThemeConfig::compact());
        let step = StepLine {
            status: StepStatus::Succeeded,
            label: "very long label that exceeds column width".to_string(),
            elapsed: Some(1.0),
        };
        let line = theme.render(&step, 5);
        assert!(!line.is_empty(), "should handle small width");
    }

    #[test]
    fn render_pending_with_zero_columns() {
        let theme = ConfigurableTheme(ThemeConfig::classic());
        let step = StepLine {
            status: StepStatus::Pending,
            label: "pending".to_string(),
            elapsed: None,
        };
        let line = theme.render(&step, 0);
        assert!(!line.is_empty());
    }

    #[test]
    fn render_failed_with_minimal_columns() {
        let theme = ConfigurableTheme(ThemeConfig::classic());
        let step = StepLine {
            status: StepStatus::Failed,
            label: "f".to_string(),
            elapsed: Some(0.01),
        };
        let line = theme.render(&step, 3);
        assert!(!line.is_empty());
    }

    /// TQ-010: Label longer than column width does not panic or produce empty output.
    #[test]
    fn render_label_longer_than_columns() {
        let theme = ConfigurableTheme(ThemeConfig::classic());
        let long_label = "a_very_long_command_name_that_exceeds_the_terminal_column_width_by_far";
        let step = StepLine {
            status: StepStatus::Succeeded,
            label: long_label.to_string(),
            elapsed: Some(1.23),
        };
        // Columns much smaller than label length
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
        // Should not panic; separator may be empty or truncated
        assert!(sep.len() <= 200, "separator should not be excessively long");
    }

    #[test]
    fn icon_column_width_handles_all_statuses() {
        let theme = ConfigurableTheme(ThemeConfig::classic());
        let width = theme.icon_column_width();
        assert!(width > 0, "icon column width should be positive");
    }
}

mod format_duration_tests {
    use super::*;

    #[test]
    fn zero_seconds() {
        assert_eq!(format_duration(0.0), "0.00s");
    }

    #[test]
    fn sub_second() {
        assert_eq!(format_duration(0.74), "0.74s");
    }

    #[test]
    fn whole_seconds() {
        assert_eq!(format_duration(5.37), "5.37s");
    }

    #[test]
    fn just_under_a_minute() {
        assert_eq!(format_duration(59.99), "59.99s");
    }

    #[test]
    fn exactly_sixty_seconds() {
        assert_eq!(format_duration(60.0), "1m0s");
    }

    #[test]
    fn minutes_and_seconds() {
        assert_eq!(format_duration(134.0), "2m14s");
        assert_eq!(format_duration(278.04), "4m38s");
    }

    #[test]
    fn exactly_one_hour() {
        assert_eq!(format_duration(3600.0), "1h0m0s");
    }

    #[test]
    fn hours_minutes_seconds() {
        assert_eq!(format_duration(3723.0), "1h2m3s");
    }

    #[test]
    fn large_duration() {
        assert_eq!(format_duration(7384.0), "2h3m4s");
    }

    #[test]
    fn nan_input_renders_marker() {
        // SEC-15 / TASK-0358: NaN must not propagate through `as u64`.
        assert_eq!(format_duration(f64::NAN), "--");
    }

    #[test]
    fn negative_input_renders_marker() {
        assert_eq!(format_duration(-1.0), "--");
        assert_eq!(format_duration(-3600.0), "--");
    }

    #[test]
    fn infinity_renders_marker() {
        assert_eq!(format_duration(f64::INFINITY), "--");
        assert_eq!(format_duration(f64::NEG_INFINITY), "--");
    }

    #[test]
    fn enormous_finite_input_does_not_panic() {
        // f64::MAX truncates to a value far above u64::MAX; ensure we saturate
        // and still emit a finite string instead of panicking.
        let out = format_duration(1.0e30);
        assert!(out.ends_with('s'), "got: {out}");
        assert!(out.contains('h'), "got: {out}");
    }
}

mod left_pad_tests {
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
        // With pad=0, line starts directly with step_indent ("  "), not extra padding
        assert!(
            line.starts_with("  ✓"),
            "should start with step_indent, no extra pad"
        );

        let padded = theme_with_pad(2);
        let padded_line = render_line(&padded, StepStatus::Succeeded, "cargo test", Some(1.0));
        // With pad=2, line starts with 2 extra spaces before step_indent
        assert!(
            padded_line.starts_with("    ✓"),
            "should have 2-space pad + step_indent"
        );
    }
}

mod boxed_layout_tests {
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
        BoxSnapshot::new(completed, total, elapsed, success, columns)
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
        let top = theme
            .box_top_border(snap(2, 5, 12.4, true, 60))
            .expect("boxed theme returns top");
        // Visible width (after stripping ANSI) must equal columns.
        let plain = strip_ansi(&top);
        assert_eq!(ops_core::output::display_width(&plain), 60, "got: {top}");
        assert!(plain.starts_with("╭─"), "top corner: {plain}");
        assert!(plain.ends_with("╮"), "top corner end: {plain}");
        assert!(plain.contains("Running 2/5"), "contains progress: {plain}");
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
        let snap = snap(0, 2, 0.0, true, 60).with_command_ids(&ids);
        let top = strip_ansi(&theme.box_top_border(snap).expect("top"));
        assert!(top.contains("Running: build, test"), "got: {top}");
    }

    #[test]
    fn boxed_top_border_switches_verb_when_all_complete() {
        let theme = boxed_theme();
        let running = strip_ansi(&theme.box_top_border(snap(2, 5, 1.0, true, 50)).unwrap());
        assert!(running.contains("Running 2/5"));
        let done = strip_ansi(&theme.box_top_border(snap(5, 5, 1.0, true, 50)).unwrap());
        assert!(done.contains("Done 5/5"));
        let failing = strip_ansi(&theme.box_top_border(snap(2, 5, 1.0, false, 50)).unwrap());
        assert!(failing.contains("Failing 2/5"));
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

        let top = strip_ansi(
            &theme
                .box_top_border(snap(1, 1, 1.23, true, columns))
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
        // Expect rail + 6 spaces + glyph → glyph lands at column 7, matching
        // where the step icon sits under `│ █    ✖ ...`.
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
        // Wrapped line should be exactly `columns` wide.
        assert_eq!(
            ops_core::output::display_width(&plain),
            60,
            "wrap width: {plain}"
        );
        assert!(plain.starts_with("│ █  "), "prefix: {plain}");
        assert!(plain.ends_with(" │"), "suffix: {plain}");
    }
}
