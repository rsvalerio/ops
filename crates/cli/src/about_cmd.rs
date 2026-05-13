//! `ops about setup` — interactively choose about card fields.

use std::collections::HashSet;
use std::io::Write;
use std::path::Path;

use ops_core::config::{edit_ops_toml, ensure_table};
use ops_extension::DataRegistry;

use crate::tty::SelectOption;

pub fn run_about_setup(
    config: &ops_core::config::Config,
    data_registry: &DataRegistry,
    workspace_root: &Path,
) -> anyhow::Result<()> {
    run_about_setup_with(
        config,
        data_registry,
        workspace_root,
        &mut std::io::stdout(),
        crate::tty::is_stdout_tty,
    )
}

fn run_about_setup_with<F>(
    config: &ops_core::config::Config,
    data_registry: &DataRegistry,
    workspace_root: &Path,
    w: &mut dyn Write,
    is_tty: F,
) -> anyhow::Result<()>
where
    F: FnOnce() -> bool,
{
    crate::tty::require_tty_with("about setup", is_tty)?;

    let about_fields = data_registry.about_fields("project_identity");
    if about_fields.is_empty() {
        anyhow::bail!("no project_identity provider registered — cannot configure about fields");
    }

    // The Config is threaded from `run()` so the warn-and-default policy
    // applies to the whole CLI invocation, including the `about setup`
    // save path.
    let currently_enabled = config.about.fields.as_deref();

    let options: Vec<SelectOption> = about_fields
        .iter()
        .map(|f| SelectOption {
            name: f.id.to_string(),
            description: f.description.to_string(),
        })
        .collect();

    // Probe the set of currently-enabled field ids in
    // O(1) per candidate instead of an O(N*M) `Vec::any` scan; field
    // counts grow with extension count and this path is hit on every render
    // / re-configure.
    let enabled_set: Option<HashSet<&str>> =
        currently_enabled.map(|fields| fields.iter().map(String::as_str).collect());

    let defaults: Vec<usize> = about_fields
        .iter()
        .enumerate()
        .filter(|(_, f)| match &enabled_set {
            None => true,
            Some(set) => set.contains(f.id),
        })
        .map(|(i, _)| i)
        .collect();

    let selected = inquire::MultiSelect::new("Select fields to show on the about card:", options)
        .with_default(&defaults)
        .prompt()?;

    let field_ids: Vec<String> = selected.into_iter().map(|o| o.name).collect();

    save_about_fields(&field_ids, workspace_root)?;

    write_about_setup_confirmation(w, &field_ids)
}

/// Post-prompt confirmation rendering split out so unit
/// tests can drive the message text against a `Vec<u8>` without a TTY. The
/// public entry threads `std::io::stdout()` through; tests buffer-capture.
fn write_about_setup_confirmation(w: &mut dyn Write, field_ids: &[String]) -> anyhow::Result<()> {
    writeln!(w, "About card will show: {}", field_ids.join(", "))?;
    Ok(())
}

