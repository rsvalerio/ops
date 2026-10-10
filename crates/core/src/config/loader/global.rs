//! Global-config path resolution and load-from-disk.
//!
//! Owns the [`GLOBAL_CONFIG_PATH`] cache, the `XDG_CONFIG_HOME` /
//! `APPDATA` / `HOME` precedence matrix, and the bare-vs-`.toml`
//! filename precedence with the silent-shadow warn.

use std::path::{Path, PathBuf};
use std::sync::{PoisonError, RwLock, RwLockWriteGuard};

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
/// The base path is rejected if it is empty or relative — those shapes
/// cannot be the right config home, and honouring them would hide the
/// misconfiguration. `XDG_CONFIG_HOME` stays authoritative when set, so a
/// Windows user inheriting a Unix-style value (WSL leakage, dotfile sync)
/// bypasses `%APPDATA%\ops\config.toml`; the chosen source (XDG vs APPDATA
/// vs HOME) and the resolved path are logged at `tracing::debug` to make
/// that visible when diagnosing "config not loading" reports.
///
/// The resolution is cached so the env lookups and the debug breadcrumb
/// fire once per resolution rather than once per load. The outer `Option`
/// is "have we resolved yet"; the inner `Option<PathBuf>` is the result
/// (`None` when the base directory is empty or non-absolute and the global
/// config is skipped).
///
/// **Process-lifetime contract** (mirrors
/// [`crate::expand::Variables::from_env`]'s `TMPDIR_DISPLAY`): the resolved
/// path is captured on the first [`global_config_path`] call and reused.
/// Setting `XDG_CONFIG_HOME` / `APPDATA` / `HOME` via `std::env::set_var`
/// after the first call is **not** observed by later callers. The cache is
/// a `RwLock` rather than a `OnceLock` so the test-support hook
/// `reset_global_config_path_cache` can clear it between scenarios in one
/// binary.
static GLOBAL_CONFIG_PATH: RwLock<CachedGlobalConfigPath> = RwLock::new(None);

/// The value [`GLOBAL_CONFIG_PATH`] guards.
// The nesting is meaningful: the outer `Option` is "has the cache been
// populated?", the inner one is "was a global config found?".
#[allow(clippy::option_option)]
type CachedGlobalConfigPath = Option<Option<PathBuf>>;

/// Write-lock [`GLOBAL_CONFIG_PATH`], recovering from poisoning.
///
/// The guarded value is a memoised resolution: it is either unset or a
/// fully written result, so a holder that panicked cannot have left it
/// torn. Recovering the guard and clearing the poison flag keeps one
/// unrelated panic from failing every later config load in the process.
fn write_global_config_path() -> RwLockWriteGuard<'static, CachedGlobalConfigPath> {
    GLOBAL_CONFIG_PATH.write().unwrap_or_else(|poisoned| {
        GLOBAL_CONFIG_PATH.clear_poison();
        poisoned.into_inner()
    })
}

fn global_config_path() -> Option<PathBuf> {
    {
        // A poisoned read still sees a valid cached value; see
        // `write_global_config_path`.
        let r = GLOBAL_CONFIG_PATH
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(cached) = r.as_ref() {
            return cached.clone();
        }
    }
    let mut w = write_global_config_path();
    if let Some(cached) = w.as_ref() {
        return cached.clone();
    }
    let resolved = resolve_global_config_path();
    *w = Some(resolved.clone());
    resolved
}

/// Zero-sized capability token for [`reset_global_config_path_cache`].
///
/// Constructable only via [`GlobalConfigPathResetToken::new`], which is
/// itself gated to `#[cfg(any(test, feature = "test-support"))]` so an
/// accidental production caller cannot compile the reset path.
#[cfg(any(test, feature = "test-support"))]
#[non_exhaustive]
pub struct GlobalConfigPathResetToken {
    _private: (),
}

