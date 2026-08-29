//! Async command execution: running a built [`Command`], capturing output,
//! emitting [`RunnerEvent`]s, and applying timeouts.
//!
//! # Stdin: captured steps are non-interactive (ASYNC-6 / TASK-1918)
//!
//! Captured steps ([`exec_command`], via `spawn_capped`) are spawned with
//! `stdin` set to [`Stdio::null()`]. A captured child therefore reads EOF
//! immediately and can never block waiting for terminal input it has no way
//! of prompting for — its prompt would go to the captured pipe, and under
//! `run_plan_parallel` up to `OPS_MAX_PARALLEL` children would otherwise race
//! each other for the same fd 0.
//!
//! **Interactive commands must run in `--raw` mode**
//! ([`exec_command_raw`]), which inherits all three stdio slots by design.
//! A step that needs a credential, a passphrase, or a confirmation prompt
//! (`git push` over HTTPS, `docker login`, `cargo publish`) belongs there.
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
use super::build::{build_command_async, CwdEscapePolicy, WorkspaceCanonicalCache};
use super::events::RunnerEvent;
use super::process_group::{configure_process_group, ChildGroup};
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
    // Both arms consume `future` and `.await` it. `map_or_else` would need two
    // closures that each move the same future, and a non-async closure cannot
    // await it at all.
    #[allow(clippy::option_if_let_else)]
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
    // `usize` is at most 64 bits on every target this builds for, so the
    // conversion is exact and the fallback is unreachable; `u64::MAX` is
    // nonetheless the right saturation value here — it is `take`'s "no
    // additional limit", which is what a cap wider than u64 would mean.
    let mut limited = reader.take(u64::try_from(cap).unwrap_or(u64::MAX));
    limited.read_to_end(&mut head).await?;
    let mut inner = limited.into_inner();
    let dropped = tokio::io::copy(&mut inner, &mut tokio::io::sink()).await?;
    Ok((head, dropped))
}

/// Outcome of one drain task: the join result wrapping the read result.
type DrainOutcome = Result<std::io::Result<(Vec<u8>, u64)>, tokio::task::JoinError>;

/// CONC-9 / TASK-1919: how long the post-exit pipe drain may run before the
/// step is assumed to be held open by an orphan.
///
/// `child.wait()` returning does **not** close the capture pipes: any
/// grandchild that inherited the write end (a daemonised watcher, a
/// backgrounded `&` job inside an `sh -c` step, a lingering `rustc`) keeps
/// them open, and `read_capped` runs to EOF with no bound of its own. Before
/// this deadline existed, such a step never completed and — with
/// `timeout_secs` unset, which is the default — hung the entire plan on a
/// child that had already exited.
///
/// Generous on purpose: a healthy step drains in microseconds, so this is
/// only ever paid by a step that is already misbehaving.
///
/// CONC-9 / TASK-2022: this deadline can now terminate a descendant
/// process, which makes it the highest-consequence default in the module —
/// a step that deliberately leaves a grandchild streaming to the inherited
/// pipe has its capture truncated and that grandchild killed. Operators
/// whose workload legitimately needs longer (or who want the deadline
/// tighter on a heavily loaded CI runner where a slow-but-progressing
/// drain could reach 5s) override it through
/// [`DRAIN_GRACE_ENV`] via [`post_exit_drain_grace`], using the same
/// parse / clamp / warn-on-fallback contract as `OPS_OUTPUT_BYTE_CAP` and
/// `OPS_MAX_PARALLEL`.
const DEFAULT_POST_EXIT_DRAIN_GRACE_SECS: u64 = 5;

/// Operator override for [`DEFAULT_POST_EXIT_DRAIN_GRACE_SECS`], in whole
/// seconds. Unset or empty means the default; `0`, an unparseable value, or
/// a value above [`MAX_POST_EXIT_DRAIN_GRACE_SECS`] falls back / clamps
/// with a `tracing::warn!`, exactly like `OPS_MAX_PARALLEL`.
const DRAIN_GRACE_ENV: &str = "OPS_OUTPUT_DRAIN_GRACE_SECS";

