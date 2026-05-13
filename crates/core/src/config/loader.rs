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
///
/// ERR-1 / TASK-1389: a non-UTF-8 `OPS__*` key (rare but possible on Unix —
/// e.g. an exec'd shim that wrote raw bytes via `OsString::from_vec`) is
/// invisible to the `config` crate's `Environment::with_prefix("OPS")` source
/// but is still operator intent. Count and `tracing::warn!` once per
/// `merge_env_vars` call so the "OPS__ override didn't apply" symptom has a
/// breadcrumb instead of vanishing silently.
///
/// PERF-3 / TASK-1414: the success-path early-out (`vars_os().any(...)`)
/// short-circuits without allocating a `Vec<String>` of every OPS__ key. The
/// error-context closures are the only callers that need the materialised key
/// list, so the collection is deferred into [`collect_ops_keys`] and only runs
/// on the failure path.
fn merge_env_vars(config: &mut Config) -> anyhow::Result<()> {
    let (has_ops_keys, non_utf8_count) = scan_ops_env_keys();
    if non_utf8_count > 0 {
        tracing::warn!(
            count = non_utf8_count,
            "ignored non-UTF-8 OPS__ environment keys; the `config` crate cannot \
             observe them — operator overrides relying on these keys will not apply"
        );
    }
    if !has_ops_keys {
        return Ok(());
    }
    let env_config = config_crate::Config::builder()
        .add_source(config_crate::Environment::with_prefix("OPS").separator("__"))
        .build()
        .with_context(|| {
            let keys = collect_ops_keys();
            format!("failed to build OPS__ env config (keys: {keys:?})")
        })?;
    let env_overlay: ConfigOverlay = env_config.try_deserialize().with_context(|| {
        let keys = collect_ops_keys();
        format!("failed to deserialize OPS__ env config (keys: {keys:?})")
    })?;
    merge_config(config, env_overlay);
    Ok(())
}

/// Return `(has_ops_keys, non_utf8_count)` for the current process env.
///
/// PERF-3 / TASK-1414: avoids the `Vec<String>` allocation on the success
/// path. ERR-1 / TASK-1389: tracks non-UTF-8 `OPS__*` keys via the raw
/// `OsStr::as_encoded_bytes` prefix so the diagnostic warn can fire even when
/// `OsString::into_string()` would have dropped the entry.
fn scan_ops_env_keys() -> (bool, usize) {
    let mut has_ops = false;
    let mut non_utf8 = 0usize;
    for (k, _) in std::env::vars_os() {
        match k.to_str() {
            Some(s) if s.starts_with("OPS__") => has_ops = true,
            Some(_) => {}
            None => {
                if k.as_encoded_bytes().starts_with(b"OPS__") {
                    non_utf8 += 1;
                }
            }
        }
    }
    (has_ops, non_utf8)
}

