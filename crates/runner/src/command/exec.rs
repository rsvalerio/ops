//! Command execution helpers: building, running, and handling output.
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
//! The `warn_if_sensitive_env_key()` function logs a warning when it detects
//! sensitive-looking variable names or values that appear to be secrets
//! (e.g., long base64-like strings, common secret formats).

use super::events::RunnerEvent;
use super::results::{CommandOutput, StepResult};
use ops_core::config::{CommandId, ExecCommandSpec};
use ops_core::expand::Variables;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::sync::mpsc;

/// Lexically normalize a path by resolving `.` and `..` components without I/O.
fn normalize_path(p: &std::path::Path) -> std::path::PathBuf {
    use std::path::Component;
    let mut out = std::path::PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Policy for how to treat spec `cwd` values that escape the workspace root.
///
/// SEC-14: interactive invocations (`ops <cmd>`) tolerate escapes with a
/// warning — `.ops.toml` is trusted the way a Makefile is trusted.
/// Hook-triggered invocations (`run-before-commit`, `run-before-push`) are
/// strict: a co-worker's PR can land a `.ops.toml` that runs on every
/// commit the maintainer makes, so the hook path fails closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CwdEscapePolicy {
    /// Log a warning and spawn anyway. Default for interactive `ops run`.
    #[default]
    WarnAndAllow,
    /// Refuse to spawn; return an error. Used by git-hook-triggered paths.
    ///
    /// Kept in the public API so hook-triggered entry points can opt in
    /// once they thread a policy through `CommandRunner`. Currently only
    /// constructed in tests; the default interactive path stays
    /// `WarnAndAllow` to avoid a behaviour change for existing users.
    #[allow(dead_code)]
    Deny,
}

/// Resolve an exec spec's `cwd` field against the workspace root, canonicalizing
/// both sides before the containment check so symlinks cannot smuggle an
/// absolute path past the check lexically.
///
/// Returns an error when the resolved path escapes the workspace root **and**
/// `policy == Deny` (SEC-14 hook path). Otherwise logs and continues.
pub fn resolve_spec_cwd(
    spec_cwd: Option<&std::path::Path>,
    workspace_cwd: &std::path::Path,
    vars: &Variables,
    policy: CwdEscapePolicy,
) -> Result<std::path::PathBuf, std::io::Error> {
    let Some(p) = spec_cwd else {
        return Ok(workspace_cwd.to_path_buf());
    };

    let lossy = p.to_string_lossy();
    let expanded = vars.expand(&lossy);
    let ep = std::path::PathBuf::from(expanded.as_ref());
    if !ep.is_relative() {
        return Ok(ep);
    }

    let joined = workspace_cwd.join(&ep);
    // Lexical check first (fast, no IO). Canonicalize both sides when the
    // joined path exists so a symlink inside the workspace that targets
    // outside is still caught.
    let lexically_escapes = !normalize_path(&joined).starts_with(workspace_cwd);
    let canonically_escapes = match (
        std::fs::canonicalize(&joined).ok(),
        std::fs::canonicalize(workspace_cwd).ok(),
    ) {
        (Some(a), Some(b)) => !a.starts_with(&b),
        _ => false,
    };

    if lexically_escapes || canonically_escapes {
        match policy {
            CwdEscapePolicy::Deny => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!(
                        "SEC-14: refusing to spawn: spec cwd {} escapes workspace root {}",
                        ep.display(),
                        workspace_cwd.display()
                    ),
                ));
            }
            CwdEscapePolicy::WarnAndAllow => {
                tracing::warn!(
                    cwd = %workspace_cwd.display(),
                    spec_cwd = %ep.display(),
                    resolved = %joined.display(),
                    "SEC-004: spec cwd escapes workspace root"
                );
            }
        }
    }

    Ok(joined)
}

/// Build a tokio Command from an exec spec and working directory.
///
/// ## SEC-004 / SEC-14: cwd traversal guard
///
/// Delegates to [`resolve_spec_cwd`] with [`CwdEscapePolicy::WarnAndAllow`],
/// which warns but still spawns (interactive trust model). Callers that
/// need fail-closed behaviour (git hooks) should call [`build_command_with`]
/// with [`CwdEscapePolicy::Deny`].
///
/// Note: `current_dir` is validated by the OS when the command is spawned — if the
/// path does not exist, `Command::output()` returns an `io::Error` that propagates
/// through the existing error handling in `exec_command`.
pub fn build_command(spec: &ExecCommandSpec, cwd: &std::path::Path, vars: &Variables) -> Command {
    build_command_with(spec, cwd, vars, CwdEscapePolicy::WarnAndAllow)
        .expect("WarnAndAllow policy never returns Err")
}

