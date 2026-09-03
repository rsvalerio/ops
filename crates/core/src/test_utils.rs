//! Shared test utilities for ops unit tests.
//!
//! # Surface index (READ-3)
//!
//! This module is compiled under `#[cfg(any(test, feature = "test-support"))]`
//! and re-exported to downstream crates via the `test-support` feature. The
//! sections below enumerate the public surface and what each item guarantees;
//! anything not listed is internal and may move or change without notice.
//!
//! ## Stability contract
//!
//! - **Test-only API.** Nothing here is part of `ops-core`'s production
//!   surface. Items may evolve faster than the main API and are not bound
//!   by the crate's semver promises.
//! - **Cross-crate consumers** (e.g. `ops-cli`, `ops-runner`) bind to the
//!   public-under-feature surface enumerated below.
//! - **`#[cfg(test)]` helpers** are compile-gated to `cargo test` of this
//!   crate and are not visible to downstream `test-support` consumers; mark
//!   anything new the same way when it depends on a dev-only dependency.
//!
//! ## `CommandSpec` / `ExecCommandSpec` constructors (public-under-feature)
//!
//! - [`exec_spec`], [`exec_spec_with_cwd`] — build an [`ExecCommandSpec`].
//! - [`platform_exec_spec`] — pick between Unix and Windows invocation forms.
//! - [`echo_cmd`], [`true_cmd`], [`false_cmd`], [`sleep_cmd`] — common
//!   cross-platform stand-ins for shell builtins.
//! - [`composite_cmd`], [`parallel_cmd`] — build a [`CompositeCommandSpec`]
//!   (sequential / parallel respectively).
//! - [`make_test_output`] — synthesize a [`std::process::Output`] with a
//!   given exit code and stdio bytes; abstracts the per-platform
//!   `ExitStatusExt::from_raw` quirk.
//!
//! ## Config builders (public-under-feature)
//!
//! - [`TestConfigBuilder`] — fluent builder for [`Config`]. See its rustdoc
//!   for the kept-in-parity method list with [`ConfigOverlayBuilder`].
//! - [`ConfigOverlayBuilder`] — fluent builder for [`ConfigOverlay`].
//! - [`test_config_with_commands`] — one-shot [`Config`] from a command map.
//!
//! ## Environment / runtime helpers (public-under-feature)
//!
//! - [`EnvGuard`] — RAII guard that restores an env var on drop. Requires
//!   `#[serial]` from `serial_test` on the test; see the struct rustdoc.
//! - [`isolate_global_config`] — point `XDG_CONFIG_HOME` at an empty
//!   directory and clear the global-config resolver cache for the lifetime
//!   of the returned [`GlobalConfigIsolation`] guard. Also requires
//!   `#[serial]`.
//! - [`is_root_euid`] — true on Unix when EUID is 0; tests that depend on
//!   DAC-permission denial must `return` early when this is true (see
//!   TEST-19 in the function rustdoc).
//! - [`CwdGuard`] / [`CWD_MUTEX`] — DRY-1 / TASK-2034: the workspace's one
//!   working-directory guard. It serialises on the mutex itself, so a caller
//!   that forgets `#[serial]` still cannot race another CWD-dependent test.
//!
//! ## Tracing capture (public-under-feature)
//!
//! DUP-3 / TASK-2014: the workspace's single tracing-capture harness, so no
//! crate has to re-derive the global-dispatcher pin whose absence is a silent
//! flake. `ops_about::test_support` re-exports these for the extensions.
//!
//! - [`capture_tracing`] — run a closure under a thread-local subscriber at a
//!   given level, returning the rendered output and the closure's value.
//! - [`capture_warn`] — [`capture_tracing`] fixed at `WARN`.
//! - [`capture_dispatch`] — the same subscriber as a detached
//!   [`tracing::Dispatch`], for work that does not run on the calling thread.
//! - [`count_warnings`] — the same, counting `WARN` events instead of
//!   rendering them.
//! - [`TracingBuf`], [`WarnCounter`] — the underlying sinks, for the rare
//!   test that installs its own subscriber (e.g. one per spawned thread).
//! - [`pin_global_dispatcher`] — what those rare tests must call first; the
//!   helpers above already do.
//!
//! ## Internal helpers (not part of the surface contract)
//!
//! - `proptest_strategies` (test-only) — proptest generators used by this
//!   crate's property tests only.
//!
//! [`Config`]: crate::config::Config
//! [`ConfigOverlay`]: crate::config::ConfigOverlay
//! [`ExecCommandSpec`]: crate::config::ExecCommandSpec
//! [`CompositeCommandSpec`]: crate::config::CompositeCommandSpec

