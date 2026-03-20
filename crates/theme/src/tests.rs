//! Tests for theme types and rendering.

use super::*;
use indexmap::IndexMap;
use ops_core::output::{ErrorDetail, StepLine, StepStatus};

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
    let lines = theme.render_plan_header(&ids, 80);
    assert_eq!(lines.len(), 3);
    assert!(lines[0].is_empty(), "upper space");
    assert_eq!(lines[1], " ┌ Running: build, clippy, test");
    assert_eq!(lines[2], " │");
}

#[test]
fn compact_plan_header_plain() {
    let theme = ConfigurableTheme(ThemeConfig::compact());
    let ids = vec!["build".into(), "test".into()];
    let lines = theme.render_plan_header(&ids, 80);
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
        let lines = configurable.render_plan_header(&["a".into(), "b".into()], 80);
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
        let lines = theme.render_plan_header(&["build".into()], 80);
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
