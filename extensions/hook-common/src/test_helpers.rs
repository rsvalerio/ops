//! Test-only helpers shared by hook crates.
//!
//! Gated behind the `test-helpers` cargo feature so production builds of
//! `ops-hook-common` do not pull this code in. The wrapper crates
//! (`ops-run-before-commit`, `ops-run-before-push`) opt in via
//! `dev-dependencies` so their `#[cfg(test)]` modules can reuse the same
//! guards and avoid drift between near-identical copies.

/// RAII guard that restores an env var to its previous value on drop.
///
/// Pair with `#[serial_test::serial]` to prevent races with other env-mutating
/// tests: `std::env::set_var`/`remove_var` mutate process-wide state and will
/// become `unsafe` under the 2024 edition because they race with concurrent
/// `getenv` calls. The guard centralises the pattern so the eventual edition
/// bump is a single-file change.
pub struct EnvGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvGuard {
    /// Capture the current value of `key`, then remove it. The original value
    /// is restored when the returned guard is dropped.
    #[must_use]
    pub fn remove(key: &'static str) -> Self {
        let original = std::env::var(key).ok();
        std::env::remove_var(key);
        Self { key, original }
    }

    /// Capture the current value of `key`, then set it to `value`. The
    /// original value (or absence) is restored when the guard is dropped.
    #[must_use]
    pub fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var(key).ok();
        std::env::set_var(key, value);
        Self { key, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}
