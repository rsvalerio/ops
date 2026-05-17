//! Configuration loading from files, directories, and environment variables.
//!
//! # Module layout (ARCH-1 / TASK-1471)
//!
//! - [`env`] — `OPS__*` env-var overlay merge.
//! - [`global`] — global config path resolver (XDG / APPDATA / HOME) and
//!   `~/.config/ops/config(.toml)` loader.
//! - [`conf_d`] — `.ops.d/*.toml` overlay walker and merger.
//! - this `mod.rs` — public `load_config*` entry points, the
//!   `.ops.toml` byte-cap reader, and the call-counter test-support
//!   surface.
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

mod conf_d;
mod env;
mod global;

use std::io::Read;
use std::path::Path;
use std::sync::OnceLock;

use anyhow::Context;
use tracing::{debug, instrument};

use super::merge::merge_config;
use super::{default_ops_toml, Config, ConfigOverlay};
use crate::text::{cached_byte_cap_env, open_refusing_symlinks};

#[cfg(test)]
pub(crate) use global::resolve_global_config_path;

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
    // SEC-25 (TASK-1468): refuse to follow symlinks at config paths. An
    // adversarial repo planting `.ops.toml -> /etc/shadow` would otherwise
    // be slurped into the TOML parser and echoed back through diagnostics.
    // Shared with `text::read_capped_to_string_with` so the two read_capped_*
    // entry points cannot diverge again.
    let mut file = match open_refusing_symlinks(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            // SEC-21 (TASK-1472): Debug-format the path so a user-controlled
            // cwd containing newlines / ANSI escapes cannot forge log lines
            // through `tracing::warn!` consumers of this anyhow chain (e.g.
            // `load_config_or_default_with`). Matches the policy at
            // `text.rs::for_each_trimmed_line_with` and `stack/detect.rs`.
            return Err(e)
                .with_context(|| format!("failed to open config file: {:?}", path.display()));
        }
    };
    let limit = cap.saturating_add(1);
    let mut content = String::new();
    (&mut file)
        .take(limit)
        .read_to_string(&mut content)
        .with_context(|| format!("failed to read config file: {:?}", path.display()))?;
    if content.len() as u64 > cap {
        // SEC-21 (TASK-1472): same Debug-format policy for the bounded-read
        // bail. The `?` debug repr keeps newlines / ANSI escapes inert.
        anyhow::bail!(
            "config file at {:?} exceeds {cap} bytes (override via {OPS_TOML_MAX_BYTES_ENV})",
            path.display()
        );
    }
    Ok(Some(content))
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

    global::load_global_config(&mut config).context("loading global config")?;

    let local_path = workspace_root.join(".ops.toml");
    if let Some(overlay) =
        read_config_file(&local_path).context("loading local .ops.toml config")?
    {
        debug!(path = ?local_path.display(), "merging local config");
        merge_config(&mut config, overlay);
    }

    conf_d::merge_conf_d(&mut config, workspace_root).context("loading .ops.d overlay configs")?;

    env::merge_env_vars(&mut config).context("loading OPS__ environment overlay")?;

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
        .with_context(|| format!("failed to parse config file: {:?}", path.display()))?;
    Ok(Some(overlay))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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

    /// SEC-25 / TASK-1468: a planted `.ops.toml -> /etc/passwd` (or any
    /// other privileged target) must be rejected by `read_capped_toml_file`
    /// before `File::open` follows the symlink. The shared
    /// `text::open_refusing_symlinks` helper applies `O_NOFOLLOW` so the
    /// kernel atomically refuses to dereference the link — closing the
    /// TOCTOU race that a `symlink_metadata` pre-probe would have left open.
    #[cfg(unix)]
    #[test]
    fn read_capped_toml_file_refuses_to_follow_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("secret.toml");
        fs::write(&target, b"[secret]\nkey = \"sentinel-must-not-leak\"\n").unwrap();
        let link = dir.path().join(".ops.toml");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let err = read_capped_toml_file_with(&link, 1024)
            .expect_err("symlinked .ops.toml must be rejected");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("symlink"),
            "error should mention symlink, got: {msg}"
        );
        assert!(
            !msg.contains("sentinel-must-not-leak"),
            "target contents must not leak through the error, got: {msg}"
        );
    }

    /// SEC-21 / TASK-1472: when a `.ops.toml` path contains a newline / ANSI
    /// escape, the bounded-read `bail!` and the open/read `with_context`
    /// formatters must render the path through Debug so consumers that emit
    /// the anyhow chain via `tracing::warn!` cannot be tricked into forged
    /// log lines. We exercise the bail branch (oversize) — it goes through
    /// the same Debug-format policy as the open/read branches just above.
    #[cfg(unix)]
    #[test]
    fn read_capped_toml_file_error_debug_escapes_control_characters() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("evil\n\u{1b}[31m.ops.toml");
        fs::write(&path, "x".repeat(256)).unwrap();

        let err = read_capped_toml_file_with(&path, 16)
            .expect_err("oversize payload must error so we see the bail!");
        let msg = format!("{err:#}");
        assert!(
            !msg.contains('\n'),
            "raw newline must be Debug-escaped, got: {msg}"
        );
        assert!(
            !msg.contains('\u{1b}'),
            "ANSI escape must be Debug-escaped, got: {msg}"
        );
        assert!(
            msg.contains("\\n"),
            "newline must render as escape sequence, got: {msg}"
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
