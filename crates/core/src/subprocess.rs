//! Bounded-wait subprocess execution.
//!
//! [`run_with_timeout`] runs a `std::process::Command` and returns its
//! captured [`Output`], killing the child and returning a distinct
//! [`RunError::Timeout`] when the wall-clock deadline expires. Shared by the
//! cargo-invoking data providers in `extensions-rust/*` so network-touching
//! cargo subprocesses never hang indefinitely.
//!
//! Per-operation defaults can be overridden with the
//! `OPS_SUBPROCESS_TIMEOUT_SECS` environment variable (see
//! [`default_timeout`]).
//!
//! # Sync-only — async callers must offload
//!
//! [`run_with_timeout`] is a fully synchronous helper. It uses
//! `wait_timeout::ChildExt::wait_timeout` (TASK-0451) so the wait is a
//! single OS-level wait rather than a 100 ms `thread::sleep` poll loop —
//! no battery-burning wakeups for a 30 s `cargo metadata`, and idle waits
//! cooperate with macOS App Nap. The wait blocks the calling thread for
//! the full duration; async callers MUST wrap the invocation in
//! [`tokio::task::spawn_blocking`] (or introduce a dedicated
//! `tokio::process`-based variant) rather than awaiting it on the runtime
//! thread.
//!
//! ## SIGINT / Ctrl-C behaviour
//!
//! In typical interactive use the user's Ctrl-C lands on the foreground
//! process group, which the spawned child belongs to by default
//! (`std::process::Command` does not detach the child into a new pgrp).
//! That means the child receives the same SIGINT and exits, then
//! [`run_with_timeout`] observes the exit via `wait_timeout` and returns
//! normally. No extra wiring is required for that case.
//!
//! When a non-interactive parent receives a signal that does **not**
//! propagate to the child (e.g. a programmatic `kill(parent_pid, …)` or a
//! supervisor that sends SIGTERM only to the leader), the child outlives
//! the parent until the deadline. This helper is sync-only and does not
//! install signal handlers — closing that gap would require a dedicated
//! cancellation token threaded from the caller, which today's callers do
//! not need.

use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;
use wait_timeout::ChildExt;

/// Environment variable used to override the per-operation default timeout.
pub const TIMEOUT_ENV: &str = "OPS_SUBPROCESS_TIMEOUT_SECS";

/// SEC-33 / TASK-1050: environment variable used to override the per-stream
/// byte cap applied by [`run_with_timeout`]'s drain threads. Mirrors the
/// runner's `command::exec::read_capped` shape (PERF-1 / TASK-0764) so a
/// runaway cargo subprocess cannot grow the in-memory capture buffer
/// without bound. Reuses the same env var name the runner already
/// documents — `ops` users only have one knob to tune.
pub const OUTPUT_CAP_ENV: &str = "OPS_OUTPUT_BYTE_CAP";

/// Default per-stream byte cap applied to captured stdout/stderr in
/// [`run_with_timeout`]. Matches the runner's
/// `DEFAULT_OUTPUT_BYTE_CAP` (4 MiB) so the cap is consistent across the
/// project's two subprocess paths. Once the cap is reached the drain
/// thread keeps reading from the pipe (so the child does not block on a
/// full pipe and risk a timeout) but discards the bytes and increments a
/// `dropped` counter that surfaces via `tracing::warn!`.
pub const DEFAULT_OUTPUT_BYTE_CAP: usize = 4 * 1024 * 1024;

/// Fallback timeout applied when a caller has no operation-specific default
/// and `OPS_SUBPROCESS_TIMEOUT_SECS` is unset or unparseable.
pub const FALLBACK_TIMEOUT: Duration = Duration::from_secs(180);

/// ASYNC-6 / TASK-0304: upper bound on `OPS_SUBPROCESS_TIMEOUT_SECS`.
///
/// The whole point of [`run_with_timeout`] is bounded execution; allowing an
/// env-driven `u64::MAX` effectively disables the timeout and silently
/// breaks the helper's contract. 1 hour is generous (the longest legitimate
/// caller is `cargo update`, capped well below this) while still preventing
/// an unbounded hang.
pub const MAX_TIMEOUT_SECS: u64 = 3600;

