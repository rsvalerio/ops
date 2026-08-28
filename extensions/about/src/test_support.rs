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
    /// `ops-about-metadata`.
    ///
    /// Each crate previously redefined the same
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
        type Writer = Self;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }
}

#[cfg(feature = "test-support")]
pub use tracing_capture::TracingBuf;

/// DUP-3 / TASK-1735: level-counting counterpart to [`TracingBuf`].
///
/// `TracingBuf` captures *rendered text*; several tests only need "how many
/// WARN events did this call emit?", and each grew its own bespoke
/// `tracing::Subscriber` impl inside a production module file — along with a
/// private copy of the global-dispatcher workaround below, whose absence is a
/// silent flake rather than a failure.
#[cfg(feature = "test-support")]
mod level_counter {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Minimal `tracing::Subscriber` that counts `WARN`-level events, so a
    /// test can assert on warn counts without pulling `tracing-subscriber`
    /// layer machinery into the assertion.
    #[derive(Clone, Default)]
    pub struct WarnCounter(Arc<AtomicUsize>);

    impl WarnCounter {
        /// Number of `WARN` events observed so far.
        #[must_use]
        pub fn count(&self) -> usize {
            self.0.load(Ordering::SeqCst)
        }
    }

    impl tracing::Subscriber for WarnCounter {
        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            if *event.metadata().level() == tracing::Level::WARN {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        fn enter(&self, _span: &tracing::span::Id) {}
        fn exit(&self, _span: &tracing::span::Id) {}
    }

    /// Count the `WARN` events `f` emits.
    ///
    /// Runs `f` with a fresh counting subscriber installed as the
    /// thread-local default, returning its result alongside the warn count.
    /// Handles the `Interest`-cache pin above, so callers never have to
    /// rediscover that hazard.
    ///
    /// **Scope:** the subscriber is the *thread-local* default, so only
    /// warnings `f` emits on the calling thread are counted. If `f` fans out
    /// to worker threads — an `ignore` parallel walker, `rayon`, a spawned
    /// scope — their warnings reach the global dispatcher instead and are not
    /// counted. Do not assert warn counts across a parallel walk with this.
    pub fn count_warnings<T>(f: impl FnOnce() -> T) -> (T, usize) {
        super::pin_global_dispatcher();
        let counter = WarnCounter::default();
        let out = tracing::subscriber::with_default(counter.clone(), f);
        (out, counter.count())
    }
}

#[cfg(feature = "test-support")]
pub use level_counter::{count_warnings, WarnCounter};

/// Keep one globally-registered dispatcher alive for the whole test binary.
///
/// `tracing` caches each callsite's `Interest` process-wide against the
/// dispatchers registered the moment that callsite is first hit; with only
/// scoped (`with_default`) subscribers, a parallel test thread can first-hit a
/// warn callsite while none is registered, caching `Interest::never()` so the
/// warn these helpers observe never fires again and the assertion fails at
/// random — under `cargo test`'s shared-process threads as well as under
/// nextest. A global dispatcher is never unregistered, so the cache can no
/// longer answer "never". This one counts into a throwaway counter nobody
/// reads.
///
/// DUP-3 / TASK-1735, TASK-1794: one definition for the whole workspace. Every
/// per-crate copy of this workaround was a copy whose *absence* is a silent
/// flake rather than a failure, so the copies could not be spotted by a
/// failing test.
#[cfg(feature = "test-support")]
fn pin_global_dispatcher() {
    static INSTALL: std::sync::Once = std::sync::Once::new();
    INSTALL.call_once(|| {
        let _ = tracing::subscriber::set_global_default(WarnCounter::default());
        // Callsites hit before this point resolved against an empty
        // dispatcher list; recompute them now that one is registered.
        tracing::callsite::rebuild_interest_cache();
    });
}

/// Capture the rendered `WARN`-level tracing records `body` emits on the
/// calling thread.
///
/// DUP-3 / TASK-1794: the text-capturing counterpart to [`count_warnings`] —
/// crates that assert on the *content* of a warn record (that a field was
/// `Debug`-formatted, that a specific reason was logged) previously each grew
/// their own `BufWriter` + `MakeWriter` + dispatcher-pin module. Uses
/// [`TracingBuf`], whose lock and UTF-8 handling recover rather than panic.
///
/// ANSI colouring is disabled so the capture contains no escape bytes of the
/// subscriber's own making — assertions about escapes in the *record* stay
/// meaningful.
///
/// **Scope:** the subscriber is the thread-local default, so only records
/// `body` emits on the calling thread are captured.
#[cfg(feature = "test-support")]
pub fn capture_warn<F: FnOnce()>(body: F) -> String {
    pin_global_dispatcher();
    let buf = TracingBuf::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(buf.clone())
        .with_max_level(tracing::Level::WARN)
        .with_ansi(false)
        .finish();
    tracing::subscriber::with_default(subscriber, body);
    buf.captured()
}

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
///
/// # Panics
///
/// If `value`'s `Debug` rendering leaks a raw newline or ANSI escape — that
/// is the assertion this helper exists to make.
pub fn assert_debug_escapes_control_chars<T: std::fmt::Debug>(value: T) {
    let rendered = format!("{value:?}");
    assert_rendered_escapes_control_chars(&rendered);
    assert!(
        rendered.contains("\\n"),
        "expected escaped newline in Debug rendering: {rendered}"
    );
}

/// The half of [`assert_debug_escapes_control_chars`] that applies to an
/// already-rendered string: no raw newline and no raw ANSI `ESC` survived.
///
/// DUP-3 / TASK-1794: callers that capture a real `tracing` record (rather
/// than rendering a value themselves) assert the same property on the captured
/// text — trim the record's own trailing newline first. Splitting it out keeps
/// one definition instead of a second copy at every capture site.
///
/// # Panics
///
/// If `rendered` carries a raw newline or a raw ANSI `ESC` — that is the
/// assertion this helper exists to make.
pub fn assert_rendered_escapes_control_chars(rendered: &str) {
    assert!(
        !rendered.contains('\n'),
        "raw newline leaked into rendered output: {rendered}"
    );
    assert!(
        !rendered.contains('\u{1b}'),
        "raw ANSI ESC leaked into rendered output: {rendered}"
    );
}

/// Write `content` to `path`, creating any missing parent directories.
///
/// DUP-1 / TASK-1736: the `about` extensions' `#[cfg(test)]` modules each
/// grew a byte-identical six-line `write` helper for building tempdir
/// fixtures. Hoisting it here gives the family one definition, so a future
/// tightening (propagating the IO error, richer `expect` messages) lands
/// once instead of drifting between copies. Import it as
/// `use ops_about::test_support::write_file as write;` to keep existing
/// call sites unchanged.
///
/// # Panics
///
/// If the parent directories cannot be created or the file cannot be
/// written — this is fixture setup, where a failure is a broken test, not a
/// condition to handle.
pub fn write_file(path: &std::path::Path, content: &str) {
    // The workspace bans `unwrap`/`expect`/`panic!` outside `#[cfg(test)]`,
    // and this module is compiled as library code, so failures are reported
    // through `assert!` — which carries the same message and the same
    // "broken fixture" semantics.
    if let Some(parent) = path.parent() {
        let created = std::fs::create_dir_all(parent);
        assert!(
            created.is_ok(),
            "create fixture dir {}: {:?}",
            parent.display(),
            created.err()
        );
    }
    let written = std::fs::write(path, content);
    assert!(
        written.is_ok(),
        "write fixture file {}: {:?}",
        path.display(),
        written.err()
    );
}
