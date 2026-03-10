//! CLI-specific test utilities.
//!
//! Re-exports shared helpers from cargo-ops-core and cargo-ops-runner,
//! and adds CLI-specific ones (CwdGuard-based helpers, etc.).

// Re-export shared test helpers from core
pub use cargo_ops_core::test_utils::*;

// Re-export runner test support (EventAssertions, test_runner)
#[cfg(test)]
#[allow(unused_imports)]
pub use cargo_ops_runner::test_support::{test_runner, EventAssertions};

/// Create a test Context with default config and given path.
#[cfg(test)]
#[allow(dead_code)]
pub fn test_context(path: std::path::PathBuf) -> cargo_ops_extension::Context {
    use std::sync::Arc;
    cargo_ops_extension::Context::new(Arc::new(cargo_ops_core::config::Config::default()), path)
}

/// DUP-006: Register an extension and return both registries.
///
/// This helper reduces boilerplate in tests that need to set up extensions.
#[cfg(test)]
#[allow(dead_code)]
pub fn register_extension(
    ext: &dyn cargo_ops_extension::Extension,
) -> (
    cargo_ops_extension::CommandRegistry,
    cargo_ops_extension::DataRegistry,
) {
    let mut cmd_registry = cargo_ops_extension::CommandRegistry::new();
    let mut data_registry = cargo_ops_extension::DataRegistry::new();
    ext.register_commands(&mut cmd_registry);
    ext.register_data_providers(&mut data_registry);
    (cmd_registry, data_registry)
}

/// DUP-013: Helper to create a temp directory with .ops.toml content.
///
/// Returns the temp directory (for cleanup) and the CwdGuard.
#[cfg(test)]
#[allow(dead_code)]
pub fn with_temp_config(content: &str) -> (tempfile::TempDir, crate::CwdGuard) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join(".ops.toml"), content).expect("write .ops.toml");
    let guard = crate::CwdGuard::new(dir.path()).expect("CwdGuard");
    (dir, guard)
}
