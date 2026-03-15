//! Tests for configuration loading and merging.
//!
//! # Test Serialization (TQ-003, TQ-004)
//!
//! Some tests in this module are annotated with `#[serial]` because they modify
//! process-global state (environment variables). Without serialization, parallel
//! test execution would cause race conditions where one test's env var changes
//! affect another test.
//!
//! **Trade-off**: Serialization reduces parallelism for these tests, but it's
//! necessary for correctness. Future improvements could use process-isolated
//! tests (e.g., running each test in a subprocess) to restore parallelism.

use super::*;
use crate::config::theme_types::ThemeConfig;
use crate::test_utils::{exec_spec, EnvGuard, TestConfigBuilder};
use indexmap::IndexMap;
use serial_test::serial;

#[test]
fn default_ops_file_exists_and_deserializes() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/.default.ops.toml");
    assert!(
        path.exists(),
        "src/.default.ops.toml must exist in the repo (base config; stack commands in .default.<stack>.ops.toml)"
    );
    let c: Config = toml::from_str(default_ops_toml()).expect("default config must deserialize");
    assert_eq!(c.output.theme, "classic");
    assert_eq!(c.output.columns, 120);
    assert!(c.output.show_error_detail);
    // Commands are provided by stack defaults (from .default.<stack>.ops.toml), not the base file.
    assert!(
        c.commands.is_empty(),
        "default config should have no commands; stack defaults are loaded separately"
    );
}

#[test]
fn init_template_with_rust_stack_includes_commands() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
    )
    .expect("write Cargo.toml");
    let sections = InitSections::from_flags(true, true, true);
    let content = init_template(dir.path(), &sections).expect("init_template must succeed");
    assert!(
        content.contains("[commands.build]"),
        "Rust stack init template must include [commands.build]"
    );
    assert!(
        content.contains("[commands.clippy]"),
        "Rust stack init template must include [commands.clippy]"
    );
    assert!(
        content.contains("[commands.verify]"),
        "Rust stack init template must include [commands.verify]"
    );
    assert!(
        content.contains("stack = \"rust\""),
        "Rust stack init template must set stack = \"rust\""
    );
}

#[test]
fn init_template_without_stack_omits_stack_commands() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sections = InitSections::from_flags(true, true, true);
    let content = init_template(dir.path(), &sections).expect("init_template must succeed");
    assert!(
        content.contains("[output]"),
        "init template must include base [output]"
    );
    // No stack detected, so no stack-specific commands; base has no commands.
    let config: Config = toml::from_str(&content).expect("init template must deserialize");
    assert!(
        config.commands.is_empty(),
        "init without detected stack should have no commands"
    );
}

#[test]
fn exec_spec_timeout_some() {
    let mut e = exec_spec("cargo", &["build"]);
    e.timeout_secs = Some(300);
    assert_eq!(e.timeout(), Some(Duration::from_secs(300)));
}

#[test]
fn exec_spec_timeout_none() {
    let e = exec_spec("cargo", &["build"]);
    assert_eq!(e.timeout(), None);
}

#[test]
fn exec_spec_display_cmd() {
    let e = exec_spec("cargo", &["clippy", "--all"]);
    assert_eq!(e.display_cmd(), "cargo clippy --all");
}

#[test]
fn exec_spec_display_cmd_no_args() {
    let e = exec_spec("make", &[]);
    assert_eq!(e.display_cmd(), "make");
}

fn base_config() -> Config {
    TestConfigBuilder::new()
        .theme("classic")
        .columns(80)
        .show_error_detail(true)
        .exec("build", "cargo", &["build"])
        .build()
}

fn make_exec_spec(program: &str, args: &[&str]) -> CommandSpec {
    CommandSpec::Exec(exec_spec(program, args))
}

#[test]
fn merge_config_overlay_adds_commands() {
    let mut base = base_config();
    let overlay = ConfigOverlay {
        commands: Some({
            let mut m = IndexMap::new();
            m.insert("test".into(), make_exec_spec("cargo", &["test"]));
            m
        }),
        ..Default::default()
    };
    merge_config(&mut base, &overlay);
    assert!(
        base.commands.contains_key("build"),
        "base command preserved"
    );
    assert!(base.commands.contains_key("test"), "overlay command added");
}

#[test]
fn merge_config_overlay_overrides_existing_command() {
    let mut base = base_config();
    let overlay = ConfigOverlay {
        commands: Some({
            let mut m = IndexMap::new();
            m.insert(
                "build".into(),
                make_exec_spec("cargo", &["build", "--release"]),
            );
            m
        }),
        ..Default::default()
    };
    merge_config(&mut base, &overlay);
    match &base.commands["build"] {
        CommandSpec::Exec(e) => assert_eq!(e.args, vec!["build", "--release"]),
        _ => panic!("expected Exec"),
    }
}