/// Collect the UTF-8 `OPS__*` env keys. Only the error-context closures call
/// this; the success path skips it entirely (TASK-1414).
fn collect_ops_keys() -> Vec<String> {
    std::env::vars_os()
        .filter_map(|(k, _)| k.into_string().ok())
        .filter(|k| k.starts_with("OPS__"))
        .collect()
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

/// Load the layered ops config rooted at the current process working
/// directory.
///
/// READ-5 / TASK-1446: this entry point is **cwd-sensitive** — it resolves
/// `.ops.toml` and `.ops.d/` relative to the live process cwd. Callers that
/// need to be explicit about the workspace root (long-running daemons, code
/// that spawns work across cwds, future async refactors) should call
/// [`load_config_at`] with a known [`Path`] instead. The `#[serial_test::serial]`
/// discipline on `tests/loader.rs` exists for the same reason.
#[instrument(skip_all)]
pub fn load_config() -> anyhow::Result<Config> {
    let cwd = std::env::current_dir().context("resolving workspace root from current_dir")?;
    load_config_at(&cwd)
}

/// Load the layered ops config rooted at `workspace_root`.
///
/// `.ops.toml` and `.ops.d/` are resolved relative to `workspace_root`;
/// the global config and `OPS__` env overlay are independent of the
/// workspace root. Prefer this entry point in production callers so the
/// cwd coupling lives in the type signature rather than in
/// `Path::new(".ops.toml")` literals deep in the loader.
#[instrument(skip_all)]
pub fn load_config_at(workspace_root: &Path) -> anyhow::Result<Config> {
    #[cfg(any(test, feature = "test-support"))]
    LOAD_CONFIG_CALL_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut config: Config =
        toml::from_str(default_ops_toml()).context("failed to parse internal default config")?;
    debug!("loaded internal default config");

    load_global_config(&mut config).context("loading global config")?;

    let local_path = workspace_root.join(".ops.toml");
    if let Some(overlay) =
        read_config_file(&local_path).context("loading local .ops.toml config")?
    {
        debug!(path = ?local_path.display(), "merging local config");
        merge_config(&mut config, overlay);
    }

    merge_conf_d(&mut config, workspace_root).context("loading .ops.d overlay configs")?;

    merge_env_vars(&mut config).context("loading OPS__ environment overlay")?;

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
///
/// READ-5 / TASK-1446: cwd-sensitive convenience that delegates to
/// [`load_config_or_default_at`]; prefer the explicit form in production
/// callers.
pub fn load_config_or_default(context: &str) -> Config {
    load_config_or_default_with(load_config(), context)
}

/// Workspace-root-aware variant of [`load_config_or_default`]. Use this in
/// CLI entry points and extensions where the workspace root is captured
/// explicitly (via `std::env::current_dir()`) at the boundary.
pub fn load_config_or_default_at(workspace_root: &Path, context: &str) -> Config {
    load_config_or_default_with(load_config_at(workspace_root), context)
}

fn load_config_or_default_with(result: anyhow::Result<Config>, context: &str) -> Config {
    match result {
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

/// Read sorted `.toml` files from a directory.
///
/// Returns `Ok(None)` only when the directory itself does not exist —
/// every other failure (permission flip, racing rename on a `DirEntry`,
/// `read_dir` IO error) is surfaced as an `Err` so the layered-config
/// load fails loudly. See [`merge_conf_d`] for the "loud failure"
/// contract.
///
/// ERR-7 / TASK-1400: a `DirEntry` whose `?` access fails used to be
/// dropped with a warn-and-skip; this asymmetry meant a permission flip
/// or racing rename on a single overlay file made it disappear while
/// the rest of the merge proceeded, producing a config that differed
/// from what the operator authored.
fn read_conf_d_files(dir: &Path) -> anyhow::Result<Option<Vec<PathBuf>>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e)
                .with_context(|| format!("failed to read .ops.d directory: {}", dir.display()));
        }
    };
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.with_context(|| {
            format!(
                "failed to read entry in .ops.d directory: {}",
                dir.display()
            )
        })?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            files.push(path);
        }
    }
    files.sort();
    Ok(Some(files))
}

