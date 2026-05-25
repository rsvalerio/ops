---
id: TASK-1127
title: >-
  SEC-21: log_and_redact_spawn_error logs spec.program via Display field,
  allowing log injection from .ops.toml-supplied program names
status: Done
assignee:
  - TASK-1259
created_date: '2026-05-08 07:28'
updated_date: '2026-05-08 13:25'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: \`crates/runner/src/command/exec.rs:213\`

**What**: \`tracing::debug!(error = %e, program = %program, context, "exec spawn failed (full error)")\` formats \`program\` (the raw \`spec.program\` string from \`.ops.toml\`) via Display (\`%\`). Identical to the log-injection vector TASK-0940 fixed in TapWriter and TASK-0975 fixed in cargo-update parse_update_output: a config-supplied program containing newlines, ANSI escapes, or carriage returns can forge log lines or terminal control sequences when the debug log lands on stderr/journald.

**Why it matters**: The spec is part of the project trust model (covered by docs at the top of exec.rs), so the threat is not "external attacker" but "co-worker landing a malicious .ops.toml in a PR" — exactly the same threat model as SEC-14/TASK-0886 hook policy. A program field of \`cargo\\nFAKE_LOG_LINE\\n\` would forge log entries on every spawn failure, which an attacker can arrange by depending on a binary that doesn't exist. The redacted user-facing message (\`redact_spawn_error\`) already uses Debug formatting on the kind, so this is a one-line fix on the tracing call site.

**Fix**: \`program = ?program\` (Debug) instead of \`program = %program\` (Display). Mirrors TASK-0940's TapWriter sweep and TASK-1102's RedactedUrl scrub.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 log_and_redact_spawn_error formats program via Debug, not Display
- [x] #2 Test: a program containing \n / \u{1b}[31m round-trips through log_and_redact_spawn_error without surviving as a literal control character in the rendered tracing event
- [x] #3 Sibling tracing call sites that log spec.program (e.g. build_command_async trace event in build.rs:471-476) audited and follow the same Debug-format rule where untrusted
<!-- AC:END -->
