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

/// Build a tokio Command from an exec spec and working directory.
///
/// Note: `current_dir` is validated by the OS when the command is spawned — if the
/// path does not exist, `Command::output()` returns an `io::Error` that propagates
/// through the existing error handling in `exec_command`.
pub fn build_command(spec: &ExecCommandSpec, cwd: &std::path::Path, vars: &Variables) -> Command {
    let mut cmd = Command::new(vars.expand(&spec.program).as_ref());
    let expanded_args: Vec<_> = spec
        .args
        .iter()
        .map(|a| vars.expand(a).into_owned())
        .collect();
    cmd.args(&expanded_args);
    let resolved_cwd = match spec.cwd.as_deref() {
        Some(p) => {
            let lossy = p.to_string_lossy();
            let expanded = vars.expand(&lossy);
            let ep = std::path::PathBuf::from(expanded.as_ref());
            if ep.is_relative() {
                cwd.join(ep)
            } else {
                ep
            }
        }
        None => cwd.to_path_buf(),
    };
    cmd.current_dir(&resolved_cwd);
    for (k, v) in &spec.env {
        let expanded_v = vars.expand(v);
        warn_if_sensitive_env(k, &expanded_v);
        cmd.env(k, expanded_v.as_ref());
    }
    cmd.kill_on_drop(true);
    cmd
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
pub(crate) fn looks_like_secret_value(value: &str) -> bool {
    if value.len() < 20 {
        return false;
    }

    has_high_entropy(value)
        || looks_like_jwt(value)
        || looks_like_aws_key(value)
        || looks_like_uuid(value)
}

/// CQ-005: Extracted helper predicates for secret detection.
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
    alphanumeric > 15 && digits > 3 && lowercase > 3 && uppercase > 3
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

/// Execute a command with an optional timeout.
pub async fn execute_with_timeout(
    mut cmd: Command,
    timeout: Option<Duration>,
) -> Result<std::process::Output, std::io::Error> {
    let future = cmd.output();
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
    let start = Instant::now();

    let cmd = build_command(spec, cwd, vars);
    let output = match execute_with_timeout(cmd, spec.timeout()).await {
        Ok(o) => CommandOutput::from_raw(o),
        Err(e) => {
            let duration = start.elapsed();
            let msg = e.to_string();
            emit(RunnerEvent::StepFailed {
                id: id.into(),
                duration_secs: duration.as_secs_f64(),
                message: msg.clone(),
                display_cmd,
            });
            return StepResult::failure(id, duration, msg);
        }
    };
    let duration = start.elapsed();

    emit_output_events(id, &output.stdout, &output.stderr, emit);
    emit_step_completion(id, duration, &output, display_cmd, emit);
    build_step_result(id, duration, output)
}

/// Standalone exec used by parallel plan: runs one command, sends events via channel, respects abort flag.
#[allow(clippy::too_many_arguments)]
pub async fn exec_standalone(
    id: CommandId,
    spec: ExecCommandSpec,
    cwd: PathBuf,
    vars: Variables,
    tx: mpsc::UnboundedSender<RunnerEvent>,
    abort: Arc<AtomicBool>,
) -> StepResult {
    if abort.load(Ordering::Acquire) {
        let display_cmd = Some(spec.display_cmd().into_owned());
        let _ = tx.send(RunnerEvent::StepSkipped {
            id: id.clone(),
            display_cmd,
        });
        return StepResult::skipped(id);
    }
    exec_command(&id, &spec, &cwd, &vars, &mut |ev| {
        let _ = tx.send(ev);
    })
    .await
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
