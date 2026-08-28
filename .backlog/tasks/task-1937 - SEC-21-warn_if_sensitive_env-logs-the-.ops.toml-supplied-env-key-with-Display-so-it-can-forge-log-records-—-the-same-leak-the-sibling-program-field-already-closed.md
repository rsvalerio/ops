---
id: TASK-1937
title: >-
  SEC-21: warn_if_sensitive_env logs the .ops.toml-supplied env key with
  Display, so it can forge log records — the same leak the sibling program field
  already closed
status: To Do
assignee:
  - TASK-1986
created_date: '2026-08-27 15:47'
updated_date: '2026-08-28 14:10'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/runner/src/command/secret_patterns.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/secret_patterns.rs:44-48` and `:53-58` (`warn_if_sensitive_env`), called per env var per spawn from `crates/runner/src/command/build.rs:622-625`

**What**: both warn sites render the env key with the `%` (Display) formatter:

```rust
tracing::warn!(
    key = %key,
    "SEC-002: env variable name suggests sensitive data; ..."
);
```

`key` is the raw key from a command's `env` table in `.ops.toml` (`build_command_with` iterates `&spec.env` and passes `k` straight through), so it is config-supplied text with no character restrictions. Rendered via Display, an embedded newline plus a crafted prefix emits what looks like an additional log record, and an embedded `\u{1b}[` repaints the operator's terminal.

This crate has already decided the policy and applied it to the neighbouring field. `log_and_redact_spawn_error` (`exec.rs:234-239`) formats the equally config-supplied program name with `?`:

```rust
// SEC-21 / TASK-1127: format `program` via Debug so newlines/ANSI escape sequences
// smuggled through `.ops.toml`-supplied program names cannot forge log entries.
tracing::debug!(error = %e, program = ?program, context, "exec spawn failed (full error)");
```

`build_command_async` (`build.rs:565-570`) does the same for its trace event, and `tap.rs` does it for config-supplied paths (ERR-7 / TASK-0940). `secret_patterns.rs` was missed. The escape is pinned by a unit test for `program` (`exec.rs:812-819`) and for tap paths (`tap.rs:218-225`); there is no equivalent for env keys.

Note the *value* is already handled correctly — only `value_len` is logged, never the value itself — so the fix is confined to the two `key = %key` fields.

Severity is bounded by the trust model documented at the top of `exec.rs` (a local `.ops.toml` is trusted like a Makefile), but the same is true of `program`, and the project chose to close that hole anyway; the SEC-14 `CwdEscapePolicy::Deny` docs spell out the reason — a coworker's PR can land a `.ops.toml` that runs on a maintainer's next commit, so config content is not fully trusted on the hook path.

**Why it matters**: it is an inconsistency in an already-adopted hardening policy, on the one remaining `.ops.toml`-derived string that reaches a log field unescaped. Left alone it will read to the next reviewer as a deliberate exception rather than an oversight.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 both warn! sites in warn_if_sensitive_env render the env key with the Debug (?) formatter, matching the SEC-21 / TASK-1127 policy applied to program and to tap paths
- [ ] #2 a unit test pins the escape for a key containing a newline and an ANSI escape, mirroring program_field_debug_escapes_control_characters in exec.rs
- [ ] #3 a sweep confirms no other .ops.toml-derived string in this crate still reaches a tracing field via Display
<!-- AC:END -->
