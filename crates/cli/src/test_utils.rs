//! CLI-specific test utilities.
//!
//! Re-exports shared helpers from ops-core and ops-runner,
//! and adds CLI-specific ones (CwdGuard-based helpers, etc.).

// Re-export shared test helpers from core
pub use ops_core::test_utils::*;

// Re-export runner test support (EventAssertions, test_runner)
#[cfg(test)]
#[allow(unused_imports)]
pub use ops_runner::test_support::{test_runner, EventAssertions};

/// Process-wide mutex for tests that change the current working directory.
/// Rust tests run in parallel by default; `std::env::set_current_dir` is
/// process-global, so CWD-dependent tests must serialize on this lock.
///
/// # Mutex Poisoning Recovery
///
/// If a test panics while holding this lock, the mutex becomes "poisoned".
/// We intentionally recover from poisoned state (rather than propagating
/// the panic) because:
/// 1. The panic has already been reported by the test framework
/// 2. Subsequent tests should be allowed to run
/// 3. CWD restoration failure is non-critical (test isolation is best-effort)
pub(crate) static CWD_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// RAII guard that acquires CWD_MUTEX, switches to a target directory,
/// and restores the original CWD on drop.
///
/// # Test Isolation Note
///
/// This guard serializes CWD-dependent tests. While this prevents race
/// conditions, it means these tests cannot run in parallel with each other.
/// Prefer using `tempfile::tempdir()` and passing paths explicitly when
/// possible to avoid CWD mutations entirely.
///
/// # Rust 2024 Compatibility (E104)
///
/// `std::env::set_current_dir` is `unsafe` in Rust 2024 edition.
/// All calls are wrapped in `unsafe` blocks with SAFETY comments.
pub(crate) struct CwdGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
    original_dir: std::path::PathBuf,
}

impl CwdGuard {
    pub fn new(target: &std::path::Path) -> Result<Self, std::io::Error> {
        let lock = CWD_MUTEX.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("CWD_MUTEX poisoned by previous test panic, recovering");
            poisoned.into_inner()
        });
        let original_dir = std::env::current_dir()?;
        // SAFETY: Test-only. CWD_MUTEX serializes all CWD-dependent tests.
        // unsafe required in Rust 2024 edition; allow unused_unsafe for 2021.
        #[allow(unused_unsafe)]
        unsafe {
            std::env::set_current_dir(target)?
        };
        Ok(Self {
            _lock: lock,
            original_dir,
        })
    }
}

impl Drop for CwdGuard {
    #[allow(unused_unsafe)]
    fn drop(&mut self) {
        // SAFETY: Test-only. CWD_MUTEX serializes all CWD-dependent tests.
        if let Err(e) = unsafe { std::env::set_current_dir(&self.original_dir) } {
            tracing::warn!("CwdGuard: failed to restore original directory: {}", e);
        }
    }
}

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
/// Returns the temp directory (for cleanup) and the CwdGuard.
#[cfg(test)]
#[allow(dead_code)]
pub fn with_temp_config(content: &str) -> (tempfile::TempDir, CwdGuard) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join(".ops.toml"), content).expect("write .ops.toml");
    let guard = CwdGuard::new(dir.path()).expect("CwdGuard");
    (dir, guard)
}

/// Shared tracing-event capture helper for tests across the cli crate.
/// Installs a thread-local subscriber at `level` for the duration of `f`
/// and returns the captured text.
#[cfg(test)]
#[allow(dead_code)]
pub fn capture_tracing<F: FnOnce()>(level: tracing::Level, f: F) -> String {
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct BufWriter(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for BufWriter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl<'a> MakeWriter<'a> for BufWriter {
        type Writer = BufWriter;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    let buf = BufWriter::default();
    let captured = buf.0.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(buf)
        .with_max_level(level)
        .with_ansi(false)
        .finish();
    tracing::subscriber::with_default(subscriber, f);
    let bytes = captured.lock().unwrap().clone();
    String::from_utf8(bytes).unwrap()
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
            .unwrap_or(dir.path().to_path_buf());
        assert_eq!(
            current_canonical, dir_canonical,
            "should change to target directory"
        );
    }

    #[test]
    fn cwd_guard_mutex_is_recoverable() {
        let _lock = CWD_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
    }
}