/// Anchor the saved `.ops.toml` to the same root the rest
/// of the CLI threads through (`crate::cwd()` → `Stack::resolve(...)`), so
/// running `ops about setup` from a subdirectory writes the file alongside
/// the loaded config rather than next to the user's cwd.
fn save_about_fields(fields: &[String], workspace_root: &Path) -> anyhow::Result<()> {
    let config_path = workspace_root.join(".ops.toml");
    edit_ops_toml(&config_path, |doc| {
        let about = ensure_table(doc, "about")?;
        // When `fields` already exists, mutate the
        // existing Array in place (clear + push) so the user's inline
        // comments and trailing decor survive a re-save. The
        // wholesale-replace path (insert) is reserved for the fresh-array
        // case where there is no prior decor to preserve.
        if let Some(existing) = about
            .get_mut("fields")
            .and_then(toml_edit::Item::as_value_mut)
            .and_then(toml_edit::Value::as_array_mut)
        {
            existing.clear();
            for f in fields {
                existing.push(f.as_str());
            }
        } else {
            let mut arr = toml_edit::Array::new();
            for f in fields {
                arr.push(f.as_str());
            }
            about.insert("fields", toml_edit::value(arr));
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_about_setup_non_tty_returns_error() {
        let registry = DataRegistry::new();
        let config = ops_core::config::Config::empty();
        let dir = tempfile::tempdir().expect("tempdir");
        let mut buf = Vec::new();
        let result = run_about_setup_with(&config, &registry, dir.path(), &mut buf, || false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("interactive terminal"));
        assert!(buf.is_empty(), "non-TTY path must not write anything");
    }

    /// Pin the post-prompt confirmation message. The
    /// `inquire::MultiSelect` picker requires a TTY, but the confirmation
    /// rendering is deterministic format-and-write — exercising
    /// `write_about_setup_confirmation` directly covers the happy-path
    /// message without emulating a terminal.
    #[test]
    fn write_about_setup_confirmation_renders_joined_fields() {
        let mut buf = Vec::new();
        write_about_setup_confirmation(&mut buf, &["project".to_string(), "codebase".to_string()])
            .expect("write_about_setup_confirmation");
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "About card will show: project, codebase\n"
        );
    }

    #[test]
    fn save_about_fields_creates_new_file() {
        let dir = tempfile::tempdir().expect("tempdir");

        let fields = vec!["project".to_string(), "codebase".to_string()];
        save_about_fields(&fields, dir.path()).expect("save should succeed");

        let config_path = dir.path().join(".ops.toml");
        assert!(config_path.exists());
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("[about]"), "got: {content}");
        assert!(content.contains("project"), "got: {content}");
        assert!(content.contains("codebase"), "got: {content}");
    }

    #[test]
    fn save_about_fields_preserves_existing_config() {
        let dir = tempfile::tempdir().expect("tempdir");

        let existing = "[output]\ntheme = \"classic\"\n";
        std::fs::write(dir.path().join(".ops.toml"), existing).unwrap();

        let fields = vec!["authors".to_string(), "repository".to_string()];
        save_about_fields(&fields, dir.path()).expect("save should succeed");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(
            content.contains("theme = \"classic\""),
            "should preserve existing: {content}"
        );
        assert!(content.contains("[about]"), "got: {content}");
        assert!(content.contains("authors"), "got: {content}");
    }

    #[test]
    fn save_about_fields_updates_existing_about_section() {
        let dir = tempfile::tempdir().expect("tempdir");

        let existing = "[about]\nfields = [\"project\"]\n";
        std::fs::write(dir.path().join(".ops.toml"), existing).unwrap();

        let fields = vec![
            "project".to_string(),
            "codebase".to_string(),
            "repository".to_string(),
        ];
        save_about_fields(&fields, dir.path()).expect("save should succeed");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("codebase"), "got: {content}");
        assert!(content.contains("repository"), "got: {content}");
    }

    /// A trailing comment on the `fields` array must
    /// survive a re-save. Pre-fix, `save_about_fields` did
    /// `about.insert("fields", toml_edit::value(arr))`, replacing the entry
    /// wholesale and dropping any inline comments/decor on the prior array.
    #[test]
    fn save_about_fields_preserves_fields_array_decor() {
        let dir = tempfile::tempdir().expect("tempdir");
        let existing = "[about]\nfields = [\"project\"] # keep\n";
        std::fs::write(dir.path().join(".ops.toml"), existing).unwrap();

        save_about_fields(&["project".to_string(), "codebase".to_string()], dir.path())
            .expect("save should succeed");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(
            content.contains("# keep"),
            "trailing comment on fields must survive re-save: {content}"
        );
        assert!(content.contains("codebase"), "got: {content}");
    }

    #[test]
    fn save_about_fields_refuses_to_overwrite_malformed_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(".ops.toml");
        let malformed = "not = = valid\n{{{";
        std::fs::write(&path, malformed).unwrap();

        let result = save_about_fields(&["project".to_string()], dir.path());
        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), malformed);
    }

    #[test]
    fn save_about_fields_empty_selection() {
        let dir = tempfile::tempdir().expect("tempdir");

        save_about_fields(&[], dir.path()).expect("save should succeed");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("[about]"), "got: {content}");
        assert!(content.contains("fields = []"), "got: {content}");
    }

    /// When the user runs `ops about setup`
    /// from a subdirectory but the workspace root is one level up, the saved
    /// `.ops.toml` must land at the workspace root — not in the cwd.
    #[test]
    fn save_about_fields_writes_to_workspace_root_from_subdir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let workspace_root = dir.path();
        let subdir = workspace_root.join("nested/deeper");
        std::fs::create_dir_all(&subdir).unwrap();
        let _guard = crate::CwdGuard::new(&subdir).expect("CwdGuard");

        save_about_fields(&["project".to_string()], workspace_root).expect("save should succeed");

        assert!(workspace_root.join(".ops.toml").exists());
        assert!(
            !subdir.join(".ops.toml").exists(),
            "must not have written into the subdirectory cwd"
        );
    }
}