/// Returned when [`run_with_timeout`] has to kill the child because it
/// outran the deadline. The label is the human-readable operation name
/// passed in by the caller (e.g. `"cargo metadata"`).
#[derive(Debug)]
#[non_exhaustive]
pub struct TimeoutError {
    pub label: String,
    pub timeout: Duration,
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} timed out after {}s",
            self.label,
            self.timeout.as_secs()
        )
    }
}

impl std::error::Error for TimeoutError {}

/// Returned when [`run_with_timeout`] cannot spawn the child process.
///
/// ERR-4 / TASK-0925: a bare `io::Error` from `Command::spawn` renders as
/// `No such file or directory (os error 2)` with no indication of which
/// subprocess failed. Wrapping the error with the caller-supplied label
/// and the program name (e.g. `cargo`) makes the rendered message
/// self-describing — `"cargo metadata: failed to spawn cargo: No such
/// file or directory (os error 2)"` — while preserving `source()` to the
/// original `io::Error` so structured callers can still inspect the kind.
#[derive(Debug)]
#[non_exhaustive]
pub struct SpawnError {
    pub label: String,
    pub program: String,
    pub source: io::Error,
}

impl std::fmt::Display for SpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: failed to spawn {}: {}",
            self.label, self.program, self.source
        )
    }
}

impl std::error::Error for SpawnError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Error returned by [`run_with_timeout`]: either spawn failed, post-spawn
/// IO failed, or the child outran the deadline.
#[derive(Debug)]
#[non_exhaustive]
pub enum RunError {
    Io(io::Error),
    Spawn(SpawnError),
    Timeout(TimeoutError),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunError::Io(e) => write!(f, "{e}"),
            RunError::Spawn(e) => write!(f, "{e}"),
            RunError::Timeout(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RunError::Io(e) => Some(e),
            RunError::Spawn(e) => Some(e),
            RunError::Timeout(e) => Some(e),
        }
    }
}

impl From<io::Error> for RunError {
    fn from(e: io::Error) -> Self {
        RunError::Io(e)
    }
}

/// SEC-33 / TASK-1050: resolve the per-stream byte cap once per process.
/// `OPS_OUTPUT_BYTE_CAP` overrides [`DEFAULT_OUTPUT_BYTE_CAP`] when present
/// and parses to a positive `usize`; any other value falls back to the
/// default with a one-shot `tracing::warn!`. Cached behind a `OnceLock` so
/// repeated subprocess invocations do not re-read the env on every call.
fn output_byte_cap() -> usize {
    static CAP: OnceLock<usize> = OnceLock::new();
    *CAP.get_or_init(|| match std::env::var(OUTPUT_CAP_ENV) {
        Err(_) => DEFAULT_OUTPUT_BYTE_CAP,
        Ok(s) => match s.parse::<usize>() {
            Ok(n) if n > 0 => n,
            _ => {
                tracing::warn!(
                    env_var = OUTPUT_CAP_ENV,
                    raw = %s,
                    default = DEFAULT_OUTPUT_BYTE_CAP,
                    "OPS_OUTPUT_BYTE_CAP is not a positive integer; using default"
                );
                DEFAULT_OUTPUT_BYTE_CAP
            }
        },
    })
}

/// SEC-33 / TASK-1050: result type returned by drain threads. `(captured,
/// dropped, error_during_read)` where `captured.len() <= cap` and
/// `dropped` counts bytes read past the cap.
type DrainResult = (Vec<u8>, u64, Option<io::Error>);

/// SEC-33 / TASK-1050: drain `reader` into `buf` up to `cap` bytes, then
/// keep reading and discarding the remainder so the child does not block
/// on a full pipe. Returns the number of bytes that were dropped past the
/// cap (`0` when the stream fit within the cap) plus any IO error
/// encountered mid-read.
///
/// Mirrors the runner's `command::exec::read_capped` (PERF-1 / TASK-0764)
/// shape adapted to the synchronous `std::io::Read` world used here.
fn read_capped<R: Read>(mut reader: R, buf: &mut Vec<u8>, cap: usize) -> (u64, Option<io::Error>) {
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
        match reader.read(&mut chunk) {
            Ok(0) => return (dropped, None),
            Ok(n) => {
                let remaining = cap.saturating_sub(buf.len());
                if remaining == 0 {
                    dropped = dropped.saturating_add(n as u64);
                } else if n <= remaining {
                    buf.extend_from_slice(&chunk[..n]);
                } else {
                    buf.extend_from_slice(&chunk[..remaining]);
                    dropped = dropped.saturating_add((n - remaining) as u64);
                }
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return (dropped, Some(e)),
        }
    }
}