#[test]
fn merge_config_overlay_overrides_output() {
    let mut base = base_config();
    let overlay = ConfigOverlay {
        output: Some(OutputConfigOverlay {
            theme: Some("compact".into()),
            columns: Some(120),
            show_error_detail: Some(false),
        }),
        ..Default::default()
    };
    merge_config(&mut base, &overlay);
    assert_eq!(base.output.theme, "compact");
    assert_eq!(base.output.columns, 120);
    assert!(!base.output.show_error_detail);
}

#[test]
fn merge_config_partial_overlay_preserves_base() {
    let mut base = base_config();
    let overlay = ConfigOverlay {
        output: Some(OutputConfigOverlay {
            theme: None,
            columns: Some(200),
            show_error_detail: None,
        }),
        ..Default::default()
    };
    merge_config(&mut base, &overlay);
    assert_eq!(base.output.theme, "classic", "theme preserved from base");
    assert_eq!(base.output.columns, 200, "columns overridden by overlay");
    assert!(
        base.output.show_error_detail,
        "show_error_detail preserved from base"
    );
    assert!(
        base.commands.contains_key("build"),
        "commands preserved from base"
    );
}

/// TQ-015: DataConfigOverlay merging preserves base when overlay is None.
#[test]
fn merge_config_data_overlay_sets_path() {
    let mut base = base_config();
    base.data.path = Some(std::path::PathBuf::from("/original/path"));
    let overlay = ConfigOverlay {
        data: Some(DataConfigOverlay {
            path: Some(std::path::PathBuf::from("/new/path")),
        }),
        ..Default::default()
    };
    merge_config(&mut base, &overlay);
    assert_eq!(
        base.data.path,
        Some(std::path::PathBuf::from("/new/path")),
        "data path should be overridden"
    );
}

#[test]
fn merge_config_data_overlay_none_path_preserves_base() {
    let mut base = base_config();
    base.data.path = Some(std::path::PathBuf::from("/original/path"));
    let overlay = ConfigOverlay {
        data: Some(DataConfigOverlay { path: None }),
        ..Default::default()
    };
    merge_config(&mut base, &overlay);
    assert_eq!(
        base.data.path,
        Some(std::path::PathBuf::from("/original/path")),
        "data path should be preserved when overlay.path is None"
    );
}

/// TQ-015: ExtensionConfigOverlay merging.
#[test]
fn merge_config_extension_overlay_sets_enabled() {
    let mut base = base_config();
    let overlay = ConfigOverlay {
        extensions: Some(ExtensionConfigOverlay {
            enabled: Some(vec!["metadata".to_string()]),
        }),
        ..Default::default()
    };
    merge_config(&mut base, &overlay);
    assert_eq!(
        base.extensions.enabled,
        Some(vec!["metadata".to_string()]),
        "extensions.enabled should be set from overlay"
    );
}

#[test]
fn merge_config_extension_overlay_none_preserves_base() {
    let mut base = base_config();
    base.extensions.enabled = Some(vec!["metadata".to_string()]);
    let overlay = ConfigOverlay {
        extensions: Some(ExtensionConfigOverlay { enabled: None }),
        ..Default::default()
    };
    merge_config(&mut base, &overlay);
    assert_eq!(
        base.extensions.enabled,
        Some(vec!["metadata".to_string()]),
        "extensions.enabled should be preserved when overlay is None"
    );
}

#[test]
fn merge_config_overlay_adds_themes() {
    let mut base = base_config();
    let overlay = ConfigOverlay {
        themes: Some({
            let mut m = IndexMap::new();
            m.insert("dark".into(), ThemeConfig::compact());
            m
        }),
        ..Default::default()
    };
    merge_config(&mut base, &overlay);
    assert!(base.themes.contains_key("dark"));
}

#[test]
fn read_config_file_valid_toml() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("test.toml");
    std::fs::write(
        &path,
        r#"
[output]
theme = "compact"
columns = 100
show_error_detail = false

[commands.hello]
program = "echo"
args = ["hi"]
"#,
    )
    .unwrap();
    let overlay = read_config_file(&path).expect("valid toml should parse");
    let output = overlay.output.expect("output section present");
    assert_eq!(output.theme, Some("compact".to_string()));
    assert_eq!(output.columns, Some(100));
    assert_eq!(output.show_error_detail, Some(false));
    assert!(overlay
        .commands
        .expect("commands present")
        .contains_key("hello"));
}

#[test]
fn read_config_file_invalid_toml_returns_none() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("bad.toml");
    std::fs::write(&path, "not valid { toml }}}").unwrap();
    assert!(read_config_file(&path).is_none());
}

#[test]
fn read_config_file_missing_returns_none() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("nonexistent.toml");
    assert!(read_config_file(&path).is_none());
}

#[test]
#[serial]
fn global_config_path_uses_xdg_config_home() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let _guard = EnvGuard::set(
        "XDG_CONFIG_HOME",
        temp_dir.path().to_string_lossy().as_ref(),
    );
    let path = global_config_path();
    assert!(path.is_some());
    let path = path.unwrap();
    assert!(path.starts_with(temp_dir.path()));
    assert!(path.ends_with("ops/config"));
}

