//! Configuration merge logic: overlay application and field-level merging.

use indexmap::IndexMap;

use super::{Config, ConfigOverlay, OutputConfig, OutputConfigOverlay};

pub(super) fn merge_field<T>(base: &mut T, overlay: Option<T>) {
    if let Some(v) = overlay {
        *base = v;
    }
}

pub(super) fn merge_indexmap<K: Clone + Eq + std::hash::Hash, V: Clone>(
    base: &mut IndexMap<K, V>,
    overlay: Option<&IndexMap<K, V>>,
) {
    if let Some(items) = overlay {
        for (k, v) in items {
            base.insert(k.clone(), v.clone());
        }
    }
}

fn merge_output(base: &mut OutputConfig, overlay: &OutputConfigOverlay) {
    merge_field(&mut base.theme, overlay.theme.clone());
    merge_field(&mut base.columns, overlay.columns);
    merge_field(&mut base.show_error_detail, overlay.show_error_detail);
    merge_field(&mut base.stderr_tail_lines, overlay.stderr_tail_lines);
    merge_field(&mut base.category_order, overlay.category_order.clone());
}

/// Merge overlay into base — only explicitly-set values overwrite.
///
/// Uses destructuring so adding a field to the overlay types without
/// handling it here causes a compile error.
pub fn merge_config(base: &mut Config, overlay: &ConfigOverlay) {
    let ConfigOverlay {
        output,
        commands,
        data,
        themes,
        extensions,
        about,
        stack,
        tools,
    } = overlay;

    if let Some(output_overlay) = output {
        merge_output(&mut base.output, output_overlay);
    }
    merge_indexmap(&mut base.commands, commands.as_ref());
    if let Some(data_overlay) = data {
        if let Some(path) = &data_overlay.path {
            base.data.path = Some(path.clone());
        }
    }
    merge_indexmap(&mut base.themes, themes.as_ref());
    if let Some(ext_overlay) = extensions {
        if let Some(enabled) = &ext_overlay.enabled {
            base.extensions.enabled = Some(enabled.clone());
        }
    }
    if let Some(about_overlay) = about {
        if let Some(fields) = &about_overlay.fields {
            base.about.fields = Some(fields.clone());
        }
    }
    if let Some(s) = stack {
        base.stack = Some(s.clone());
    }
    merge_indexmap(&mut base.tools, tools.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    // --- merge_field ---

    #[test]
    fn merge_field_some_overwrites_base() {
        let mut base = "original".to_string();
        merge_field(&mut base, Some("overlay".to_string()));
        assert_eq!(base, "overlay");
    }

    #[test]
    fn merge_field_none_preserves_base() {
        let mut base = "original".to_string();
        merge_field::<String>(&mut base, None);
        assert_eq!(base, "original");
    }

    #[test]
    fn merge_field_bool() {
        let mut base = true;
        merge_field(&mut base, Some(false));
        assert!(!base);

        merge_field(&mut base, None);
        assert!(!base);
    }

    // --- merge_indexmap ---

    #[test]
    fn merge_indexmap_inserts_new_keys() {
        let mut base = IndexMap::new();
        base.insert("a", 1);
        let overlay = IndexMap::from([("b", 2)]);
        merge_indexmap(&mut base, Some(&overlay));
        assert_eq!(base.len(), 2);
        assert_eq!(base["b"], 2);
    }

    #[test]
    fn merge_indexmap_overwrites_existing_keys() {
        let mut base = IndexMap::from([("a", 1)]);
        let overlay = IndexMap::from([("a", 99)]);
        merge_indexmap(&mut base, Some(&overlay));
        assert_eq!(base["a"], 99);
    }

    #[test]
    fn merge_indexmap_none_preserves_base() {
        let mut base = IndexMap::from([("a", 1)]);
        merge_indexmap::<&str, i32>(&mut base, None);
        assert_eq!(base.len(), 1);
        assert_eq!(base["a"], 1);
    }

    // --- merge_output ---

    #[test]
    fn merge_output_partial_overlay() {
        let mut base = OutputConfig::default();
        let overlay = OutputConfigOverlay {
            theme: Some("compact".to_string()),
            columns: None,
            show_error_detail: Some(false),
            stderr_tail_lines: None,
            category_order: None,
        };
        merge_output(&mut base, &overlay);
        assert_eq!(base.theme, "compact");
        assert!(!base.show_error_detail);
        // Unchanged fields
        assert_eq!(base.stderr_tail_lines, 5);
    }

    #[test]
    fn merge_output_all_none_preserves_base() {
        let mut base = OutputConfig {
            theme: "custom".to_string(),
            columns: 120,
            show_error_detail: false,
            stderr_tail_lines: 10,
            category_order: vec!["dev".to_string()],
        };
        let overlay = OutputConfigOverlay::default();
        let expected_theme = base.theme.clone();
        merge_output(&mut base, &overlay);
        assert_eq!(base.theme, expected_theme);
        assert_eq!(base.columns, 120);
        assert!(!base.show_error_detail);
        assert_eq!(base.stderr_tail_lines, 10);
        assert_eq!(base.category_order, vec!["dev"]);
    }

    // --- merge_config ---

    #[test]
    fn merge_config_stack_override() {
        let mut base = Config::default();
        assert!(base.stack.is_none());
        let overlay = ConfigOverlay {
            stack: Some("rust".to_string()),
            ..Default::default()
        };
        merge_config(&mut base, &overlay);
        assert_eq!(base.stack.as_deref(), Some("rust"));
    }

    #[test]
    fn merge_config_empty_overlay_preserves_base() {
        let mut base = Config {
            stack: Some("java".to_string()),
            ..Config::default()
        };
        let overlay = ConfigOverlay::default();
        merge_config(&mut base, &overlay);
        assert_eq!(base.stack.as_deref(), Some("java"));
    }

    #[test]
    fn merge_config_data_path_override() {
        let mut base = Config::default();
        let overlay = ConfigOverlay {
            data: Some(super::super::DataConfigOverlay {
                path: Some(PathBuf::from("/custom/data.db")),
            }),
            ..Default::default()
        };
        merge_config(&mut base, &overlay);
        assert_eq!(
            base.data.path.as_deref(),
            Some(Path::new("/custom/data.db"))
        );
    }

    #[test]
    fn merge_config_extensions_enabled_override() {
        let mut base = Config::default();
        assert!(base.extensions.enabled.is_none());
        let overlay = ConfigOverlay {
            extensions: Some(super::super::ExtensionConfigOverlay {
                enabled: Some(vec!["about".to_string()]),
            }),
            ..Default::default()
        };
        merge_config(&mut base, &overlay);
        assert_eq!(
            base.extensions.enabled.as_deref(),
            Some(vec!["about".to_string()].as_slice())
        );
    }

    #[test]
    fn merge_config_about_fields_override() {
        let mut base = Config::default();
        let overlay = ConfigOverlay {
            about: Some(super::super::AboutConfigOverlay {
                fields: Some(vec!["project".to_string(), "codebase".to_string()]),
            }),
            ..Default::default()
        };
        merge_config(&mut base, &overlay);
        assert_eq!(
            base.about.fields,
            Some(vec!["project".to_string(), "codebase".to_string()])
        );
    }

    #[test]
    fn merge_config_commands_overlay() {
        use super::super::{CommandSpec, ExecCommandSpec};
        let mut base = Config::default();
        base.commands.insert(
            "existing".to_string(),
            CommandSpec::Exec(ExecCommandSpec {
                program: "echo".to_string(),
                ..Default::default()
            }),
        );
        let mut overlay_cmds = IndexMap::new();
        overlay_cmds.insert(
            "new_cmd".to_string(),
            CommandSpec::Exec(ExecCommandSpec {
                program: "cargo".to_string(),
                args: vec!["test".to_string()],
                ..Default::default()
            }),
        );
        let overlay = ConfigOverlay {
            commands: Some(overlay_cmds),
            ..Default::default()
        };
        merge_config(&mut base, &overlay);
        assert!(base.commands.contains_key("existing"));
        assert!(base.commands.contains_key("new_cmd"));
    }
}