/// PERF-3 / TASK-1218: pure parser for the `OPS_SUBPROCESS_TIMEOUT_SECS`
/// raw value. Returns `Some(secs)` (clamped to [`MAX_TIMEOUT_SECS`]) when
/// the input is a positive `u64`, `None` otherwise so the caller falls
/// back to the operation-specific default. Factored out so the
/// clamp/zero/unset matrix is unit-testable without poking the
/// process-global `OnceLock`.
fn parse_subprocess_timeout(raw: Option<&str>) -> Option<u64> {
    let parsed = raw?.parse::<u64>().ok().filter(|&s| s > 0)?;
    if parsed > MAX_TIMEOUT_SECS {
        tracing::warn!(
            requested = parsed,
            clamped_to = MAX_TIMEOUT_SECS,
            env = TIMEOUT_ENV,
            "ASYNC-6: clamping subprocess timeout to upper bound; bounded execution is the helper's contract"
        );
        Some(MAX_TIMEOUT_SECS)
    } else {
        Some(parsed)
    }
}

/// PERF-3 / TASK-1218: cache the resolved `OPS_SUBPROCESS_TIMEOUT_SECS`
/// value behind a `OnceLock<Option<u64>>` so each subprocess spawn does
/// not re-acquire the global env lock and re-allocate the raw `String`.
/// `None` means "env unset / zero / unparseable — fall back to
/// op_default"; `Some(secs)` is the already-clamped override (the warn
/// fires once at cache init). Mirrors the `output_byte_cap` discipline
/// one function above.
fn cached_subprocess_timeout() -> Option<u64> {
    static CACHED: OnceLock<Option<u64>> = OnceLock::new();
    *CACHED.get_or_init(|| parse_subprocess_timeout(std::env::var(TIMEOUT_ENV).ok().as_deref()))
}

/// Resolve an effective timeout: `OPS_SUBPROCESS_TIMEOUT_SECS` overrides the
/// caller-provided default if present and parses to a non-zero u64; otherwise
/// the operation-specific default is returned unchanged.
///
/// ASYNC-6 / TASK-0304: the override is clamped to [`MAX_TIMEOUT_SECS`] and
/// emits a warning when it had to be clamped, so an accidental
/// `OPS_SUBPROCESS_TIMEOUT_SECS=18446744073709551615` does not silently
/// disable the helper's bounded-wait contract.
///
/// PERF-3 / TASK-1218: the env knob is resolved at most once per process
/// via [`cached_subprocess_timeout`]. Tests that exercise the parse/clamp
/// matrix should call [`parse_subprocess_timeout`] directly to bypass the
/// cache.
#[must_use]
pub fn default_timeout(op_default: Duration) -> Duration {
    match cached_subprocess_timeout() {
        Some(secs) => Duration::from_secs(secs),
        None => op_default,
    }
}

/// Run `cmd` with stdout/stderr captured, returning its [`Output`]. Kills
/// the child and returns [`RunError::Timeout`] when the deadline expires.
///
/// `label` is embedded in the timeout error message so callers don't need to
/// wrap the error themselves.
///
/// # Blocking
///
/// Synchronous: a single `wait_timeout` call blocks the current thread
/// until the child exits or the deadline expires. Async callers MUST run
/// this inside `tokio::task::spawn_blocking` — see the module docs.
///
/// # Errors
///
/// Returns [`RunError::Io`] if spawning or waiting on the child fails, and
/// [`RunError::Timeout`] if the child outruns `timeout`.
///
/// ## Panic-handling guarantees (ERR-1 / TASK-0901)
///
/// - A panic inside a stdout/stderr drain thread is propagated as
///   [`RunError::Io`] rather than silently substituting an empty
///   `Vec<u8>`. The previous behaviour made a successful command appear
///   to produce no output, indistinguishable from a clean empty stream
///   — downstream cargo callers that drove decisions off `stdout`
///   silently saw a wrong empty result.
/// - A drain thread that fails its `read_to_end` mid-read still returns
///   the bytes captured before the error, with a `tracing::warn!`
///   breadcrumb (ERR-1 / TASK-0694).
/// - `Output.stdout` / `Output.stderr` therefore mean exactly "what the
///   child wrote and the kernel handed us"; an empty value here always
///   means the child produced no output, never that we lost it.
pub fn run_with_timeout(
    cmd: &mut Command,
    timeout: Duration,
    label: &str,
) -> Result<Output, RunError> {
    run_with_timeout_inner(cmd, timeout, label, output_byte_cap())
}