use indexmap::IndexMap;
use std::collections::HashMap;

use crate::config::theme_types::ThemeConfig;
use crate::config::{
    CommandSpec, CompositeCommandSpec, ConfigOverlay, ExecCommandSpec, ExtensionConfigOverlay,
    OutputConfig, OutputConfigOverlay,
};

/// Create an `ExecCommandSpec` with the given program and args.
#[must_use]
pub fn exec_spec(program: &str, args: &[&str]) -> ExecCommandSpec {
    exec_spec_with_cwd(program, args, None)
}

/// Create an `ExecCommandSpec` with an optional cwd.
pub fn exec_spec_with_cwd(
    program: &str,
    args: &[&str],
    cwd: Option<std::path::PathBuf>,
) -> ExecCommandSpec {
    ExecCommandSpec {
        program: program.to_string(),
        args: args.iter().map(std::string::ToString::to_string).collect(),
        cwd,
        ..Default::default()
    }
}

/// Create an `ExecCommandSpec` that works on both Unix and Windows.
#[must_use]
pub fn platform_exec_spec(unix: (&str, &[&str]), windows: (&str, &[&str])) -> ExecCommandSpec {
    if cfg!(windows) {
        exec_spec(windows.0, windows.1)
    } else {
        exec_spec(unix.0, unix.1)
    }
}

/// Create an `ExecCommandSpec` that echoes a message.
#[must_use]
pub fn echo_cmd(msg: &str) -> ExecCommandSpec {
    platform_exec_spec(("echo", &[msg]), ("cmd", &["/C", "echo", msg]))
}

/// Create an `ExecCommandSpec` that exits with success (true).
#[must_use]
pub fn true_cmd() -> ExecCommandSpec {
    platform_exec_spec(("true", &[]), ("cmd", &["/C", "exit", "0"]))
}

/// Create an `ExecCommandSpec` that exits with failure (false).
#[must_use]
pub fn false_cmd() -> ExecCommandSpec {
    platform_exec_spec(("false", &[]), ("cmd", &["/C", "exit", "1"]))
}

/// Create an `ExecCommandSpec` that sleeps for the given number of seconds.
#[must_use]
pub fn sleep_cmd(secs: u64) -> ExecCommandSpec {
    let secs_str = secs.to_string();
    if cfg!(windows) {
        exec_spec(
            "ping",
            &["-n", &format!("{}", secs.saturating_add(1)), "127.0.0.1"],
        )
    } else {
        exec_spec("sleep", &[&secs_str])
    }
}

/// Create a composite command spec from a list of command names.
#[allow(dead_code)]
pub fn composite_cmd(commands: &[&str]) -> CompositeCommandSpec {
    CompositeCommandSpec {
        commands: commands
            .iter()
            .map(std::string::ToString::to_string)
            .collect(),
        parallel: false,
        fail_fast: true,
        help: None,
        aliases: Vec::new(),
        category: None,
    }
}

/// Create a parallel composite command spec from a list of command names.
#[allow(dead_code)]
pub fn parallel_cmd(commands: &[&str]) -> CompositeCommandSpec {
    CompositeCommandSpec {
        commands: commands
            .iter()
            .map(std::string::ToString::to_string)
            .collect(),
        parallel: true,
        fail_fast: true,
        help: None,
        aliases: Vec::new(),
        category: None,
    }
}

/// Builder for creating test configs.
///
/// # DUP-002: Shared Pattern with `ConfigOverlayBuilder`
///
/// Both `TestConfigBuilder` and `ConfigOverlayBuilder` provide similar fluent APIs
/// (`exec()`, `composite()`, `theme()`). While a shared trait could reduce duplication,
/// the builders produce different output types (`Config` vs `ConfigOverlay`), making
/// a trait abstraction overly complex for test utilities. The current duplication is
/// acceptable because:
///
/// 1. Both builders are test-only and not part of the public API
/// 2. The pattern is simple enough that maintenance burden is low
/// 3. A trait would require associated types and make the API less ergonomic
#[allow(dead_code)]
#[derive(Debug)]
pub struct TestConfigBuilder {
    output: OutputConfig,
    commands: IndexMap<String, CommandSpec>,
}

