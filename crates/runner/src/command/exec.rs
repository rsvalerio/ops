//! Async command execution: running a built [`Command`], capturing output,
//! emitting [`RunnerEvent`]s, and applying timeouts.
//!
//! # Security Model
//!
//! Commands are executed directly from configuration (`.ops.toml`) without
//! sanitization. This is **intentional by design** — `ops` follows the
//! same trust model as `make`, `npm run`, and other build tools:
//!
//! - Local `.ops.toml` files are implicitly trusted
//! - Users should only run `cargo ops` in directories they trust
//! - This is documented in `config::load_config` and the README
//!
//! ## Environment Variables (SEC-002, SEC-003)
//!
//! **WARNING: Do NOT store secrets in `.ops.toml` files.**
//!
//! Environment variables from the `env` section of command definitions are
//! passed directly to child processes. This means:
//!
//! - **Secrets are visible in process listings** (`ps aux`, `/proc`, Task Manager)
//! - **Secrets may appear in logs** if debug logging is enabled
//! - **Config files may be committed to version control** accidentally
//!
//! Instead, use one of these approaches:
//! 1. Set secrets via OS environment: `MY_SECRET=xxx cargo ops build`
//! 2. Use a secrets manager and reference via environment
//! 3. Use `.env` files that are gitignored
//!
//! The [`warn_if_sensitive_env`](super::secret_patterns::warn_if_sensitive_env)
//! function logs a warning when it detects sensitive-looking variable names or
//! values that appear to be secrets (e.g., long base64-like strings, common
//! secret formats).

use super::abort::AbortSignal;
use super::build::{build_command_async, CwdEscapePolicy};
use super::events::RunnerEvent;
use super::results::{CommandOutput, StepResult};
use ops_core::config::{CommandId, ExecCommandSpec};
use ops_core::expand::Variables;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt};
#[cfg(test)]
use tokio::process::Command;
use tokio::sync::mpsc;

/// Await a future with an optional timeout, mapping elapsed timeouts to an
/// `io::ErrorKind::TimedOut` with a unified "timed out after Ns" message.
async fn await_with_timeout<F, T>(future: F, timeout: Option<Duration>) -> Result<T, std::io::Error>
where
    F: std::future::Future<Output = Result<T, std::io::Error>>,
{
    if let Some(t) = timeout {
        match tokio::time::timeout(t, future).await {
            Ok(result) => result,
            Err(_) => Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("timed out after {}s", t.as_secs()),
            )),
        }
    } else {
        future.await
    }
}

/// Run a future to completion, tracking elapsed duration and applying an
/// optional timeout. Shared by [`exec_command`] and [`exec_command_raw`] so
/// both paths produce identical timeout messages and duration semantics.
async fn run_with_timeout<F, T>(
    future: F,
    timeout: Option<Duration>,
) -> (Result<T, std::io::Error>, Duration)
where
    F: std::future::Future<Output = Result<T, std::io::Error>>,
{
    let start = Instant::now();
    let result = await_with_timeout(future, timeout).await;
    (result, start.elapsed())
}

/// PERF-1 / TASK-0764: read up to `cap` bytes from `reader` into a `Vec`,
/// then drain the rest into a sink, counting the dropped bytes. Bounds peak
/// memory near `cap` even if the child writes orders of magnitude more.
async fn read_capped<R: AsyncRead + Unpin>(
    reader: R,
    cap: usize,
) -> std::io::Result<(Vec<u8>, u64)> {
    let mut head = Vec::new();
    let mut limited = reader.take(cap as u64);
    limited.read_to_end(&mut head).await?;
    let mut inner = limited.into_inner();
    let dropped = tokio::io::copy(&mut inner, &mut tokio::io::sink()).await?;
    Ok((head, dropped))
}