/// Ceiling for [`DRAIN_GRACE_ENV`] (one hour). A bounded drain is the whole
/// point of the deadline, so an operator cannot restore the unbounded
/// pre-TASK-1919 hang by setting an arbitrarily large value.
const MAX_POST_EXIT_DRAIN_GRACE_SECS: u64 = 3600;

/// PERF-3: resolved once per process, mirroring `output_byte_cap`'s
/// `OnceLock` contract — the value is process-global and constant for a
/// run, and the drain path runs under every parallel step.
static POST_EXIT_DRAIN_GRACE: std::sync::OnceLock<Duration> = std::sync::OnceLock::new();

/// The `(default, ceiling)` pair the shared resolver expects, in `usize`.
/// Both constants are small literals, so the conversion is infallible on
/// every supported target; `unwrap_or(usize::MAX)` keeps it total without an
/// `as` cast (see `docs/clippy.md` on `as_conversions`).
fn drain_grace_bounds() -> (usize, usize) {
    (
        usize::try_from(DEFAULT_POST_EXIT_DRAIN_GRACE_SECS).unwrap_or(usize::MAX),
        usize::try_from(MAX_POST_EXIT_DRAIN_GRACE_SECS).unwrap_or(usize::MAX),
    )
}

/// Resolve the post-exit drain deadline, honouring [`DRAIN_GRACE_ENV`].
fn post_exit_drain_grace() -> Duration {
    *POST_EXIT_DRAIN_GRACE.get_or_init(|| {
        let (default, ceiling) = drain_grace_bounds();
        let secs = super::parallel::resolve_env_usize(DRAIN_GRACE_ENV, default, ceiling);
        // The resolver clamps to `ceiling`, so this conversion cannot fail.
        Duration::from_secs(u64::try_from(secs).unwrap_or(MAX_POST_EXIT_DRAIN_GRACE_SECS))
    })
}

/// Second, short deadline applied after the orphan holding the pipes has
/// been `SIGKILL`ed: the read side should observe EOF essentially at once.
///
/// CONC-9 / TASK-2022: deliberately *not* operator-tunable, unlike
/// [`DEFAULT_POST_EXIT_DRAIN_GRACE_SECS`]. This is not a wait on a workload
/// — the process holding the write end has already been `SIGKILL`ed, so the
/// only thing outstanding is the kernel closing its descriptors. There is no
/// legitimate configuration in which that needs longer, and a knob here
/// would only offer a way to lengthen the already-failed path.
const POST_KILL_DRAIN_GRACE: Duration = Duration::from_secs(2);

/// Await both drain tasks by id, recording each into its own slot.
///
/// Split out of [`spawn_capped`] so the collection can be *resumed*: the
/// first call runs under [`post_exit_drain_grace`] and, if that expires, the
/// second call resumes with whichever stream already returned instead of
/// restarting from nothing.
async fn collect_drain_results(
    drains: &mut tokio::task::JoinSet<std::io::Result<(Vec<u8>, u64)>>,
    stdout_id: tokio::task::Id,
    stderr_id: tokio::task::Id,
    stdout_result: &mut Option<DrainOutcome>,
    stderr_result: &mut Option<DrainOutcome>,
) -> std::io::Result<()> {
    // ERR-5 / TASK-1139: encode "exactly two drains" as a logged io::Error
    // rather than `unreachable!` / `expect`, so a refactor adding a third
    // drain (e.g. stdin watchdog) surfaces as a StepFailed event instead of
    // a panic-with-payload that SEC-21 / TASK-0334 redaction does not cover
    // (that hardening only wraps the outer parallel JoinSet, not this inner
    // one).
    while stdout_result.is_none() || stderr_result.is_none() {
        match drains.join_next_with_id().await {
            Some(Ok((id, val))) if id == stdout_id => *stdout_result = Some(Ok(val)),
            Some(Ok((id, val))) if id == stderr_id => *stderr_result = Some(Ok(val)),
            Some(Err(e)) if e.id() == stdout_id => *stdout_result = Some(Err(e)),
            Some(Err(e)) if e.id() == stderr_id => *stderr_result = Some(Err(e)),
            // Any join result whose id is neither drain.
            Some(_) => {
                tracing::error!("unexpected drain id from spawn_capped JoinSet");
                return Err(std::io::Error::other("spawn_capped: unexpected drain id"));
            }
            None => return Err(missing_drain()),
        }
    }
    Ok(())
}