/// Build a tokio Command with an explicit cwd-escape policy. Returns `Err`
/// only when `policy == Deny` and the spec's cwd escapes the workspace root.
pub fn build_command_with(
    spec: &ExecCommandSpec,
    cwd: &std::path::Path,
    vars: &Variables,
    policy: CwdEscapePolicy,
) -> Result<Command, std::io::Error> {
    let mut cmd = Command::new(vars.expand(&spec.program).as_ref());
    let expanded_args: Vec<_> = spec
        .args
        .iter()
        .map(|a| vars.expand(a).into_owned())
        .collect();
    cmd.args(&expanded_args);
    let resolved_cwd = resolve_spec_cwd(spec.cwd.as_deref(), cwd, vars, policy)?;
    cmd.current_dir(&resolved_cwd);
    for (k, v) in &spec.env {
        let expanded_v = vars.expand(v);
        warn_if_sensitive_env(k, &expanded_v);
        cmd.env(k, expanded_v.as_ref());
    }
    cmd.kill_on_drop(true);
    Ok(cmd)
}

/// DUP-001: Shared patterns for detecting sensitive environment variable names.
/// Used by warn_if_sensitive_env() for warnings and is_sensitive_env_key() for dry-run redaction.
///
/// `SENSITIVE_REDACTION_PATTERNS` is a strict subset of this list.
/// The extra entries ("access_key", "session") trigger warnings but are not redacted in dry-run
/// output because they commonly appear in non-secret contexts.
const SENSITIVE_KEY_PATTERNS: &[&str] = &[
    "password",
    "secret",
    "token",
    "api_key",
    "apikey",
    "private",
    "credential",
    "auth",
    "access_key",
    "session",
];

/// DUP-001: Subset of SENSITIVE_KEY_PATTERNS used for dry-run redaction.
/// Every entry here must also appear in SENSITIVE_KEY_PATTERNS.
const SENSITIVE_REDACTION_PATTERNS: &[&str] = &[
    "password",
    "secret",
    "token",
    "api_key",
    "apikey",
    "private",
    "credential",
    "auth",
];

/// SEC-002: Warn if environment variable key or value looks sensitive.
///
/// Checks for:
/// - Key names containing patterns from SENSITIVE_KEY_PATTERNS
/// - Values that look like secrets: long base64-like strings, AWS-style keys, JWT-like tokens
pub fn warn_if_sensitive_env(key: &str, value: &str) {
    let lower = key.to_lowercase();
    for pattern in SENSITIVE_KEY_PATTERNS {
        if lower.contains(pattern) {
            tracing::warn!(
                key = %key,
                "SEC-002: env variable name suggests sensitive data; use OS environment instead of config"
            );
            return;
        }
    }

    if looks_like_secret_value(value) {
        tracing::warn!(
            key = %key,
            value_len = value.len(),
            "SEC-002: env variable value looks like a secret (long random-looking string); use OS environment instead of config"
        );
    }
}

/// DUP-001: Check if an env key looks like it might contain sensitive data.
///
/// This is used by dry-run mode to redact sensitive values in output.
/// Returns true if the key name suggests it contains a secret.
pub fn is_sensitive_env_key(key: &str) -> bool {
    let lower = key.to_lowercase();
    SENSITIVE_REDACTION_PATTERNS
        .iter()
        .any(|p| lower.contains(p))
}

/// Check if a value looks like it might be a secret.
///
/// CQ-011: Uses named predicates for each detection strategy, making the
/// logic explicit and testable. Each predicate checks a specific pattern:
///
/// - `has_high_entropy`: Mixed alphanumeric with digits, lowercase, uppercase
/// - `looks_like_jwt`: Starts with "eyJ" (base64-encoded JSON) and contains "."
/// - `looks_like_aws_key`: 40 chars, alphanumeric plus +/=
/// - `looks_like_uuid`: 36 chars with 4 hyphens in UUID format
pub fn looks_like_secret_value(value: &str) -> bool {
    if value.len() < 20 {
        return false;
    }

    has_high_entropy(value)
        || looks_like_jwt(value)
        || looks_like_aws_key(value)
        || looks_like_uuid(value)
}

