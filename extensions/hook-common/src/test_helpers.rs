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
/// DRY-1 / TASK-2059: re-exported from `ops_core::test_utils` rather than
/// redefined, so the workspace carries one env guard instead of four. The
/// shared guard keeps both the `remove` spelling this copy used and the
/// `unset` spelling the `EnvVarGuard` copies used, and it redacts the captured
/// original value in its `Debug` output.
pub use ops_core::test_utils::EnvGuard;

/// DRY-1 / TASK-2034: the working-directory guard hook tests use is the
/// workspace's single [`ops_core::test_utils::CwdGuard`], re-exported here so
/// existing `ops_hook_common::test_helpers::CwdGuard` imports keep working.
///
/// This crate's own copy relied on each call site remembering
/// `#[serial_test::serial]` — the convention `EnvGuard` above still follows —
/// while the cli copy serialised on a process-wide mutex. Two guards with the
/// same name and different safety contracts meant a new hook test could reach
/// for the weaker one and get a silent cwd race. The shared guard takes the
/// mutex itself, so that is no longer reachable. It still exists so hook
/// crates can exercise the *production* entry points that read
/// `std::env::current_dir()` rather than only their `dir`-parameterised inner
/// helpers (TEST-5 / TASK-1908).
pub use ops_core::test_utils::CwdGuard;