/// The drain `JoinSet` ran dry before both readers reported.
fn missing_drain() -> std::io::Error {
    tracing::error!("spawn_capped JoinSet drained without yielding both readers");
    std::io::Error::other("spawn_capped: drain JoinSet exhausted before stdout/stderr returned")
}

/// PERF-1 / TASK-0764: spawn `cmd` with piped stdio, stream both pipes through
/// `read_capped`, and assemble a `CommandOutput`. Replaces `cmd.output()` so
/// runaway children cannot peak the runner's RSS at the full output size — the
/// excess bytes are sinked, not buffered.
///
/// CONC-9 / TASK-1919: the child is also spawned as its own process-group
/// leader and owned by a [`ChildGroup`] guard, so a cancelled step (timeout
/// or `fail_fast`) tears down the child's whole descendant tree rather than
/// only its root, and the post-exit drain is bounded by
/// [`post_exit_drain_grace`] so an orphan holding the pipe cannot hang the
/// plan.
async fn spawn_capped(
    cmd: &mut tokio::process::Command,
    cap: usize,
) -> std::io::Result<CommandOutput> {
    // ASYNC-6 / TASK-1918: pin all three stdio slots. `tokio::process::Command`
    // defaults stdin to `Stdio::inherit()`, so without this a captured child
    // inherits the runner's fd 0 and an input-prompting program (`git` asking
    // for a credential, `sudo`, `ssh`, `npm login`) blocks on `read(0)` while
    // its prompt sits in the *captured* pipe — invisible to the user, and
    // unbounded because `timeout_secs` is opt-in. The captured contract is
    // "no interaction, we own the output", so `null` is the correct stdin:
    // the child sees immediate EOF and fails fast with its own diagnostic,
    // which lands in the captured stderr the display already renders.
    // Interactive steps must use `--raw` ([`exec_command_raw`]), which keeps
    // `Stdio::inherit()` deliberately.
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // CONC-9 / TASK-1919: make the child its own process-group leader so
    // cancellation can address the tree it forks, not just the leader.
    configure_process_group(cmd);
    let mut child = cmd.spawn()?;
    // Armed for the whole body: every early return and every drop of this
    // future (timeout, `JoinSet::abort_all`) now tears the group down.
    let mut group = ChildGroup::new(&child);
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
    let (stdout_id, stderr_id) = (stdout_handle.id(), stderr_handle.id());
    let mut stdout_result: Option<DrainOutcome> = None;
    let mut stderr_result: Option<DrainOutcome> = None;
    // CONC-9 / TASK-1919: bound the post-exit drain. The leader has exited,
    // so anything still holding the write end of these pipes is a
    // grandchild that outlived it; `read_capped` would otherwise wait for an
    // EOF that never comes.
    let drain_grace = post_exit_drain_grace();
    if tokio::time::timeout(
        drain_grace,
        collect_drain_results(
            &mut drains,
            stdout_id,
            stderr_id,
            &mut stdout_result,
            &mut stderr_result,
        ),
    )
    .await
    .is_err()
    {
        tracing::warn!(
            grace_secs = drain_grace.as_secs(),
            env = DRAIN_GRACE_ENV,
            "captured output still open {}s after the child exited; killing the process group",
            drain_grace.as_secs()
        );
        // Straight to SIGKILL: these are orphans of an already-exited
        // leader, there is nothing left to shut down gracefully, and the
        // step is blocked until they release the fd.
        group.kill_now();
        tokio::time::timeout(
            POST_KILL_DRAIN_GRACE,
            collect_drain_results(
                &mut drains,
                stdout_id,
                stderr_id,
                &mut stdout_result,
                &mut stderr_result,
            ),
        )
        .await
        .map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "captured output pipe held open after the child exited; step abandoned",
            )
        })??;
    }
    // Completed normally: a descendant that is still alive here was
    // deliberately backgrounded by the step and closed its pipes, so it is
    // not the runner's to signal.
    group.disarm();
    let (stdout_bytes, stdout_dropped) = stdout_result
        .ok_or_else(missing_drain)?
        .map_err(join_to_io)??;
    let (stderr_bytes, stderr_dropped) = stderr_result
        .ok_or_else(missing_drain)?
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
    // SEC-21 / TASK-1127: format `program` via Debug so newlines/ANSI escape sequences
    // smuggled through `.ops.toml`-supplied program names cannot forge log entries.
    tracing::debug!(error = %e, program = ?program, context, "exec spawn failed (full error)");
    redact_spawn_error(program, e)
}

