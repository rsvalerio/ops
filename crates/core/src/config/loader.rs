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
use std::sync::OnceLock;

use anyhow::Context;
use config as config_crate;
use tracing::{debug, instrument};

use super::merge::merge_config;
use super::{default_ops_toml, Config, ConfigOverlay};
use crate::text::cached_byte_cap_env;

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

/// READ-5 / TASK-1129 + ARCH-9 / TASK-1228: cache the resolved cap behind a
/// `OnceLock<u64>` and emit a one-shot warn on unparseable values. Mirrors
/// `crates/core/src/text.rs::manifest_max_bytes`.
static OPS_TOML_MAX_BYTES: OnceLock<u64> = OnceLock::new();

/// Resolve the current `.ops.toml` byte cap, honouring the
/// [`OPS_TOML_MAX_BYTES_ENV`] override.
///
/// READ-5 / TASK-1129: cached behind a `OnceLock<u64>`. The env knob is
/// process-global; subsequent calls do not touch `std::env`. Unparseable or
/// zero values fall back to [`DEFAULT_OPS_TOML_MAX_BYTES`] with a one-shot
/// `tracing::warn!`. Tests that need to override the cap must set the env
/// var before the first call (directly or via [`read_capped_toml_file`]).
pub fn ops_toml_max_bytes() -> u64 {
    cached_byte_cap_env(
        &OPS_TOML_MAX_BYTES,
        OPS_TOML_MAX_BYTES_ENV,
        DEFAULT_OPS_TOML_MAX_BYTES,
    )
}

/// Read a `.ops.toml`-style file with a hard byte cap.
///
/// Returns `Ok(None)` if the file does not exist, `Ok(Some(content))`
/// otherwise. Errors include both real IO failures and the bounded-read
/// rejection — an oversized file fails with a typed message naming the
/// cap and the override env var, rather than being slurped into memory.
pub(crate) fn read_capped_toml_file(path: &Path) -> anyhow::Result<Option<String>> {
    read_capped_toml_file_with(path, ops_toml_max_bytes())
}

/// READ-5 / TASK-1129: testable variant of [`read_capped_toml_file`] that
/// takes an explicit cap. Production callers go through
/// `read_capped_toml_file`; tests use this to bypass the
/// `ops_toml_max_bytes` `OnceLock` (which is process-global and cannot be
/// re-initialised once another test has populated it).
pub(crate) fn read_capped_toml_file_with(path: &Path, cap: u64) -> anyhow::Result<Option<String>> {
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e)
                .with_context(|| format!("failed to open config file: {}", path.display()));
        }
    };
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
///
/// CONC-7 (TASK-1093): this counter is **process-global**. Two parallel tests
/// that both call `reset_load_config_call_count()` and assert
/// `load_config_call_count() == N` will race — one test's `fetch_add` lands in
/// the other test's window. Every call site MUST be marked
/// `#[serial_test::serial]` so cargo's default parallel test runner does not
/// interleave them. The race is gated by convention, not by the type system;
/// reviewers grepping for `load_config_call_count` should verify each hit also
/// carries `#[serial]`.
#[cfg(any(test, feature = "test-support"))]
static LOAD_CONFIG_CALL_COUNT: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

/// Snapshot the current `load_config` invocation count.
///
/// **Hazard**: process-global state. See [`LOAD_CONFIG_CALL_COUNT`] for the
/// CONC-7 race details. Callers MUST be `#[serial_test::serial]`.
#[cfg(any(test, feature = "test-support"))]
pub fn load_config_call_count() -> usize {
    LOAD_CONFIG_CALL_COUNT.load(std::sync::atomic::Ordering::Relaxed)
}

/// Reset the `load_config` invocation count to zero.
///
/// **Hazard**: process-global state. See [`LOAD_CONFIG_CALL_COUNT`] for the
/// CONC-7 race details. Callers MUST be `#[serial_test::serial]`.
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

