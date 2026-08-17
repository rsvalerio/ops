//! ARCH-1 / TASK-1471: drain machinery for [`super::run_with_timeout`],
//! extracted from the historical grab-bag module.
//!
//! Owns the bounded-read primitive ([`read_capped`]), the per-pipe drain
//! thread spawner ([`spawn_drain`]), the result-collecting reaper
//! ([`collect_drain`]), and the timeout-cleanup helper
//! ([`drain_after_timeout`]).

use std::io::{self, Read};
use std::thread;
use std::time::{Duration, Instant};

use super::cap::OUTPUT_CAP_ENV;
use super::RunError;

/// SEC-33 / TASK-1050: result type returned by drain threads. `(captured,
/// dropped, error_during_read)` where `captured.len() <= cap` and
/// `dropped` counts bytes read past the cap.
pub(super) type DrainResult = (Vec<u8>, u64, Option<io::Error>);

/// SEC-33 / TASK-1050: drain `reader` into `buf` up to `cap` bytes, then
/// keep reading and discarding the remainder so the child does not block
/// on a full pipe. Returns the number of bytes that were dropped past the
/// cap (`0` when the stream fit within the cap) plus any IO error
/// encountered mid-read.
///
/// PERF-3 / TASK-1473: once the in-memory buffer reaches `cap`, the discard
/// path dispatches to [`io::copy`] into [`io::sink`] rather than spinning
/// in user space on per-8 KiB chunks. The kernel-side copy loop in
/// `io::copy` is the same shape the stdlib uses for `read_to_end` discard,
/// and we no longer re-check `remaining` on every iteration after the cap
/// is hit.
pub(super) fn read_capped<R: Read>(
    mut reader: R,
    buf: &mut Vec<u8>,
    cap: usize,
) -> (u64, Option<io::Error>) {
    // 8 KiB matches `std::io::DEFAULT_BUF_SIZE` and is the granularity
    // `read_to_end` uses internally; large enough that the syscall overhead
    // is amortised, small enough that the sink path stays cheap.
    let mut chunk = [0u8; 8 * 1024];
    // PERF-3 / TASK-1425: pre-size the capture buffer so multi-MiB streams
    // (cargo metadata, large stdout) skip the O(log N) Vec-doubling chain
    // from empty. Bounded by `cap` so a tiny cap doesn't over-reserve, and
    // by 64 KiB so a huge cap (256 MiB default) doesn't allocate up-front
    // memory for streams that turn out to be empty.
    const INITIAL_CAP: usize = 64 * 1024;
    let want = cap.min(INITIAL_CAP);
    if buf.capacity() < want {
        buf.reserve(want - buf.len());
    }
    let mut dropped: u64 = 0;
    loop {
        if buf.len() >= cap {
            // PERF-3 / TASK-1473: switch the discard path to io::copy
            // -> io::sink. Bytes past the cap are still consumed (so the
            // child does not block on a full pipe) but stdlib drives the
            // read/discard loop instead of our per-iteration `remaining`
            // recompute. `io::copy` already handles `Interrupted`
            // internally and returns the total bytes drained.
            return match io::copy(&mut reader, &mut io::sink()) {
                Ok(n) => (dropped.saturating_add(n), None),
                Err(e) => (dropped, Some(e)),
            };
        }
        match reader.read(&mut chunk) {
            Ok(0) => return (dropped, None),
            Ok(n) => {
                let remaining = cap - buf.len();
                if n <= remaining {
                    buf.extend_from_slice(&chunk[..n]);
                } else {
                    buf.extend_from_slice(&chunk[..remaining]);
                    dropped = dropped.saturating_add((n - remaining) as u64);
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return (dropped, Some(e)),
        }
    }
}

/// DUP-4 / TASK-1399: shared helper for spawning a drain thread that
/// captures the bytes a child wrote to one pipe, bounded by `cap`. Both
/// stdout and stderr go through this one entry point so the two halves
/// cannot diverge on the next change to read-cap or panic semantics.
pub(super) fn spawn_drain<R>(pipe: Option<R>, cap: usize) -> Option<thread::JoinHandle<DrainResult>>
where
    R: Read + Send + 'static,
{
    pipe.map(|mut s| {
        thread::spawn(move || -> DrainResult {
            let mut buf = Vec::new();
            let (dropped, err) = read_capped(&mut s, &mut buf, cap);
            (buf, dropped, err)
        })
    })
}

/// Join a pipe-drain thread, log any `read_to_end` failure or join panic
/// against `label`/`stream`, and return whatever bytes were successfully
/// read.
///
/// ERR-1 / TASK-0694: a truncated buffer (the partial-read case) is still
/// returned with a tracing breadcrumb so callers see what was captured
/// before the read failure.
///
/// ERR-1 / TASK-0901: a *panicked* drain thread is now propagated as
/// `RunError::Io` instead of an empty `Vec<u8>`. Returning `Vec::new()` on
/// panic made a successful command appear to have produced no output —
/// indistinguishable from a clean empty stream — and downstream cargo
/// callers (cargo metadata / cargo update parsers) silently drove
/// decisions off that empty buffer.
pub(super) fn collect_drain(
    handle: Option<thread::JoinHandle<DrainResult>>,
    label: &str,
    stream: &'static str,
) -> Result<Vec<u8>, RunError> {
    let Some(handle) = handle else {
        return Ok(Vec::new());
    };
    match handle.join() {
        Ok((buf, dropped, None)) => {
            // SEC-33 / TASK-1050: warn-once-per-stream when the capture was
            // bounded so callers parsing the output see a breadcrumb that
            // explains a truncated stdout/stderr instead of treating
            // "missing trailing JSON" as a parser bug.
            if dropped > 0 {
                tracing::warn!(
                    label,
                    stream,
                    bytes_kept = buf.len(),
                    bytes_dropped = dropped,
                    env_var = OUTPUT_CAP_ENV,
                    "subprocess output exceeded cap; trailing bytes were discarded"
                );
            }
            Ok(buf)
        }
        Ok((buf, dropped, Some(err))) => {
            // ARCH-2 / TASK-1426: a mid-read IO failure that captured *zero*
            // bytes is indistinguishable from a clean empty stream once we
            // drop the error, which contradicts the panic-handling contract
            // ("an empty value here always means the child produced no
            // output, never that we lost it"). Surface RunError::Io so
            // downstream cargo parsers don't silently drive decisions off
            // an authoritatively-empty buffer that was actually an EIO.
            //
            // Partial reads (buf non-empty) keep the existing
            // truncated-with-breadcrumb behaviour: the bytes captured before
            // the failure are real and callers have historically used them.
            if buf.is_empty() {
                tracing::warn!(
                    label,
                    stream,
                    bytes_dropped = dropped,
                    error = %err,
                    "subprocess pipe drain failed with no bytes captured; surfacing as RunError::Io"
                );
                return Err(RunError::Io(io::Error::other(format!(
                    "subprocess `{label}` {stream} drain failed before any bytes were captured: {err}"
                ))));
            }
            tracing::warn!(
                label,
                stream,
                bytes_read = buf.len(),
                bytes_dropped = dropped,
                error = %err,
                "subprocess pipe drain failed mid-read; captured output is truncated"
            );
            Ok(buf)
        }
        Err(_) => {
            tracing::warn!(
                label,
                stream,
                "subprocess pipe drain thread panicked; surfacing as RunError::Io"
            );
            Err(RunError::Io(io::Error::other(format!(
                "subprocess `{label}` {stream} drain thread panicked; captured output is unrecoverable"
            ))))
        }
    }
}

/// ASYNC-6: how long the post-timeout cleanup waits for a drain thread to
/// see EOF before abandoning it. Mirrors the stderr drain grace the hook
/// crates use for the same reason.
const DRAIN_GRACE: Duration = Duration::from_millis(500);

/// Poll interval while waiting out [`DRAIN_GRACE`]. Short enough that the
/// common case (EOF already delivered) costs one tick, long enough not to
/// spin.
const DRAIN_POLL: Duration = Duration::from_millis(10);

/// ERR-1 / TASK-1466: post-timeout-kill drain reaper. Joins both pipe
/// drains and emits a `tracing::warn!` against `label`/`stream` when a
/// drain ended in an IO error or thread panic. `collect_drain` already
/// emits its own internal warn; the extra "during timeout cleanup"
/// breadcrumb here lets operators correlate the loss of captured bytes
/// with the `RunError::Timeout` that the caller observed.
///
/// ASYNC-6: the join is bounded by [`DRAIN_GRACE`] rather than unbounded.
/// Killing the child does *not* guarantee EOF on its pipes: any surviving
/// grandchild inherits the write ends and holds them open (`sh -c "sleep
/// 30"` under a shell that forks instead of exec'ing leaves the `sleep`
/// behind). Joining unconditionally there made the timed-out call outrun
/// its own deadline by the grandchild's whole lifetime — the bounded-wait
/// contract the timeout exists to enforce. A drain thread still blocked
/// after the grace is abandoned: it owns only its capped buffer and exits
/// on its own once the pipe finally closes.
pub(super) fn drain_after_timeout(
    stdout: Option<thread::JoinHandle<DrainResult>>,
    stderr: Option<thread::JoinHandle<DrainResult>>,
    label: &str,
) {
    let deadline = Instant::now() + DRAIN_GRACE;
    for (handle, stream) in [(stdout, "stdout"), (stderr, "stderr")] {
        let Some(handle) = handle else {
            continue;
        };
        while !handle.is_finished() && Instant::now() < deadline {
            thread::sleep(DRAIN_POLL);
        }
        if !handle.is_finished() {
            tracing::warn!(
                label,
                stream,
                grace_ms = DRAIN_GRACE.as_millis(),
                "pipe drain still blocked after the child was killed; abandoning it — \
                 a surviving grandchild is holding the pipe open, and captured bytes are lost"
            );
            continue;
        }
        if let Err(e) = collect_drain(Some(handle), label, stream) {
            tracing::warn!(
                label,
                stream,
                error = %e,
                "drain failed during timeout cleanup; captured bytes are unrecoverable"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SEC-33 / TASK-1050: `read_capped` is the workhorse that bounds the
    /// drain-thread allocation. Tested in isolation with an in-memory
    /// `Cursor` so the invariant ("kept + dropped == input length, kept <=
    /// cap") doesn't depend on an OS pipe.
    #[test]
    fn read_capped_bounds_buffer_and_counts_overflow() {
        let input: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
        let cap = 1024;
        let mut buf = Vec::new();
        let (dropped, err) = read_capped(std::io::Cursor::new(&input), &mut buf, cap);
        assert!(err.is_none(), "in-memory cursor must not error");
        assert_eq!(buf.len(), cap, "buffer must be capped exactly to {cap}");
        assert_eq!(
            buf.len() as u64 + dropped,
            input.len() as u64,
            "kept + dropped must equal input length"
        );
        // Spot-check the head bytes match the source so we kept the *first*
        // cap bytes, not the tail.
        assert_eq!(&buf[..16], &input[..16]);
    }

    /// SEC-33 / TASK-1050: when the child's output fits inside the cap,
    /// `read_capped` must report zero dropped bytes and behave identically
    /// to the previous `read_to_end` path.
    #[test]
    fn read_capped_under_cap_is_lossless() {
        let input = b"short payload";
        let mut buf = Vec::new();
        let (dropped, err) = read_capped(&input[..], &mut buf, 4096);
        assert!(err.is_none());
        assert_eq!(dropped, 0);
        assert_eq!(buf, input);
    }

    /// PERF-3 / TASK-1425: a large-cap drain should pre-allocate ~64 KiB
    /// up-front rather than doubling from 0 → 8 → 16 → 32 → 64. Starting
    /// from `Vec::new()` (capacity 0), the post-call capacity must be at
    /// least 64 KiB, proving the reservation actually happened.
    #[test]
    fn read_capped_pre_sizes_buffer_for_large_cap() {
        let mut buf = Vec::new();
        // 1 MiB of synthetic stdout to exercise the multi-chunk path.
        let input: Vec<u8> = vec![b'x'; 1024 * 1024];
        let (dropped, err) = read_capped(std::io::Cursor::new(&input), &mut buf, 4 * 1024 * 1024);
        assert!(err.is_none());
        assert_eq!(dropped, 0);
        assert_eq!(buf.len(), input.len());
        assert!(
            buf.capacity() >= 64 * 1024,
            "expected pre-sized capacity >= 64 KiB, got {}",
            buf.capacity()
        );
    }

    /// PERF-3 / TASK-1425: a tiny cap must NOT over-reserve. With cap=128
    /// and an empty stream, the buffer capacity must stay bounded by the
    /// cap (not 64 KiB).
    #[test]
    fn read_capped_pre_size_respects_small_cap() {
        let mut buf = Vec::new();
        let input: &[u8] = b"";
        let (_dropped, err) = read_capped(input, &mut buf, 128);
        assert!(err.is_none());
        assert!(
            buf.capacity() <= 128,
            "small-cap reservation must not over-allocate, got {}",
            buf.capacity()
        );
    }

    /// PERF-3 / TASK-1473: a stream that dwarfs the cap (here, 16 MiB into
    /// a 64 KiB cap) must drain to EOF via the `io::copy` → `io::sink` path,
    /// keeping exactly `cap` bytes and accounting for every dropped byte.
    ///
    /// TEST-15 / TASK-1664: the `elapsed < 2s` assertion that used to close
    /// this test is **deleted**, not converted, because it could not fail for
    /// any reason except load. Three findings, in order:
    ///
    /// 1. It had no resolution for the regression it named. The "per-8 KiB
    ///    user-space spin" it guarded is 2048 iterations over an in-memory
    ///    cursor — milliseconds, nowhere near two seconds. Same defect as the
    ///    `ops-about` ratio bounds: the assertion names a regression it cannot
    ///    observe.
    /// 2. It could not catch non-termination either. `elapsed` is read *after*
    ///    `read_capped` returns, so a discard loop that never reaches EOF hangs
    ///    the test rather than tripping the bound.
    /// 3. There was never a shipped slow path to regress from: the `io::copy`
    ///    discard and this test landed in the same commit (524af94).
    ///
    /// A counting seam is not the answer here either — wrapping the reader to
    /// count reads would defeat the `BufRead` specialisation that makes
    /// `io::copy` fast, so the measurement would change what it measures. The
    /// byte accounting below is the contract that can actually be asserted.
    #[test]
    fn read_capped_post_cap_discard_keeps_cap_and_accounts_for_the_rest() {
        let cap = 64 * 1024;
        let input: Vec<u8> = vec![b'a'; 16 * 1024 * 1024];
        let mut buf = Vec::new();
        let (dropped, err) = read_capped(std::io::Cursor::new(&input), &mut buf, cap);
        assert!(err.is_none(), "in-memory cursor must not error");
        assert_eq!(buf.len(), cap);
        assert_eq!(
            buf.len() as u64 + dropped,
            input.len() as u64,
            "kept + dropped must equal input length"
        );
    }

    /// ARCH-2 / TASK-1426: when `read_capped` returns an `io::Error` and the
    /// captured buffer is empty, `collect_drain` must surface a
    /// `RunError::Io` rather than `Ok(Vec::new())`. Returning `Ok` here
    /// would make a mid-read EIO indistinguishable from a clean empty
    /// stream — the same hazard the panic-handling contract closes for
    /// drain-thread panics.
    #[test]
    fn collect_drain_empty_with_error_surfaces_io_error() {
        let handle = thread::spawn(|| -> DrainResult {
            (Vec::new(), 0, Some(io::Error::other("synthetic EIO")))
        });
        let err = collect_drain(Some(handle), "arch-2 test", "stdout")
            .expect_err("empty-buf + Some(err) must propagate as RunError::Io");
        match err {
            RunError::Io(e) => {
                let s = e.to_string();
                assert!(
                    s.contains("arch-2 test") && s.contains("stdout"),
                    "rendered error {s:?} should name label and stream"
                );
                assert!(
                    s.contains("synthetic EIO"),
                    "rendered error {s:?} should chain the underlying io error"
                );
            }
            other => panic!("expected RunError::Io, got {other:?}"),
        }
    }

    /// ARCH-2 / TASK-1426: partial reads (buf non-empty + Some(err)) keep
    /// the existing truncated-with-breadcrumb contract — bytes captured
    /// before the failure are real and historically consumed by callers.
    #[test]
    fn collect_drain_partial_read_with_error_returns_captured_bytes() {
        let handle = thread::spawn(|| -> DrainResult {
            (
                b"partial".to_vec(),
                0,
                Some(io::Error::other("synthetic mid-read EIO")),
            )
        });
        let buf = collect_drain(Some(handle), "arch-2 test", "stdout")
            .expect("non-empty buf + Some(err) must keep the truncated-Ok contract");
        assert_eq!(buf, b"partial");
    }

    /// ERR-1 / TASK-1466: the timeout-cleanup branch must surface a
    /// `tracing::warn!` when a drain join errors or panics, so the loss of
    /// captured bytes alongside `RunError::Timeout` is not invisible. We
    /// inject a panicking drain thread into `drain_after_timeout` directly;
    /// the real timeout path uses the same helper.
    #[test]
    fn drain_after_timeout_panic_fires_breadcrumb() {
        let panicking: thread::JoinHandle<DrainResult> =
            thread::spawn(|| panic!("synthetic drain-thread panic"));
        let (logs, ()) = crate::test_utils::capture_tracing(tracing::Level::WARN, || {
            drain_after_timeout(Some(panicking), None, "task-1466-test");
        });
        assert!(
            logs.contains("task-1466-test"),
            "warn must name the caller label, got: {logs}"
        );
        assert!(
            logs.contains("stdout"),
            "warn must name the offending stream, got: {logs}"
        );
        assert!(
            logs.contains("drain"),
            "warn must mention the drain failure, got: {logs}"
        );
    }
}