/// Emit `StepOutput` events for captured stdout and stderr.
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
    stdout: &Arc<str>,
    stderr: &Arc<str>,
    emit: &mut impl FnMut(RunnerEvent),
) {
    // PERF-3 / TASK-1136: callers wrap the captured String into `Arc<str>`
    // exactly once and pass the handle in. Per-line events Arc::clone the
    // shared buffer, never re-allocating. The previous signature took
    // `&str` and re-wrapped on every call, which (in addition to the
    // already-existing per-stream allocation) discouraged callers from
    // reusing the same Arc across multiple call sites.
    for (buf, is_stderr) in [(stdout, false), (stderr, true)] {
        if buf.is_empty() {
            continue;
        }
        let mut start = 0usize;
        let bytes = buf.as_bytes();
        while start < bytes.len() {
            // `start < bytes.len()` holds by the loop guard, so the tail
            // slice always exists; stop scanning rather than panicking if
            // that ever stops being true.
            let Some(rest) = bytes.get(start..) else {
                break;
            };
            let rel = rest.iter().position(|b| *b == b'\n');
            let (line_end, next_start) = rel.map_or((bytes.len(), bytes.len()), |off| {
                // `off` indexes into `bytes[start..]`, so `start + off` is a
                // valid index into `bytes` and the saturating forms below are
                // exactly equal to `+`/`-` here: the sum is `< bytes.len()`,
                // the `end > start` guard makes `end >= 1`, and `end` is a
                // newline index so `end + 1 <= bytes.len()`.
                let end = start.saturating_add(off);
                // Mirror `str::lines` and strip an optional preceding `\r`.
                let trimmed_end = if end > start
                    && bytes
                        .get(end.saturating_sub(1))
                        .is_some_and(|b| *b == b'\r')
                {
                    end.saturating_sub(1)
                } else {
                    end
                };
                (trimmed_end, end.saturating_add(1))
            });
            emit(RunnerEvent::StepOutput {
                id: id.into(),
                line: crate::command::OutputLine::slice(
                    std::sync::Arc::clone(buf),
                    start..line_end,
                ),
                stderr: is_stderr,
            });
            start = next_start;
        }
    }
}

/// Emit final step event (`StepFinished` or `StepFailed`) based on success.
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

