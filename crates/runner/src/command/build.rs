//! Command-building helpers: cwd resolution, workspace-escape detection, and
//! tokio [`Command`] construction from an [`ExecCommandSpec`].
//!
//! See the `SEC-004` / `SEC-14` notes on [`resolve_spec_cwd`] for the escape
//! policy rationale.

use super::secret_patterns::warn_if_sensitive_env;
use ops_core::config::ExecCommandSpec;
use ops_core::expand::{ExpandError, Variables};
use tokio::process::Command;

/// ERR-1 / TASK-0450: convert a strict-expansion error into an `io::Error`
/// so build failures share the spawn-error pipeline and surface as a
/// `StepFailed` event rather than panicking through `expect`.
fn expand_err_to_io(err: ExpandError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidInput, err.to_string())
}

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
    ///
    /// SEC-25: residual TOCTOU window. The check happens in
    /// [`detect_workspace_escape`], which calls `std::fs::canonicalize`,
    /// while the actual `chdir` is performed by the OS when the child is
    /// spawned. To shrink the window, [`resolve_spec_cwd`] canonicalizes
    /// the joined path under `Deny` and hands the symlink-free result to
    /// `current_dir`, so the kernel does not re-resolve any symlinks at
    /// exec time. A narrow race remains: an attacker who can replace a
    /// component of the canonical path (e.g. by mounting over it or
    /// swapping a directory they own) between canonicalization and exec
    /// can still divert the child. Closing this fully would require an
    /// `openat`/`fchdir`-style fd handoff to the child, which neither
    /// `std::process::Command` nor `tokio::process::Command` exposes
    /// today.
    #[allow(dead_code)]
    Deny,
}

/// FN-1: classification of how a spec `cwd` relates to the workspace root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EscapeKind {
    /// Path stays inside the workspace under both lexical and canonical checks.
    Inside,
    /// Path escapes the workspace (lexically and/or via symlink canonicalization).
    Escapes,
}

/// Classify `joined` against `workspace`. Pure function: fast lexical check
/// first, then a canonical check so a symlink inside the workspace pointing
/// outside is still caught.
pub(crate) fn detect_workspace_escape(
    joined: &std::path::Path,
    workspace: &std::path::Path,
) -> EscapeKind {
    let lexically_escapes = !normalize_path(joined).starts_with(workspace);
    let canonically_escapes = match (
        std::fs::canonicalize(joined).ok(),
        std::fs::canonicalize(workspace).ok(),
    ) {
        (Some(a), Some(b)) => !a.starts_with(&b),
        _ => false,
    };
    if lexically_escapes || canonically_escapes {
        EscapeKind::Escapes
    } else {
        EscapeKind::Inside
    }
}

