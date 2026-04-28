use super::*;
use crate::config::theme_types::ThemeConfig;

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
            stderr_tail_lines: Some(20),
            category_order: None,
        }),
        ..Default::default()
    };
    merge_config(&mut base, &overlay);
    assert_eq!(base.output.theme, "compact");
    assert_eq!(base.output.columns, 120);
    assert!(!base.output.show_error_detail);
    assert_eq!(base.output.stderr_tail_lines, 20);
}

#[test]
fn merge_config_partial_overlay_preserves_base() {
    let mut base = base_config();
    let overlay = ConfigOverlay {
        output: Some(OutputConfigOverlay {
            theme: None,
            columns: Some(200),
            show_error_detail: None,
            stderr_tail_lines: None,
            category_order: None,
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
    assert_eq!(
        base.output.stderr_tail_lines, 5,
        "stderr_tail_lines preserved from base"
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
                    stderr_tail_lines: 5,
                    category_order: Vec::new(),
                },
                ..Default::default()
            };
            let overlay = ConfigOverlay {
                output: Some(OutputConfigOverlay {
                    theme: None,
                    columns: Some(overlay_columns),
                    show_error_detail: None,
                    stderr_tail_lines: None,
                    category_order: None,
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
