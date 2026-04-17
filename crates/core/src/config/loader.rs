//! Configuration loading from files, directories, and environment variables.

use std::path::{Path, PathBuf};

use anyhow::Context;
use config as config_crate;
use tracing::{debug, instrument};

use super::merge::merge_config;
use super::{default_ops_toml, Config, ConfigOverlay};

/// Merge environment variables with OPS prefix into config.
///
/// Only applies overlay when OPS__ prefixed env vars exist.
/// Without this guard, the `config` crate deserializes an empty config with
/// all-default values, and merge_config unconditionally overwrites the local
/// config's intentional settings.
fn merge_env_vars(config: &mut Config) {
    let has_ops_env = std::env::vars().any(|(k, _)| k.starts_with("OPS__"));
    if !has_ops_env {
        return;
    }
    let env_config = config_crate::Config::builder()
        .add_source(config_crate::Environment::with_prefix("OPS").separator("__"))
        .build();
    match env_config {
        Ok(merged) => match merged.try_deserialize::<ConfigOverlay>() {
            Ok(env_overlay) => merge_config(config, &env_overlay),
            Err(e) => tracing::warn!(error = %e, "failed to deserialize OPS__ env config"),
        },
        Err(e) => tracing::warn!(error = %e, "failed to build OPS__ env config"),
    }
}

#[instrument(skip_all)]
pub fn load_config() -> anyhow::Result<Config> {
    let mut config: Config =
        toml::from_str(default_ops_toml()).context("failed to parse internal default config")?;
    debug!("loaded internal default config");

    load_global_config(&mut config);

    let local_path = PathBuf::from(".ops.toml");
    match read_config_file(&local_path) {
        Ok(Some(overlay)) => {
            debug!(path = %local_path.display(), "merging local config");
            merge_config(&mut config, &overlay);
        }
        Ok(None) => {}
        Err(e) => return Err(e),
    }

    merge_conf_d(&mut config);

    merge_env_vars(&mut config);

    config.validate()?;

    debug!(command_count = config.commands.len(), "config loaded");
    Ok(config)
}

pub fn read_config_file(path: &Path) -> anyhow::Result<Option<ConfigOverlay>> {
    let s = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e)
                .with_context(|| format!("failed to read config file: {}", path.display()));
        }
    };
    let overlay = toml::from_str(&s)
        .with_context(|| format!("failed to parse config file: {}", path.display()))?;
    Ok(Some(overlay))
}

/// Read sorted `.toml` files from a directory, returning None if the directory
/// doesn't exist or can't be read.
fn read_conf_d_files(dir: &Path) -> Option<Vec<PathBuf>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            tracing::warn!(
                path = %dir.display(),
                error = %e,
                "failed to read .ops.d directory"
            );
            return None;
        }
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    files.sort();
    Some(files)
}

fn merge_conf_d(config: &mut Config) {
    let Some(files) = read_conf_d_files(Path::new(".ops.d")) else {
        return;
    };
    for path in files {
        match read_config_file(&path) {
            Ok(Some(overlay)) => {
                debug!(path = %path.display(), "merging conf.d config");
                merge_config(config, &overlay);
            }
            Ok(None) => {}
            Err(e) => tracing::warn!(error = %e, "skipping conf.d file"),
        }
    }
}

/// Path to global config file (e.g. ~/.config/ops/config.toml).
///
/// Respects `XDG_CONFIG_HOME` when set; falls back to `$HOME/.config/`.
pub(crate) fn global_config_path() -> Option<PathBuf> {
    let config_dir = if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
        PathBuf::from(home).join(".config")
    };
    Some(config_dir.join("ops/config"))
}

/// Load global config from standard paths.
fn load_global_config(config: &mut Config) {
    let Some(global_path) = global_config_path() else {
        return;
    };
    let to_try = [global_path.with_extension("toml"), global_path];
    for path in &to_try {
        match read_config_file(path) {
            Ok(Some(overlay)) => {
                debug!(path = %path.display(), "merging global config");
                merge_config(config, &overlay);
                return;
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(error = %e, "skipping global config file");
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn read_conf_d_files_sorts_and_filters() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("b.toml"), "").unwrap();
        fs::write(dir.path().join("a.toml"), "").unwrap();
        fs::write(dir.path().join("readme.md"), "").unwrap();

        let files = read_conf_d_files(dir.path()).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files[0].ends_with("a.toml"));
        assert!(files[1].ends_with("b.toml"));
    }

    #[test]
    fn read_conf_d_files_missing_dir_returns_none() {
        let result = read_conf_d_files(std::path::Path::new("/nonexistent/ops.d"));
        assert!(result.is_none());
    }

    #[test]
    fn merge_conf_d_applies_overlays() {
        let dir = tempfile::tempdir().unwrap();
        let ops_d = dir.path().join(".ops.d");
        fs::create_dir(&ops_d).unwrap();
        fs::write(
            ops_d.join("extra.toml"),
            r#"
[commands.extra]
program = "echo"
args = ["hello"]
"#,
        )
        .unwrap();

        let mut config = Config::default();
        // Change cwd temporarily so merge_conf_d finds our test .ops.d
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        merge_conf_d(&mut config);
        std::env::set_current_dir(original).unwrap();

        assert!(config.commands.contains_key("extra"));
    }

    #[test]
    fn validate_rejects_empty_program() {
        let mut config = Config::default();
        config.commands.insert(
            "bad".to_string(),
            super::super::CommandSpec::Exec(super::super::ExecCommandSpec {
                program: String::new(),
                ..Default::default()
            }),
        );
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("program must not be empty"));
    }

    #[test]
    fn validate_rejects_zero_timeout() {
        let mut config = Config::default();
        config.commands.insert(
            "bad".to_string(),
            super::super::CommandSpec::Exec(super::super::ExecCommandSpec {
                program: "echo".to_string(),
                timeout_secs: Some(0),
                ..Default::default()
            }),
        );
        let result = config.validate();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("timeout_secs must be greater than 0"));
    }

    #[test]
    fn validate_accepts_valid_config() {
        let mut config = Config::default();
        config.commands.insert(
            "good".to_string(),
            super::super::CommandSpec::Exec(super::super::ExecCommandSpec {
                program: "echo".to_string(),
                timeout_secs: Some(30),
                ..Default::default()
            }),
        );
        assert!(config.validate().is_ok());
    }
}
