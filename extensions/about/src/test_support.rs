//! Shared test helpers for the about extensions.
//!
//! DUP-3 / TASK-0985: the ERR-7 sweep (TASK-0818 / TASK-0930 / TASK-0809)
//! pinned that path / directive tracing fields flow through `Debug` so
//! embedded newlines / ANSI escapes cannot forge log records. Each
//! provider grew its own `*_path_debug_escapes_control_characters` test
//! that re-proved the same property of `std::fmt::Debug`. Per-site tests
//! still exist (so the sweep contract is visible at every call site), but
//! they now share the assertion logic — deletions of one site no longer
//! weaken coverage silently.

/// DUP-3 / TASK-1157: shared tracing-capture harness lives behind the
/// `test-support` feature so consuming crates explicitly opt in. The
/// `assert_debug_escapes_control_chars` helper below remains available to
/// in-crate `cfg(test)` callers without the feature.
#[cfg(feature = "test-support")]
mod tracing_capture {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    /// DUP-3 / TASK-1157: shared tracing-capture harness used by the
    /// poison-recovery and warn-once tests in `ops-about-rust` and
    /// `ops-about-metadata`. Each crate previously redefined the same
    /// `BufWriter(Arc<Mutex<Vec<u8>>>)` + `Write` + `MakeWriter` shim
    /// inline (3+ copies, ~17 lines each); style drift between copies led
    /// to inconsistent log capture.
    ///
    /// Construct via [`TracingBuf::default`], hand the buffer to
    /// `tracing_subscriber::fmt::Subscriber::with_writer`, and read the
    /// captured bytes via [`TracingBuf::captured`] after the subscriber drops.
    #[derive(Clone, Default)]
    pub struct TracingBuf(Arc<Mutex<Vec<u8>>>);

    impl TracingBuf {
        /// Snapshot of the captured tracing output as a UTF-8 string. Tests
        /// typically assert on substrings, so we tolerate a flush that
        /// splits a multi-byte char by going through `from_utf8_lossy`.
        #[must_use]
        pub fn captured(&self) -> String {
            let guard = self
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            String::from_utf8_lossy(&guard).into_owned()
        }
    }

    impl Write for TracingBuf {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .extend_from_slice(b);
            Ok(b.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for TracingBuf {
        type Writer = TracingBuf;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }
}

#[cfg(feature = "test-support")]
pub use tracing_capture::TracingBuf;

/// Pin the property guaranteed by `Debug` formatting on `Path::display()`
/// (or any value carrying user-controlled text):
///
/// 1. raw newlines must not survive in the rendered field,
/// 2. ANSI escape (ESC, U+001B) must not survive,
/// 3. the rendered field must contain the escaped form `\n`.
///
/// Each `about` extension's per-provider test calls this with a value
/// shaped like its own tracing site, so removing one provider's site
/// does not weaken sweep coverage elsewhere.
pub fn assert_debug_escapes_control_chars<T: std::fmt::Debug>(value: T) {
    let rendered = format!("{value:?}");
    assert!(
        !rendered.contains('\n'),
        "raw newline leaked into Debug rendering: {rendered}"
    );
    assert!(
        !rendered.contains('\u{1b}'),
        "raw ANSI ESC leaked into Debug rendering: {rendered}"
    );
    assert!(
        rendered.contains("\\n"),
        "expected escaped newline in Debug rendering: {rendered}"
    );
}
