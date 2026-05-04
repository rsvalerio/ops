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

use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Context;
use config as config_crate;
use tracing::{debug, instrument};

use super::merge::merge_config;
use super::{default_ops_toml, Config, ConfigOverlay};

/// SEC-33 / TASK-0943: default cap on `.ops.toml` (and `.ops.d/*.toml`,
/// global config) reads. Real-world ops configs are well under 256 KiB,
/// so this cap sits comfortably above any legitimate use while preventing
/// a symlink to `/dev/zero` or an adversarially-large config from
/// exhausting memory. Mirrors the
/// `extensions_terraform_plan::OPS_PLAN_JSON_MAX_BYTES` posture
/// (TASK-0915) and `extensions::git::config::MAX_GIT_CONFIG_BYTES`
/// (TASK-0910). Operators expecting larger configs can raise the cap via
/// [`OPS_TOML_MAX_BYTES_ENV`].
pub const DEFAULT_OPS_TOML_MAX_BYTES: u64 = 256 * 1024;
// Compile-time guard for the documented >=256 KiB floor.
const _: () = assert!(DEFAULT_OPS_TOML_MAX_BYTES >= 256 * 1024);

/// Environment variable that overrides [`DEFAULT_OPS_TOML_MAX_BYTES`].
/// A value of `0` or an unparseable value falls back to the default.
pub const OPS_TOML_MAX_BYTES_ENV: &str = "OPS_TOML_MAX_BYTES";

/// Resolve the current `.ops.toml` byte cap, honouring the
/// [`OPS_TOML_MAX_BYTES_ENV`] override.
pub fn ops_toml_max_bytes() -> u64 {
    std::env::var(OPS_TOML_MAX_BYTES_ENV)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_OPS_TOML_MAX_BYTES)
}

/// Read a `.ops.toml`-style file with a hard byte cap.
///
/// Returns `Ok(None)` if the file does not exist, `Ok(Some(content))`
/// otherwise. Errors include both real IO failures and the bounded-read
/// rejection — an oversized file fails with a typed message naming the
/// cap and the override env var, rather than being slurped into memory.
pub(crate) fn read_capped_toml_file(path: &Path) -> anyhow::Result<Option<String>> {
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e)
                .with_context(|| format!("failed to open config file: {}", path.display()));
        }
    };
    let cap = ops_toml_max_bytes();
    let limit = cap.saturating_add(1);
    let mut content = String::new();
    (&mut file)
        .take(limit)
        .read_to_string(&mut content)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    if content.len() as u64 > cap {
        anyhow::bail!(
            "config file at {} exceeds {cap} bytes (override via {OPS_TOML_MAX_BYTES_ENV})",
            path.display()
        );
    }
    Ok(Some(content))
}

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
    let ops_keys: Vec<String> = std::env::vars_os()
        .filter_map(|(k, _)| k.into_string().ok())
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
    merge_config(config, env_overlay);
    Ok(())
}

/// Counter for `load_config` invocations. Used by the CLI regression test
/// (TASK-0427) to assert that a typical `ops <cmd>` flow only loads
/// `.ops.toml` once. Gated behind `cfg(any(test, feature = "test-support"))`
/// so production CLI binaries do not carry the AtomicUsize or its symbols.
#[cfg(any(test, feature = "test-support"))]
static LOAD_CONFIG_CALL_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Snapshot the current `load_config` invocation count.
#[cfg(any(test, feature = "test-support"))]
pub fn load_config_call_count() -> usize {
    LOAD_CONFIG_CALL_COUNT.load(std::sync::atomic::Ordering::Relaxed)
}

/// Reset the `load_config` invocation count to zero.
#[cfg(any(test, feature = "test-support"))]
pub fn reset_load_config_call_count() {
    LOAD_CONFIG_CALL_COUNT.store(0, std::sync::atomic::Ordering::Relaxed);
}

#[instrument(skip_all)]
pub fn load_config() -> anyhow::Result<Config> {
    #[cfg(any(test, feature = "test-support"))]
    LOAD_CONFIG_CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut config: Config =
        toml::from_str(default_ops_toml()).context("failed to parse internal default config")?;
    debug!("loaded internal default config");

    load_global_config(&mut config)?;

    let local_path = PathBuf::from(".ops.toml");
    match read_config_file(&local_path) {
        Ok(Some(overlay)) => {
            debug!(path = ?local_path.display(), "merging local config");
            merge_config(&mut config, overlay);
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
            Config::empty()
        }
    }
}

