---
id: TASK-0192
title: >-
  ERR-1: ProgressDisplay::tap_line silently discards writeln! errors on every
  output line
status: Done
assignee: []
created_date: '2026-04-22 21:26'
updated_date: '2026-04-23 14:59'
labels:
  - rust-code-review
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:488-492` (tap_line).

**What**: `fn tap_line(&mut self, line: &str) { if let Some(ref mut f) = self.tap_file { let _ = writeln!(f, "{}", line); } }` silently drops the Result from `writeln!`. If the tap file fd goes bad mid-run (disk full, NFS disconnect, fd closed underneath us), every subsequent output line is silently dropped without the user seeing any indication the tap is broken.

**Why it matters**: ERR-1 (errors should be propagated OR logged at the handling site, never ignored). Compare `write_stderr` two functions above which does `tracing::debug!(error = %e, ...)` on failure. Fix: apply the same pattern — on first write error, log once and close/replace the tap handle so we do not spam debug on every line.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tap_line logs on write failure (once) and disables further tap writes
<!-- AC:END -->
