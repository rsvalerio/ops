//! Shared test utilities for ops unit tests.

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
    }
}

/// Create a parallel composite command spec from a list of command names.
#[allow(dead_code)]
pub fn parallel_cmd(commands: &[&str]) -> CompositeCommandSpec {
    CompositeCommandSpec {
        commands: commands.iter().map(|s| s.to_string()).collect(),
        parallel: true,
        fail_fast: true,
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

    pub fn build(self) -> crate::config::Config {
        crate::config::Config {
            output: self.output,
            commands: self.commands,
            data: crate::config::DataConfig::default(),
            themes: IndexMap::new(),
            extensions: crate::config::ExtensionConfig::default(),
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
