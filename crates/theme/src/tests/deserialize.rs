//! `ThemeConfig` TOML deserialization, including required-field and
//! unknown-field rejection.

use super::*;

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