/// Build `StepResult` from command output.
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
    spec: &Arc<ExecCommandSpec>,
    workspace_cache: &Arc<WorkspaceCanonicalCache>,
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
    // PERF-3 / TASK-1125: spec is now `Arc<ExecCommandSpec>` end-to-end;
    // only an atomic refcount bump per spawn, no deep clone of args/env.
    let mut cmd = match build_command_async(
        Arc::clone(workspace_cache),
        Arc::clone(spec),
        Arc::clone(cwd),
        Arc::clone(vars),
        policy,
    )
    .await
    {
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

    // PERF-3 / TASK-1136: wrap each captured stream into `Arc<str>` exactly
    // once for the per-line event fan-out. Per-line emission is then a cheap
    // `Arc::clone` (refcount bump), not a fresh `Arc::from(&str)` per call.
    // The original `output.stdout`/`output.stderr` `String`s remain owned
    // and are moved into `StepResult` by `build_step_result` below.
    let stdout_arc: Arc<str> = if output.stdout.is_empty() {
        Arc::from("")
    } else {
        Arc::from(output.stdout.as_str())
    };
    let stderr_arc: Arc<str> = if output.stderr.is_empty() {
        Arc::from("")
    } else {
        Arc::from(output.stderr.as_str())
    };
    emit_output_events(id, &stdout_arc, &stderr_arc, emit);
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
#[allow(clippy::too_many_arguments)]
pub async fn exec_command_raw(
    id: &str,
    spec: &Arc<ExecCommandSpec>,
    workspace_cache: &Arc<WorkspaceCanonicalCache>,
    cwd: &Arc<PathBuf>,
    vars: &Arc<Variables>,
    policy: CwdEscapePolicy,
) -> StepResult {
    // CONC-5 / TASK-0330: see exec_command above.
    // PERF-3 / TASK-1125: Arc::clone — no spec deep clone per spawn.
    let mut cmd = match build_command_async(
        Arc::clone(workspace_cache),
        Arc::clone(spec),
        Arc::clone(cwd),
        Arc::clone(vars),
        policy,
    )
    .await
    {
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
/// Groups the runner-scoped handles (`cwd`, `vars`, the outbound event
/// `tx`, the `abort` signal, the cwd-escape policy, and the workspace
/// canonicalize cache) so each parallel spawn site clones the bag once
/// via `Clone` rather than threading positional arguments through
/// `spawn_parallel_tasks`. The struct uses `Arc`/`Sender` semantics so
/// cloning is a refcount bump per field — the parallel hot path retains
/// the allocation profile that TASK-0462 established.
///
/// # Stability contract
///
/// API / TASK-1233: `ExecTaskCtx` is `#[non_exhaustive]` (matching the
/// runner's other public bag types like `RunnerEvent` and `StepResult`)
/// so adding a new runner-scoped handle (e.g. per-task telemetry, a
/// cancellation token, an execution budget) is *not* a `SemVer` break for
/// downstream embedders. Embedders MUST construct the struct via
/// [`ExecTaskCtx::new`] (or `..` syntax over a value the runner
/// provides) — the field-level public constructors stay public for
/// ergonomic in-place mutation but struct-literal construction outside
/// this crate is forbidden by `#[non_exhaustive]`.
#[derive(Clone)]
#[non_exhaustive]
pub struct ExecTaskCtx {
    pub cwd: Arc<PathBuf>,
    pub vars: Arc<Variables>,
    pub tx: mpsc::Sender<RunnerEvent>,
    pub abort: Arc<AbortSignal>,
    /// SEC-14 / TASK-0886: cwd-escape policy threaded down from
    /// `CommandRunner` so parallel tasks share the same fail-closed
    /// guarantee that the sequential path applies via `exec_command`.
    pub policy: CwdEscapePolicy,
    /// ARCH-9 / TASK-1126: runner-scoped workspace canonicalize cache so
    /// parallel spawns share the same cache that
    /// `CommandRunner::invalidate_workspace_cache` mutates. Previously the
    /// spawn path read a process-global static, which made the public
    /// invalidate API a no-op against the cache that decided escape outcomes.
    pub workspace_cache: Arc<WorkspaceCanonicalCache>,
}

#[allow(dead_code)]
impl ExecTaskCtx {
    /// Construct an [`ExecTaskCtx`] from its current required handles.
    /// API / TASK-1233: prefer this over struct-literal construction so
    /// future additive fields (per-task telemetry, cancellation token,
    /// execution budget) can land without churning every embedder's
    /// call site.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        cwd: Arc<PathBuf>,
        vars: Arc<Variables>,
        tx: mpsc::Sender<RunnerEvent>,
        abort: Arc<AbortSignal>,
        policy: CwdEscapePolicy,
        workspace_cache: Arc<WorkspaceCanonicalCache>,
    ) -> Self {
        Self {
            cwd,
            vars,
            tx,
            abort,
            policy,
            workspace_cache,
        }
    }
}

/// CONC-3 / CONC-6 / CONC-9: spawn the per-task event forwarder.
///
/// Drains `local_rx` into the outer `RunnerEvent` channel, racing each
/// `outer.send(..)` against `abort.cancelled()` so a stuck display pump
/// cannot keep the forwarder alive after `fail_fast` tripped. Returned in a
/// `JoinSet` so its Drop aborts the forwarder if the parent task is
/// cancelled mid-flight (a bare `tokio::spawn` `JoinHandle` would not).
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

/// CONC-9 / TASK-0459+0571: forward a terminal event (`StepFinished` /
/// `StepFailed` / `StepSkipped`) on `tx`, dropping it if abort fires first so
/// `fail_fast` can stop a sibling task without blocking on a full bounded
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
                id = ?id.as_str(),
                "CONC-9: dropping terminal event under abort to avoid blocking on full outer channel"
            );
        }
    }
}

