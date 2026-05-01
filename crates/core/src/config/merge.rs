//! Configuration merge logic: overlay application and field-level merging.

use indexmap::IndexMap;

use super::{Config, ConfigOverlay, OutputConfig, OutputConfigOverlay};

pub(super) fn merge_field<T>(base: &mut T, overlay: Option<T>) {
    if let Some(v) = overlay {
        *base = v;
    }
}

/// Merge overlay entries into base, overwriting existing keys (last-write-wins).
///
/// When overlay keys collide with base keys, the replacement is logged at
/// `tracing::debug!` so `OPS_LOG_LEVEL=debug` reveals the resolution during
/// layered config loading.
pub(super) fn merge_indexmap<K: Eq + std::hash::Hash + std::fmt::Debug, V>(
    base: &mut IndexMap<K, V>,
    overlay: Option<IndexMap<K, V>>,
) {
    if let Some(items) = overlay {
        // SEC-21 / TASK-0745: pre-format each colliding key into its Debug
        // representation (which escapes control characters and newlines)
        // *before* the tracing event records the field. Recording the raw
        // `&K` slice via `?replaced` already escapes the rendered form
        // through any Debug-aware subscriber, but a subscriber that pulls
        // the field as a Display-rendered string (or a flat-line log
        // sink) loses that escaping — and a config with a key like
        // `foo\n2026-01-01 ERROR injected` could forge a log line. Owning
        // the escaped form removes the renderer-shape dependency.
        let replaced: Vec<String> = items
            .keys()
            .filter(|k| base.contains_key(*k))
            .map(|k| format!("{k:?}"))
            .collect();
        if !replaced.is_empty() {
            tracing::debug!(
                keys = ?replaced,
                "config overlay shadows base entries (last-write-wins)"
            );
        }
        base.extend(items);
    }
}

fn merge_output(base: &mut OutputConfig, overlay: &OutputConfigOverlay) {
    merge_field(&mut base.theme, overlay.theme.clone());
    merge_field(&mut base.columns, overlay.columns);
    merge_field(&mut base.show_error_detail, overlay.show_error_detail);
    merge_field(&mut base.stderr_tail_lines, overlay.stderr_tail_lines);
    merge_field(&mut base.category_order, overlay.category_order.clone());
}

/// Copy a per-field `Option<T>` overlay into a `base.field: Option<T>`.
///
/// Collapses the `if let Some(section_overlay) = section { if let Some(v) =
/// &section_overlay.field { base.section.field = Some(v.clone()); } }`
/// pattern that used to appear four times (data/path, extensions/enabled,
/// about/fields, …). If `section` is `None` the base is preserved; if it's
/// `Some(..)` and its inner field is `Some(v)`, the base field is overwritten
/// with an owned copy; otherwise preserved.
fn copy_optional_field<T: Clone>(dst: &mut Option<T>, src: Option<&Option<T>>) {
    if let Some(Some(v)) = src {
        *dst = Some(v.clone());
    }
}

/// Merge overlay into base — only explicitly-set values overwrite.
///
/// Uses destructuring so adding a field to the overlay types without
/// handling it here causes a compile error.
///
/// Takes the overlay by value so the contained `IndexMap`s
/// (`commands`/`themes`/`tools`) move directly into the merge instead of
/// being cloned twice (once at the call site, once inside `merge_indexmap`).
pub fn merge_config(base: &mut Config, overlay: ConfigOverlay) {
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
        merge_output(&mut base.output, &output_overlay);
    }
    merge_indexmap(&mut base.commands, commands);
    copy_optional_field(&mut base.data.path, data.as_ref().map(|d| &d.path));
    merge_indexmap(&mut base.themes, themes);
    copy_optional_field(
        &mut base.extensions.enabled,
        extensions.as_ref().map(|e| &e.enabled),
    );
    copy_optional_field(&mut base.about.fields, about.as_ref().map(|a| &a.fields));
    if let Some(s) = stack {
        base.stack = Some(s);
    }
    merge_indexmap(&mut base.tools, tools);
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
        merge_indexmap(&mut base, Some(overlay));
        assert_eq!(base.len(), 2);
        assert_eq!(base["b"], 2);
    }

    #[test]
    fn merge_indexmap_overwrites_existing_keys() {
        let mut base = IndexMap::from([("a", 1)]);
        let overlay = IndexMap::from([("a", 99)]);
        merge_indexmap(&mut base, Some(overlay));
        assert_eq!(base["a"], 99);
    }

    #[test]
    fn merge_indexmap_emits_debug_on_collision() {
        // Verify that when an overlay shadows a base key, the merged result
        // has the overlay value. The tracing::debug event is emitted but
        // we only verify the data outcome (the log line is a side effect).
        let mut base = IndexMap::from([("cmd-a", 1), ("cmd-b", 2)]);
        let overlay = IndexMap::from([("cmd-a", 10), ("cmd-c", 3)]);
        merge_indexmap(&mut base, Some(overlay));
        assert_eq!(base["cmd-a"], 10, "overlay should win on collision");
        assert_eq!(base["cmd-b"], 2, "non-collided base key preserved");
        assert_eq!(base["cmd-c"], 3, "new overlay key inserted");
    }

    /// SEC-21 / TASK-0745: a config-supplied key with embedded control
    /// characters must not forge log entries. Subscribe with a plain
    /// fmt-layer writer (which renders fields via Display) and assert that
    /// the captured output contains no raw newline within the keys field
    /// — the injected `\n2026-01-01 ERROR ...` line shape must not appear
    /// as a separate physical line.
    #[test]
    #[serial_test::serial]
    fn merge_indexmap_collision_log_escapes_control_characters_in_keys() {
        use std::io::Write;
        use std::sync::{Arc, Mutex};

        #[derive(Clone)]
        struct VecWriter(Arc<Mutex<Vec<u8>>>);
        impl Write for VecWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().write(buf)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for VecWriter {
            type Writer = VecWriter;
            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(VecWriter(Arc::clone(&buf)))
            .with_ansi(false)
            .finish();

        let injected = "foo\n2026-01-01T00:00:00Z ERROR forged log line".to_string();
        tracing::subscriber::with_default(subscriber, || {
            let mut base: IndexMap<String, i32> = IndexMap::new();
            base.insert(injected.clone(), 1);
            let overlay = IndexMap::from([(injected.clone(), 99)]);
            merge_indexmap(&mut base, Some(overlay));

            let logged = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
            assert!(
                logged.contains("config overlay shadows base entries"),
                "expected debug log, got: {logged}"
            );
            // The forged tail must not appear as the start of a physical line —
            // the escaping turns the embedded \n into the two characters `\n`.
            for line in logged.lines() {
                assert!(
                    !line.starts_with("2026-01-01"),
                    "control-char in key forged a log line: {logged}"
                );
            }
            assert!(
                logged.contains("\\n"),
                "expected escaped \\n in captured output: {logged}"
            );
        });
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
        merge_config(&mut base, overlay);
        assert_eq!(base.stack.as_deref(), Some("rust"));
    }

    #[test]
    fn merge_config_empty_overlay_preserves_base() {
        let mut base = Config {
            stack: Some("java".to_string()),
            ..Config::default()
        };
        let overlay = ConfigOverlay::default();
        merge_config(&mut base, overlay);
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
        merge_config(&mut base, overlay);
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
        merge_config(&mut base, overlay);
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
        merge_config(&mut base, overlay);
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
        merge_config(&mut base, overlay);
        assert!(base.commands.contains_key("existing"));
        assert!(base.commands.contains_key("new_cmd"));
    }
}
