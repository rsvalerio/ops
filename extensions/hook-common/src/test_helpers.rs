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
/// tests: `std::env::set_var`/`remove_var` mutate process-wide state and race
/// with concurrent `getenv` calls.
///
/// Re-exported from `ops_core::test_utils` rather than redefined, so the
/// workspace carries a single env guard. It offers both the `remove` and the
/// `unset` spelling, and redacts the captured original value in its `Debug`
/// output.
pub use ops_core::test_utils::EnvGuard;

/// The working-directory guard for hook tests: the workspace's single
/// [`ops_core::test_utils::CwdGuard`], re-exported under
/// `ops_hook_common::test_helpers`.
///
/// Unlike `EnvGuard` above, which relies on each call site carrying
/// `#[serial_test::serial]`, this guard serialises on a process-wide mutex
/// itself, so a test cannot get a cwd race by forgetting the attribute. It
/// lets hook crates exercise the *production* entry points that read
/// `std::env::current_dir()` rather than only their `dir`-parameterised inner
/// helpers.
pub use ops_core::test_utils::CwdGuard;