/// CQ-005: Extracted helper predicates for secret detection.
///
/// Thresholds below are heuristic caps: a string is flagged as "high-entropy"
/// when it is long enough (>15 alphanumerics) and mixes digits, lowercase, and
/// uppercase in non-trivial amounts (>3 of each). This is deliberately strict
/// — legitimate words hit one or two of these but rarely all four — and keeps
/// false positives low on identifiers like `version_1_2_3`.
const HIGH_ENTROPY_MIN_ALPHANUMERIC: usize = 15;
const HIGH_ENTROPY_MIN_DIGITS: usize = 3;
const HIGH_ENTROPY_MIN_LOWERCASE: usize = 3;
const HIGH_ENTROPY_MIN_UPPERCASE: usize = 3;

pub(crate) fn has_high_entropy(value: &str) -> bool {
    let (mut alphanumeric, mut digits, mut lowercase, mut uppercase) = (0usize, 0, 0, 0);
    for c in value.chars() {
        if c.is_ascii_digit() {
            digits += 1;
            alphanumeric += 1;
        } else if c.is_ascii_lowercase() {
            lowercase += 1;
            alphanumeric += 1;
        } else if c.is_ascii_uppercase() {
            uppercase += 1;
            alphanumeric += 1;
        } else if c.is_alphanumeric() {
            alphanumeric += 1;
        }
    }
    alphanumeric > HIGH_ENTROPY_MIN_ALPHANUMERIC
        && digits > HIGH_ENTROPY_MIN_DIGITS
        && lowercase > HIGH_ENTROPY_MIN_LOWERCASE
        && uppercase > HIGH_ENTROPY_MIN_UPPERCASE
}

pub(crate) fn looks_like_jwt(value: &str) -> bool {
    value.starts_with("eyJ") && value.contains('.')
}

pub(crate) fn looks_like_aws_key(value: &str) -> bool {
    value.len() == 40
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
}