/// Merge every `.ops.d/*.toml` overlay, in sorted order.
///
/// ERR-1: a parse or IO error on any single overlay file surfaces as a hard
/// error with the offending path in context rather than being silently
/// dropped. Users whose overlay "mysteriously does nothing" in CI should see
/// a loud failure instead of a tracing warning that gets swallowed.
///
/// ERR-4 / TASK-1448: a `.toml` entry that resolves to a broken symlink
/// (`DirEntry::path` exists in the listing but `File::open` reports
/// `NotFound`) is treated as a hard error here rather than being silently
/// mapped to `Ok(None)` by [`read_capped_toml_file`]. The listing already
/// proved the entry existed; an unreadable target between listing and open
/// is the "loud failure" contract, not benign absence.
fn merge_conf_d(config: &mut Config, workspace_root: &Path) -> anyhow::Result<()> {
    let Some(files) = read_conf_d_files(&workspace_root.join(".ops.d"))? else {
        return Ok(());
    };
    for path in files {
        match read_config_file(&path) {
            Ok(Some(overlay)) => {
                debug!(path = ?path.display(), "merging conf.d config");
                merge_config(config, overlay);
            }
            Ok(None) => {
                anyhow::bail!(
                    "config overlay listed in .ops.d disappeared or is a broken symlink: {}",
                    path.display()
                );
            }
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

pub(crate) fn global_config_path() -> Option<PathBuf> {
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
fn load_global_config(config: &mut Config) -> anyhow::Result<()> {
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

        let files = read_conf_d_files(dir.path()).unwrap().unwrap();
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
        let result = read_conf_d_files(std::path::Path::new("/nonexistent/ops.d")).unwrap();
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
        merge_conf_d(&mut config, dir.path()).unwrap();

        assert!(config.commands.contains_key("extra"));
    }

    /// ERR-7 / TASK-1400: a `read_dir` failure (e.g. permission denied on
    /// the `.ops.d` directory itself) must surface as a hard error with the
    /// offending path attached rather than being warn-and-skipped. Mirrors
    /// the "loud failure" contract that already governs parse errors.
    #[cfg(unix)]
    #[test]
    fn read_conf_d_files_propagates_read_dir_error() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let unreadable = dir.path().join("locked");
        fs::create_dir(&unreadable).unwrap();
        // Strip read+execute bits so read_dir fails with EACCES.
        let mut perms = fs::metadata(&unreadable).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&unreadable, perms).unwrap();

        let result = read_conf_d_files(&unreadable);

        // Restore perms before asserting so tempdir cleanup succeeds.
        let mut restore = fs::metadata(&unreadable).unwrap().permissions();
        restore.set_mode(0o700);
        fs::set_permissions(&unreadable, restore).ok();

        let err = result.expect_err("unreadable .ops.d must error");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("locked") && msg.contains(".ops.d"),
            "error must name the offending directory, got: {msg}"
        );
    }

    /// ERR-4 / TASK-1448: a broken `.toml` symlink in `.ops.d` is listed by
    /// `read_dir` but fails to open at merge time. The "loud failure"
    /// contract on `merge_conf_d` requires this to abort the load, not to be
    /// silently mapped to `Ok(None)` by `read_capped_toml_file`.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn merge_conf_d_rejects_broken_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let ops_d = dir.path().join(".ops.d");
        fs::create_dir(&ops_d).unwrap();
        std::os::unix::fs::symlink(
            dir.path().join("does-not-exist.toml"),
            ops_d.join("dangling.toml"),
        )
        .unwrap();

        let mut config = Config::default();
        let result = merge_conf_d(&mut config, dir.path());

        let err = result.expect_err("broken symlink overlay must error");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("dangling.toml"),
            "error must name the broken overlay, got: {msg}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn merge_conf_d_propagates_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let ops_d = dir.path().join(".ops.d");
        fs::create_dir(&ops_d).unwrap();
        fs::write(ops_d.join("broken.toml"), "not = = valid {{{").unwrap();

        let mut config = Config::default();
        let result = merge_conf_d(&mut config, dir.path());

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

    /// ERR-1 / TASK-1421: a parse failure in any single load layer must
    /// surface with a top-level "loading <layer> ..." breadcrumb so a
    /// future reorder of the layer chain stays visible in error output.
    #[test]
    #[serial_test::serial]
    fn load_config_local_parse_error_names_layer() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".ops.toml"), "not = = valid {{{").unwrap();

        // Neutralise XDG/global config lookups so the failure pins to the
        // local layer instead of either preceding step.
        let prev_xdg = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", dir.path().join("xdg-empty"));
        let result = load_config_at(dir.path());
        match prev_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }

        let err = result.expect_err("broken .ops.toml must error");
        let msg = format!("{err:#}");
        assert!(
            msg.starts_with("loading local .ops.toml config"),
            "error chain must start with the layer breadcrumb, got: {msg}"
        );
    }

    /// ERR-1 / TASK-1389: a non-UTF-8 `OPS__*` key in the process env is
    /// invisible to the `config` crate's `Environment::with_prefix("OPS")`
    /// source. `scan_ops_env_keys` must surface the count so
    /// [`merge_env_vars`] can emit the diagnostic warn (rather than dropping
    /// it silently as the prior `into_string().ok()` filter did).
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn scan_ops_env_keys_counts_non_utf8_ops_keys() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        // Build a key that begins with `OPS__` but contains non-UTF-8 trailing
        // bytes. The raw bytes are valid as an OsString but `into_string()`
        // returns Err so the previous diagnostic path would have dropped it.
        let mut raw = b"OPS__BAD_".to_vec();
        raw.extend_from_slice(&[0xff, 0xfe, 0xfd]);
        let key: OsString = OsString::from_vec(raw);
        // SAFETY: test-only guard via #[serial] attribute.
        unsafe { std::env::set_var(&key, "x") };

        let (_, non_utf8) = scan_ops_env_keys();

        // SAFETY: test-only guard via #[serial] attribute.
        unsafe { std::env::remove_var(&key) };

        assert!(non_utf8 >= 1, "non-UTF-8 OPS__ key must be counted");
    }

    /// ERR-1 / TASK-1389: with no non-UTF-8 OPS__ keys present, the diagnostic
    /// counter must stay at zero so the warn does not fire spuriously.
    #[test]
    #[serial_test::serial]
    fn scan_ops_env_keys_zero_when_only_utf8_keys() {
        // The harness env may already carry OPS__ vars from prior tests; this
        // assertion only pins the non-UTF-8 counter, not the presence flag.
        let (_, non_utf8) = scan_ops_env_keys();
        assert_eq!(
            non_utf8, 0,
            "no non-UTF-8 OPS__ keys expected in baseline env"
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
