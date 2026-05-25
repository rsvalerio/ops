---
id: TASK-0981
title: >-
  SEC-21: ui::emit writes user-supplied message to stderr without escaping
  control characters
status: Done
assignee:
  - TASK-1010
created_date: '2026-05-04 21:58'
updated_date: '2026-05-06 06:54'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/ui.rs:15-18`

**What**: `fn emit(level, message)` calls `writeln!(err, \"ops: {level}: {message}\")` with a borrowed `&str` straight from the caller. Callers (`load_config_or_default`, `Stack::resolve`, …) feed it strings derived from `.ops.toml` content, env vars, and `anyhow` chains via `format!(\"{e:#}\")`. A config or extension error message that contains a literal `\\n` (e.g. an offending TOML key or a nested error chain rendered through user data) lets an attacker forge a second physical line beginning with `ops: error:` — the same pattern SEC-21/TASK-0745 (merge_indexmap collision log) and TASK-0818/0809/0944 already swept for tracing emitters. Embedded ANSI / `\\x1b` escapes flow straight through to a TTY and can rewrite earlier lines or hide the real error.

**Why it matters**: this is the *user-visible* sibling of the tracing log-injection sweep, and it is the channel operators actually read (`stderr` "ops: error:" lines drive CI failure triage). Pre-escape the message with the same Debug-format / `escape_default` posture used by the tracing fix in stack.rs:26-30 / merge.rs:32-36, or strip control bytes (`< 0x20 except \\t`) before formatting. Newlines inside legitimate multi-line errors should be re-prefixed with `ops: <level>:   ` so a multi-line anyhow `{e:#}` still renders distinguishably without yielding a forged top-level line.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ui::note/warn/error escape or sanitise control characters (newline, ANSI ESC) before writing to stderr
- [ ] #2 Multi-line legitimate error chains stay readable (e.g. continuation lines re-indented under the prefix)
- [ ] #3 Regression test asserts an injected newline in the message does not start a new line beginning with 'ops:'
<!-- AC:END -->
