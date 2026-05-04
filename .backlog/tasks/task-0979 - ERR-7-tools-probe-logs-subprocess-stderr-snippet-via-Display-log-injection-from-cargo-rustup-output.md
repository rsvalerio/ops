---
id: TASK-0979
title: >-
  ERR-7: tools::probe logs subprocess stderr snippet via Display, log injection
  from cargo/rustup output
status: Done
assignee: []
created_date: '2026-05-04 21:58'
updated_date: '2026-05-04 23:05'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:113-118, 262-267`

**What**: When `cargo --list` or `rustup component list --installed` exits non-zero, both probes log the first 200 chars of stderr via the Display formatter:
```
tracing::warn!(tool = name, code = ?output.status.code(), stderr = %stderr_snippet, "cargo --list exited non-zero; ...");
tracing::warn!(component = component, code = ?output.status.code(), stderr = %stderr_snippet, "rustup component list exited non-zero; ...");
```
`stderr_snippet` is built with `String::from_utf8_lossy(&output.stderr).chars().take(200).collect()`. Cargo and rustup emit ANSI color escapes by default and may surface upstream registry messages that contain control characters. Display passes those through unescaped, so an upstream crates.io diagnostic with embedded newlines or ANSI escapes can forge log records or repaint the operator's terminal. Sister sweep to TASK-0941 / TASK-0947 / TASK-0965.

**Why it matters**: The probe failure path is reached on every CI run that has a flaky registry or is missing a toolchain — operator logs are primary triage signal. Forged log records hide the real failure under fake breadcrumbs. Lower-impact than the workspace-member case (TASK-0941) because cargo/rustup are not directly attacker-controlled, but registry-served metadata can flow into stderr.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both warn sites format stderr via Debug (?) instead of Display (%)
- [ ] #2 tracing breadcrumb still preserves the leading 200-char cap so log volume stays bounded
<!-- AC:END -->
