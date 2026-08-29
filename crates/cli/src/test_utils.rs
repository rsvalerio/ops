//! CLI-specific test utilities.
//!
//! Re-exports shared helpers from ops-core and ops-runner, and adds
//! CLI-specific ones on top.
//!
//! DRY-1 / TASK-2034: `CwdGuard` and `CWD_MUTEX` used to be defined here.
//! They live in `ops_core::test_utils` now — one guard for the workspace —
//! and reach this module through the glob re-export below.

// Re-export shared test helpers from core
pub use ops_core::test_utils::*;

// Re-export runner test support (EventAssertions, test_runner)
#[cfg(test)]
#[allow(unused_imports)]
pub use ops_runner::test_support::{test_runner, EventAssertions};

/// Create a test Context with default config and given path.
#[cfg(test)]
#[allow(dead_code)]
pub fn test_context(path: std::path::PathBuf) -> ops_extension::Context {
    use std::sync::Arc;
    ops_extension::Context::new(Arc::new(ops_core::config::Config::empty()), path)
}

/// Register an extension and return both registries.
///
/// This helper reduces boilerplate in tests that need to set up extensions.
#[cfg(test)]
#[allow(dead_code)]
pub fn register_extension(
    ext: &dyn ops_extension::Extension,
) -> (ops_extension::CommandRegistry, ops_extension::DataRegistry) {
    let mut cmd_registry = ops_extension::CommandRegistry::new();
    let mut data_registry = ops_extension::DataRegistry::new();
    ext.register_commands(&mut cmd_registry);
    ext.register_data_providers(&mut data_registry);
    (cmd_registry, data_registry)
}

/// Helper to create a temp directory with .ops.toml content.
///
/// Returns the temp directory (for cleanup) and the `CwdGuard`.
#[cfg(test)]
#[allow(dead_code)]
pub fn with_temp_config(content: &str) -> (tempfile::TempDir, CwdGuard) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join(".ops.toml"), content).expect("write .ops.toml");
    let guard = CwdGuard::new(dir.path()).expect("CwdGuard");
    (dir, guard)
}

/// Shared tracing-event capture helper for tests across the cli crate.
///
/// DUP-3 / TASK-2014: delegates to `ops_core::test_utils`, which owns the
/// workspace's single copy of the buffer shim and of the global-dispatcher
/// pin. `tracing` caches each callsite's `Interest` process-wide, so with
/// only scoped subscribers a parallel test thread can cache
/// `Interest::never()` and a capture comes back empty at random; the pin
/// lives inside the shared helper, so no call site here can forget it.
#[cfg(test)]
#[allow(dead_code)]
pub fn capture_tracing<F: FnOnce()>(level: tracing::Level, f: F) -> String {
    ops_core::test_utils::capture_tracing(level, f).0
}

#[cfg(test)]
#[allow(dead_code)]
pub fn capture_warnings<F: FnOnce()>(f: F) -> String {
    capture_tracing(tracing::Level::WARN, f)
}

#[cfg(test)]
#[allow(dead_code)]
pub fn capture_debug<F: FnOnce()>(f: F) -> String {
    capture_tracing(tracing::Level::DEBUG, f)
}

/// RAII guard for a process-global environment variable: snapshot the
/// original value on construction and restore it on drop, **including when
/// the test panics between the two**.
///
/// TEST-23 (TASK-1752): a bare `remove_var` after a fallible assertion never
/// runs when that assertion fails, so the variable leaks into every later
/// test in the same binary. `serial_test::serial` serialises execution; it
/// does not restore process state. Every test in this crate that touches the
/// environment goes through this guard.
pub struct EnvVarGuard {
    name: &'static str,
    original: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    /// Remove `name` for the scope of the guard.
    pub fn unset(name: &'static str) -> Self {
        let guard = Self {
            name,
            original: std::env::var_os(name),
        };
        std::env::remove_var(name);
        guard
    }

    /// Set `name` to `value` for the scope of the guard.
    pub fn set(name: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let guard = Self {
            name,
            original: std::env::var_os(name),
        };
        std::env::set_var(name, value);
        guard
    }

    /// Change the value again without losing the original snapshot, for
    /// tests that sweep a variable through several values in one loop.
    pub fn set_value(&self, value: impl AsRef<std::ffi::OsStr>) {
        std::env::set_var(self.name, value);
    }

    /// Clear the value without losing the original snapshot.
    pub fn unset_value(&self) {
        std::env::remove_var(self.name);
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match self.original.take() {
            Some(v) => std::env::set_var(self.name, v),
            None => std::env::remove_var(self.name),
        }
    }
}

#[cfg(test)]
mod cwd_guard_tests {
    use super::*;

    #[test]
    fn cwd_guard_changes_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");
        let current = std::env::current_dir().expect("current cwd");
        let current_canonical = current.canonicalize().unwrap_or(current);
        let dir_canonical = dir
            .path()
            .canonicalize()
            .unwrap_or_else(|_| dir.path().to_path_buf());
        assert_eq!(
            current_canonical, dir_canonical,
            "should change to target directory"
        );
    }

    #[test]
    fn cwd_guard_mutex_is_recoverable() {
        let _lock = CWD_MUTEX
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
    }
}