/// PERF-1 / TASK-0764: spawn `cmd` with piped stdio, stream both pipes through
/// `read_capped`, and assemble a `CommandOutput`. Replaces `cmd.output()` so
/// runaway children cannot peak the runner's RSS at the full output size — the
/// excess bytes are sinked, not buffered.
async fn spawn_capped(
    cmd: &mut tokio::process::Command,
    cap: usize,
) -> std::io::Result<CommandOutput> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    // ERR-5 / TASK-0906: tokio guarantees the handles are populated when
    // stdio is set to `piped` immediately above, but a future refactor
    // moving the stdio setup upward (or feeding partially-configured
    // commands in) would silently regress to a panic. Surface the
    // invariant as a typed io::Error so the existing
    // log_and_redact_spawn_error path catches it; debug_assert keeps the
    // invariant visible during development.
    debug_assert!(
        child.stdout.is_some() && child.stderr.is_some(),
        "stdio must be piped before spawn"
    );
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| std::io::Error::other("stdout pipe missing after spawn"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| std::io::Error::other("stderr pipe missing after spawn"))?;
    // CONC-9 / TASK-1064: own the drain tasks in a local `JoinSet` rather than
    // bare `tokio::spawn`. If the surrounding parallel task is aborted (e.g.
    // `JoinSet::abort_all` after a fail_fast trip while the child is wedged
    // and `child.wait()` is parked), `JoinSet::drop` aborts these readers so
    // they cannot keep draining the pipes after the parent has been
    // cancelled. The `AbortOnDropHandle`-via-JoinSet pattern matches what
    // `spawn_event_forwarder` does for the same reason.
    let mut drains: tokio::task::JoinSet<std::io::Result<(Vec<u8>, u64)>> =
        tokio::task::JoinSet::new();
    let stdout_handle = drains.spawn(read_capped(stdout, cap));
    let stderr_handle = drains.spawn(read_capped(stderr, cap));
    let status = child.wait().await?;
    let join_to_io = |e: tokio::task::JoinError| std::io::Error::other(e);
    // Await the specific handles by id so we keep the per-stream result
    // mapping; `JoinSet::join_next` would yield in completion order.
    type DrainOutcome = Result<std::io::Result<(Vec<u8>, u64)>, tokio::task::JoinError>;
    let mut stdout_result: Option<DrainOutcome> = None;
    let mut stderr_result: Option<DrainOutcome> = None;
    while stdout_result.is_none() || stderr_result.is_none() {
        match drains.join_next_with_id().await {
            Some(Ok((id, val))) if id == stdout_handle.id() => stdout_result = Some(Ok(val)),
            Some(Ok((id, val))) if id == stderr_handle.id() => stderr_result = Some(Ok(val)),
            Some(Ok(_)) => unreachable!("only stdout/stderr drains spawned"),
            Some(Err(e)) if e.id() == stdout_handle.id() => stdout_result = Some(Err(e)),
            Some(Err(e)) if e.id() == stderr_handle.id() => stderr_result = Some(Err(e)),
            Some(Err(_)) => unreachable!("only stdout/stderr drains spawned"),
            None => unreachable!("two drains spawned, two results expected"),
        }
    }
    let (stdout_bytes, stdout_dropped) = stdout_result
        .expect("stdout drain awaited")
        .map_err(join_to_io)??;
    let (stderr_bytes, stderr_dropped) = stderr_result
        .expect("stderr drain awaited")
        .map_err(join_to_io)??;
    Ok(CommandOutput::from_streamed(
        status,
        stdout_bytes,
        stdout_dropped,
        stderr_bytes,
        stderr_dropped,
    ))
}

/// PERF-1 / TASK-0764 test shim: expose `spawn_capped` for the streaming-cap
/// regression test.
#[cfg(test)]
pub async fn spawn_capped_for_test(
    cmd: &mut tokio::process::Command,
    cap: usize,
) -> std::io::Result<CommandOutput> {
    spawn_capped(cmd, cap).await
}

/// Execute a command with an optional timeout, capturing its output.
#[cfg(test)]
pub async fn execute_with_timeout(
    mut cmd: Command,
    timeout: Option<Duration>,
) -> Result<std::process::Output, std::io::Error> {
    await_with_timeout(cmd.output(), timeout).await
}

/// Render a spawn failure without leaking the resolved absolute path.
///
/// Uses the bare program name from the spec plus the textual `ErrorKind`
/// (e.g. `NotFound`, `PermissionDenied`) rather than `io::Error::to_string`,
/// which embeds system-specific strings including the full resolved path.
/// Timeouts retain their longer descriptive message because the timeout
/// formatter already strips path info.
fn redact_spawn_error(program: &str, e: &std::io::Error) -> String {
    if e.kind() == std::io::ErrorKind::TimedOut {
        return e.to_string();
    }
    format!("failed to spawn `{program}`: {kind:?}", kind = e.kind())
}