/// Standalone exec used by parallel plan: runs one command, sends events via channel, respects abort flag.
pub async fn exec_standalone(
    id: CommandId,
    spec: Arc<ExecCommandSpec>,
    ctx: ExecTaskCtx,
) -> StepResult {
    let ExecTaskCtx {
        cwd,
        vars,
        tx,
        abort,
        policy,
        workspace_cache,
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
    // PERF-3 / TASK-1125: spec passed by &Arc; Arc::clone on the spawn path.
    let result = exec_command(
        &id,
        &spec,
        &workspace_cache,
        &cwd,
        &vars,
        policy,
        &mut |ev| {
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
        },
    )
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
    //
    // ERR-1 / TASK-1174: a closed receiver (display has already torn down,
    // e.g. fail_fast shutdown race) returns `Err(SendError)`. The whole
    // point of TASK-0457 is that this count never disappears, so log a
    // structured warning when the outer channel rejects the event — the
    // count survives in the log instead of being silently lost.
    if dropped_outputs > 0 {
        if let Err(mpsc::error::SendError(_)) = tx
            .send(RunnerEvent::StepOutputDropped {
                id: id.clone(),
                dropped_count: dropped_outputs,
            })
            .await
        {
            tracing::warn!(
                step_id = ?id.as_str(),
                dropped_count = dropped_outputs,
                "outer event channel closed; dropped-output count cannot be sent to display \
                 (recording in logs so the count survives)",
            );
        }
    }
    if let Some(ev) = terminal {
        forward_terminal_event_or_drop(&tx, ev, &abort, &id).await;
    }
    result
}

/// Emit a zero-duration `StepFailed` event for resolution errors (unknown or composite-in-leaf).
pub fn emit_instant_failure(id: &str, message: &str, on_event: &mut impl FnMut(RunnerEvent)) {
    on_event(RunnerEvent::StepFailed {
        id: id.into(),
        duration_secs: 0.0,
        message: message.to_string(),
        display_cmd: None,
    });
}

/// Emit failure event and return a `StepResult` for resolution errors (unknown command or composite in leaf list).
pub fn resolution_failure(
    id: &str,
    message: String,
    on_event: &mut impl FnMut(RunnerEvent),
) -> StepResult {
    emit_instant_failure(id, &message, on_event);
    StepResult::failure(id, Duration::ZERO, message)
}

#[cfg(test)]
mod drain_grace_knob_tests {
    use super::{drain_grace_bounds, DRAIN_GRACE_ENV};
    use crate::command::parallel::resolve_env_usize;

    /// Run `body` with `DRAIN_GRACE_ENV` set to `value` (or removed when
    /// `None`), restoring the previous value afterwards.
    fn with_env<T>(value: Option<&str>, body: impl FnOnce() -> T) -> T {
        let prev = std::env::var_os(DRAIN_GRACE_ENV);
        // SAFETY: these tests are serialised via `serial_test`, so no other
        // thread reads the environment mid-mutation.
        unsafe {
            match value {
                Some(v) => std::env::set_var(DRAIN_GRACE_ENV, v),
                None => std::env::remove_var(DRAIN_GRACE_ENV),
            }
        }
        let out = body();
        unsafe {
            match prev {
                Some(v) => std::env::set_var(DRAIN_GRACE_ENV, v),
                None => std::env::remove_var(DRAIN_GRACE_ENV),
            }
        }
        out
    }

    /// The resolver is the shared one, so the cached `post_exit_drain_grace`
    /// wrapper cannot be exercised twice in one process; drive
    /// `resolve_env_usize` directly, the way the `OPS_MAX_PARALLEL` tests do.
    fn resolve(value: Option<&str>) -> usize {
        let (default, ceiling) = drain_grace_bounds();
        with_env(value, || {
            resolve_env_usize(DRAIN_GRACE_ENV, default, ceiling)
        })
    }

    #[test]
    #[serial_test::serial(env_drain_grace)]
    fn unset_uses_the_default() {
        assert_eq!(resolve(None), drain_grace_bounds().0);
    }

    #[test]
    #[serial_test::serial(env_drain_grace)]
    fn a_valid_value_is_honoured() {
        assert_eq!(resolve(Some("30")), 30);
    }

    /// CONC-9 / TASK-2022 AC #2: an unparseable or zero value falls back to
    /// the default rather than silently disabling the deadline.
    #[test]
    #[serial_test::serial(env_drain_grace)]
    fn unusable_values_fall_back_to_the_default() {
        let default = drain_grace_bounds().0;
        assert_eq!(resolve(Some("later")), default);
        assert_eq!(resolve(Some("0")), default);
        assert_eq!(resolve(Some("")), default);
    }

    /// An operator cannot restore the unbounded pre-TASK-1919 hang.
    #[test]
    #[serial_test::serial(env_drain_grace)]
    fn out_of_range_clamps_to_the_ceiling() {
        assert_eq!(
            resolve(Some("999999")),
            drain_grace_bounds().1,
            "an arbitrarily large grace must clamp, not pass through"
        );
    }
}

#[cfg(test)]
mod spawn_error_log_format_tests {
    /// SEC-21 / TASK-1127: `log_and_redact_spawn_error` formats `program`
    /// (the raw `.ops.toml`-supplied program string) via the `?` (Debug)
    /// formatter so embedded newlines / ANSI escapes cannot forge log
    /// records or repaint operator terminals. Pin the value-level escape
    /// without requiring a tracing-subscriber dev-dep, mirroring
    /// `stderr_snippet_debug_escapes_control_characters` in tools/probe.rs.
    #[test]
    fn program_field_debug_escapes_control_characters() {
        let program = "evil\nFAKE_LOG_LINE\n\u{1b}[31mred\u{1b}[0m";
        let rendered = format!("{program:?}");
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
    }

    /// SEC-21 / TASK-2020: `.ops.toml` table keys and `aliases` entries
    /// become `CommandId`s and reach `tracing` fields across the crate.
    /// They are rendered with the `?` (Debug) formatter applied to
    /// `as_str()`, which escapes embedded newlines / ANSI escapes — so a
    /// crafted id cannot forge a log record or repaint the operator's
    /// terminal — while keeping the field shape a bare quoted string
    /// rather than `CommandId("...")`.
    #[test]
    fn command_id_field_debug_escapes_control_characters() {
        let id = ops_core::config::CommandId::from("evil\nFAKE_LOG_LINE\n\u{1b}[31mred\u{1b}[0m");

        let rendered = format!("{:?}", id.as_str());
        assert!(!rendered.contains('\n'), "got: {rendered}");
        assert!(!rendered.contains('\u{1b}'), "got: {rendered}");
        assert!(rendered.contains("\\n"), "got: {rendered}");
        assert!(
            rendered.starts_with('"') && rendered.ends_with('"'),
            "got: {rendered}"
        );
        assert!(!rendered.contains("CommandId"), "got: {rendered}");

        // The Display rendering this replaced would not have escaped.
        assert!(format!("{id}").contains('\n'));
    }
}
