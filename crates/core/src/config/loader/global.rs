//! ARCH-1 / TASK-1471: global-config path resolution and load-from-disk.
//!
//! Extracted from the historical grab-bag `loader.rs`. Owns the
//! [`GLOBAL_CONFIG_PATH`] `OnceLock`, the `XDG_CONFIG_HOME` /
//! `APPDATA` / `HOME` precedence matrix, and the bare-vs-`.toml`
//! filename precedence with the silent-shadow warn.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use tracing::debug;

use super::super::{merge::merge_config, Config};

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
///
/// PERF-3 / TASK-1419: cache the resolved global config path behind a
/// `OnceLock<Option<PathBuf>>` so the env lookups (`XDG_CONFIG_HOME`,
/// `APPDATA`, `HOME`/`USERPROFILE`) and source-of-base-dir `tracing::debug`
/// fire at most once per process.
///
/// **Process-lifetime contract** (mirrors
/// [`crate::expand::Variables::from_env`]'s `TMPDIR_DISPLAY`): the resolved
/// path is captured on the first [`global_config_path`] call and never
/// refreshed. Setting `XDG_CONFIG_HOME` / `APPDATA` / `HOME` via
/// `std::env::set_var` after the first call will **not** be observed by
/// subsequent callers. Tests that need a specific base directory MUST set
/// the relevant env var before any code path that triggers
/// `load_config` / `load_config_at` / `load_config_or_default*` runs.
static GLOBAL_CONFIG_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

fn global_config_path() -> Option<PathBuf> {
    GLOBAL_CONFIG_PATH
        .get_or_init(resolve_global_config_path)
        .clone()
}

/// Inner resolver invoked exactly once by the [`GLOBAL_CONFIG_PATH`]
/// `OnceLock` initialiser. Splitting the resolution out keeps the env
/// lookups and the one-shot `tracing::debug` source breadcrumb co-located
/// while letting the caller hand back an `Option<PathBuf>` clone on every
/// hit.
///
/// Exposed `pub(crate)` so tests that need to drive the env-precedence
/// matrix (XDG vs HOME vs APPDATA) can bypass the
/// [`GLOBAL_CONFIG_PATH`] `OnceLock` — production callers should always go
/// through [`global_config_path`] so the cache discipline holds.
pub(crate) fn resolve_global_config_path() -> Option<PathBuf> {
    let (config_dir, source) = if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        (PathBuf::from(xdg), "XDG_CONFIG_HOME")
    } else if cfg!(windows) {
        // CL-3: fall back through the shared `paths::home_dir` helper so
        // Windows-native paths use the same HOME → USERPROFILE order as the
        // rest of the crate. READ-1 / TASK-1434: `home_dir` is the single
        // source of truth for the HOME-vs-USERPROFILE precedence policy on
        // non-Unix targets; documented there.
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
pub(super) fn load_global_config(config: &mut Config) -> anyhow::Result<()> {
    let Some(global_path) = global_config_path() else {
        return Ok(());
    };
    load_global_config_at(config, &global_path)
}

/// Test-friendly inner: try `<base>.toml` then `<base>` (bare-extension
/// legacy fallback). See [`load_global_config`] for the precedence contract.
///
/// READ-5 / TASK-1403: when both the canonical `<base>.toml` and the legacy
/// bare-extension `<base>` exist, the bare file is shadowed. A `tracing::warn`
/// surfaces the situation so operators who left a stale legacy file in place
/// see a signal at the level the silent-edit-loss deserves.
fn load_global_config_at(config: &mut Config, global_path: &Path) -> anyhow::Result<()> {
    let toml_path = global_path.with_extension("toml");
    let bare_path = global_path.to_path_buf();
    if toml_path != bare_path && toml_path.exists() && bare_path.exists() {
        tracing::warn!(
            canonical = ?toml_path.display(),
            legacy = ?bare_path.display(),
            "global config: legacy bare-extension file is shadowed by canonical .toml; edits to the legacy file are ignored"
        );
    }
    for path in &[toml_path, bare_path] {
        match super::read_config_file(path) {
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
}
