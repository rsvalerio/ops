//! Test-only scaffolding shared across the crate's test modules.
//!
//! Every item here is a re-export of the workspace-wide harness in
//! `ops_core::test_utils`. This module is the crate's single import point for
//! them, so call sites in `parse/upgrade`, `parse/deny`, `format` and the
//! crate-root tests all reach the same definitions.

/// RAII guard restoring a process-global environment variable on drop,
/// including on the unwind path.
///
/// `#[serial]` orders tests; it does not clean up after a panicking one. A
/// hand-written restore placed after the call under test would be skipped by
/// exactly the assertion failure the test exists to produce, and the leak
/// then travels: a leaked `CARGO` redirects every later cargo spawn in the
/// binary, and a leaked `OPS_SUBPROCESS_TIMEOUT_SECS` makes later tests fail
/// with timeouts that look like real product bugs.
///
/// `OsStr`-keyed, with `set` / `unset` constructors.
pub use ops_core::test_utils::EnvGuard as EnvVarGuard;

/// RAII guard restoring the process working directory, and serialising on a
/// process-wide mutex so a caller that forgets `#[serial]` cannot race
/// another CWD-dependent test.
///
/// The leak this prevents is worse than it looks: the directory a test
/// chdirs into is a `tempfile::TempDir` deleted on drop, so a skipped
/// restore would leave the *whole test binary* running in a deleted
/// directory, and every later test touching a relative path (e.g.
/// `run_deps`, whose `build_user_context` resolves
/// `std::env::current_dir`) would fail for unrelated reasons.
pub use ops_core::test_utils::CwdGuard;

/// Run a closure with `tracing` output captured, returning the captured text
/// alongside the closure's value. Pins the global dispatcher for the
/// duration, so the capture is not raced by another test's subscriber.
pub use ops_core::test_utils::capture_tracing;
