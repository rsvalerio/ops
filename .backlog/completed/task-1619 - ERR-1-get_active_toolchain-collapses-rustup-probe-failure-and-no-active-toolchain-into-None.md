---
id: TASK-1619
title: >-
  ERR-1: get_active_toolchain collapses rustup probe failure and 'no active
  toolchain' into None
status: Done
assignee:
  - TASK-1638
created_date: '2026-05-22 06:55'
updated_date: '2026-05-22 13:23'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe/rustup.rs:8-24`, used by `extensions-rust/tools/src/install.rs:159-163`

**What**: `get_active_toolchain` returns `Option<String>` and folds three distinct outcomes onto `None`:

1. `rustup show active-toolchain` timed out / failed to spawn / exited non-zero (the `ProbeOutcome::Failed` branch).
2. The first `output.status.success()` check failed (rustup returned a non-success exit).
3. `parse_active_toolchain` could not extract an identifier (rustup ≥1.28 'no active toolchain configured' output).

`install_tool` then reports all three as `"could not determine active toolchain"`, which is the same operator-facing message regardless of cause.

**Why it matters**: This is the same defect class that TASK-1200 fixed for `ToolStatus`. There, collapsing "probe failed" into "not installed" caused `tools_cmd::run_install` to reinstall a perfectly working toolchain whenever rustup was wedged. Here, the same collapsing means:

- A transient rustup IO/timeout failure is indistinguishable from a deliberate `rustup default none` (no active toolchain).
- Operators debugging an install failure get one generic message ("could not determine active toolchain") with no hint that the underlying rustup probe blew up — the `run_probe_with_timeout_inner` warn line is the only breadcrumb.
- `install_tool` cannot make a policy decision (e.g. retry vs. fail-fast vs. ask the user to run `rustup default`).

`parse_active_toolchain` already grew explicit guards for diagnostic-prefixed lines (TASK-1197) and the "no" sentinel (TASK-1566) — the next step is propagating the "did the probe answer at all?" distinction up through the caller, analogous to `ProbeOutcome`.

**Suggested shape**: change `get_active_toolchain -> ProbeOutcome<Option<String>>` (or a typed `enum ActiveToolchain { Resolved(String), None, ProbeFailed }`), update `install_tool` to surface the probe-failure case with a distinct error message that points operators at rustup, and let the "no active toolchain" case keep its current message.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 get_active_toolchain (or a typed wrapper) distinguishes probe-failed from no-active-toolchain from resolved
- [x] #2 install_tool surfaces a probe-failure-specific error message, distinct from the existing 'could not determine active toolchain'
- [x] #3 Unit tests cover all three branches without spawning rustup
<!-- AC:END -->