pub(crate) fn looks_like_uuid(value: &str) -> bool {
    if value.len() != 36 {
        return false;
    }
    let parts: Vec<&str> = value.split('-').collect();
    parts.len() == 5
        && parts[0].len() == 8
        && parts[1].len() == 4
        && parts[2].len() == 4
        && parts[3].len() == 4
        && parts[4].len() == 12
        && parts
            .iter()
            .all(|p| p.chars().all(|c| c.is_ascii_hexdigit()))
}

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

    let mut cmd = build_command(spec, cwd, vars);
    let (result, duration) = run_with_timeout(cmd.output(), spec.timeout()).await;
    let output = match result {
        Ok(o) => CommandOutput::from_raw(o),
        Err(e) => {
            // SEC-22: `io::Error::to_string()` on a spawn failure embeds the
            // resolved absolute program path and cwd (e.g. `/home/alice/…`).
            // That surfaces in `StepFailed::message` → progress UI → TAP
            // file, which leaks the developer's home path into CI logs.
            // Keep the full error at debug level and surface a shorter
            // "failed to spawn `<program>`: <kind>" to the user.
            tracing::debug!(error = %e, program = %spec.program, "exec spawn failed (full error)");
            let msg = redact_spawn_error(&spec.program, &e);
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
    let mut cmd = build_command(spec, cwd, vars);
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
            // SEC-22: same redaction as in `exec_command`.
            tracing::debug!(error = %e, program = %spec.program, "raw exec spawn failed");
            StepResult::failure(id, duration, redact_spawn_error(&spec.program, &e))
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
        let display_cmd = Some(spec.display_cmd().into_owned());
        let _ = tx
            .send(RunnerEvent::StepSkipped {
                id: id.clone(),
                display_cmd,
            })
            .await;
        return StepResult::skipped(id);
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
    let forwarder = tokio::spawn(async move {
        while let Some(ev) = local_rx.recv().await {
            if outer.send(ev).await.is_err() {
                break;
            }
        }
    });
    let result = exec_command(&id, &spec, &cwd, &vars, &mut |ev| {
        if let Err(mpsc::error::TrySendError::Full(_)) = local_tx.try_send(ev) {
            tracing::debug!("per-task event buffer full; dropping event under backpressure");
        }
    })
    .await;
    drop(local_tx);
    let _ = forwarder.await;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_sensitive_env_key_detects_password() {
        assert!(is_sensitive_env_key("PASSWORD"));
        assert!(is_sensitive_env_key("MY_PASSWORD"));
        assert!(is_sensitive_env_key("password"));
        assert!(is_sensitive_env_key("db_password"));
    }

    #[test]
    fn is_sensitive_env_key_detects_secret() {
        assert!(is_sensitive_env_key("SECRET"));
        assert!(is_sensitive_env_key("CLIENT_SECRET"));
        assert!(is_sensitive_env_key("my_secret_key"));
    }

    #[test]
    fn is_sensitive_env_key_detects_token() {
        assert!(is_sensitive_env_key("TOKEN"));
        assert!(is_sensitive_env_key("ACCESS_TOKEN"));
        assert!(is_sensitive_env_key("api_token"));
    }

    #[test]
    fn is_sensitive_env_key_detects_api_key() {
        assert!(is_sensitive_env_key("API_KEY"));
        assert!(is_sensitive_env_key("apikey"));
        assert!(is_sensitive_env_key("X_API_KEY"));
    }

    #[test]
    fn is_sensitive_env_key_detects_auth() {
        assert!(is_sensitive_env_key("AUTH"));
        assert!(is_sensitive_env_key("AUTHORIZATION"));
        assert!(is_sensitive_env_key("auth_header"));
    }

    #[test]
    fn is_sensitive_env_key_allows_non_sensitive() {
        assert!(!is_sensitive_env_key("PATH"));
        assert!(!is_sensitive_env_key("HOME"));
        assert!(!is_sensitive_env_key("USER"));
        assert!(!is_sensitive_env_key("DEBUG"));
        assert!(!is_sensitive_env_key("LOG_LEVEL"));
    }

    #[test]
    fn looks_like_secret_value_detects_jwt() {
        let jwt_start = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        assert!(looks_like_secret_value(jwt_start));
    }

    #[test]
    fn looks_like_secret_value_detects_uuid() {
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert!(looks_like_secret_value(uuid));
    }

    #[test]
    fn looks_like_secret_value_rejects_short_values() {
        assert!(!looks_like_secret_value("short"));
        assert!(!looks_like_secret_value("1234567890"));
    }

    #[test]
    fn looks_like_secret_value_rejects_simple_strings() {
        assert!(!looks_like_secret_value("this is a normal string value"));
        assert!(!looks_like_secret_value("a simple path /to/some/file"));
    }

    // SEC-14 / FN-1 regression tests for the extracted resolve_spec_cwd.
    #[test]
    fn resolve_spec_cwd_none_returns_workspace() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws);
        let out = resolve_spec_cwd(None, &ws, &vars, CwdEscapePolicy::WarnAndAllow).unwrap();
        assert_eq!(out, ws);
    }

    #[test]
    fn resolve_spec_cwd_absolute_is_returned_verbatim() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws);
        let abs = std::path::Path::new("/opt/elsewhere");
        let out = resolve_spec_cwd(Some(abs), &ws, &vars, CwdEscapePolicy::Deny).unwrap();
        assert_eq!(out, std::path::PathBuf::from("/opt/elsewhere"));
    }

    #[test]
    fn resolve_spec_cwd_deny_rejects_escape() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws);
        let escaping = std::path::Path::new("../etc");
        let err = resolve_spec_cwd(Some(escaping), &ws, &vars, CwdEscapePolicy::Deny)
            .expect_err("escape should fail under Deny");
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(err.to_string().contains("SEC-14"));
    }

    #[test]
    fn resolve_spec_cwd_warn_allows_escape() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws);
        let escaping = std::path::Path::new("../etc");
        let out =
            resolve_spec_cwd(Some(escaping), &ws, &vars, CwdEscapePolicy::WarnAndAllow).unwrap();
        // Still joined; caller trusts `.ops.toml` in interactive mode.
        assert_eq!(out, ws.join("../etc"));
    }

    #[test]
    fn resolve_spec_cwd_relative_inside_workspace_is_joined() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws);
        let inside = std::path::Path::new("sub/dir");
        let out = resolve_spec_cwd(Some(inside), &ws, &vars, CwdEscapePolicy::Deny).unwrap();
        assert_eq!(out, ws.join("sub/dir"));
    }

    #[test]
    fn redaction_patterns_is_subset_of_key_patterns() {
        for pattern in SENSITIVE_REDACTION_PATTERNS {
            assert!(
                SENSITIVE_KEY_PATTERNS.contains(pattern),
                "SENSITIVE_REDACTION_PATTERNS entry {:?} missing from SENSITIVE_KEY_PATTERNS",
                pattern
            );
        }
    }
}