#[allow(dead_code)]
impl TestConfigBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            output: OutputConfig::default(),
            commands: IndexMap::new(),
        }
    }

    #[must_use]
    pub fn exec(mut self, name: &str, program: &str, args: &[&str]) -> Self {
        self.commands.insert(
            name.to_string(),
            CommandSpec::Exec(exec_spec(program, args)),
        );
        self
    }

    /// DUP-002: Create a `CommandSpec::Exec` variant directly.
    ///
    /// This is useful when you need the `CommandSpec` variant for tests that
    /// require the wrapped type rather than the inner `ExecCommandSpec`.
    #[must_use]
    pub fn raw_exec(_name: &str, program: &str, args: &[&str]) -> CommandSpec {
        CommandSpec::Exec(exec_spec(program, args))
    }

    #[must_use]
    pub fn command(mut self, name: &str, spec: CommandSpec) -> Self {
        self.commands.insert(name.to_string(), spec);
        self
    }

    #[must_use]
    pub fn composite(mut self, name: &str, commands: &[&str]) -> Self {
        self.commands.insert(
            name.to_string(),
            CommandSpec::Composite(composite_cmd(commands)),
        );
        self
    }

    #[must_use]
    pub fn parallel_composite(mut self, name: &str, commands: &[&str]) -> Self {
        self.commands.insert(
            name.to_string(),
            CommandSpec::Composite(parallel_cmd(commands)),
        );
        self
    }

    #[must_use]
    pub fn theme(mut self, theme: &str) -> Self {
        self.output.theme = theme.to_string();
        self
    }

    #[must_use]
    pub const fn columns(mut self, columns: u16) -> Self {
        self.output.columns = columns;
        self
    }

    #[must_use]
    pub const fn show_error_detail(mut self, show: bool) -> Self {
        self.output.show_error_detail = show;
        self
    }

    #[must_use]
    pub const fn stderr_tail_lines(mut self, n: usize) -> Self {
        self.output.stderr_tail_lines = n;
        self
    }

    #[must_use]
    pub fn build(self) -> crate::config::Config {
        crate::config::Config {
            output: self.output,
            commands: self.commands,
            data: crate::config::DataConfig::default(),
            themes: IndexMap::new(),
            extensions: crate::config::ExtensionConfig::default(),
            about: crate::config::AboutConfig::default(),
            stack: None,
        }
    }
}

impl Default for TestConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// DUP-001: Builder for creating `ConfigOverlay` in tests.
///
/// Reduces boilerplate in config tests by providing a fluent API
/// for constructing overlays with only the fields needed.
#[allow(dead_code)]
#[derive(Debug)]
pub struct ConfigOverlayBuilder {
    output: Option<OutputConfigOverlay>,
    commands: Option<IndexMap<String, CommandSpec>>,
    themes: Option<IndexMap<String, ThemeConfig>>,
    extensions: Option<ExtensionConfigOverlay>,
}

#[allow(dead_code)]
impl ConfigOverlayBuilder {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            output: None,
            commands: None,
            themes: None,
            extensions: None,
        }
    }

    #[must_use]
    pub fn output(mut self, output: OutputConfigOverlay) -> Self {
        self.output = Some(output);
        self
    }

    #[must_use]
    pub fn theme(self, theme: impl Into<String>) -> Self {
        self.output(OutputConfigOverlay {
            theme: Some(theme.into()),
            ..Default::default()
        })
    }

    #[must_use]
    pub fn columns(self, columns: u16) -> Self {
        self.output(OutputConfigOverlay {
            columns: Some(columns),
            ..Default::default()
        })
    }

    #[must_use]
    pub fn show_error_detail(self, show: bool) -> Self {
        self.output(OutputConfigOverlay {
            show_error_detail: Some(show),
            ..Default::default()
        })
    }

    #[must_use]
    pub fn commands(mut self, commands: IndexMap<String, CommandSpec>) -> Self {
        self.commands = Some(commands);
        self
    }

    #[must_use]
    pub fn exec(self, name: &str, program: &str, args: &[&str]) -> Self {
        let mut cmds = self.commands.unwrap_or_default();
        cmds.insert(
            name.to_string(),
            CommandSpec::Exec(exec_spec(program, args)),
        );
        Self {
            commands: Some(cmds),
            ..self
        }
    }

    #[must_use]
    pub fn composite(self, name: &str, commands: &[&str]) -> Self {
        let mut cmds = self.commands.unwrap_or_default();
        cmds.insert(
            name.to_string(),
            CommandSpec::Composite(composite_cmd(commands)),
        );
        Self {
            commands: Some(cmds),
            ..self
        }
    }

    #[must_use]
    pub fn themes(mut self, themes: IndexMap<String, ThemeConfig>) -> Self {
        self.themes = Some(themes);
        self
    }

    #[must_use]
    pub fn custom_theme(self, name: &str, theme: ThemeConfig) -> Self {
        let mut themes = self.themes.unwrap_or_default();
        themes.insert(name.to_string(), theme);
        Self {
            themes: Some(themes),
            ..self
        }
    }

    #[must_use]
    pub fn extensions(mut self, extensions: ExtensionConfigOverlay) -> Self {
        self.extensions = Some(extensions);
        self
    }

    #[must_use]
    pub fn enabled_extensions(self, enabled: Vec<String>) -> Self {
        self.extensions(ExtensionConfigOverlay {
            enabled: Some(enabled),
        })
    }

    #[must_use]
    pub fn build(self) -> ConfigOverlay {
        ConfigOverlay {
            output: self.output,
            commands: self.commands,
            data: None,
            themes: self.themes,
            extensions: self.extensions,
            about: None,
            stack: None,
        }
    }
}

