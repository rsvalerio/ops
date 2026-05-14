//! `cargo ops theme` - theme management commands.

use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use std::sync::OnceLock;

use anyhow::Context;
use ops_core::config;
use ops_core::config::ensure_table;
use ops_core::output::display_width;

fn parse_default_config() -> Result<ops_core::config::Config, anyhow::Error> {
    toml::from_str::<ops_core::config::Config>(config::default_ops_toml())
        .context("failed to parse default config")
}

/// Cache the set of built-in theme names parsed from the embedded default
/// config. The previous shape rebuilt the entire `Config` on every
/// `collect_theme_options` call (twice — once each from `run_theme_list`
/// and `run_theme_select`) only to compute a `contains_key` boolean.
/// Aligned with the OnceLock discipline used by
/// `crates/core/src/expand.rs::TMPDIR_DISPLAY` and
/// `crates/core/src/text.rs::MANIFEST_MAX_BYTES`.
static BUILTIN_THEME_NAMES: OnceLock<HashSet<String>> = OnceLock::new();

/// Panic message used when the embedded default config
/// fails to parse. The embedded TOML is compiled into the binary
/// (`config::default_ops_toml()`), so a parse failure is a compile-time
/// invariant violation — not a runtime condition. Crashing loud at startup
/// surfaces a broken default immediately instead of silently degrading to
/// an empty set and mislabelling every built-in theme as `(custom)` for
/// the process lifetime.
const EMBEDDED_DEFAULT_CONFIG_PARSE_EXPECT: &str = "embedded default config must parse";

fn builtin_theme_names() -> &'static HashSet<String> {
    BUILTIN_THEME_NAMES.get_or_init(|| {
        parse_default_config()
            .expect(EMBEDDED_DEFAULT_CONFIG_PARSE_EXPECT)
            .themes
            .into_keys()
            .collect()
    })
}

/// Extracted helper to collect theme options from config.
///
/// Used by both `run_theme_list` and `run_theme_select` to avoid duplication.
fn collect_theme_options(config: &ops_core::config::Config) -> Vec<ThemeOption> {
    let builtins = builtin_theme_names();

    let mut options: Vec<ThemeOption> = config
        .themes
        .iter()
        .map(|(name, theme_config)| ThemeOption {
            name: name.clone(),
            description: theme_config
                .description
                .as_deref()
                .unwrap_or("Custom theme")
                .to_string(),
            is_custom: !builtins.contains(name),
        })
        .collect();

    options.sort_by(|a, b| a.name.cmp(&b.name));
    options
}

/// Lists all available themes (from config, including built-in classic/compact).
///
/// Prints theme names with descriptions to stdout. Local overrides are marked.
pub fn run_theme_list(config: &config::Config) -> anyhow::Result<()> {
    run_theme_list_to(config, &mut std::io::stdout())
}

fn run_theme_list_to(config: &config::Config, w: &mut dyn Write) -> anyhow::Result<()> {
    let options = collect_theme_options(config);

    // Theme names are user-supplied via `[themes.<name>]` and may contain
    // CJK / emoji / combining marks. Mirror the tools_cmd / help.rs
    // alignment pattern: measure by `display_width` so wide characters and
    // emoji align with ASCII names in the same column. Row styling (cyan
    // name, dim description + marker, padded to `max_name_width`) routes
    // through [`crate::row::write_list_row`] so the `theme list` and
    // `tools list` surfaces share a single colour / padding policy.
    let max_name_width = options
        .iter()
        .map(|o| display_width(&o.name))
        .max()
        .unwrap_or(0);

    for option in options {
        let marker = theme_custom_marker(option.is_custom);
        crate::row::write_list_row(
            w,
            crate::row::ListRow {
                leading: "  ",
                name: &option.name,
                name_width: max_name_width,
                gap: "   ",
                description: &option.description,
                suffix: marker,
            },
        )?;
    }

    Ok(())
}