/// Load config and degrade to an empty [`Config`] on failure, surfacing the
/// error via both `tracing::warn!` (structured log) and [`crate::ui::warn`]
/// (user-visible). `context` describes the caller path (`"hook install"`,
/// `"about"`, `"early"`) and is included verbatim in both messages so logs
/// can be filtered and the user can correlate the warning to what they ran.
///
/// The fallback is [`Config::empty`] (no commands, themes, or stack), not
/// [`Config::default`]: TRAIT-4 / TASK-0872 gated `default()` to test
/// scaffolding so production fallbacks never carry blank-slate values that
/// a caller could mistake for a real config.
///
/// DUP-3 / TASK-0345: collapses the same fallback block previously duplicated
/// across `cli/main.rs`, `cli/about_cmd.rs`, and `cli/hook_shared.rs`.
pub fn load_config_or_default(context: &str) -> Config {
    match load_config() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %format!("{e:#}"), %context, "failed to load config");
            crate::ui::warn(format!(
                "failed to load config ({context}): {e:#}\n  continuing with an empty config (no commands, themes, or stack)"
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
///
/// PATTERN-1 (TASK-1222): the chosen *source* of the base directory (XDG vs
/// APPDATA vs HOME) is logged at debug too, so a Windows user inheriting a
/// Unix-style `XDG_CONFIG_HOME` (WSL leakage, dotfile sync) can spot why the
/// documented `%APPDATA%\ops\config.toml` location is being silently bypassed.
/// The base path is also rejected if it is empty or relative — those shapes
/// cannot be the right config home and silently honouring them only hides the
/// misconfiguration. Cross-platform tooling generally treats `XDG_CONFIG_HOME`
/// as authoritative when set, so we keep that precedence; the WSL leakage
/// edge case is documented rather than papered over.
pub(crate) fn global_config_path() -> Option<PathBuf> {
    let (config_dir, source) = if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        (PathBuf::from(xdg), "XDG_CONFIG_HOME")
    } else if cfg!(windows) {
        // CL-3: fall back through the shared `paths::home_dir` helper so
        // Windows-native paths use the same HOME → USERPROFILE order as the
        // rest of the crate.
        let dir = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(|| crate::paths::home_dir().map(|h| h.join("AppData/Roaming")))?;
        (dir, "APPDATA")
    } else {
        (crate::paths::home_dir()?.join(".config"), "HOME")
    };
    if config_dir.as_os_str().is_empty() || !config_dir.is_absolute() {
        debug!(
            source,
            base = ?config_dir.display(),
            "global config base path is empty or non-absolute; skipping global config"
        );
        return None;
    }
    let path = config_dir.join("ops/config");
    debug!(source, path = ?path.display(), "resolved global config base path");
    Some(path)
}

/// Load global config from standard paths.
///
/// ERR-1: a read/parse error on the global config surfaces as a hard error
/// with the path attached — a corrupted `~/.config/ops/config.toml` should
/// not be silently ignored, leaving the user thinking their config applied.
///
/// PATTERN-1 (TASK-1090): two filenames are tried, **in this order**:
///
/// 1. `<dir>/ops/config.toml` — the documented, conventional name.
/// 2. `<dir>/ops/config` — a bare-extension fallback retained for legacy
///    layouts (e.g. an older deployment that wrote the file without a `.toml`
///    suffix). The first existing file wins; if both exist, `config.toml`
///    takes precedence and the bare `config` is **silently ignored**. The
///    actually-loaded path is logged at `tracing::debug` so operators can
///    diagnose silent shadowing without strace.
fn load_global_config(config: &mut Config) -> anyhow::Result<()> {
    let Some(global_path) = global_config_path() else {
        return Ok(());
    };
    load_global_config_at(config, &global_path)
}

/// Test-friendly inner: try `<base>.toml` then `<base>` (bare-extension
/// legacy fallback). See [`load_global_config`] for the precedence contract.
fn load_global_config_at(config: &mut Config, global_path: &Path) -> anyhow::Result<()> {
    let to_try = [
        global_path.with_extension("toml"),
        global_path.to_path_buf(),
    ];
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

        // READ-5 / TASK-1129: `ops_toml_max_bytes` is now `OnceLock`-cached,
        // so an env-var dance here would observe whichever value an earlier
        // test happened to populate. Drive the cap-rejection branch via the
        // pure helper instead — the bail message is what the test pins.
        let result = read_capped_toml_file_with(&path, 64);

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

    /// PATTERN-1 / TASK-1090: when both `<dir>/ops/config.toml` and the
    /// legacy bare-extension `<dir>/ops/config` exist, `config.toml` MUST
    /// win. A stray bare-extension file (e.g. an extracted backup) must not
    /// silently shadow the documented filename.
    #[test]
    fn load_global_config_precedence_toml_over_bare() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("ops").join("config");
        fs::create_dir_all(base.parent().unwrap()).unwrap();

        // Bare file declares a command we expect NOT to be merged.
        fs::write(
            &base,
            r#"
[commands.from_bare]
program = "echo"
args = ["bare"]
"#,
        )
        .unwrap();
        // .toml file declares a different command — this one must win.
        fs::write(
            base.with_extension("toml"),
            r#"
[commands.from_toml]
program = "echo"
args = ["toml"]
"#,
        )
        .unwrap();

        let mut config = Config::default();
        load_global_config_at(&mut config, &base).unwrap();
        assert!(
            config.commands.contains_key("from_toml"),
            "config.toml must be loaded"
        );
        assert!(
            !config.commands.contains_key("from_bare"),
            "bare-extension config must be shadowed by config.toml"
        );
    }

    /// PATTERN-1 / TASK-1090: the bare-extension legacy fallback still
    /// loads when `config.toml` is absent. Removing this would silently
    /// break operators relying on the legacy layout.
    #[test]
    fn load_global_config_falls_back_to_bare_when_toml_missing() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("ops").join("config");
        fs::create_dir_all(base.parent().unwrap()).unwrap();
        fs::write(
            &base,
            r#"
[commands.from_bare]
program = "echo"
args = ["bare"]
"#,
        )
        .unwrap();

        let mut config = Config::default();
        load_global_config_at(&mut config, &base).unwrap();
        assert!(
            config.commands.contains_key("from_bare"),
            "bare fallback must load when config.toml is absent"
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