#[test]
#[serial]
fn global_config_path_falls_back_to_home_config() {
    let _xdg_guard = EnvGuard::remove("XDG_CONFIG_HOME");
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let _home_guard = EnvGuard::set("HOME", temp_dir.path().to_string_lossy().as_ref());
    let _userprofile_guard = EnvGuard::remove("USERPROFILE");

    let path = global_config_path();

    assert!(path.is_some());
    let path = path.unwrap();
    assert!(path.to_string_lossy().contains(".config"));
    assert!(path.ends_with("ops/config"));
}

mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn merge_config_overlay_overrides_base_commands(
            cmd_name in "cmd[a-zA-Z0-9_]{0,8}",
            base_program in "base[a-zA-Z0-9_]{0,8}",
            overlay_program in "over[a-zA-Z0-9_]{0,8}"
        ) {
            let mut base = Config {
                commands: {
                    let mut m = IndexMap::new();
                    m.insert(
                        cmd_name.clone(),
                        CommandSpec::Exec(ExecCommandSpec {
                            program: base_program,
                            ..Default::default()
                        }),
                    );
                    m
                },
                ..Default::default()
            };
            let overlay = ConfigOverlay {
                commands: Some({
                    let mut m = IndexMap::new();
                    m.insert(
                        cmd_name.clone(),
                        CommandSpec::Exec(ExecCommandSpec {
                            program: overlay_program.clone(),
                            ..Default::default()
                        }),
                    );
                    m
                }),
                ..Default::default()
            };
            merge_config(&mut base, &overlay);
            prop_assert!(base.commands.contains_key(&cmd_name));
            if let Some(CommandSpec::Exec(e)) = base.commands.get(&cmd_name) {
                prop_assert_eq!(e.program.as_str(), overlay_program.as_str());
            } else {
                prop_assert!(false, "expected Exec variant");
            }
        }

        #[test]
        fn merge_config_partial_overlay_keeps_base_values(
            base_columns in 10u16..200u16,
            overlay_columns in 10u16..200u16
        ) {
            let mut base = Config {
                output: OutputConfig {
                    theme: "classic".into(),
                    columns: base_columns,
                    show_error_detail: true,
                },
                ..Default::default()
            };
            let overlay = ConfigOverlay {
                output: Some(OutputConfigOverlay {
                    theme: None,
                    columns: Some(overlay_columns),
                    show_error_detail: None,
                }),
                ..Default::default()
            };
            merge_config(&mut base, &overlay);
            prop_assert_eq!(base.output.theme, "classic");
            prop_assert_eq!(base.output.columns, overlay_columns);
            prop_assert!(base.output.show_error_detail);
        }
    }
}

#[test]
fn extension_config_default_is_none() {
    let config = ExtensionConfig::default();
    assert!(config.enabled.is_none());
}

#[test]
fn merge_config_overlay_enables_extensions() {
    let mut base = base_config();
    let overlay = ConfigOverlay {
        extensions: Some(ExtensionConfigOverlay {
            enabled: Some(vec!["metadata".into(), "cargo-toml".into()]),
        }),
        ..Default::default()
    };
    merge_config(&mut base, &overlay);
    assert_eq!(
        base.extensions.enabled,
        Some(vec!["metadata".into(), "cargo-toml".into()])
    );
}

#[test]
fn parse_config_with_extensions() {
    let toml = r#"
[output]
theme = "classic"

[extensions]
enabled = ["ops-db", "metadata"]
"#;
    let overlay: ConfigOverlay = toml::from_str(toml).expect("should parse");
    assert!(overlay.extensions.is_some());
    let ext = overlay.extensions.unwrap();
    assert_eq!(
        ext.enabled,
        Some(vec!["ops-db".to_string(), "metadata".to_string()])
    );
}

/// TQ-EFF-001: Permission-denied error path tests.
///
/// These tests are Unix-only because Windows has different permission semantics
/// (ACLs vs. Unix mode bits). On Windows, the behavior is verified at compile-time
/// via conditional compilation, but runtime testing is skipped.
mod read_config_file_error_paths {
    use super::*;

    /// TQ-EFF-001: Test that permission-denied errors are handled gracefully.
    ///
    /// This test is Unix-only because it uses `std::os::unix::fs::PermissionsExt`
    /// to set file permissions. Windows file permissions work differently (ACLs)
    /// and would require a different test approach.
    #[cfg(unix)]
    #[test]
    fn read_config_file_permission_denied_returns_none() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("unreadable.toml");
        std::fs::write(&path, "[output]\ntheme = \"classic\"").unwrap();

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

        let result = read_config_file(&path);
        assert!(result.is_none(), "permission denied should return None");

        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644));
    }
}