/// Interactively selects a theme and updates `.ops.toml`.
///
/// Requires an interactive terminal. Shows a selection prompt with all
/// available themes, then updates the config file with the chosen theme.
///
/// # Testing Limitation
///
/// The interactive path using `inquire::Select` requires a TTY and cannot
/// be fully tested in automated test environments. The non-TTY error path
/// is tested via `run_theme_select_non_tty_returns_error`.
///
/// To test the interactive path, you would need to:
/// 1. Mock `inquire::Select` using a trait
/// 2. Use a TTY emulation library
/// 3. Run manual testing with `cargo ops theme select`
pub fn run_theme_select(config: &config::Config, workspace_root: &Path) -> anyhow::Result<()> {
    run_theme_select_with_tty_check(
        config,
        workspace_root,
        &mut std::io::stdout(),
        crate::tty::is_stdout_tty,
    )
}

fn run_theme_select_with_tty_check<F>(
    config: &config::Config,
    workspace_root: &Path,
    w: &mut dyn Write,
    is_tty: F,
) -> anyhow::Result<()>
where
    F: FnOnce() -> bool,
{
    crate::tty::require_tty_with("theme select", is_tty)?;

    let options = collect_theme_options(config);

    let current_theme = &config.output.theme;

    let starting_cursor = options
        .iter()
        .position(|o| o.name == *current_theme)
        .unwrap_or_else(|| {
            tracing::debug!(
                current_theme = %current_theme,
                available_themes = ?options.iter().map(|o| &o.name).collect::<Vec<_>>(),
                "current theme not found in list, defaulting to first position"
            );
            0
        });

    let selected = inquire::Select::new("Select a theme:", options)
        .with_starting_cursor(starting_cursor)
        .prompt()?;

    write_theme_select_result(w, &selected.name, current_theme, workspace_root)
}

/// Post-prompt result rendering split out so unit tests
/// can drive the happy-path message text (both "already set" and "set to")
/// against a `Vec<u8>` buffer without spinning up a TTY for the picker.
fn write_theme_select_result(
    w: &mut dyn Write,
    selected_name: &str,
    current_theme: &str,
    workspace_root: &Path,
) -> anyhow::Result<()> {
    if selected_name == current_theme {
        writeln!(w, "Theme already set to '{}'", selected_name)?;
        return Ok(());
    }

    update_theme_in_config(workspace_root, selected_name)?;

    writeln!(w, "Theme set to '{}'", selected_name)?;
    Ok(())
}

struct ThemeOption {
    name: String,
    description: String,
    is_custom: bool,
}

/// Shared "(custom)" suffix used by both surfaces that
/// render `ThemeOption` — the `theme list` table (`run_theme_list_to`) and the
/// `theme select` picker (`Display`). Centralising prevents the two surfaces
/// from drifting on marker text / position.
fn theme_custom_marker(is_custom: bool) -> &'static str {
    if is_custom {
        " (custom)"
    } else {
        ""
    }
}

impl std::fmt::Display for ThemeOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Mirror the `theme list` row order
        // (`name   description{marker}`) so a user running both surfaces sees
        // the same shape (sans column padding / colour) — the list adds
        // alignment padding around `name` and TTY colour, the picker omits
        // both since inquire's prompt does its own selection-cursor styling.
        write!(
            f,
            "{}   {}{}",
            self.name,
            self.description,
            theme_custom_marker(self.is_custom),
        )
    }
}

/// Anchor the saved `.ops.toml` to the same root the rest of the CLI
/// threads through (`crate::cwd()` → `Stack::resolve(...)`), so running
/// `ops theme select` from a subdirectory writes the file alongside the
/// loaded config rather than next to the user's cwd. Mirrors the
/// `save_about_fields` fix in about_cmd.
fn update_theme_in_config(workspace_root: &Path, theme_name: &str) -> anyhow::Result<()> {
    let config_path = workspace_root.join(".ops.toml");
    config::edit_ops_toml(&config_path, |doc| set_theme(doc, theme_name))
}

/// Route the `[output]` lookup through the shared
/// `ensure_table` helper so a legacy/malformed `.ops.toml` containing
/// `output = "classic"` (or any non-table value) surfaces a clean
/// `anyhow::Error` instead of panicking inside `toml_edit`'s `IndexMut`.
fn set_theme(doc: &mut toml_edit::DocumentMut, theme_name: &str) -> anyhow::Result<()> {
    let output = ensure_table(doc, "output")?;
    output.insert("theme", toml_edit::value(theme_name));
    Ok(())
}