impl Default for ConfigOverlayBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a simple test config with the given commands.
pub fn test_config_with_commands<S: std::hash::BuildHasher>(
    commands: HashMap<String, CommandSpec, S>,
) -> crate::config::Config {
    let mut builder = TestConfigBuilder::new();
    for (name, spec) in commands {
        builder = builder.command(&name, spec);
    }
    builder.build()
}

/// DUP-011: Platform-specific output creation for tests.
///
/// Creates a `std::process::Output` with the given status code and output bytes.
#[allow(dead_code)]
#[must_use]
pub fn make_test_output(status_code: i32, stdout: &[u8], stderr: &[u8]) -> std::process::Output {
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt;

    #[cfg(unix)]
    {
        std::process::Output {
            status: std::process::ExitStatus::from_raw(status_code << 8),
            stdout: stdout.to_vec(),
            stderr: stderr.to_vec(),
        }
    }
    #[cfg(windows)]
    {
        std::process::Output {
            status: std::process::ExitStatus::from_raw(status_code),
            stdout: stdout.to_vec(),
            stderr: stderr.to_vec(),
        }
    }
}

/// RAII guard for environment variable manipulation in tests.
/// Restores original value (or removes if not set) on drop.
///
/// # Rust 2024 Compatibility (E104)
///
/// `std::env::set_var` and `std::env::remove_var` are `unsafe` in Rust 2024.
/// All calls are wrapped in `unsafe` blocks with SAFETY comments.
/// Thread-safety is ensured by requiring callers to use `#[serial]`.
///
/// # Thread Safety (TQ-010)
///
/// Environment variables are process-global state. `EnvGuard` does NOT provide
/// automatic synchronization between tests. Users must ensure:
///
/// 1. Tests using `EnvGuard` for the same key are marked with `#[serial]`
/// 2. Or use different keys per test to avoid conflicts
///
/// The `serial_test` crate is already a dev-dependency for this purpose.
/// Example:
///
/// ```ignore
/// #[test]
/// #[serial]
/// fn test_with_env() {
///     let _guard = EnvGuard::set("MY_VAR", "test_value");
///     // test code
/// }
/// ```
///
/// # DRY-1 / TASK-2059
///
/// This is the workspace's only env-var guard. `ops_cli::test_utils`,
/// `ops_hook_common::test_helpers` and `ops_deps::test_support` each used to
/// carry a copy — two of them spelled `EnvVarGuard`, with different key and
/// value types — and all three now re-export this one. Keys and values are
/// `OsStr`-valued so the `OsString`-keyed call sites keep working, and both
/// spellings of the removal constructor ([`EnvGuard::remove`] and its alias
/// [`EnvGuard::unset`]) survive so no call site lost a capability.
#[allow(dead_code)]
pub struct EnvGuard {
    key: std::ffi::OsString,
    original: Option<std::ffi::OsString>,
}

