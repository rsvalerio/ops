//! `ops about setup` — interactively choose about card fields.

use std::io::Write;
use std::path::PathBuf;

use anyhow::Context;
use ops_core::config::edit_ops_toml;
use ops_extension::DataRegistry;

use crate::tty::SelectOption;

pub fn run_about_setup(
    config: &ops_core::config::Config,
    data_registry: &DataRegistry,
) -> anyhow::Result<()> {
    run_about_setup_with(config, data_registry, crate::tty::is_stdout_tty)
}

fn run_about_setup_with<F>(
    config: &ops_core::config::Config,
    data_registry: &DataRegistry,
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

    // ERR-5 / DUP-3 / TASK-0427: the Config is now threaded from `run()` so
    // the warn-and-default policy applies to the whole CLI invocation,
    // including the `about setup` save path.
    let currently_enabled = config.about.fields.as_deref();

    let options: Vec<SelectOption> = about_fields
        .iter()
        .map(|f| SelectOption {
            name: f.id.to_string(),
            description: f.description.to_string(),
        })
        .collect();

    let defaults: Vec<usize> = about_fields
        .iter()
        .enumerate()
        .filter(|(_, f)| match currently_enabled {
            None => true,
            Some(fields) => fields.iter().any(|enabled| enabled == f.id),
        })
        .map(|(i, _)| i)
        .collect();

    let selected = inquire::MultiSelect::new("Select fields to show on the about card:", options)
        .with_default(&defaults)
        .prompt()?;

    let field_ids: Vec<String> = selected.into_iter().map(|o| o.name).collect();

    save_about_fields(&field_ids)?;

    writeln!(
        std::io::stdout(),
        "About card will show: {}",
        field_ids.join(", ")
    )?;
    Ok(())
}

fn save_about_fields(fields: &[String]) -> anyhow::Result<()> {
    let config_path = PathBuf::from(".ops.toml");
    edit_ops_toml(&config_path, |doc| {
        if !doc.contains_key("about") {
            doc["about"] = toml_edit::Item::Table(toml_edit::Table::new());
        }
        let about = doc["about"]
            .as_table_mut()
            .context("[about] is not a table in .ops.toml")?;
        let mut arr = toml_edit::Array::new();
        for f in fields {
            arr.push(f.as_str());
        }
        about.insert("fields", toml_edit::value(arr));
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_about_setup_non_tty_returns_error() {
        let registry = DataRegistry::new();
        let config = ops_core::config::Config::default();
        let result = run_about_setup_with(&config, &registry, || false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("interactive terminal"));
    }

    #[test]
    fn save_about_fields_creates_new_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");

        let fields = vec!["project".to_string(), "codebase".to_string()];
        save_about_fields(&fields).expect("save should succeed");

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
        let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");

        let existing = "[output]\ntheme = \"classic\"\n";
        std::fs::write(dir.path().join(".ops.toml"), existing).unwrap();

        let fields = vec!["authors".to_string(), "repository".to_string()];
        save_about_fields(&fields).expect("save should succeed");

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
        let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");

        let existing = "[about]\nfields = [\"project\"]\n";
        std::fs::write(dir.path().join(".ops.toml"), existing).unwrap();

        let fields = vec![
            "project".to_string(),
            "codebase".to_string(),
            "repository".to_string(),
        ];
        save_about_fields(&fields).expect("save should succeed");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("codebase"), "got: {content}");
        assert!(content.contains("repository"), "got: {content}");
    }

    #[test]
    fn save_about_fields_refuses_to_overwrite_malformed_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");
        let path = dir.path().join(".ops.toml");
        let malformed = "not = = valid\n{{{";
        std::fs::write(&path, malformed).unwrap();

        let result = save_about_fields(&["project".to_string()]);
        assert!(result.is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), malformed);
    }

    #[test]
    fn save_about_fields_empty_selection() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");

        save_about_fields(&[]).expect("save should succeed");

        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(content.contains("[about]"), "got: {content}");
        assert!(content.contains("fields = []"), "got: {content}");
    }
}
