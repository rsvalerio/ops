---
id: TASK-1175
title: >-
  SEC-22: ExpandError messages can leak environment variable names into
  StepFailed user-facing text
status: Done
assignee:
  - TASK-1259
created_date: '2026-05-08 08:08'
updated_date: '2026-05-08 13:30'
labels:
  - code-review-rust
  - sec
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:215`

**What**: `expand_err_to_io` converts an `ExpandError` directly into an `io::Error` via `err.to_string()`, and the result becomes the `StepFailed.message` (and TAP file) text. `ExpandError` Display includes the raw variable name and may include the surrounding context. If a `.ops.toml` references `\${OPS_TOKEN}` or similar in a typo'd form, the user-facing diagnostic surfaces the variable name into CI logs/TAP.

**Why it matters**: SEC-22 closed the spawn-error path against `io::Error::to_string()` leaking absolute paths; the parallel expand-error path is still uncensored. Variable-expansion failures are an attacker-influenced surface (a coworker PR adds \${ATTACKER_VAR} to .ops.toml) and the message is what gets uploaded to CI artifacts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The expand-error rendering path emits a generic operator message (e.g. variable expansion failed) to StepFailed.message, with the full chain logged at tracing::debug! like log_and_redact_spawn_error.
- [x] #2 Regression test asserts a triggering env var name does not appear in the StepFailed message body.
<!-- AC:END -->