// TRAIT-1: manual Debug impl redacts the captured original value. Env
// vars frequently hold credentials (DATABASE_URL, AWS_SECRET_ACCESS_KEY,
// API tokens); leaking them via a `{:?}` print in a downstream test
// fixture would defeat the point of capturing them privately.
impl std::fmt::Debug for EnvGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EnvGuard")
            .field("key", &self.key)
            .field("original", &self.original.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

#[allow(dead_code, unused_unsafe)]
impl EnvGuard {
    /// Set an environment variable, returning a guard that restores it on drop.
    ///
    /// # Safety (E104)
    ///
    /// Uses `unsafe` for `set_var` which is unsafe in Rust 2024 edition.
    /// This is test-only code guarded by `#[cfg(test)]` consumers and
    /// thread-safety is ensured via `#[serial]` test attributes.
    pub fn set(key: impl AsRef<std::ffi::OsStr>, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let key = key.as_ref().to_os_string();
        let original = std::env::var_os(&key);
        // SAFETY: Test-only. Callers use #[serial] to prevent concurrent env access.
        unsafe { std::env::set_var(&key, value.as_ref()) };
        Self { key, original }
    }

    /// Remove an environment variable, returning a guard that restores it on drop.
    ///
    /// # Safety (E104)
    ///
    /// Uses `unsafe` for `remove_var` which is unsafe in Rust 2024 edition.
    /// This is test-only code guarded by `#[cfg(test)]` consumers and
    /// thread-safety is ensured via `#[serial]` test attributes.
    pub fn remove(key: impl AsRef<std::ffi::OsStr>) -> Self {
        let key = key.as_ref().to_os_string();
        let original = std::env::var_os(&key);
        // SAFETY: Test-only. Callers use #[serial] to prevent concurrent env access.
        unsafe { std::env::remove_var(&key) };
        Self { key, original }
    }

    /// [`Self::remove`] under the name the `EnvVarGuard` copies used, so
    /// `unset`-spelled call sites keep reading the same way after DRY-1 /
    /// TASK-2059 collapsed the four guards into this one.
    pub fn unset(key: impl AsRef<std::ffi::OsStr>) -> Self {
        Self::remove(key)
    }

    /// Change the value again without losing the original snapshot, for tests
    /// that sweep a variable through several values in one loop.
    pub fn set_value(&self, value: impl AsRef<std::ffi::OsStr>) {
        // SAFETY: Test-only. Callers use #[serial] to prevent concurrent env access.
        unsafe { std::env::set_var(&self.key, value.as_ref()) };
    }

    /// Clear the value without losing the original snapshot.
    pub fn unset_value(&self) {
        // SAFETY: Test-only. Callers use #[serial] to prevent concurrent env access.
        unsafe { std::env::remove_var(&self.key) };
    }
}

/// Isolate the global config layer for one test.
///
/// Points `XDG_CONFIG_HOME` at an empty directory under `dir` and clears the
/// global-config resolver cache, so a config test observes only the layers it
/// wrote itself and never the developer's real `~/.config/ops/config.toml`.
///
/// Returns the guard; the caller must keep it alive for the whole test, and
/// the test must be `#[serial]` because this mutates the process environment.
#[allow(dead_code)]
#[must_use]
pub fn isolate_global_config(dir: &std::path::Path) -> GlobalConfigIsolation {
    crate::config::reset_global_config_path_cache(crate::config::GlobalConfigPathResetToken::new());
    let guard = EnvGuard::set(
        "XDG_CONFIG_HOME",
        dir.join("xdg-empty").display().to_string(),
    );
    crate::config::reset_global_config_path_cache(crate::config::GlobalConfigPathResetToken::new());
    GlobalConfigIsolation { _env: guard }
}

/// Guard returned by [`isolate_global_config`].
///
/// Restoring `XDG_CONFIG_HOME` is not enough on its own: the resolved path is
/// memoised in `GLOBAL_CONFIG_PATH`, so without this `Drop` the cache would
/// still hold the (now-deleted) tempdir path when the next test resolves the
/// global config.
#[derive(Debug)]
pub struct GlobalConfigIsolation {
    _env: EnvGuard,
}

impl Drop for GlobalConfigIsolation {
    fn drop(&mut self) {
        // This body runs before `_env`'s own `Drop` restores
        // `XDG_CONFIG_HOME`, which is harmless: the cache is left *empty*,
        // not repopulated, so the next resolution reads whatever the
        // environment holds by then. These tests are `#[serial]`, so nothing
        // resolves in the window between.
        crate::config::reset_global_config_path_cache(
            crate::config::GlobalConfigPathResetToken::new(),
        );
    }
}

/// TEST-19 (TASK-1033): true when the current effective UID is 0 on Unix.
///
/// Tests that rely on DAC permission denial (`chmod 0o000` + assert read
/// fails) silently invert their assertion when run as root because the
/// kernel skips the permission check for UID 0. Container CI (Docker
/// default UID 0, rootful devcontainers, privileged self-hosted runners)
/// hits this routinely. Callers should `if is_root_euid() { return; }` at
/// the top of the test and explain inline why the guard is mandatory.
///
/// On non-Unix targets this always returns `false`; callers should also
/// be `#[cfg(unix)]`-gated since the underlying chmod assertion is too.
#[allow(dead_code)]
#[cfg(unix)]
#[must_use]
pub fn is_root_euid() -> bool {
    // Avoid pulling in a libc dep just for one syscall: declare the FFI
    // signature locally. `geteuid` is async-signal-safe and infallible per
    // POSIX, so no errno handling is required.
    unsafe extern "C" {
        fn geteuid() -> u32;
    }
    // SAFETY: `geteuid` takes no arguments and cannot fail per POSIX.
    unsafe { geteuid() == 0 }
}

#[allow(dead_code)]
#[cfg(not(unix))]
pub fn is_root_euid() -> bool {
    false
}

/// Process-wide mutex for tests that change the current working directory.
///
/// Rust tests run in parallel by default and `std::env::set_current_dir` is
/// process-global, so CWD-dependent tests must serialize on this lock.
///
/// # Mutex poisoning recovery
///
/// If a test panics while holding this lock the mutex becomes poisoned. We
/// deliberately recover rather than propagate: the panic has already been
/// reported by the test framework, later tests should still run, and a failed
/// CWD restoration is non-critical (test isolation is best-effort).
#[cfg(any(test, feature = "test-support"))]
pub static CWD_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// DRY-1 / TASK-2034: the workspace's one working-directory guard — acquires
/// [`CWD_MUTEX`], switches to a target directory, and restores the original
/// CWD on drop, **including on the unwind path**.
///
/// The lock is part of the contract, not an optimisation. The directory a
/// test chdirs into is usually a `tempfile::TempDir` deleted on drop, so a
/// racing or skipped restore leaves the whole test binary running in a
/// deleted directory and every later relative-path test fails for unrelated
/// reasons. Guards that relied on the caller remembering
/// `#[serial_test::serial]` gave a non-serial caller a silent race; holding
/// the mutex means a caller cannot get that wrong.
///
/// The guard is not reentrant: a single test must hold at most one at a time.
///
/// # Test isolation note
///
/// Serialising means these tests cannot run in parallel with each other.
/// Prefer `tempfile::tempdir()` plus an explicit path parameter where the
/// code under test accepts one, and keep this for the production entry
/// points that read `std::env::current_dir()` themselves.
///
/// # Rust 2024 compatibility (E104)
///
/// `std::env::set_current_dir` becomes `unsafe` in the 2024 edition; the
/// calls are already wrapped with SAFETY comments so the bump is a no-op.
#[cfg(any(test, feature = "test-support"))]
pub struct CwdGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    original_dir: std::path::PathBuf,
}