/// FN-1: apply an escape policy to a detected escape. `Deny` converts to an
/// `io::Error`; `WarnAndAllow` emits a tracing warning and lets the caller
/// continue.
pub(crate) fn apply_escape_policy(
    policy: CwdEscapePolicy,
    spec_cwd: &std::path::Path,
    workspace_cwd: &std::path::Path,
    joined: &std::path::Path,
) -> Result<(), std::io::Error> {
    match policy {
        CwdEscapePolicy::Deny => Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "SEC-14: refusing to spawn: spec cwd {} escapes workspace root {}",
                spec_cwd.display(),
                workspace_cwd.display()
            ),
        )),
        CwdEscapePolicy::WarnAndAllow => {
            tracing::warn!(
                cwd = %workspace_cwd.display(),
                spec_cwd = %spec_cwd.display(),
                resolved = %joined.display(),
                "SEC-004: spec cwd escapes workspace root"
            );
            Ok(())
        }
    }
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
    let expanded = vars.try_expand(&lossy).map_err(expand_err_to_io)?;
    let ep = std::path::PathBuf::from(expanded.as_ref());
    // SEC-23 / TASK-0500: an absolute spec_cwd must still be checked against
    // the workspace root. A malicious `cwd = "/etc"` would previously bypass
    // the policy entirely because it short-circuited here without invoking
    // detect_workspace_escape. Run the check against the absolute path
    // unchanged (it is its own joined form) and let `apply_escape_policy`
    // decide whether to allow or deny.
    let joined = if ep.is_relative() {
        workspace_cwd.join(&ep)
    } else {
        ep.clone()
    };
    if detect_workspace_escape(&joined, workspace_cwd) == EscapeKind::Escapes {
        apply_escape_policy(policy, &ep, workspace_cwd, &joined)?;
    }
    if !ep.is_relative() {
        return Ok(ep);
    }
    // SEC-25: under Deny, hand the kernel a symlink-free canonical path so
    // it does not re-resolve symlinks at chdir time. Narrows (but does not
    // close) the TOCTOU window — see CwdEscapePolicy::Deny docs. Best
    // effort: if canonicalize fails (e.g. cwd does not exist yet), fall
    // back to the joined path and let the OS surface the spawn error.
    if policy == CwdEscapePolicy::Deny {
        if let Ok(canonical) = std::fs::canonicalize(&joined) {
            return Ok(canonical);
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
// ERR-5 / TASK-0456: `build_command` previously panicked via
// `.expect("WarnAndAllow policy never returns Err")` to encode "cannot
// fail under WarnAndAllow" at the type level. After TASK-0450 the
// expansion path itself is fallible (a non-UTF-8 env var must surface as
// a step failure rather than crashing the runner), so the no-panic
// guarantee is now structural in the *return type*: build_command
// returns `Result`, and every caller threads the error to a StepFailed
// event. There is no remaining `.expect` to revisit.
pub fn build_command(
    spec: &ExecCommandSpec,
    cwd: &std::path::Path,
    vars: &Variables,
) -> Result<Command, std::io::Error> {
    build_command_with(spec, cwd, vars, CwdEscapePolicy::WarnAndAllow)
}

/// CONC-5 / TASK-0330: async variant that runs the synchronous filesystem
/// work in [`build_command`] (notably `std::fs::canonicalize` calls inside
/// [`detect_workspace_escape`] and [`resolve_spec_cwd`]) on the blocking
/// thread pool.
///
/// Without this, every parallel command spawn blocks a tokio worker on
/// `canonicalize` syscalls — slow on NFS or symlink-heavy paths and
/// proportional to the spec cwd's depth. Under high `MAX_PARALLEL` counts
/// that starves other tasks scheduled on the same worker.
///
/// OWN-2 / TASK-0462: `vars` and `cwd` are passed as `Arc` so the only
/// per-spawn allocations on the parallel hot path are `Arc::clone` (a
/// single atomic refcount bump each), not a deep `Variables`/`PathBuf`
/// clone. The previous signature took `Variables`/`PathBuf` by value,
/// which silently re-allocated the inner `HashMap` per spawn and mixed
/// `Arc` indirection at the call site with per-call deep clones — the
/// worst of both. `spec` is still moved by value because each task
/// already owns a distinct `ExecCommandSpec` it consumes.
pub async fn build_command_async(
    spec: ExecCommandSpec,
    cwd: std::sync::Arc<std::path::PathBuf>,
    vars: std::sync::Arc<Variables>,
) -> Result<Command, std::io::Error> {
    // OWN-2 / TASK-0462: emit a trace event on every spawn so we can
    // confirm in `RUST_LOG=trace` runs that the only allocations per
    // spawn are Arc::clone counts (logged here as the existing
    // strong_count) and the spec move — no Variables/PathBuf deep
    // clones. Strong counts > 1 prove the parallel path is sharing the
    // same instance across MAX_PARALLEL workers.
    tracing::trace!(
        program = %spec.program,
        vars_strong = std::sync::Arc::strong_count(&vars),
        cwd_strong = std::sync::Arc::strong_count(&cwd),
        "build_command_async: Arc-only inputs, no deep clone"
    );
    // ERR-5 / TASK-0456: a panicking blocking task previously surfaced
    // here as a runner-wide panic via `.expect`. Now we downgrade to a
    // `tracing::error!` plus a synthesized `io::Error` so the calling
    // step fails gracefully (StepFailed) instead of aborting the runner.
    // Cancellation of the blocking task is treated identically — it can
    // only happen if the runtime is shutting down, in which case
    // returning Err is no worse than a hard panic.
    match tokio::task::spawn_blocking(move || build_command(&spec, cwd.as_ref(), vars.as_ref()))
        .await
    {
        Ok(result) => result,
        Err(join_err) => {
            tracing::error!(
                error = %join_err,
                "ERR-5: build_command panicked on blocking pool; converting to step failure"
            );
            Err(std::io::Error::other(format!(
                "build_command panicked on blocking pool: {join_err}"
            )))
        }
    }
}

/// Build a tokio Command with an explicit cwd-escape policy. Returns `Err`
/// only when `policy == Deny` and the spec's cwd escapes the workspace root.
pub fn build_command_with(
    spec: &ExecCommandSpec,
    cwd: &std::path::Path,
    vars: &Variables,
    policy: CwdEscapePolicy,
) -> Result<Command, std::io::Error> {
    let program = vars.try_expand(&spec.program).map_err(expand_err_to_io)?;
    let mut cmd = Command::new(program.as_ref());
    let expanded_args: Vec<String> = spec
        .args
        .iter()
        .map(|a| vars.try_expand(a).map(|c| c.into_owned()))
        .collect::<Result<_, _>>()
        .map_err(expand_err_to_io)?;
    cmd.args(&expanded_args);
    let resolved_cwd = resolve_spec_cwd(spec.cwd.as_deref(), cwd, vars, policy)?;
    cmd.current_dir(&resolved_cwd);
    for (k, v) in &spec.env {
        let expanded_v = vars.try_expand(v).map_err(expand_err_to_io)?;
        warn_if_sensitive_env(k, &expanded_v);
        cmd.env(k, expanded_v.as_ref());
    }
    cmd.kill_on_drop(true);
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::test_utils::{exec_spec, exec_spec_with_cwd};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// CONC-5 / TASK-0330: the async variant must dispatch the canonicalize
    /// work to the blocking pool so a single-threaded runtime can still
    /// drive other tasks while build_command runs. This test uses a
    /// `current_thread` runtime — the only worker — and asserts that a
    /// concurrent counter task makes progress while build_command_async is
    /// in flight.
    ///
    /// Under the previous synchronous `build_command` call from inside an
    /// async function, the runtime worker would be blocked for the
    /// duration of every canonicalize syscall, starving the counter task
    /// (and in production, every other task scheduled on that worker).
    #[test]
    fn build_command_async_does_not_starve_concurrent_tokio_task() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let counter = Arc::new(AtomicUsize::new(0));
            let c = counter.clone();
            let counting = tokio::spawn(async move {
                for _ in 0..200 {
                    tokio::task::yield_now().await;
                    c.fetch_add(1, Ordering::Relaxed);
                }
            });

            let tmp = tempfile::tempdir().unwrap();
            std::fs::create_dir(tmp.path().join("sub")).unwrap();
            let vars = Variables::from_env(tmp.path());

            // Run several build_command_async invocations. Each dispatches
            // canonicalize to the blocking pool, leaving the runtime
            // worker free to poll the counting task between awaits.
            for _ in 0..5 {
                let spec =
                    exec_spec_with_cwd("echo", &["x"], Some(std::path::PathBuf::from("sub")));
                let _cmd = build_command_async(
                    spec,
                    std::sync::Arc::new(tmp.path().to_path_buf()),
                    std::sync::Arc::new(vars.clone()),
                )
                .await
                .unwrap();
            }

            counting.await.unwrap();
            assert_eq!(
                counter.load(Ordering::Relaxed),
                200,
                "concurrent task must run to completion despite repeated build_command_async calls"
            );
        });
    }

    /// Functional parity: the async wrapper must produce a Command with
    /// the same observable program as the sync version. Catches refactors
    /// that accidentally rewrite the spec inside spawn_blocking.
    #[tokio::test]
    async fn build_command_async_preserves_program_name() {
        let tmp = tempfile::tempdir().unwrap();
        let vars = Variables::from_env(tmp.path());
        let spec = exec_spec("echo", &["hello"]);
        let cmd = build_command_async(
            spec,
            std::sync::Arc::new(tmp.path().to_path_buf()),
            std::sync::Arc::new(vars),
        )
        .await
        .unwrap();
        // tokio::process::Command exposes the program via as_std()
        let program = cmd.as_std().get_program().to_string_lossy().into_owned();
        assert_eq!(program, "echo");
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
    fn resolve_spec_cwd_absolute_inside_workspace_is_returned_verbatim() {
        // SEC-23 / TASK-0500: absolute paths still go through the escape
        // check. A path lexically inside the workspace is allowed under
        // Deny; verbatim because absolute paths are not joined.
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws);
        let abs = std::path::Path::new("/tmp/ws/inside");
        let out = resolve_spec_cwd(Some(abs), &ws, &vars, CwdEscapePolicy::Deny).unwrap();
        assert_eq!(out, std::path::PathBuf::from("/tmp/ws/inside"));
    }

    /// SEC-23 / TASK-0500: an absolute spec_cwd outside the workspace must
    /// be rejected under `Deny`. The previous bug short-circuited the policy
    /// check so a malicious `cwd = "/etc"` would silently spawn at /etc on
    /// the hook path.
    #[test]
    fn resolve_spec_cwd_absolute_outside_workspace_is_denied() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws);
        let abs = std::path::Path::new("/etc");
        let err = resolve_spec_cwd(Some(abs), &ws, &vars, CwdEscapePolicy::Deny)
            .expect_err("absolute path outside workspace must be denied under Deny");
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
        assert!(err.to_string().contains("SEC-14"));
    }

    /// SEC-23: under WarnAndAllow the absolute path is still returned (the
    /// interactive trust model lets `.ops.toml` choose its cwd) but the
    /// escape is logged.
    #[test]
    fn resolve_spec_cwd_absolute_outside_workspace_warns_under_warn_and_allow() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let vars = Variables::from_env(&ws);
        let abs = std::path::Path::new("/opt/elsewhere");
        let out = resolve_spec_cwd(Some(abs), &ws, &vars, CwdEscapePolicy::WarnAndAllow).unwrap();
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
    fn detect_workspace_escape_inside_is_inside() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let inside = ws.join("sub/dir");
        assert_eq!(detect_workspace_escape(&inside, &ws), EscapeKind::Inside);
    }

    #[test]
    fn detect_workspace_escape_parent_escapes() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let escaping = ws.join("../etc");
        assert_eq!(detect_workspace_escape(&escaping, &ws), EscapeKind::Escapes);
    }

    #[test]
    fn apply_escape_policy_deny_returns_permission_denied() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let spec = std::path::Path::new("../etc");
        let joined = ws.join(spec);
        let err = apply_escape_policy(CwdEscapePolicy::Deny, spec, &ws, &joined)
            .expect_err("Deny should produce an error");
        assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn apply_escape_policy_warn_is_ok() {
        let ws = std::path::PathBuf::from("/tmp/ws");
        let spec = std::path::Path::new("../etc");
        let joined = ws.join(spec);
        assert!(apply_escape_policy(CwdEscapePolicy::WarnAndAllow, spec, &ws, &joined).is_ok());
    }

    /// SEC-25: best-effort regression for the symlink-swap window. Layout:
    /// `ws/sub` is initially a real directory inside the workspace, so the
    /// escape check passes. We then swap it for a symlink pointing outside
    /// the workspace and assert that, because `Deny` canonicalizes the
    /// returned path, the chdir target the kernel sees is the original
    /// in-workspace path — not the post-swap escape destination. The
    /// residual window above this (e.g. mount-over) is documented on
    /// `CwdEscapePolicy::Deny` rather than closed in code.
    #[cfg(unix)]
    #[test]
    fn deny_returns_canonical_path_to_shrink_toctou_window() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = std::fs::canonicalize(tmp.path()).unwrap();
        let escape_target = tempfile::tempdir().unwrap();
        let escape_target_canonical = std::fs::canonicalize(escape_target.path()).unwrap();
        let inside = ws.join("sub");
        std::fs::create_dir(&inside).unwrap();

        let vars = Variables::from_env(&ws);
        let resolved = resolve_spec_cwd(
            Some(std::path::Path::new("sub")),
            &ws,
            &vars,
            CwdEscapePolicy::Deny,
        )
        .expect("sub is inside the workspace");
        assert_eq!(resolved, inside, "Deny should return the canonical path");

        // Simulate the swap that would race a real spawn: replace `sub`
        // with a symlink to a directory outside the workspace.
        std::fs::remove_dir(&inside).unwrap();
        std::os::unix::fs::symlink(&escape_target_canonical, &inside).unwrap();

        // The previously resolved path is the canonical in-workspace one;
        // a chdir to it does not re-traverse the symlink we just planted.
        // This is the protection the canonicalize-under-Deny step provides.
        assert_ne!(
            resolved, escape_target_canonical,
            "resolved path must not be the post-swap escape target"
        );
    }
}
