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
//! - **`#[cfg(test)]` helpers** (currently only [`capture_tracing`]) are
//!   compile-gated to `cargo test` of this crate and are not visible to
//!   downstream `test-support` consumers; mark anything new the same way
//!   when it depends on dev-only deps (e.g. `tracing-subscriber`).
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
//! - [`is_root_euid`] — true on Unix when EUID is 0; tests that depend on
//!   DAC-permission denial must `return` early when this is true (see
//!   TEST-19 in the function rustdoc).
//!
//! ## Internal helpers (not part of the surface contract)
//!
//! - `capture_tracing` (test-only) — used by in-crate tests to drive a
//!   thread-local `tracing-subscriber` and capture its output. Not exposed
//!   under the `test-support` feature because `tracing-subscriber` is a
//!   dev-dependency here.
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

/// Create an ExecCommandSpec with the given program and args.
pub fn exec_spec(program: &str, args: &[&str]) -> ExecCommandSpec {
    exec_spec_with_cwd(program, args, None)
}

/// Create an ExecCommandSpec with an optional cwd.
pub fn exec_spec_with_cwd(
    program: &str,
    args: &[&str],
    cwd: Option<std::path::PathBuf>,
) -> ExecCommandSpec {
    ExecCommandSpec {
        program: program.to_string(),
        args: args.iter().map(|s| s.to_string()).collect(),
        cwd,
        ..Default::default()
    }
}

/// Create an ExecCommandSpec that works on both Unix and Windows.
pub fn platform_exec_spec(unix: (&str, &[&str]), windows: (&str, &[&str])) -> ExecCommandSpec {
    if cfg!(windows) {
        exec_spec(windows.0, windows.1)
    } else {
        exec_spec(unix.0, unix.1)
    }
}

/// Create an ExecCommandSpec that echoes a message.
pub fn echo_cmd(msg: &str) -> ExecCommandSpec {
    platform_exec_spec(("echo", &[msg]), ("cmd", &["/C", "echo", msg]))
}

/// Create an ExecCommandSpec that exits with success (true).
pub fn true_cmd() -> ExecCommandSpec {
    platform_exec_spec(("true", &[]), ("cmd", &["/C", "exit", "0"]))
}

/// Create an ExecCommandSpec that exits with failure (false).
pub fn false_cmd() -> ExecCommandSpec {
    platform_exec_spec(("false", &[]), ("cmd", &["/C", "exit", "1"]))
}

/// Create an ExecCommandSpec that sleeps for the given number of seconds.
pub fn sleep_cmd(secs: u64) -> ExecCommandSpec {
    let secs_str = secs.to_string();
    if cfg!(windows) {
        exec_spec("ping", &["-n", &format!("{}", secs + 1), "127.0.0.1"])
    } else {
        exec_spec("sleep", &[&secs_str])
    }
}

/// Create a composite command spec from a list of command names.
#[allow(dead_code)]
pub fn composite_cmd(commands: &[&str]) -> CompositeCommandSpec {
    CompositeCommandSpec {
        commands: commands.iter().map(|s| s.to_string()).collect(),
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
        commands: commands.iter().map(|s| s.to_string()).collect(),
        parallel: true,
        fail_fast: true,
        help: None,
        aliases: Vec::new(),
        category: None,
    }
}

/// Builder for creating test configs.
///
/// # DUP-002: Shared Pattern with ConfigOverlayBuilder
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
    pub fn new() -> Self {
        Self {
            output: OutputConfig::default(),
            commands: IndexMap::new(),
        }
    }

    pub fn exec(mut self, name: &str, program: &str, args: &[&str]) -> Self {
        self.commands.insert(
            name.to_string(),
            CommandSpec::Exec(exec_spec(program, args)),
        );
        self
    }

    /// DUP-002: Create a CommandSpec::Exec variant directly.
    ///
    /// This is useful when you need the CommandSpec variant for tests that
    /// require the wrapped type rather than the inner ExecCommandSpec.
    pub fn raw_exec(_name: &str, program: &str, args: &[&str]) -> CommandSpec {
        CommandSpec::Exec(exec_spec(program, args))
    }

    pub fn command(mut self, name: &str, spec: CommandSpec) -> Self {
        self.commands.insert(name.to_string(), spec);
        self
    }

    pub fn composite(mut self, name: &str, commands: &[&str]) -> Self {
        self.commands.insert(
            name.to_string(),
            CommandSpec::Composite(composite_cmd(commands)),
        );
        self
    }

    pub fn parallel_composite(mut self, name: &str, commands: &[&str]) -> Self {
        self.commands.insert(
            name.to_string(),
            CommandSpec::Composite(parallel_cmd(commands)),
        );
        self
    }

    pub fn theme(mut self, theme: &str) -> Self {
        self.output.theme = theme.to_string();
        self
    }

    pub fn columns(mut self, columns: u16) -> Self {
        self.output.columns = columns;
        self
    }

    pub fn show_error_detail(mut self, show: bool) -> Self {
        self.output.show_error_detail = show;
        self
    }

    pub fn stderr_tail_lines(mut self, n: usize) -> Self {
        self.output.stderr_tail_lines = n;
        self
    }

    pub fn build(self) -> crate::config::Config {
        crate::config::Config {
            output: self.output,
            commands: self.commands,
            data: crate::config::DataConfig::default(),
            themes: IndexMap::new(),
            extensions: crate::config::ExtensionConfig::default(),
            about: crate::config::AboutConfig::default(),
            stack: None,
            tools: IndexMap::new(),
        }
    }
}