fn run_with_timeout_inner(
    cmd: &mut Command,
    timeout: Duration,
    label: &str,
    cap: usize,
) -> Result<Output, RunError> {
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| {
            // ERR-4 / TASK-0925: name the failing operation and program in
            // the rendered error so a missing binary surfaces as
            // "cargo metadata: failed to spawn cargo: …" instead of a bare
            // OS error string.
            RunError::Spawn(SpawnError {
                label: label.to_string(),
                program: cmd.get_program().to_string_lossy().into_owned(),
                source: e,
            })
        })?;

    // ERR-1 / TASK-0694: drain threads return both the bytes read so far
    // and any IO error encountered. We surface drain failures via
    // `tracing::warn!` so callers parsing the captured output have a
    // breadcrumb when a buffer is truncated, instead of seeing a silently
    // empty stream that round-trips as "the command produced no output".
    //
    // SEC-33 / TASK-1050: drain via `read_capped` so each buffer is bounded
    // near `cap` regardless of how much the child writes. Bytes past the
    // cap are still read off the pipe (so the child does not block on a
    // full pipe and we don't false-positive into a timeout) but discarded;
    // the count is reported by `collect_drain` via `tracing::warn!`.
    let stdout_handle = child.stdout.take().map(|mut s| {
        thread::spawn(move || -> DrainResult {
            let mut buf = Vec::new();
            let (dropped, err) = read_capped(&mut s, &mut buf, cap);
            (buf, dropped, err)
        })
    });
    let stderr_handle = child.stderr.take().map(|mut s| {
        thread::spawn(move || -> DrainResult {
            let mut buf = Vec::new();
            let (dropped, err) = read_capped(&mut s, &mut buf, cap);
            (buf, dropped, err)
        })
    });

    // TASK-0451: single OS-level wait, no polling loop. Returns Ok(None)
    // on timeout, Ok(Some(status)) on exit; the underlying syscall sleeps
    // the thread cooperatively, so idle waits do not burn CPU/battery.
    let status = match child.wait_timeout(timeout)? {
        Some(s) => s,
        None => {
            // Kill first so the drain threads see EOF and unblock; then
            // collect their results before returning the timeout error.
            let _ = child.kill();
            let _ = child.wait();
            let _ = collect_drain(stdout_handle, label, "stdout");
            let _ = collect_drain(stderr_handle, label, "stderr");
            return Err(RunError::Timeout(TimeoutError {
                label: label.to_string(),
                timeout,
            }));
        }
    };

    let stdout = collect_drain(stdout_handle, label, "stdout")?;
    let stderr = collect_drain(stderr_handle, label, "stderr")?;

    Ok(Output {
        status,
        stdout,
        stderr,
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
/// `RunError::Io` instead of an empty `Vec<u8>`. Returning Vec::new() on
/// panic made a successful command appear to have produced no output —
/// indistinguishable from a clean empty stream — and downstream cargo
/// callers (cargo metadata / cargo update parsers) silently drove
/// decisions off that empty buffer.
fn collect_drain(
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

/// Run `cargo <args...>` in `working_dir` under [`run_with_timeout`].
///
/// `op_default` is the operation-specific timeout; the caller can still
/// override it via `OPS_SUBPROCESS_TIMEOUT_SECS` (handled by
/// [`default_timeout`]). `label` flows through to [`TimeoutError`].
///
/// Centralises the build-Command + run + label pattern shared by
/// `cargo update`, `cargo metadata`, `cargo upgrade`, `cargo deny`, and
/// `cargo llvm-cov` callers in the Rust extensions.
///
/// PORT-5 (TASK-0697): the cargo binary is resolved via `$CARGO` first and
/// only falls back to a `$PATH` lookup of the literal `"cargo"` when the
/// variable is unset. Cargo subcommands inherit `$CARGO` from the parent
/// process pointing at the exact toolchain binary that drove the
/// invocation; honouring that variable keeps nested cargo calls on the same
/// toolchain (matters under `cargo +nightly ops <cmd>` and vendored rustup
/// layouts). Standard plugins like `clippy` and `cargo-llvm-cov` follow the
/// same convention.
///
/// # Errors
///
/// Returns [`RunError::Io`] if the subprocess fails to spawn and
/// [`RunError::Timeout`] if it outruns the (possibly env-overridden)
/// deadline.
pub fn run_cargo(
    args: &[&str],
    working_dir: &Path,
    op_default: Duration,
    label: &str,
) -> Result<Output, RunError> {
    run_with_timeout(
        Command::new(resolve_cargo_bin())
            .args(args)
            .current_dir(working_dir),
        default_timeout(op_default),
        label,
    )
}

/// Resolve the cargo binary, honouring `$CARGO` so nested cargo calls stay
/// on the parent toolchain. PORT-5 (TASK-0697).
#[must_use]
pub fn resolve_cargo_bin() -> std::ffi::OsString {
    std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into())
}

/// Resolve the rustup binary, honouring `$RUSTUP` for symmetry with
/// [`resolve_cargo_bin`]. PORT (TASK-0792): keeps direct rustup spawns in
/// extensions on the same toolchain layout the parent process selected
/// rather than forcing a fresh `$PATH` lookup.
#[must_use]
pub fn resolve_rustup_bin() -> std::ffi::OsString {
    std::env::var_os("RUSTUP").unwrap_or_else(|| "rustup".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ASYNC-6 / TASK-0304 + PERF-3 / TASK-1218: the matrix tests now
    /// exercise the pure [`parse_subprocess_timeout`] helper directly. The
    /// public [`default_timeout`] memoises via `cached_subprocess_timeout`,
    /// so a serial-mutating test would either be the first to populate the
    /// cache (winning) or observe a stale snapshot, depending on test
    /// ordering — exactly the surprise the cache is meant to remove.
    mod env_override {
        use super::*;

        #[test]
        fn clamps_huge_value_to_max() {
            let secs = parse_subprocess_timeout(Some(&u64::MAX.to_string()))
                .expect("huge value clamps but is still Some");
            assert_eq!(secs, MAX_TIMEOUT_SECS);
        }

        #[test]
        fn zero_value_falls_back_to_op_default() {
            assert!(
                parse_subprocess_timeout(Some("0")).is_none(),
                "zero must fall back so default_timeout returns op_default"
            );
        }

        #[test]
        fn unset_returns_op_default() {
            assert!(
                parse_subprocess_timeout(None).is_none(),
                "unset must fall back so default_timeout returns op_default"
            );
        }

        #[test]
        fn within_bounds_is_honored() {
            assert_eq!(parse_subprocess_timeout(Some("30")), Some(30));
        }

        #[test]
        fn unparseable_falls_back() {
            assert!(parse_subprocess_timeout(Some("not-a-number")).is_none());
        }
    }

    /// PORT-5 (TASK-0697): cargo subcommands inherit `$CARGO` from the
    /// parent cargo process; `run_cargo` must honour it instead of relying
    /// on a fresh `$PATH` lookup that may resolve to a different toolchain.
    mod cargo_bin {
        use super::*;
        use serial_test::serial;

        const CARGO_ENV: &str = "CARGO";

        #[test]
        #[serial]
        fn honours_cargo_env_when_set() {
            unsafe { std::env::set_var(CARGO_ENV, "/opt/toolchain/bin/cargo") };
            let resolved = resolve_cargo_bin();
            unsafe { std::env::remove_var(CARGO_ENV) };
            assert_eq!(
                resolved,
                std::ffi::OsString::from("/opt/toolchain/bin/cargo")
            );
        }

        #[test]
        #[serial]
        fn falls_back_to_literal_cargo_when_unset() {
            unsafe { std::env::remove_var(CARGO_ENV) };
            let resolved = resolve_cargo_bin();
            assert_eq!(resolved, std::ffi::OsString::from("cargo"));
        }

        const RUSTUP_ENV: &str = "RUSTUP";

        #[test]
        #[serial]
        fn rustup_honours_env_when_set() {
            unsafe { std::env::set_var(RUSTUP_ENV, "/opt/toolchain/bin/rustup") };
            let resolved = resolve_rustup_bin();
            unsafe { std::env::remove_var(RUSTUP_ENV) };
            assert_eq!(
                resolved,
                std::ffi::OsString::from("/opt/toolchain/bin/rustup")
            );
        }

        #[test]
        #[serial]
        fn rustup_falls_back_to_literal_when_unset() {
            unsafe { std::env::remove_var(RUSTUP_ENV) };
            let resolved = resolve_rustup_bin();
            assert_eq!(resolved, std::ffi::OsString::from("rustup"));
        }
    }

    #[test]
    fn completes_before_timeout() {
        let out = run_with_timeout(
            Command::new("sh").args(["-c", "printf hello"]),
            Duration::from_secs(5),
            "sh echo",
        )
        .expect("fast command should not time out");
        assert!(out.status.success());
        assert_eq!(out.stdout, b"hello");
    }

    /// ERR-4 / TASK-0925: a spawn failure (binary not on PATH) used to
    /// surface as a bare `RunError::Io` carrying only `No such file or
    /// directory (os error 2)`. The error must now be a `RunError::Spawn`
    /// whose Display includes the caller-supplied label and the program
    /// name, while preserving `source()` to the original `io::Error`.
    #[test]
    fn spawn_failure_includes_label_and_program() {
        let err = run_with_timeout(
            &mut Command::new("ops-nonexistent-binary-task-0925"),
            Duration::from_secs(5),
            "cargo metadata",
        )
        .expect_err("missing binary should fail to spawn");
        let rendered = err.to_string();
        assert!(
            rendered.contains("cargo metadata"),
            "rendered error {rendered:?} should contain caller label"
        );
        assert!(
            rendered.contains("ops-nonexistent-binary-task-0925"),
            "rendered error {rendered:?} should contain program name"
        );
        match err {
            RunError::Spawn(s) => {
                assert_eq!(s.label, "cargo metadata");
                assert_eq!(s.program, "ops-nonexistent-binary-task-0925");
                assert_eq!(s.source.kind(), io::ErrorKind::NotFound);
                // source() must still chain to the original io::Error so
                // structured callers can inspect the kind.
                let src = std::error::Error::source(&s)
                    .expect("SpawnError::source must expose the io::Error");
                assert!(src.to_string().contains("No such file"));
            }
            other => panic!("expected RunError::Spawn, got {other:?}"),
        }
    }

    /// SEC-33 / TASK-1050: a child that emits `> cap` bytes must be
    /// truncated to `<= cap` so a runaway cargo subprocess cannot grow the
    /// in-memory capture buffer without bound. The pre-fix behaviour
    /// called `read_to_end` and would have buffered the full output.
    #[test]
    fn truncates_output_past_cap() {
        // 64 KiB cap, child writes 256 KiB. Stays small enough that the
        // test runs in milliseconds even on a cold sandbox.
        const CAP: usize = 64 * 1024;
        const TOTAL: usize = 256 * 1024;
        // Use `head -c` to write exactly TOTAL bytes of /dev/zero. POSIX
        // shells on macOS / Linux both support this.
        let script = format!("head -c {TOTAL} /dev/zero");
        let out = run_with_timeout_inner(
            Command::new("sh").args(["-c", &script]),
            Duration::from_secs(10),
            "sec-33 cap test",
            CAP,
        )
        .expect("child should complete within timeout");
        assert!(out.status.success(), "child exited non-zero: {:?}", out);
        assert!(
            out.stdout.len() <= CAP,
            "captured stdout {} exceeded cap {}",
            out.stdout.len(),
            CAP
        );
        assert_eq!(
            out.stdout.len(),
            CAP,
            "expected the buffer to fill exactly to the cap when child output > cap"
        );
    }

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

    #[test]
    fn fires_timeout_on_hung_subprocess() {
        let err = run_with_timeout(
            Command::new("sh").args(["-c", "sleep 30"]),
            Duration::from_millis(300),
            "sh sleep",
        )
        .expect_err("slow command should time out");
        match err {
            RunError::Timeout(t) => {
                assert_eq!(t.label, "sh sleep");
                assert_eq!(t.timeout, Duration::from_millis(300));
            }
            RunError::Io(e) => panic!("expected timeout, got io error: {e}"),
            RunError::Spawn(e) => panic!("expected timeout, got spawn error: {e}"),
        }
    }
}