/// DUP-3 / TASK-0305: log the full spawn error at debug (for operators
/// chasing SEC-22 leaks) and return the redacted user-facing message.
///
/// Both `exec_command` and `exec_command_raw` need exactly this pair on
/// every spawn failure; centralising it removes the drift risk if redaction
/// fields evolve. `context` is included as a tracing field so the two call
/// sites remain distinguishable in logs ("captured" vs "raw").
fn log_and_redact_spawn_error(program: &str, e: &std::io::Error, context: &'static str) -> String {
    tracing::debug!(error = %e, program = %program, context, "exec spawn failed (full error)");
    redact_spawn_error(program, e)
}

/// Emit StepOutput events for captured stdout and stderr.
///
/// PERF-3 / TASK-0732: each capture buffer is wrapped in a single
/// `Arc<str>` and per-line events carry an [`OutputLine`] view onto a byte
/// sub-range of that shared buffer. A noisy step that previously paid one
/// heap allocation per line via `line.to_string()` now pays one per buffer;
/// the per-line event emission is just an `Arc::clone` (atomic refcount
/// increment).
///
/// PERF-3 / TASK-0838 — allocation accounting (be explicit, not aspirational):
/// `Arc::<str>::from(&str)` allocates a fresh `Arc<str>` *and* memcpys the
/// `&str` contents into it. Per stream that is one allocation + one
/// `output.len()` byte copy on top of the existing `CommandOutput.{stdout,
/// stderr}: String` buffers. We do *not* claim zero-copy here: the source
/// `String`s are subsequently moved into `StepResult` by [`build_step_result`]
/// (`StepResult.stdout: String` is part of the public runner API), so we
/// cannot consume them in this function without forcing a re-`String`
/// conversion at the consumer side. The trade is "one buffer-copy per
/// stream" against "one heap allocation per emitted line"; on the noisy
/// 4 MiB-cap worst case the per-stream copy is bounded by
/// `OPS_OUTPUT_BYTE_CAP` and dominates over per-line allocation savings
/// already at a few hundred lines.
pub fn emit_output_events(
    id: &str,
    stdout: &str,
    stderr: &str,
    emit: &mut impl FnMut(RunnerEvent),
) {
    for (output, is_stderr) in [(stdout, false), (stderr, true)] {
        if output.is_empty() {
            continue;
        }
        let buf: std::sync::Arc<str> = std::sync::Arc::from(output);
        let mut start = 0usize;
        let bytes = buf.as_bytes();
        while start < bytes.len() {
            let rel = bytes[start..].iter().position(|b| *b == b'\n');
            let (line_end, next_start) = match rel {
                Some(off) => {
                    let end = start + off;
                    // Mirror `str::lines` and strip an optional preceding `\r`.
                    let trimmed_end = if end > start && bytes[end - 1] == b'\r' {
                        end - 1
                    } else {
                        end
                    };
                    (trimmed_end, end + 1)
                }
                None => (bytes.len(), bytes.len()),
            };
            emit(RunnerEvent::StepOutput {
                id: id.into(),
                line: crate::command::OutputLine::slice(
                    std::sync::Arc::clone(&buf),
                    start..line_end,
                ),
                stderr: is_stderr,
            });
            start = next_start;
        }
    }
}

/// Emit final step event (StepFinished or StepFailed) based on success.
pub fn emit_step_completion(
    id: &str,
    duration: Duration,
    output: &CommandOutput,
    display_cmd: Option<String>,
    emit: &mut impl FnMut(RunnerEvent),
) {
    if output.success {
        emit(RunnerEvent::StepFinished {
            id: id.into(),
            duration_secs: duration.as_secs_f64(),
            display_cmd,
        });
    } else {
        emit(RunnerEvent::StepFailed {
            id: id.into(),
            duration_secs: duration.as_secs_f64(),
            message: output.status_message.clone(),
            display_cmd,
        });
    }
}

/// Build StepResult from command output.
pub fn build_step_result(id: &str, duration: Duration, output: CommandOutput) -> StepResult {
    StepResult {
        id: id.into(),
        success: output.success,
        duration,
        stdout: output.stdout,
        stderr: output.stderr,
        message: if output.success {
            None
        } else {
            Some(output.status_message)
        },
    }
}