#[cfg(any(test, feature = "test-support"))]
impl CwdGuard {
    /// Acquire [`CWD_MUTEX`], capture the current directory, then switch to
    /// `target`.
    ///
    /// # Errors
    ///
    /// If the current directory cannot be read or `target` cannot be entered.
    pub fn new(target: &std::path::Path) -> std::io::Result<Self> {
        let lock = CWD_MUTEX.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("CWD_MUTEX poisoned by previous test panic, recovering");
            poisoned.into_inner()
        });
        let original_dir = std::env::current_dir()?;
        // SAFETY: Test-only. CWD_MUTEX serializes all CWD-dependent tests.
        // `unsafe` is required in the 2024 edition; allow unused_unsafe for 2021.
        #[allow(unused_unsafe)]
        unsafe {
            std::env::set_current_dir(target)?;
        };
        Ok(Self {
            _lock: lock,
            original_dir,
        })
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Drop for CwdGuard {
    #[allow(unused_unsafe)]
    fn drop(&mut self) {
        // SAFETY: Test-only. CWD_MUTEX serializes all CWD-dependent tests.
        if let Err(e) = unsafe { std::env::set_current_dir(&self.original_dir) } {
            tracing::warn!(
                path = %self.original_dir.display(),
                error = %e,
                "CwdGuard: failed to restore the original working directory"
            );
        }
    }
}

/// DUP-3 / TASK-2014, TASK-2025: the workspace's one tracing-capture harness.
///
/// Every crate that asserts on `tracing` output used to grow its own copy of
/// the buffer / `MakeWriter` shim *and* of the global-dispatcher pin below,
/// whose absence is a silent flake rather than a failure. The harness lives
/// here — the crate every other one already depends on — and is re-exported
/// by `ops_about::test_support` for the extension family.
#[cfg(any(test, feature = "test-support"))]
mod tracing_capture {
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    /// Shared capture buffer: a `Clone`able `Write` + `MakeWriter` sink that
    /// accumulates a subscriber's rendered output in memory.
    ///
    /// Construct via [`TracingBuf::default`], hand a clone to
    /// `tracing_subscriber::fmt::Subscriber::with_writer`, and read the
    /// captured bytes via [`TracingBuf::captured`].
    #[derive(Clone, Default)]
    pub struct TracingBuf(Arc<Mutex<Vec<u8>>>);