/// Update the theme value in TOML content, preserving formatting.
#[cfg(test)]
fn update_toml_theme(content: &str, theme_name: &str) -> String {
    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .expect("test input must be valid TOML");
    set_theme(&mut doc, theme_name).expect("test input must contain a table at [output] or none");
    doc.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The injection-defence contract is that a
    /// malicious theme name cannot break out of its string-value slot and
    /// inject new TOML structure. Pin three observable properties:
    ///
    ///   1. surrounding `[output]` keys survive unchanged,
    ///   2. the rewritten document parses without errors,
    ///   3. `[output]` contains only the expected keys — no payload-leaked
    ///      sibling entries from a successful injection.
    ///
    /// A regression that pasted the raw payload byte-for-byte (breaking
    /// neighbour keys) would have silently passed the prior round-trip-only
    /// assertion.
    #[test]
    fn update_toml_theme_injection_prevention() {
        let input = r#"[output]
theme = "compact"
columns = 80
"#;
        let malicious = r#"malicious"theme"#;
        let result = update_toml_theme(input, malicious);
        let doc: toml_edit::DocumentMut = result.parse().expect("rewritten TOML must parse");
        assert_eq!(doc["output"]["theme"].as_str().unwrap(), malicious);
        assert_eq!(
            doc["output"]["columns"].as_integer().unwrap(),
            80,
            "sibling key under [output] must survive the rewrite unchanged"
        );
        let output_table = doc["output"]
            .as_table()
            .expect("[output] must remain a table");
        let keys: Vec<&str> = output_table.iter().map(|(k, _)| k).collect();
        assert_eq!(
            keys,
            vec!["theme", "columns"],
            "[output] must contain only the expected keys — no payload-leaked siblings"
        );
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

    /// The `theme list` row and the `theme select` picker
    /// (Display) render `ThemeOption` with the same shape — `name`, then
    /// description, then `(custom)` marker. List adds column padding and TTY
    /// colour around the name; the picker omits both. The (name, description,
    /// marker) ordering and the `   ` separator are the shared contract that
    /// keeps the two operator-facing surfaces from drifting.
    #[test]
    fn theme_option_display_matches_list_row_shape() {
        let mut config = ops_core::config::Config::empty();
        config.themes.insert(
            "my-theme".to_string(),
            ops_core::config::theme_types::ThemeConfig {
                description: Some("Custom theme".to_string()),
                ..ops_core::config::theme_types::ThemeConfig::classic()
            },
        );

        let mut buf = Vec::new();
        run_theme_list_to(&config, &mut buf).expect("run_theme_list_to");
        let list_output = String::from_utf8(buf).unwrap();

        let opt = ThemeOption {
            name: "my-theme".to_string(),
            description: "Custom theme".to_string(),
            is_custom: true,
        };
        let display = format!("{}", opt);

        // Both surfaces produce the same `name   description (custom)` ordering.
        assert_eq!(display, "my-theme   Custom theme (custom)");
        let list_line = list_output
            .lines()
            .find(|l| l.contains("my-theme"))
            .expect("my-theme row in list output");
        // List row carries the same suffix order: description before marker.
        let desc_idx = list_line.find("Custom theme").expect("description");
        let marker_idx = list_line.find("(custom)").expect("marker");
        assert!(
            marker_idx > desc_idx,
            "list row must render description before the (custom) marker like Display: {list_line}"
        );
    }

    /// Pin the loud-failure contract. The embedded
    /// default config is compiled in, so a parse failure is a compile-time
    /// invariant violation. `builtin_theme_names` must panic with the
    /// named message rather than silently degrading to an empty set and
    /// mislabelling every built-in theme as `(custom)` for the rest of
    /// the process lifetime.
    #[test]
    fn embedded_default_config_parse_failure_panic_message_is_named() {
        // The constant is the load-bearing contract — any rename would
        // break operator-visible diagnostics on a broken default.
        assert_eq!(
            EMBEDDED_DEFAULT_CONFIG_PARSE_EXPECT,
            "embedded default config must parse",
        );
        // And on a healthy build the embedded TOML must parse, so the
        // expect path is never taken in practice.
        assert!(
            parse_default_config().is_ok(),
            "embedded default config is expected to parse on a healthy build"
        );
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
        let config = ops_core::config::Config::empty();
        let dir = tempfile::tempdir().expect("tempdir");
        let mut buf = Vec::new();
        let result = run_theme_select_with_tty_check(&config, dir.path(), &mut buf, || false);
        assert!(result.is_err(), "run_theme_select should fail without TTY");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("interactive terminal"));
        assert!(buf.is_empty(), "non-TTY path must not write anything");
    }

    /// Pin the happy-path message text against an
    /// in-memory buffer. The picker itself still requires a TTY, but the
    /// post-prompt rendering is deterministic format-and-write — exercising
    /// `write_theme_select_result` directly covers both branches without
    /// emulating a terminal.
    #[test]
    fn write_theme_select_result_already_set_message() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut buf = Vec::new();
        write_theme_select_result(&mut buf, "classic", "classic", dir.path())
            .expect("write_theme_select_result");
        let out = String::from_utf8(buf).unwrap();
        assert_eq!(out, "Theme already set to 'classic'\n");
        assert!(
            !dir.path().join(".ops.toml").exists(),
            "no-op path must not touch .ops.toml"
        );
    }

    #[test]
    fn write_theme_select_result_set_to_message() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut buf = Vec::new();
        write_theme_select_result(&mut buf, "compact", "classic", dir.path())
            .expect("write_theme_select_result");
        let out = String::from_utf8(buf).unwrap();
        assert_eq!(out, "Theme set to 'compact'\n");
        let written = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(written.contains("theme = \"compact\""), "got: {written}");
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
    fn update_theme_in_config_refuses_to_overwrite_malformed_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(".ops.toml");
        let malformed = "not = = valid\n{{{";
        std::fs::write(&path, malformed).unwrap();

        let result = update_theme_in_config(dir.path(), "classic");
        assert!(result.is_err(), "malformed TOML should be a hard error");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), malformed);
    }

    #[test]
    fn update_theme_in_config_creates_new_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join(".ops.toml");

        let result = update_theme_in_config(dir.path(), "classic");
        assert!(result.is_ok());
        assert!(config_path.exists());
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("theme = \"classic\""));
    }

    /// Regression — `ops theme select` from a
    /// subdirectory must write to `workspace_root/.ops.toml`, not into the
    /// subdir, so the persisted theme matches the config the rest of the
    /// CLI loaded. Mirrors `save_about_fields_writes_to_workspace_root_from_subdir`.
    #[test]
    fn update_theme_in_config_writes_to_workspace_root_from_subdir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace_root = dir.path();
        let subdir = workspace_root.join("nested/deeper");
        std::fs::create_dir_all(&subdir).unwrap();
        let _guard = crate::CwdGuard::new(&subdir).expect("CwdGuard");

        update_theme_in_config(workspace_root, "classic").expect("save");

        assert!(workspace_root.join(".ops.toml").exists());
        assert!(
            !subdir.join(".ops.toml").exists(),
            "must not have written into the subdirectory cwd"
        );
    }

    /// When `.ops.toml` already has a non-table value at
    /// `output` (e.g. `output = "classic"`), `set_theme` must bail with a
    /// clear anyhow error rather than panicking inside `toml_edit`'s
    /// `IndexMut`. The previous shape (`doc["output"]["theme"] = ...`) only
    /// guarded against missing keys, not type mismatches.
    #[test]
    fn set_theme_bails_on_non_table_output() {
        let mut doc: toml_edit::DocumentMut = "output = \"classic\"\n".parse().unwrap();
        let result = set_theme(&mut doc, "compact");
        let err = result.expect_err("non-table [output] must be a clean error, not a panic");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("[output] is not a table"),
            "error must explain the type mismatch, got: {msg}"
        );
    }

    mod run_theme_list_tests {
        use super::*;

        #[test]
        fn run_theme_list_includes_builtin_themes() {
            let (_dir, _guard) = crate::test_utils::with_temp_config("");

            let config = ops_core::config::load_config_or_default("test");
            let mut buf = Vec::new();
            run_theme_list_to(&config, &mut buf).expect("should succeed");
            let output = String::from_utf8(buf).unwrap();
            assert!(output.contains("classic"), "should list classic: {output}");
            assert!(output.contains("compact"), "should list compact: {output}");
        }

        /// Theme name padding is measured by display width (not byte
        /// length / char count) so wide-character names (CJK / emoji /
        /// combining marks) align the description column at the same
        /// visual column as ASCII names. Mirrors the
        /// `tools_list_aligns_wide_char_names_by_display_width` test in
        /// tools_cmd.rs.
        #[test]
        fn run_theme_list_aligns_wide_char_names_by_display_width() {
            // Build the config in-process: ThemeConfig has many required
            // fields that the TOML round-trip would otherwise duplicate
            // verbatim. Description differentiates the two rows so we can
            // locate them in the rendered output.
            let mut config = ops_core::config::Config::empty();
            config.themes.insert(
                "ビルド".to_string(),
                ops_core::config::theme_types::ThemeConfig {
                    description: Some("Wide name".to_string()),
                    ..ops_core::config::theme_types::ThemeConfig::classic()
                },
            );
            config.themes.insert(
                "plain".to_string(),
                ops_core::config::theme_types::ThemeConfig {
                    description: Some("ASCII name".to_string()),
                    ..ops_core::config::theme_types::ThemeConfig::classic()
                },
            );

            let mut buf = Vec::new();
            run_theme_list_to(&config, &mut buf).expect("run_theme_list_to");
            let output = String::from_utf8(buf).unwrap();
            let lines: Vec<&str> = output.lines().collect();
            let wide = lines
                .iter()
                .find(|l| l.contains("ビルド") && l.contains("Wide name"))
                .unwrap_or_else(|| panic!("wide-name line not found in:\n{output}"));
            let ascii = lines
                .iter()
                .find(|l| l.contains("plain") && l.contains("ASCII name"))
                .unwrap_or_else(|| panic!("ascii line not found in:\n{output}"));
            let wide_col = display_width(&wide[..wide.find("Wide name").unwrap()]);
            let ascii_col = display_width(&ascii[..ascii.find("ASCII name").unwrap()]);
            assert_eq!(
                wide_col, ascii_col,
                "description columns should align by display width: ビルド at {wide_col}, plain at {ascii_col}"
            );
        }

        #[test]
        fn collect_theme_options_includes_custom() {
            let mut config = ops_core::config::Config::empty();
            config.themes.insert(
                "my-custom".to_string(),
                ops_core::config::theme_types::ThemeConfig {
                    description: Some("My custom theme".to_string()),
                    ..ops_core::config::theme_types::ThemeConfig::classic()
                },
            );
            let options = collect_theme_options(&config);
            let names: Vec<&str> = options.iter().map(|o| o.name.as_str()).collect();
            assert!(
                names.contains(&"my-custom"),
                "should include custom theme: {names:?}"
            );
            let custom = options.iter().find(|o| o.name == "my-custom").unwrap();
            assert!(custom.is_custom, "custom theme should be marked as custom");
        }

        /// The built-in theme name set is parsed
        /// at most once across N calls. Asserted indirectly by checking
        /// that two `builtin_theme_names()` calls return the *same*
        /// `&'static HashSet` reference (the `OnceLock` initialiser fired
        /// exactly once); a re-parse would have to rebuild a different
        /// `HashSet` to allocate fresh leaked strings.
        #[test]
        fn builtin_theme_names_parser_runs_at_most_once() {
            let a = builtin_theme_names();
            let b = builtin_theme_names();
            assert!(
                std::ptr::eq(a, b),
                "OnceLock must hand out the same set reference across calls"
            );
            assert!(a.contains("classic"));
            assert!(a.contains("compact"));
        }

        #[test]
        fn collect_theme_options_marks_builtin_correctly() {
            let config: ops_core::config::Config =
                toml::from_str(ops_core::config::default_ops_toml()).unwrap();
            let options = collect_theme_options(&config);
            for opt in &options {
                assert!(!opt.is_custom, "{} should not be marked custom", opt.name);
            }
        }
    }
}
