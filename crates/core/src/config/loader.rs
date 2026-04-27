//! Configuration loading from files, directories, and environment variables.
//!
//! # Test discipline
//!
//! Tests that mutate process-global state — environment variables **and**
//! the current working directory — must be marked `#[serial_test::serial]`.
//! `cargo test` runs these in parallel by default; a parallel test that
//! happens to read relative paths or env vars will observe the mutation
//! window and flake. Apply `#[serial]` (or isolate via subprocess) whenever
//! a test calls `std::env::set_var`, `std::env::remove_var`, or
//! `std::env::set_current_dir`.

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
///
/// Fails fast on deserialization errors (SEC-11 / ERR-1): a mistyped
/// `OPS__OUTOUT__THEME` in CI should surface as a loud error rather than a
/// silent misconfiguration that drops every other OPS__ variable.
fn merge_env_vars(config: &mut Config) -> anyhow::Result<()> {
    let ops_keys: Vec<String> = std::env::vars()
        .map(|(k, _)| k)
        .filter(|k| k.starts_with("OPS__"))
        .collect();
    if ops_keys.is_empty() {
        return Ok(());
    }
    let env_config = config_crate::Config::builder()
        .add_source(config_crate::Environment::with_prefix("OPS").separator("__"))
        .build()
        .with_context(|| format!("failed to build OPS__ env config (keys: {ops_keys:?})"))?;
    let env_overlay: ConfigOverlay = env_config
        .try_deserialize()
        .with_context(|| format!("failed to deserialize OPS__ env config (keys: {ops_keys:?})"))?;
    merge_config(config, &env_overlay);
    Ok(())
}

#[instrument(skip_all)]
pub fn load_config() -> anyhow::Result<Config> {
    let mut config: Config =
        toml::from_str(default_ops_toml()).context("failed to parse internal default config")?;
    debug!("loaded internal default config");

    load_global_config(&mut config)?;

    let local_path = PathBuf::from(".ops.toml");
    match read_config_file(&local_path) {
        Ok(Some(overlay)) => {
            debug!(path = %local_path.display(), "merging local config");
            merge_config(&mut config, &overlay);
        }
        Ok(None) => {}
        Err(e) => return Err(e),
    }

    merge_conf_d(&mut config)?;

    merge_env_vars(&mut config)?;

    config.validate()?;

    debug!(command_count = config.commands.len(), "config loaded");
    Ok(config)
}

/// Load config and degrade to `Config::default()` on failure, surfacing the
/// error via both `tracing::warn!` (structured log) and [`crate::ui::warn`]
/// (user-visible). `context` describes the caller path (`"hook install"`,
/// `"about"`, `"early"`) and is included verbatim in both messages so logs
/// can be filtered and the user can correlate the warning to what they ran.
///
/// DUP-3 / TASK-0345: collapses the same `match load_config { Ok => c, Err =>
/// { ui::warn(...); Config::default() } }` block previously duplicated
/// across `cli/main.rs`, `cli/about_cmd.rs`, and `cli/hook_shared.rs`.
pub fn load_config_or_default(context: &str) -> Config {
    match load_config() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %format!("{e:#}"), %context, "failed to load config");
            crate::ui::warn(format!(
                "failed to load config ({context}): {e:#}\n  using built-in defaults"
            ));
            Config::default()
        }
    }
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

/// Merge every `.ops.d/*.toml` overlay, in sorted order.
///
/// ERR-1: a parse or IO error on any single overlay file surfaces as a hard
/// error with the offending path in context rather than being silently
/// dropped. Users whose overlay "mysteriously does nothing" in CI should see
/// a loud failure instead of a tracing warning that gets swallowed.
fn merge_conf_d(config: &mut Config) -> anyhow::Result<()> {
    let Some(files) = read_conf_d_files(Path::new(".ops.d")) else {
        return Ok(());
    };
    for path in files {
        match read_config_file(&path) {
            Ok(Some(overlay)) => {
                debug!(path = %path.display(), "merging conf.d config");
                merge_config(config, &overlay);
            }
            Ok(None) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
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
///
/// ERR-1: a read/parse error on the global config surfaces as a hard error
/// with the path attached — a corrupted `~/.config/ops/config.toml` should
/// not be silently ignored, leaving the user thinking their config applied.
fn load_global_config(config: &mut Config) -> anyhow::Result<()> {
    let Some(global_path) = global_config_path() else {
        return Ok(());
    };
    let to_try = [global_path.with_extension("toml"), global_path];
    for path in &to_try {
        match read_config_file(path) {
            Ok(Some(overlay)) => {
                debug!(path = %path.display(), "merging global config");
                merge_config(config, &overlay);
                return Ok(());
            }
            Ok(None) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
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
    #[serial_test::serial]
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
        merge_conf_d(&mut config).unwrap();
        std::env::set_current_dir(original).unwrap();

        assert!(config.commands.contains_key("extra"));
    }

    #[test]
    #[serial_test::serial]
    fn merge_conf_d_propagates_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let ops_d = dir.path().join(".ops.d");
        fs::create_dir(&ops_d).unwrap();
        fs::write(ops_d.join("broken.toml"), "not = = valid {{{").unwrap();

        let mut config = Config::default();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        let result = merge_conf_d(&mut config);
        std::env::set_current_dir(original).unwrap();

        let err = result.expect_err("parse failure should surface");
        assert!(format!("{err:#}").contains("broken.toml"));
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