pub fn read_config_file(path: &Path) -> anyhow::Result<Option<ConfigOverlay>> {
    // SEC-33 / TASK-0943: route through the byte-capped reader so a
    // multi-GB or symlink-to-/dev/zero `.ops.toml` cannot OOM the CLI.
    let Some(s) = read_capped_toml_file(path)? else {
        return Ok(None);
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
        .filter_map(|e| match e {
            Ok(entry) => Some(entry),
            Err(err) => {
                tracing::warn!(
                    path = %dir.display(),
                    error = %err,
                    "failed to read entry in .ops.d directory; skipping"
                );
                None
            }
        })
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
                debug!(path = ?path.display(), "merging conf.d config");
                merge_config(config, overlay);
            }
            Ok(None) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Path to global config file (e.g. ~/.config/ops/config.toml on Unix,
/// `%APPDATA%\ops\config.toml` on Windows).
///
/// Resolution order:
/// - `XDG_CONFIG_HOME` is honoured on every platform when set (cross-platform
///   tooling — Git, Helix, etc. — uses XDG on Windows too).
/// - On Windows otherwise: `%APPDATA%`, then `%USERPROFILE%\AppData\Roaming`
///   (the value `%APPDATA%` resolves to). The final file is
///   `%APPDATA%\ops\config.toml`, matching the Windows convention.
/// - On Unix otherwise: `$HOME/.config`.
///
/// PORT-5 (TASK-0696): the previous fallback unconditionally appended
/// `.config/ops/config` to whatever `$HOME` or `$USERPROFILE` resolved to,
/// producing `C:\Users\X\.config\ops\config.toml` on Windows — a
/// non-idiomatic location that silently diverges from the documented
/// platform path. The resolved path is logged at `tracing::debug` so the
/// chosen base directory is visible when diagnosing "config not loading"
/// reports.
pub(crate) fn global_config_path() -> Option<PathBuf> {
    let config_dir = if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(xdg)
    } else if cfg!(windows) {
        // CL-3: fall back through the shared `paths::home_dir` helper so
        // Windows-native paths use the same HOME → USERPROFILE order as the
        // rest of the crate.
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| crate::paths::home_dir().map(|h| h.join("AppData/Roaming")))?
    } else {
        crate::paths::home_dir()?.join(".config")
    };
    let path = config_dir.join("ops/config");
    debug!(path = ?path.display(), "resolved global config base path");
    Some(path)
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
                debug!(path = ?path.display(), "merging global config");
                merge_config(config, overlay);
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

    /// ERR-7 / TASK-0965: tracing fields for `.ops.d/*.toml` overlay paths
    /// flow through the `?` formatter so an attacker-controlled filename with
    /// embedded newlines / ANSI escapes cannot forge a log record. Mirrors the
    /// regression guard pattern used by the `manifest_declares_workspace` /
    /// hook-common ERR-7 sweep tests.
    #[test]
    fn conf_d_path_debug_escapes_control_characters() {
        let p = PathBuf::from("malicious\n[fake] info: pwned\u{1b}[31m.toml");
        let rendered = format!("{:?}", p.display());
        assert!(
            !rendered.contains('\n'),
            "raw newline must be escaped, got: {rendered}"
        );
        assert!(
            !rendered.contains('\u{1b}'),
            "ANSI escape must be escaped, got: {rendered}"
        );
        assert!(
            rendered.contains("\\n"),
            "newline must render as escape sequence, got: {rendered}"
        );
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

    /// SEC-33 / TASK-0943: a `.ops.toml` larger than the configured cap
    /// must be rejected with a bounded-read error rather than silently
    /// slurped into memory. Override the cap to 64 bytes via
    /// `OPS_TOML_MAX_BYTES` so the test stays fast.
    #[test]
    #[serial_test::serial]
    fn read_config_file_rejects_oversized_payload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".ops.toml");
        // Payload well over the 64-byte cap below.
        fs::write(&path, "x".repeat(4096)).unwrap();

        // SAFETY: serial-marked; restore prior value at end.
        let saved = std::env::var(OPS_TOML_MAX_BYTES_ENV).ok();
        unsafe { std::env::set_var(OPS_TOML_MAX_BYTES_ENV, "64") };
        let result = read_config_file(&path);
        unsafe {
            match saved {
                Some(v) => std::env::set_var(OPS_TOML_MAX_BYTES_ENV, v),
                None => std::env::remove_var(OPS_TOML_MAX_BYTES_ENV),
            }
        }

        let err = result.expect_err("oversized .ops.toml must error");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("exceeds 64 bytes"),
            "error must name the cap, got: {msg}"
        );
        assert!(
            msg.contains(OPS_TOML_MAX_BYTES_ENV),
            "error must name the override env var, got: {msg}"
        );
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