/// ASYNC-6 / TASK-0159: no pre-spawn retries.
///
/// Transient spawn failures (EAGAIN under fork load, temporary PATH
/// resolution hiccups, NFS `current_dir` hiccups) are reported directly
/// without retry. The decision is intentional and the reasoning is:
///
/// - `exec_command` already wraps `cmd.output()` in `run_with_timeout`;
///   users who want retries can configure a wrapping composite step.
/// - Retries carry their own failure modes: a `Command` that has begun
///   spawning may be half-executed on the OS side (mkdir/chmod/write
///   commands are very much not idempotent at the exec level). The
///   boundary between "pre-spawn" and "post-spawn" is not visible from
///   outside the tokio runtime, so we cannot safely distinguish.
/// - The existing error message already surfaces the underlying
///   `io::ErrorKind` via `SEC-22` redaction, so users can opt in to
///   external retry logic at the CI level where context is richer.
///
/// Revisit if CI flakiness metrics ever point to transient spawn errors as
/// the dominant cause of `ops run` failures.
///
/// Core command execution: build, run, collect output, emit events, return result.
#[allow(clippy::too_many_arguments)]
pub async fn exec_command(
    id: &str,
    spec: &ExecCommandSpec,
    cwd: &Arc<PathBuf>,
    vars: &Arc<Variables>,
    policy: CwdEscapePolicy,
    emit: &mut impl FnMut(RunnerEvent),
) -> StepResult {
    let display_cmd = Some(spec.display_cmd().into_owned());
    emit(RunnerEvent::StepStarted {
        id: id.into(),
        display_cmd: display_cmd.clone(),
    });

    // CONC-5 / TASK-0330: build_command performs sync std::fs::canonicalize.
    // Run it on the blocking pool so we don't stall a tokio worker per
    // spawn. The clones below are cheap relative to the process spawn itself.
    let mut cmd =
        match build_command_async(spec.clone(), Arc::clone(cwd), Arc::clone(vars), policy).await {
            Ok(c) => c,
            Err(e) => {
                // ERR-1 / TASK-0450: variable expansion or cwd-policy failure.
                // Surface as a StepFailed so non-UTF-8 env vars and similar
                // configuration errors are user-visible instead of materialising
                // a literal `${VAR}` into argv / cwd.
                let msg = log_and_redact_spawn_error(&spec.program, &e, "captured");
                emit(RunnerEvent::StepFailed {
                    id: id.into(),
                    duration_secs: 0.0,
                    message: msg.clone(),
                    display_cmd,
                });
                return StepResult::failure(id, std::time::Duration::ZERO, msg);
            }
        };
    let cap = super::results::output_byte_cap();
    let (result, duration) = run_with_timeout(spawn_capped(&mut cmd, cap), spec.timeout()).await;
    let output = match result {
        Ok(o) => o,
        Err(e) => {
            // SEC-22: `io::Error::to_string()` on a spawn failure embeds the
            // resolved absolute program path and cwd (e.g. `/home/alice/…`).
            // That surfaces in `StepFailed::message` → progress UI → TAP
            // file, which leaks the developer's home path into CI logs.
            // log_and_redact_spawn_error keeps the full error at debug
            // level and returns a shorter "failed to spawn `<program>`:
            // <kind>" for the user.
            let msg = log_and_redact_spawn_error(&spec.program, &e, "captured");
            emit(RunnerEvent::StepFailed {
                id: id.into(),
                duration_secs: duration.as_secs_f64(),
                message: msg.clone(),
                display_cmd,
            });
            return StepResult::failure(id, duration, msg);
        }
    };

    emit_output_events(id, &output.stdout, &output.stderr, emit);
    emit_step_completion(id, duration, &output, display_cmd, emit);
    build_step_result(id, duration, output)
}

/// Raw command execution: inherits child stdio directly to the terminal.
///
/// Unlike [`exec_command`], this does not capture stdout/stderr — the child
/// process writes straight to the parent's fd 1/2. No `RunnerEvent`s are
/// emitted and the returned `StepResult` has empty stdout/stderr.
///
/// Exit code and timeout behavior are preserved. Used by `--raw` mode.
pub async fn exec_command_raw(
    id: &str,
    spec: &ExecCommandSpec,
    cwd: &Arc<PathBuf>,
    vars: &Arc<Variables>,
    policy: CwdEscapePolicy,
) -> StepResult {
    // CONC-5 / TASK-0330: see exec_command above.
    let mut cmd =
        match build_command_async(spec.clone(), Arc::clone(cwd), Arc::clone(vars), policy).await {
            Ok(c) => c,
            Err(e) => {
                // ERR-1 / TASK-0450: surface expansion / policy failures rather
                // than panic. Raw mode has no event stream, so we just return.
                return StepResult::failure(
                    id,
                    std::time::Duration::ZERO,
                    log_and_redact_spawn_error(&spec.program, &e, "raw"),
                );
            }
        };
    cmd.stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit());

    let (status_result, duration) = run_with_timeout(cmd.status(), spec.timeout()).await;

    match status_result {
        Ok(status) => {
            if status.success() {
                StepResult {
                    id: id.into(),
                    success: true,
                    duration,
                    stdout: String::new(),
                    stderr: String::new(),
                    message: None,
                }
            } else {
                StepResult::failure(id, duration, status.to_string())
            }
        }
        Err(e) => {
            // SEC-22: same log+redact as `exec_command`, via the shared helper.
            StepResult::failure(
                id,
                duration,
                log_and_redact_spawn_error(&spec.program, &e, "raw"),
            )
        }
    }
}

