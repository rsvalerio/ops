---
id: TASK-2701
title: 'ERR-7: build-time step failures (cwd-escape refusal, expansion failure) are reported as ''failed to spawn <program>: <ErrorKind>'''
status: Done
assignee: []
created_date: '2026-11-18 18:31'
updated_date: '2026-11-18 21:27'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2842'
modified_files:
  - crates/runner/src/command/exec.rs
  - crates/runner/src/command/build.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-7:crates/runner/src/command/exec.rs:capture'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:576` (`capture`) and `crates/runner/src/command/exec.rs:679` (`exec_command_raw`), via `redact_spawn_error` (`exec.rs:400`); errors originate in `apply_escape_policy` (`crates/runner/src/command/build.rs:454`) and `expand_err_to_io` (`build.rs:254`)

**What**: When `build_command_async` fails, `capture` / `exec_command_raw` pass the error through `log_and_redact_spawn_error`, which keeps only the `io::ErrorKind`. So a hook run whose `.ops.toml` `cwd` escapes the workspace under `CwdEscapePolicy::Deny` shows the user `failed to spawn \`cargo\`: PermissionDenied`, and a failed `${VAR}` expansion or a non-UTF-8 cwd shows `failed to spawn \`cargo\`: InvalidInput`. Nothing was spawned in any of these cases. The real reason (the SEC-14 policy refusal, or "variable expansion failed") exists only at `tracing::debug!`. `tests/sequential.rs::deny_policy_refuses_escaping_cwd_on_hook_path` pins the `PermissionDenied` wording.

**Why it matters**: The redaction (SEC-22 / TASK-1175) exists to keep absolute paths and env-var names out of StepFailed/TAP text. It does not require collapsing runner-authored build errors into a misleading "failed to spawn … PermissionDenied", which sends a hook user looking at file permissions instead of their `cwd` setting. Build failures need their own fixed, path-free reason (e.g. "cwd escapes the workspace (policy: deny)", "variable expansion failed"), mapped at the build/exec boundary (ERR-7).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A Deny-policy cwd refusal, an expansion failure and a non-UTF-8 cwd each produce a distinct user-facing StepFailed message naming the cause, without paths or variable names, and none of them says 'failed to spawn'
- [x] #2 Real spawn failures (NotFound, PermissionDenied on exec) keep the existing redacted 'failed to spawn' text; tests cover both groups

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.05 (code-review-rust run, 2026-11-18).

Build failures carry a BuildFailure inner error (CwdEscapesWorkspace, VariableExpansion, NonUtf8Cwd, BuildTaskFailed); redact_spawn_error renders them as 'cannot run `<program>`: <cause>'. Paths/var names now only at tracing::debug. Also covered the blocking-pool panic path.

<!-- SECTION:NOTES:END -->