#[cfg(any(test, feature = "test-support"))]
impl GlobalConfigPathResetToken {
    /// Mint a token. Test-support / cfg(test) only.
    #[must_use]
    pub const fn new() -> Self {
        Self { _private: () }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Default for GlobalConfigPathResetToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Clear the `GLOBAL_CONFIG_PATH` cache so the next [`global_config_path`]
/// call re-resolves from the live env.
///
/// Test-support only: it lets a test change the base-directory env vars
/// after an earlier scenario in the same binary already resolved the path.
///
/// The `_token` parameter is a capability marker: see
/// [`GlobalConfigPathResetToken`]. Production builds (no `test-support`
/// feature) cannot construct the token and therefore cannot call the hook.
/// A lock poisoned by a panic in another test is recovered, not propagated.
#[cfg(any(test, feature = "test-support"))]
pub fn reset_global_config_path_cache(_token: GlobalConfigPathResetToken) {
    *write_global_config_path() = None;
}

/// Uncached resolver behind [`global_config_path`], which calls it whenever
/// the `RwLock`-backed [`GLOBAL_CONFIG_PATH`] cache is empty: on the first
/// lookup, and again on the first lookup after each
/// `reset_global_config_path_cache` call. Every invocation re-reads the
/// env and emits the `tracing::debug` source breadcrumb, so both happen
/// once per resolution, not once per process.
///
/// Public so tests that drive the env-precedence matrix (XDG vs HOME vs
/// APPDATA) can bypass the cache — production callers go through
/// [`global_config_path`] so the cache discipline holds.
pub fn resolve_global_config_path() -> Option<PathBuf> {
    let (config_dir, source) = if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        (PathBuf::from(xdg), "XDG_CONFIG_HOME")
    } else if cfg!(windows) {
        // Fall back through the shared `paths::home_dir` helper so
        // Windows-native paths use the same HOME → USERPROFILE order as the
        // rest of the crate. `home_dir` is the single source of truth for
        // the HOME-vs-USERPROFILE precedence policy on non-Unix targets;
        // documented there.
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
/// A read/parse error on the global config surfaces as a hard error
/// with the path attached — a corrupted `~/.config/ops/config.toml` should
/// not be silently ignored, leaving the user thinking their config applied.
///
/// Two filenames are tried, **in this order**:
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
/// When both the canonical `<base>.toml` and the legacy
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
    // The global config **follows** symlinks; the
    // workspace-relative layers do not. `~/.config/**` being a symlink is the
    // normal state under GNU Stow / chezmoi / nix home-manager, and `$HOME` is
    // the user's own — not a privilege boundary. Applying the repo-local
    // symlink refusal here aborts the whole layered load for a
    // dotfile-managed global config: `load_config_at` propagates this `Err` with `?` *before*
    // `.ops.toml` and `.ops.d` are read at all, so the user lost every command,
    // theme, and stack setting from their own repo over a symlink in their home
    // directory. The byte cap still applies. Read `super::SymlinkPolicy` before
    // changing this, and do not widen `Follow` over the workspace paths.
    for path in &[toml_path, bare_path] {
        match super::read_config_file_following_symlinks(path) {
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

    /// A panic while holding the `GLOBAL_CONFIG_PATH` lock poisons it; the
    /// cache has no invariant such a panic could break, so lookups and the
    /// reset hook must recover the guard instead of propagating the panic,
    /// and leave the lock healthy for later callers.
    #[test]
    #[serial_test::serial]
    fn global_config_path_recovers_from_a_poisoned_lock() {
        reset_global_config_path_cache(GlobalConfigPathResetToken::new());
        let expected = resolve_global_config_path();

        let poisoner = std::thread::spawn(|| {
            let _guard = GLOBAL_CONFIG_PATH.write();
            panic!("poison GLOBAL_CONFIG_PATH");
        });
        assert!(poisoner.join().is_err(), "the holder must have panicked");
        assert!(GLOBAL_CONFIG_PATH.is_poisoned());

        assert_eq!(global_config_path(), expected);
        assert!(
            !GLOBAL_CONFIG_PATH.is_poisoned(),
            "the write path must clear the poison flag"
        );
        assert_eq!(global_config_path(), expected, "the cached hit agrees");

        reset_global_config_path_cache(GlobalConfigPathResetToken::new());
    }

    /// After the `GLOBAL_CONFIG_PATH` cache has been
    /// resolved once, mutating `XDG_CONFIG_HOME` and then calling
    /// `global_config_path()` again returns the **old** value — the
    /// runtime contract is "set env before first call". The reset hook
    /// must clear the cache so a subsequent call observes the new env.
    #[test]
    #[serial_test::serial]
    fn reset_global_config_path_cache_observes_env_change() {
        // Prime the cache with one XDG value.
        let dir_a = tempfile::tempdir().unwrap();
        let prev = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", dir_a.path());
        reset_global_config_path_cache(GlobalConfigPathResetToken::new());
        let first = global_config_path().expect("XDG_CONFIG_HOME set");
        assert!(first.starts_with(dir_a.path()));

        // Now flip the env without resetting — the cache should still
        // hand back the old path, proving the cache is sticky.
        let dir_b = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", dir_b.path());
        let stale = global_config_path().expect("path resolved");
        assert!(
            stale.starts_with(dir_a.path()),
            "without reset, cache must return the prior resolution"
        );

        // After reset, the new env is observed.
        reset_global_config_path_cache(GlobalConfigPathResetToken::new());
        let fresh = global_config_path().expect("XDG_CONFIG_HOME set");
        assert!(fresh.starts_with(dir_b.path()));

        // Restore env.
        match prev {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        reset_global_config_path_cache(GlobalConfigPathResetToken::new());
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
}