/// FN-9 / TASK-0778: shared infrastructure passed to every parallel task.
///
/// Groups the four runner-scoped handles (`cwd`, `vars`, the outbound event
/// `tx`, and the `abort` signal) so each parallel spawn site clones the bag
/// once via `Clone` rather than threading four positional arguments through
/// `spawn_parallel_tasks`. The struct uses `Arc`/`Sender` semantics so
/// cloning is a refcount bump per field — the parallel hot path retains the
/// allocation profile that TASK-0462 established.
#[derive(Clone)]
pub struct ExecTaskCtx {
    pub cwd: Arc<PathBuf>,
    pub vars: Arc<Variables>,
    pub tx: mpsc::Sender<RunnerEvent>,
    pub abort: Arc<AbortSignal>,
    /// SEC-14 / TASK-0886: cwd-escape policy threaded down from
    /// `CommandRunner` so parallel tasks share the same fail-closed
    /// guarantee that the sequential path applies via `exec_command`.
    pub policy: CwdEscapePolicy,
}

/// CONC-3 / CONC-6 / CONC-9: spawn the per-task event forwarder.
///
/// Drains `local_rx` into the outer `RunnerEvent` channel, racing each
/// `outer.send(..)` against `abort.cancelled()` so a stuck display pump
/// cannot keep the forwarder alive after fail_fast tripped. Returned in a
/// `JoinSet` so its Drop aborts the forwarder if the parent task is
/// cancelled mid-flight (a bare `tokio::spawn` JoinHandle would not).
fn spawn_event_forwarder(
    mut local_rx: mpsc::Receiver<RunnerEvent>,
    outer: mpsc::Sender<RunnerEvent>,
    abort: Arc<AbortSignal>,
) -> tokio::task::JoinSet<()> {
    let mut forwarders = tokio::task::JoinSet::new();
    forwarders.spawn(async move {
        while let Some(ev) = local_rx.recv().await {
            tokio::select! {
                biased;
                send_result = outer.send(ev) => {
                    if send_result.is_err() {
                        break;
                    }
                }
                () = abort.cancelled() => {
                    break;
                }
            }
        }
    });
    forwarders
}

/// CONC-9 / TASK-0459+0571: forward a terminal event (StepFinished /
/// StepFailed / StepSkipped) on `tx`, dropping it if abort fires first so
/// fail_fast can stop a sibling task without blocking on a full bounded
/// channel.
async fn forward_terminal_event_or_drop(
    tx: &mpsc::Sender<RunnerEvent>,
    ev: RunnerEvent,
    abort: &AbortSignal,
    id: &CommandId,
) {
    tokio::select! {
        biased;
        _ = tx.send(ev) => {}
        () = abort.cancelled() => {
            tracing::debug!(
                id = %id,
                "CONC-9: dropping terminal event under abort to avoid blocking on full outer channel"
            );
        }
    }
}

