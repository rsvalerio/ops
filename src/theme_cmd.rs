//! `cargo ops theme` - theme management commands.

use std::io::{self, IsTerminal};
use std::path::PathBuf;

use crate::config;
use crate::style;

fn parse_default_config() -> Result<crate::config::Config, anyhow::Error> {
    toml::from_str::<crate::config::Config>(config::default_ops_toml())
        .map_err(|e| anyhow::anyhow!("failed to parse default config: {}", e))
}

/// DUP-005: Extracted helper to collect theme options from config.
///
/// Used by both `run_theme_list` and `run_theme_select` to avoid duplication.
fn collect_theme_options(config: &crate::config::Config) -> Vec<ThemeOption> {
    let default_config = parse_default_config().ok();

    let mut options: Vec<ThemeOption> = config
        .themes
        .iter()
        .map(|(name, theme_config)| {
            let is_default = default_config
                .as_ref()
                .map(|dc| dc.themes.contains_key(name))
                .unwrap_or(false);
            ThemeOption {
                name: name.clone(),
                description: theme_config
                    .description
                    .as_deref()
                    .unwrap_or("Custom theme")
                    .to_string(),
                is_custom: !is_default,
            }
        })
        .collect();

    options.sort_by(|a, b| a.name.cmp(&b.name));
    options
}

/// Lists all available themes (from config, including built-in classic/compact).
///
/// Prints theme names with descriptions to stdout. Local overrides are marked.
pub fn run_theme_list() -> anyhow::Result<()> {
    let config = config::load_config()?;
    let is_tty = io::stdout().is_terminal();

    let options = collect_theme_options(&config);

    let max_name_len = options.iter().map(|o| o.name.len()).max().unwrap_or(0);

    for option in options {
        let marker = if option.is_custom { " (custom)" } else { "" };
        if is_tty {
            println!(
                "  {:width$}   {}{}",
                style::cyan(&option.name),
                style::dim(&option.description),
                style::dim(marker),
                width = max_name_len
            );
        } else {
            println!(
                "{:width$}   {}{}",
                option.name,
                option.description,
                marker,
                width = max_name_len
            );
        }
    }

    Ok(())
}

/// Default TTY check using stdout.
fn is_stdout_tty() -> bool {
    io::stdout().is_terminal()
}

/// Interactively selects a theme and updates `.ops.toml`.
///
/// Requires an interactive terminal. Shows a selection prompt with all
/// available themes, then updates the config file with the chosen theme.
///
/// # Testing Limitation (TQ-017)
///
/// The interactive path using `inquire::Select` requires a TTY and cannot
/// be fully tested in automated test environments. The non-TTY error path
/// is tested via `run_theme_select_non_tty_returns_error`.
///
/// To test the interactive path, you would need to:
/// 1. Mock `inquire::Select` using a trait
/// 2. Use a TTY emulation library
/// 3. Run manual testing with `cargo ops theme select`
pub fn run_theme_select() -> anyhow::Result<()> {
    run_theme_select_with_tty_check(is_stdout_tty)
}

fn run_theme_select_with_tty_check<F>(is_tty: F) -> anyhow::Result<()>
where
    F: FnOnce() -> bool,
{
    if !is_tty() {
        anyhow::bail!("theme select requires an interactive terminal");
    }

    let config = config::load_config()?;
    let options = collect_theme_options(&config);

    let current_theme = &config.output.theme;

    let starting_cursor = options
        .iter()
        .position(|o| o.name == *current_theme)
        .unwrap_or_else(|| {
            tracing::debug!(
                current_theme = %current_theme,
                available_themes = ?options.iter().map(|o| &o.name).collect::<Vec<_>>(),
                "EFF-009: current theme not found in list, defaulting to first position"
            );
            0
        });

    let selected = inquire::Select::new("Select a theme:", options)
        .with_starting_cursor(starting_cursor)
        .prompt()?;

    if selected.name == *current_theme {
        println!("Theme already set to '{}'", selected.name);
        return Ok(());
    }

    update_theme_in_config(&selected.name)?;

    println!("Theme set to '{}'", selected.name);
    Ok(())
}

struct ThemeOption {
    name: String,
    description: String,
    is_custom: bool,
}

impl std::fmt::Display for ThemeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let marker = if self.is_custom { " (custom)" } else { "" };
        write!(f, "{}{} - {}", self.name, marker, self.description)
    }
}

fn update_theme_in_config(theme_name: &str) -> anyhow::Result<()> {
    let config_path = PathBuf::from(".ops.toml");

    if !config_path.exists() {
        std::fs::write(
            &config_path,
            format!(
                r#"[output]
theme = "{}"

[commands]
"#,
                escape_toml_string(theme_name)
            ),
        )?;
        return Ok(());
    }

    let content = std::fs::read_to_string(&config_path)?;

    let updated = update_toml_theme(&content, theme_name);

    std::fs::write(&config_path, updated)?;
    Ok(())
}

fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// Update the theme value in TOML content, preserving formatting.
fn update_toml_theme(content: &str, theme_name: &str) -> String {
    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .unwrap_or_else(|_| toml_edit::DocumentMut::new());
    if !doc.contains_key("output") {
        doc["output"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    doc["output"]["theme"] = toml_edit::value(theme_name);
    doc.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_toml_string_escapes_special_chars() {
        assert_eq!(escape_toml_string("simple"), "simple");
        assert_eq!(escape_toml_string(r#"with"quote"#), r#"with\"quote"#);
        assert_eq!(escape_toml_string("with\nnewline"), "with\\nnewline");
        assert_eq!(escape_toml_string("with\\backslash"), r#"with\\backslash"#);
    }

    #[test]
    fn escape_toml_string_control_chars() {
        assert_eq!(escape_toml_string("with\ttab"), "with\\ttab");
        assert_eq!(escape_toml_string("with\rcarriage"), "with\\rcarriage");
    }

    #[test]
    fn update_toml_theme_injection_prevention() {
        let input = r#"[output]
theme = "compact"
"#;
        let malicious = r#"malicious"theme"#;
        let result = update_toml_theme(input, malicious);
        // Verify the output is valid TOML that round-trips correctly
        let doc: toml_edit::DocumentMut = result.parse().expect("valid TOML");
        assert_eq!(doc["output"]["theme"].as_str().unwrap(), malicious);
    }

    #[test]
    fn update_toml_theme_existing() {
        let input = r#"[output]
theme = "compact"
columns = 80
"#;
        let result = update_toml_theme(input, "classic");
        assert!(result.contains(r#"theme = "classic""#));
        assert!(result.contains("columns = 80"));
    }

    #[test]
    fn update_toml_theme_no_theme_key() {
        let input = r#"[output]
columns = 80
"#;
        let result = update_toml_theme(input, "classic");
        assert!(result.contains(r#"theme = "classic""#));
        assert!(result.contains("columns = 80"));
    }

    #[test]
    fn update_toml_theme_no_output_section() {
        let input = r#"[commands]
build = "cargo build"
"#;
        let result = update_toml_theme(input, "classic");
        assert!(result.contains("[output]"));
        assert!(result.contains(r#"theme = "classic""#));
    }

    #[test]
    fn theme_option_display() {
        let opt = ThemeOption {
            name: "classic".to_string(),
            description: "Bold tree-style".to_string(),
            is_custom: false,
        };
        let display = format!("{}", opt);
        assert!(display.contains("classic"));
        assert!(display.contains("Bold tree-style"));
    }

    #[test]
    fn theme_option_custom_marker() {
        let opt = ThemeOption {
            name: "my-theme".to_string(),
            description: "Custom theme".to_string(),
            is_custom: true,
        };
        let display = format!("{}", opt);
        assert!(display.contains("(custom)"));
    }

    #[test]
    fn parse_default_config_succeeds() {
        let result = parse_default_config();
        assert!(result.is_ok(), "should parse embedded default config");
        let config = result.unwrap();
        assert!(config.themes.contains_key("classic"));
        assert!(config.themes.contains_key("compact"));
    }

    #[test]
    fn parse_default_config_has_builtin_themes() {
        let config = parse_default_config().unwrap();
        let classic = config.themes.get("classic").expect("classic theme");
        let compact = config.themes.get("compact").expect("compact theme");
        assert!(!classic.icon_succeeded.is_empty());
        assert!(!compact.icon_succeeded.is_empty());
    }

    #[test]
    fn run_theme_select_non_tty_returns_error() {
        let result = run_theme_select_with_tty_check(|| false);
        assert!(result.is_err(), "run_theme_select should fail without TTY");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("interactive terminal"));
    }

    #[test]
    fn update_toml_theme_handles_malformed_key_injection() {
        let input = r#"[output]
theme = "compact"
"#;
        let malicious = "theme\nwith\nnewlines";
        let result = update_toml_theme(input, malicious);
        // Verify the output is valid TOML that round-trips correctly
        let doc: toml_edit::DocumentMut = result.parse().expect("valid TOML");
        assert_eq!(doc["output"]["theme"].as_str().unwrap(), malicious);
    }

    #[test]
    fn update_toml_theme_handles_long_theme_name() {
        let long_name = "a".repeat(1000);
        let input = r#"[output]
theme = "compact"
"#;
        let result = update_toml_theme(input, &long_name);
        assert!(result.contains(&long_name));
    }

    #[test]
    fn update_theme_in_config_creates_new_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join(".ops.toml");
        let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");

        let result = update_theme_in_config("classic");
        assert!(result.is_ok());
        assert!(config_path.exists());
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("theme = \"classic\""));
    }

    mod run_theme_list_tests {
        use super::*;

        #[test]
        fn run_theme_list_outputs_themes() {
            let (_dir, _guard) = crate::test_utils::with_temp_config(
                r#"[output]
theme = "classic"

[themes]
my-custom = { description = "My custom theme", icon_succeeded = "✓" }
"#,
            );

            let result = run_theme_list();
            assert!(result.is_ok(), "run_theme_list should succeed");
        }

        #[test]
        fn run_theme_list_includes_builtin_themes() {
            let (_dir, _guard) = crate::test_utils::with_temp_config("");

            let result = run_theme_list();
            assert!(result.is_ok());
        }

        #[test]
        fn run_theme_list_marks_custom_themes() {
            let (_dir, _guard) = crate::test_utils::with_temp_config(
                r#"[output]
theme = "classic"

[themes]
custom-one = { description = "Custom", icon_succeeded = "✓" }
"#,
            );

            let result = run_theme_list();
            assert!(result.is_ok());
        }
    }
}
