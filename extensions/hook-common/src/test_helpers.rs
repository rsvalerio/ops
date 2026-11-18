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

/// Env guards that make every `git` child the process spawns hermetic, for
/// as long as they are held.
///
/// Points `GIT_CONFIG_GLOBAL` / `GIT_CONFIG_SYSTEM` at `/dev/null`, so a
/// developer's `commit.gpgsign`, `core.hooksPath` or `init.templateDir`
/// cannot reach a fixture repository, and removes `GIT_DIR`,
/// `GIT_WORK_TREE` and `GIT_INDEX_FILE`, which git exports to the hooks it
/// runs: a test suite launched from inside a hook would otherwise probe or
/// mutate the outer repository instead of its tempdir.
///
/// This is the process-environment twin of
/// [`ops_core::test_utils::git_fixture`], for code under test that spawns
/// `git` itself and so cannot be handed a scrubbed `Command`. It mutates the
/// process environment, so the caller must carry `#[serial_test::serial]`.
#[must_use = "the environment is restored as soon as the guards drop"]
pub fn hermetic_git_env() -> [EnvGuard; 5] {
    [
        EnvGuard::set("GIT_CONFIG_GLOBAL", "/dev/null"),
        EnvGuard::set("GIT_CONFIG_SYSTEM", "/dev/null"),
        EnvGuard::remove("GIT_DIR"),
        EnvGuard::remove("GIT_WORK_TREE"),
        EnvGuard::remove("GIT_INDEX_FILE"),
    ]
}
