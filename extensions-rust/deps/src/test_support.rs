//! Test-only scaffolding shared across the crate's test modules.
//!
//! TASK-1494 consolidated the `tracing` capture scaffold into a single
//! definition. TASK-1670 moved it here when the flat `tests.rs` was split:
//! its call sites now sit in two different modules (`parse/upgrade` and
//! `parse/deny`), and giving each one its own copy would silently revert
//! that consolidation.

/// TEST-23 / TASK-1842: RAII guard restoring a process-global environment
/// variable on drop, including on the unwind path.
///
/// `#[serial]` orders tests; it does not clean up after a panicking one. A
/// hand-written restore placed after the call under test is skipped by
/// exactly the assertion failure the test exists to produce, and the leak
/// then travels: a leaked `CARGO` redirects every later cargo spawn in the
/// binary, and a leaked `OPS_SUBPROCESS_TIMEOUT_SECS` makes later tests fail
/// with timeouts that look like real product bugs.
///
/// DRY-1 / TASK-2059: re-exported from `ops_core::test_utils` rather than
/// redefined — the shared guard is `OsStr`-keyed like this copy was and
/// offers the same `set` / `unset` constructors, so call sites are unchanged.
pub use ops_core::test_utils::EnvGuard as EnvVarGuard;

/// TEST-23 / TASK-1842: RAII guard restoring the process working directory.
///
/// The leak this prevents is worse than it looks: the directory a test
/// chdirs into is a `tempfile::TempDir` deleted on drop, so a skipped
/// restore leaves the *whole test binary* running in a deleted directory,
/// and every later test touching a relative path (e.g. `run_deps`, whose
/// `build_user_context` resolves `std::env::current_dir`) fails for
/// unrelated reasons.
///
/// DRY-1 / TASK-2034: re-exported from `ops_core::test_utils` rather than
/// redefined — that copy also serialises on a process-wide mutex, so a caller
/// that forgets `#[serial]` cannot race another CWD-dependent test.
pub use ops_core::test_utils::CwdGuard;

/// DUP-3 / TASK-2058: the crate's `tracing` capture now comes from the shared
/// harness in `ops_core::test_utils` (re-exported here) instead of a private
/// `BufWriter` + `MakeWriter` scaffold. The shared entry point also pins the
/// global dispatcher, which the local copy never did.
pub use ops_core::test_utils::capture_tracing;