    impl TracingBuf {
        /// Snapshot of the captured tracing output as a UTF-8 string. Tests
        /// typically assert on substrings, so we tolerate a flush that
        /// splits a multi-byte char by going through `from_utf8_lossy`.
        #[must_use]
        pub fn captured(&self) -> String {
            let guard = self
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            String::from_utf8_lossy(&guard).into_owned()
        }
    }

    impl Write for TracingBuf {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .extend_from_slice(b);
            Ok(b.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for TracingBuf {
        type Writer = Self;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    /// Minimal `tracing::Subscriber` that counts `WARN`-level events, so a
    /// test can assert on warn counts without pulling `tracing-subscriber`
    /// layer machinery into the assertion.
    #[derive(Clone, Default)]
    pub struct WarnCounter(Arc<AtomicUsize>);

    impl WarnCounter {
        /// Number of `WARN` events observed so far.
        #[must_use]
        pub fn count(&self) -> usize {
            self.0.load(Ordering::SeqCst)
        }
    }

    impl tracing::Subscriber for WarnCounter {
        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            if *event.metadata().level() == tracing::Level::WARN {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        fn enter(&self, _span: &tracing::span::Id) {}
        fn exit(&self, _span: &tracing::span::Id) {}
    }

    /// Keep one globally-registered `tracing` dispatcher alive for the whole
    /// test binary so scoped capture subscribers are never the *only* ones
    /// registered.
    ///
    /// `tracing` caches each callsite's `Interest` process-wide, computed from
    /// the dispatchers registered the moment that callsite is first hit. With
    /// only scoped (`with_default`) subscribers, a test thread can first-hit a
    /// callsite while no dispatcher is registered at all: the callsite then
    /// caches `Interest::never()` and every later event from it is dropped
    /// before reaching *any* subscriber — including the capture a parallel
    /// test is asserting on, which comes back empty at random, under
    /// `cargo test`'s shared-process threads as well as under nextest. A
    /// global dispatcher is never unregistered, so the interest cache can no
    /// longer answer "never".
    ///
    /// The global itself discards everything; captures still come from the
    /// scoped subscriber installed per call.
    ///
    /// Every entry point below pins first, so the hazard is unreachable
    /// through them. It is `pub` only for the rare test that cannot use them
    /// — one installing a *per-thread* subscriber over a shared
    /// [`TracingBuf`], say — which must call this itself before spawning.
    pub fn pin_global_dispatcher() {
        static INSTALL: std::sync::Once = std::sync::Once::new();
        INSTALL.call_once(|| {
            // Another test binary layout may already have set one; either way
            // an interested global is registered from here on.
            let _ = tracing::subscriber::set_global_default(WarnCounter::default());
            // Callsites hit before this point resolved against an empty
            // dispatcher list; recompute them now that one is registered.
            tracing::callsite::rebuild_interest_cache();
        });
    }

    /// Capture the rendered tracing records at or above `level` that `f`
    /// emits on the calling thread, alongside `f`'s return value.
    ///
    /// ANSI colouring is disabled so the capture contains no escape bytes of
    /// the subscriber's own making — assertions about escapes in the *record*
    /// stay meaningful. The subscriber configuration is decided here rather
    /// than per call site so "captured output" means the same thing
    /// everywhere.
    ///
    /// **Scope:** the subscriber is the *thread-local* default, so only
    /// records `f` emits on the calling thread are captured. If `f` fans out
    /// to worker threads — a parallel walker, `rayon`, a spawned scope —
    /// their records reach the global dispatcher instead and are not seen.
    pub fn capture_tracing<F, R>(level: tracing::Level, f: F) -> (String, R)
    where
        F: FnOnce() -> R,
    {
        let (dispatch, buf) = capture_dispatch(level);
        let value = tracing::dispatcher::with_default(&dispatch, f);
        (buf.captured(), value)
    }

    /// The capture subscriber [`capture_tracing`] installs, as a detached
    /// [`tracing::Dispatch`] plus the buffer it writes into.
    ///
    /// DUP-3 / TASK-2069: for the test whose work does not run on the calling
    /// thread and so cannot use the scoped form — a `tokio::spawn`ed task
    /// attached with `WithSubscriber`, say. Such a test used to hand-build the
    /// `fmt()` subscriber and its own `MakeWriter` shim, which meant a second
    /// definition of what "captured output" is *and* no
    /// [`pin_global_dispatcher`] call, leaving it open to the random-empty
    /// capture that pinning exists to prevent.
    ///
    /// Pins the global dispatcher and fixes the same configuration
    /// [`capture_tracing`] uses, so the two capture identically.
    #[must_use]
    pub fn capture_dispatch(level: tracing::Level) -> (tracing::Dispatch, TracingBuf) {
        pin_global_dispatcher();
        let buf = TracingBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf.clone())
            .with_max_level(level)
            .with_ansi(false)
            .finish();
        (subscriber.into(), buf)
    }

    /// [`capture_tracing`] fixed at `WARN` — the common case.
    pub fn capture_warn<F: FnOnce()>(body: F) -> String {
        capture_tracing(tracing::Level::WARN, body).0
    }

    /// Count the `WARN` events `f` emits on the calling thread.
    ///
    /// The counting counterpart to [`capture_warn`], for tests that only need
    /// "how many warnings did this call emit?" and should not be coupled to
    /// the rendered text. Carries the same thread-local scope caveat as
    /// [`capture_tracing`].
    pub fn count_warnings<T>(f: impl FnOnce() -> T) -> (T, usize) {
        pin_global_dispatcher();
        let counter = WarnCounter::default();
        let out = tracing::subscriber::with_default(counter.clone(), f);
        (out, counter.count())
    }
}

#[cfg(any(test, feature = "test-support"))]
pub use tracing_capture::{
    capture_dispatch, capture_tracing, capture_warn, count_warnings, pin_global_dispatcher,
    TracingBuf, WarnCounter,
};

impl Drop for EnvGuard {
    #[allow(unused_unsafe)]
    fn drop(&mut self) {
        // SAFETY: Test-only. Callers use #[serial] to prevent concurrent env access.
        unsafe {
            match &self.original {
                Some(val) => std::env::set_var(&self.key, val),
                None => std::env::remove_var(&self.key),
            }
        }
    }
}

#[cfg(test)]
mod builder_parity_tests {
    //! DUP-2 regression: [`TestConfigBuilder`] and [`ConfigOverlayBuilder`]
    //! grew drift over time (each acquired methods the other lacked). These
    //! tests fail to compile — not merely assert-fail — if the set of shared
    //! fluent methods diverges. Adding a new builder method on one side
    //! without mirroring it on the other will break `cargo test`.
    //!
    //! The list below is intentionally opinionated: it covers the methods
    //! that *must* exist on both. If a method is genuinely one-sided (e.g.
    //! `stderr_tail_lines` on `TestConfigBuilder` only because overlays have
    //! no equivalent), leave it out of this mirror test.
    use super::*;

    #[test]
    fn both_builders_expose_theme_method() {
        let _ = TestConfigBuilder::new().theme("classic").build();
        let _ = ConfigOverlayBuilder::new().theme("classic").build();
    }

    #[test]
    fn both_builders_expose_columns_method() {
        let _ = TestConfigBuilder::new().columns(80).build();
        let _ = ConfigOverlayBuilder::new().columns(80).build();
    }

    #[test]
    fn both_builders_expose_show_error_detail_method() {
        let _ = TestConfigBuilder::new().show_error_detail(true).build();
        let _ = ConfigOverlayBuilder::new().show_error_detail(true).build();
    }

    #[test]
    fn both_builders_expose_exec_method() {
        let _ = TestConfigBuilder::new().exec("c", "echo", &["x"]).build();
        let _ = ConfigOverlayBuilder::new()
            .exec("c", "echo", &["x"])
            .build();
    }

    #[test]
    fn both_builders_expose_composite_method() {
        let _ = TestConfigBuilder::new().composite("c", &["a", "b"]).build();
        let _ = ConfigOverlayBuilder::new()
            .composite("c", &["a", "b"])
            .build();
    }
}

#[cfg(test)]
pub mod proptest_strategies {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        pub fn arb_exec_spec()(
            program in "[a-zA-Z_][a-zA-Z0-9_-]{0,15}",
            args in prop::collection::vec("[a-zA-Z0-9_./-]{1,10}", 0..5)
        ) -> ExecCommandSpec {
            ExecCommandSpec {
                program,
                args,
                ..Default::default()
            }
        }
    }
}