/// Standalone exec used by parallel plan: runs one command, sends events via channel, respects abort flag.
pub async fn exec_standalone(id: CommandId, spec: ExecCommandSpec, ctx: ExecTaskCtx) -> StepResult {
    let ExecTaskCtx {
        cwd,
        vars,
        tx,
        abort,
        policy,
    } = ctx;
    if abort.is_set() {
        // ERR-1 / TASK-0408: this branch fires only when fail_fast already
        // tripped the abort flag — i.e. a sibling task failed. Use
        // `StepResult::cancelled` (success=false) instead of `skipped`
        // (success=true) so plan-success aggregation cannot silently treat
        // cancellation as a clean skip if the failing sibling is ever
        // filtered out of the results vector. The emitted display event is
        // still `StepSkipped` so the row renders identically — the
        // distinction is internal to the result type.
        let display_cmd = Some(spec.display_cmd().into_owned());
        let _ = tx
            .send(RunnerEvent::StepSkipped {
                id: id.clone(),
                display_cmd,
            })
            .await;
        return StepResult::cancelled(id);
    }
    // CONC-3: forward events through a per-task mpsc and a spawned
    // forwarder that owns the real backpressure against the global bounded
    // channel. The `exec_command` callback is synchronous `FnMut`, so we
    // cannot `await tx.send(…)` directly — `try_send` into a local buffer
    // keeps the hot path non-blocking, while the forwarder awaits on the
    // outer sender so the runner's global capacity actually governs
    // memory use. On pathological channel-full bursts events are dropped
    // with a debug log instead of silently ballooning memory.
    const LOCAL_BUF: usize = 256;
    let (local_tx, local_rx) = mpsc::channel::<RunnerEvent>(LOCAL_BUF);
    let mut forwarders = spawn_event_forwarder(local_rx, tx.clone(), Arc::clone(&abort));
    // CONC-7: terminal events (StepFinished/StepFailed/StepSkipped) bypass the
    // bounded local buffer entirely. Noisy commands (e.g. `cargo test
    // --all-features` compiling hundreds of crates) emit a StepOutput per
    // stderr line, easily overflowing the 256-slot buffer. When that happens
    // try_send drops events — and if the *terminal* event lands on a full
    // buffer the display never sees the step complete, leaving its progress
    // bar orphaned. We capture the terminal event here and forward it via the
    // outer channel after exec_command returns, so backpressure (await) gates
    // delivery instead of silently discarding it.
    let mut terminal: Option<RunnerEvent> = None;
    // CONC-7 / TASK-0457: count buffer-full drops per task so the display
    // can surface them instead of silently losing the stdout/stderr lines
    // that explain a failure.
    let mut dropped_outputs: u64 = 0;
    let result = exec_command(&id, &spec, &cwd, &vars, policy, &mut |ev| {
        // OWN-2 / TASK-0462: cwd/vars are already Arcs in this scope; the
        // `&Arc<…>` ref forwards through exec_command → build_command_async
        // without a deep clone.
        if matches!(
            ev,
            RunnerEvent::StepFinished { .. }
                | RunnerEvent::StepFailed { .. }
                | RunnerEvent::StepSkipped { .. }
        ) {
            terminal = Some(ev);
            return;
        }
        if let Err(mpsc::error::TrySendError::Full(_)) = local_tx.try_send(ev) {
            dropped_outputs = dropped_outputs.saturating_add(1);
            tracing::debug!("per-task event buffer full; dropping event under backpressure");
        }
    })
    .await;
    drop(local_tx);
    // Drain the forwarder. JoinSet drops the JoinHandle on completion; if we
    // are cancelled before reaching this point, the JoinSet's own Drop will
    // abort the forwarder so it cannot outlive the parent task.
    while forwarders.join_next().await.is_some() {}
    // CONC-7 / TASK-0457: surface the dropped count via the outer channel
    // so the display renders "(N output lines dropped under load)" next
    // to the step result. Awaited send so the count itself can never be
    // silently dropped.
    if dropped_outputs > 0 {
        let _ = tx
            .send(RunnerEvent::StepOutputDropped {
                id: id.clone(),
                dropped_count: dropped_outputs,
            })
            .await;
    }
    if let Some(ev) = terminal {
        forward_terminal_event_or_drop(&tx, ev, &abort, &id).await;
    }
    result
}

/// Emit a zero-duration StepFailed event for resolution errors (unknown or composite-in-leaf).
pub fn emit_instant_failure(id: &str, message: &str, on_event: &mut impl FnMut(RunnerEvent)) {
    on_event(RunnerEvent::StepFailed {
        id: id.into(),
        duration_secs: 0.0,
        message: message.to_string(),
        display_cmd: None,
    });
}

/// Emit failure event and return a StepResult for resolution errors (unknown command or composite in leaf list).
pub fn resolution_failure(
    id: &str,
    message: String,
    on_event: &mut impl FnMut(RunnerEvent),
) -> StepResult {
    emit_instant_failure(id, &message, on_event);
    StepResult::failure(id, Duration::ZERO, message)
}
