//! Test-only scaffolding shared across the crate's test modules.
//!
//! TASK-1494 consolidated the `tracing` capture scaffold into a single
//! definition. TASK-1670 moved it here when the flat `tests.rs` was split:
//! its call sites now sit in two different modules (`parse/upgrade` and
//! `parse/deny`), and giving each one its own copy would silently revert
//! that consolidation.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;

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
/// The guard restores the variable's *prior* state — including "was unset" —
/// rather than unconditionally removing it.
pub struct EnvVarGuard {
    key: OsString,
    prev: Option<OsString>,
}

impl EnvVarGuard {
    /// Set `key` to `value` for the guard's lifetime.
    pub fn set<K: AsRef<OsStr>, V: AsRef<OsStr>>(key: K, value: V) -> Self {
        let key = key.as_ref().to_os_string();
        let prev = std::env::var_os(&key);
        // SAFETY: call sites are `#[serial]`, so no other test thread is
        // reading or writing the environment concurrently, and the value is
        // read synchronously on this thread. Restoration is handled by
        // `Drop`, which also runs while unwinding from a panic — so the
        // mutation cannot outlive the test even when it fails.
        unsafe { std::env::set_var(&key, value) };
        Self { key, prev }
    }

    /// Remove `key` for the guard's lifetime, restoring its prior value on
    /// drop.
    ///
    /// TEST-23: the mirror of [`Self::set`], for tests whose subject reads
    /// the *absence* of an override — a config loader asserting the on-disk
    /// value wins has to be sure the ambient environment is not supplying
    /// one, or it passes for the wrong reason on a developer machine and
    /// fails on the one where the variable happens to be exported.
    pub fn unset<K: AsRef<OsStr>>(key: K) -> Self {
        let key = key.as_ref().to_os_string();
        let prev = std::env::var_os(&key);
        // SAFETY: same serial-execution argument as `set`.
        unsafe { std::env::remove_var(&key) };
        Self { key, prev }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        // SAFETY: same serial-execution argument as `EnvVarGuard::set`. This
        // runs on the normal *and* the unwind path, which is the whole point
        // of the guard.
        match self.prev.take() {
            Some(prev) => unsafe { std::env::set_var(&self.key, prev) },
            None => unsafe { std::env::remove_var(&self.key) },
        }
    }
}

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

#[derive(Clone, Default)]
pub struct BufWriter(pub Arc<Mutex<Vec<u8>>>);

impl Write for BufWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for BufWriter {
    type Writer = Self;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

pub fn with_captured_logs<F, R>(level: Level, ansi: bool, f: F) -> (R, String)
where
    F: FnOnce() -> R,
{
    let buf = BufWriter::default();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_writer(buf.clone())
        .with_ansi(ansi)
        .without_time()
        .finish();
    let result = tracing::subscriber::with_default(subscriber, f);
    let logged = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
    (result, logged)
}