impl Default for TestConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// DUP-001: Builder for creating ConfigOverlay in tests.
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
    pub fn new() -> Self {
        Self {
            output: None,
            commands: None,
            themes: None,
            extensions: None,
        }
    }

    pub fn output(mut self, output: OutputConfigOverlay) -> Self {
        self.output = Some(output);
        self
    }

    pub fn theme(self, theme: impl Into<String>) -> Self {
        self.output(OutputConfigOverlay {
            theme: Some(theme.into()),
            ..Default::default()
        })
    }

    pub fn columns(self, columns: u16) -> Self {
        self.output(OutputConfigOverlay {
            columns: Some(columns),
            ..Default::default()
        })
    }

    pub fn show_error_detail(self, show: bool) -> Self {
        self.output(OutputConfigOverlay {
            show_error_detail: Some(show),
            ..Default::default()
        })
    }

    pub fn commands(mut self, commands: IndexMap<String, CommandSpec>) -> Self {
        self.commands = Some(commands);
        self
    }

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

    pub fn themes(mut self, themes: IndexMap<String, ThemeConfig>) -> Self {
        self.themes = Some(themes);
        self
    }

    pub fn custom_theme(self, name: &str, theme: ThemeConfig) -> Self {
        let mut themes = self.themes.unwrap_or_default();
        themes.insert(name.to_string(), theme);
        Self {
            themes: Some(themes),
            ..self
        }
    }

    pub fn extensions(mut self, extensions: ExtensionConfigOverlay) -> Self {
        self.extensions = Some(extensions);
        self
    }

    pub fn enabled_extensions(self, enabled: Vec<String>) -> Self {
        self.extensions(ExtensionConfigOverlay {
            enabled: Some(enabled),
        })
    }

    pub fn build(self) -> ConfigOverlay {
        ConfigOverlay {
            output: self.output,
            commands: self.commands,
            data: None,
            themes: self.themes,
            extensions: self.extensions,
            about: None,
            stack: None,
            tools: None,
        }
    }
}

impl Default for ConfigOverlayBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a simple test config with the given commands.
pub fn test_config_with_commands(commands: HashMap<String, CommandSpec>) -> crate::config::Config {
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
#[allow(dead_code)]
pub struct EnvGuard {
    key: String,
    original: Option<String>,
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
    pub fn set(key: impl Into<String>, value: impl AsRef<str>) -> Self {
        let key = key.into();
        let original = std::env::var(&key).ok();
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
    pub fn remove(key: impl Into<String>) -> Self {
        let key = key.into();
        let original = std::env::var(&key).ok();
        // SAFETY: Test-only. Callers use #[serial] to prevent concurrent env access.
        unsafe { std::env::remove_var(&key) };
        Self { key, original }
    }
}

/// TEST-19 (TASK-1033): true when the current effective UID is 0 on Unix.
/// Tests that rely on DAC permission denial (`chmod 0o000` + assert read
/// fails) silently invert their assertion when run as root because the
/// kernel skips the permission check for UID 0. Container CI (Docker
/// default UID 0, rootful devcontainers, privileged self-hosted runners)
/// hits this routinely. Callers should `if is_root_euid() { return; }`
/// at the top of the test and explain inline why the guard is mandatory.
///
/// On non-Unix targets this always returns `false`; callers should also
/// be `#[cfg(unix)]`-gated since the underlying chmod assertion is too.
#[allow(dead_code)]
#[cfg(unix)]
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

/// Shared tracing-event capture helper for tests across the core crate.
/// Installs a thread-local subscriber at `level` for the duration of `f`,
/// captures the formatted output (ANSI off) and returns it alongside `f`'s
/// return value. Consolidates DUP-3: every in-process tracing-capture test
/// in core was open-coding the same `BufWriter` + `MakeWriter` scaffold.
#[cfg(test)]
pub fn capture_tracing<F, R>(level: tracing::Level, f: F) -> (String, R)
where
    F: FnOnce() -> R,
{
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct BufWriter(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for BufWriter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().expect("lock").extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl<'a> MakeWriter<'a> for BufWriter {
        type Writer = BufWriter;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    let buf = BufWriter::default();
    let captured = buf.0.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(buf)
        .with_max_level(level)
        .with_ansi(false)
        .finish();
    let value = tracing::subscriber::with_default(subscriber, f);
    let bytes = captured.lock().expect("lock").clone();
    let text = String::from_utf8(bytes).expect("utf8");
    (text, value)
}

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
