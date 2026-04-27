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

use super::build::build_command_async;
use super::events::RunnerEvent;
use super::results::{CommandOutput, StepResult};
use ops_core::config::{CommandId, ExecCommandSpec};
use ops_core::expand::Variables;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
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
pub fn emit_output_events(
    id: &str,
    stdout: &str,
    stderr: &str,
    emit: &mut impl FnMut(RunnerEvent),
) {
    for (output, is_stderr) in [(stdout, false), (stderr, true)] {
        for line in output.lines() {
            emit(RunnerEvent::StepOutput {
                id: id.into(),
                line: line.to_string(),
                stderr: is_stderr,
            });
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
pub async fn exec_command(
    id: &str,
    spec: &ExecCommandSpec,
    cwd: &std::path::Path,
    vars: &Variables,
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
    let mut cmd = build_command_async(spec.clone(), cwd.to_path_buf(), vars.clone()).await;
    let (result, duration) = run_with_timeout(cmd.output(), spec.timeout()).await;
    let output = match result {
        Ok(o) => CommandOutput::from_raw(o),
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
    cwd: &std::path::Path,
    vars: &Variables,
) -> StepResult {
    // CONC-5 / TASK-0330: see exec_command above.
    let mut cmd = build_command_async(spec.clone(), cwd.to_path_buf(), vars.clone()).await;
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

/// Standalone exec used by parallel plan: runs one command, sends events via channel, respects abort flag.
#[allow(clippy::too_many_arguments)]
pub async fn exec_standalone(
    id: CommandId,
    spec: ExecCommandSpec,
    cwd: Arc<PathBuf>,
    vars: Arc<Variables>,
    tx: mpsc::Sender<RunnerEvent>,
    abort: Arc<AtomicBool>,
) -> StepResult {
    if abort.load(Ordering::Acquire) {
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
    let (local_tx, mut local_rx) = mpsc::channel::<RunnerEvent>(LOCAL_BUF);
    let outer = tx.clone();
    // CONC-6 / TASK-0335: hold the forwarder in a JoinSet rather than a bare
    // JoinHandle so that if `exec_standalone` is aborted mid-flight (e.g.
    // fail_fast `abort_all`), dropping the JoinSet aborts the forwarder
    // explicitly. A bare `tokio::spawn` returns a JoinHandle whose Drop does
    // **not** cancel the task — the forwarder would survive its parent and
    // exit only because `local_tx` is also dropped (closing the channel).
    // That implicit lifetime is brittle to refactoring; making cancellation
    // explicit removes the trap.
    let mut forwarders = tokio::task::JoinSet::new();
    forwarders.spawn(async move {
        while let Some(ev) = local_rx.recv().await {
            if outer.send(ev).await.is_err() {
                break;
            }
        }
    });
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
    let result = exec_command(&id, &spec, &cwd, &vars, &mut |ev| {
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
            tracing::debug!("per-task event buffer full; dropping event under backpressure");
        }
    })
    .await;
    drop(local_tx);
    // Drain the forwarder. JoinSet drops the JoinHandle on completion; if we
    // are cancelled before reaching this point, the JoinSet's own Drop will
    // abort the forwarder so it cannot outlive the parent task.
    while forwarders.join_next().await.is_some() {}
    if let Some(ev) = terminal {
        let _ = tx.send(ev).await;
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
